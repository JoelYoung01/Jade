use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::cron::{next_occurrence, normalize_cron};
use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::{
    created_payload, deleted_payload, field_change, insert_event, updated_payload,
};
use crate::models::{
    CreateTaskInput, DueUpdate, RepeatCronUpdate, RescheduleMode, StatusUpdateResult, Tag, Task,
    TaskEventType, TaskStatus, UpdateTaskInput, UpdateTaskStatusInput,
};
use crate::time_helpers::{first_monday_next_month, next_monday, push_to_today, push_to_tomorrow};

pub fn list_tasks(db: &Db) -> Result<Vec<Task>> {
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT id, title, description, status, due_at, repeat_cron, created_at, updated_at, deleted_at
        FROM tasks
        WHERE deleted_at IS NULL
        ORDER BY due_at ASC
        ",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(TaskRow {
            id: row.get::<_, String>(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: row.get::<_, String>(3)?,
            due_at: row.get::<_, String>(4)?,
            repeat_cron: row.get::<_, Option<String>>(5)?,
            created_at: row.get::<_, String>(6)?,
            updated_at: row.get::<_, String>(7)?,
            deleted_at: row.get::<_, Option<String>>(8)?,
        })
    })?;

    let mut tasks = Vec::new();
    for row in rows {
        let row = row?;
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| Error::Message(format!("invalid task id: {e}")))?;
        let tags = load_tags_for_task(&conn, id)?;
        tasks.push(task_from_row(row, tags)?);
    }

    Ok(tasks)
}

pub fn create_task(db: &Db, input: CreateTaskInput) -> Result<Task> {
    let title = input.title.trim().to_owned();
    if title.is_empty() {
        return Err(Error::EmptyTitle);
    }

    let repeat_cron = normalize_cron(input.repeat_cron.as_deref())?;

    let now = Utc::now();
    let id = Uuid::new_v4();
    let description = input
        .description
        .map(|d| d.trim().to_owned())
        .filter(|d| !d.is_empty());

    {
        let conn = db.connection();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "
            INSERT INTO tasks (id, title, description, status, due_at, repeat_cron, created_at, updated_at, deleted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
            ",
            params![
                id.to_string(),
                title,
                description,
                TaskStatus::Inactive.as_str(),
                input.due_at.to_rfc3339(),
                repeat_cron,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;

        let mut tags = Vec::new();
        for name in &input.tag_names {
            let tag = ensure_tag_in_tx(&tx, name)?;
            tx.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                params![id.to_string(), tag.id.to_string()],
            )?;
            tags.push(tag);
        }
        tags.sort_by_key(|a| a.name.to_lowercase());

        let snapshot = Task {
            id,
            title,
            description,
            status: TaskStatus::Inactive,
            due_at: input.due_at,
            repeat_cron,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            tags,
        };
        insert_event(
            &tx,
            id,
            TaskEventType::Created,
            created_payload(&snapshot, None),
            now,
        )?;

        tx.commit()?;
    }

    get_task(db, id)
}

pub fn update_task_status(db: &Db, input: UpdateTaskStatusInput) -> Result<StatusUpdateResult> {
    let current = get_task(db, input.id)?;
    let now = Utc::now();

    let spawned = if input.status == TaskStatus::Complete {
        spawn_next_if_recurring(db, &current, now)?
    } else {
        None
    };

    let clear_cron = spawned.is_some();
    let status_changed = current.status != input.status;
    let cron_changed = clear_cron && current.repeat_cron.is_some();

    {
        let conn = db.connection();
        let tx = conn.unchecked_transaction()?;
        let updated = if clear_cron {
            // Completing a recurring task: clear cron so history can't re-spawn.
            tx.execute(
                "
                UPDATE tasks
                SET status = ?1, repeat_cron = NULL, updated_at = ?2
                WHERE id = ?3 AND deleted_at IS NULL
                ",
                params![
                    input.status.as_str(),
                    now.to_rfc3339(),
                    input.id.to_string()
                ],
            )?
        } else {
            tx.execute(
                "
                UPDATE tasks
                SET status = ?1, updated_at = ?2
                WHERE id = ?3 AND deleted_at IS NULL
                ",
                params![
                    input.status.as_str(),
                    now.to_rfc3339(),
                    input.id.to_string()
                ],
            )?
        };
        if updated == 0 {
            return Err(Error::TaskNotFound(input.id.to_string()));
        }

        if status_changed || cron_changed {
            let mut changes = Map::new();
            if status_changed {
                changes.insert(
                    "status".into(),
                    field_change(json!(current.status.as_str()), json!(input.status.as_str())),
                );
            }
            if cron_changed {
                changes.insert(
                    "repeat_cron".into(),
                    field_change(json!(current.repeat_cron), Value::Null),
                );
            }
            let after = Task {
                status: input.status,
                repeat_cron: if clear_cron {
                    None
                } else {
                    current.repeat_cron.clone()
                },
                updated_at: now,
                ..current
            };
            if let Some(payload) = updated_payload(changes, &after) {
                insert_event(&tx, input.id, TaskEventType::Updated, payload, now)?;
            }
        }

        tx.commit()?;
    }

    Ok(StatusUpdateResult {
        task: get_task(db, input.id)?,
        spawned,
    })
}

pub fn reschedule_task(
    db: &Db,
    id: Uuid,
    mode: RescheduleMode,
    custom_due_at: Option<DateTime<Utc>>,
) -> Result<Task> {
    let current = get_task(db, id)?;
    let new_due = match mode {
        RescheduleMode::Today => push_to_today(current.due_at),
        RescheduleMode::Tomorrow => push_to_tomorrow(current.due_at),
        RescheduleMode::NextMonday => next_monday(current.due_at),
        RescheduleMode::FirstMondayNextMonth => first_monday_next_month(current.due_at),
        RescheduleMode::Custom => custom_due_at
            .ok_or_else(|| Error::InvalidDueAt("custom reschedule requires due_at".into()))?,
    };

    let now = Utc::now();
    {
        let conn = db.connection();
        let tx = conn.unchecked_transaction()?;
        let updated = tx.execute(
            "
            UPDATE tasks
            SET due_at = ?1, updated_at = ?2
            WHERE id = ?3 AND deleted_at IS NULL
            ",
            params![new_due.to_rfc3339(), now.to_rfc3339(), id.to_string()],
        )?;
        if updated == 0 {
            return Err(Error::TaskNotFound(id.to_string()));
        }

        if current.due_at != new_due {
            let mut changes = Map::new();
            changes.insert(
                "due_at".into(),
                field_change(
                    json!(current.due_at.to_rfc3339()),
                    json!(new_due.to_rfc3339()),
                ),
            );
            let after = Task {
                due_at: new_due,
                updated_at: now,
                ..current
            };
            if let Some(payload) = updated_payload(changes, &after) {
                insert_event(&tx, id, TaskEventType::Updated, payload, now)?;
            }
        }

        tx.commit()?;
    }
    get_task(db, id)
}

pub fn delete_task(db: &Db, id: Uuid) -> Result<()> {
    let current = get_task(db, id)?;
    let now = Utc::now();
    let mut tombstone = current;
    tombstone.deleted_at = Some(now);
    tombstone.updated_at = now;

    let conn = db.connection();
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "
        UPDATE tasks
        SET deleted_at = ?1, updated_at = ?1
        WHERE id = ?2 AND deleted_at IS NULL
        ",
        params![now.to_rfc3339(), id.to_string()],
    )?;
    if updated == 0 {
        return Err(Error::TaskNotFound(id.to_string()));
    }
    insert_event(
        &tx,
        id,
        TaskEventType::Deleted,
        deleted_payload(&tombstone),
        now,
    )?;
    tx.commit()?;
    Ok(())
}

pub fn get_task(db: &Db, id: Uuid) -> Result<Task> {
    list_tasks(db)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| Error::TaskNotFound(id.to_string()))
}

/// Apply a partial update in a single transaction. At least one field must be set.
///
/// When status transitions to `complete` on a recurring task, the next occurrence
/// is spawned and the completed row's cron is cleared (same as `update_task_status`).
pub fn update_task(db: &Db, input: UpdateTaskInput) -> Result<StatusUpdateResult> {
    if input.title.is_none()
        && input.description.is_none()
        && input.status.is_none()
        && input.due.is_none()
        && input.tag_names.is_none()
        && input.repeat_cron.is_none()
    {
        return Err(Error::NoUpdateFields);
    }

    let current = get_task(db, input.id)?;
    let now = Utc::now();
    let resolved = resolve_update_fields(&current, &input)?;

    // Spawn before writing complete so we still have the current cron/tags.
    let becoming_complete =
        resolved.status == TaskStatus::Complete && current.status != TaskStatus::Complete;
    let spawned = if becoming_complete {
        maybe_spawn_from_update(db, &current, &input, &resolved, now)?
    } else {
        None
    };

    // Completed recurring history never keeps a live cron.
    let stored_cron = if spawned.is_some() {
        None
    } else {
        resolved.repeat_cron.clone()
    };

    write_task_update(
        db,
        &current,
        &resolved,
        stored_cron,
        input.tag_names.as_ref(),
        now,
    )?;

    Ok(StatusUpdateResult {
        task: get_task(db, input.id)?,
        spawned,
    })
}

struct ResolvedUpdate {
    title: String,
    description: Option<String>,
    status: TaskStatus,
    due_at: DateTime<Utc>,
    repeat_cron: Option<String>,
}

fn resolve_update_fields(current: &Task, input: &UpdateTaskInput) -> Result<ResolvedUpdate> {
    let title = if let Some(title) = &input.title {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err(Error::EmptyTitle);
        }
        title
    } else {
        current.title.clone()
    };

    let description = input.description.as_ref().map_or_else(
        || current.description.clone(),
        |description| {
            let trimmed = description.trim().to_owned();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        },
    );

    let status = input.status.unwrap_or(current.status);

    let due_at = match input.due {
        None => current.due_at,
        Some(DueUpdate::Tomorrow) => push_to_tomorrow(current.due_at),
        Some(DueUpdate::NextMonday) => next_monday(current.due_at),
        Some(DueUpdate::At(dt)) => dt,
    };

    let repeat_cron = match &input.repeat_cron {
        None => current.repeat_cron.clone(),
        Some(RepeatCronUpdate::Clear) => None,
        Some(RepeatCronUpdate::Set(expr)) => normalize_cron(Some(expr))?,
    };

    Ok(ResolvedUpdate {
        title,
        description,
        status,
        due_at,
        repeat_cron,
    })
}

fn maybe_spawn_from_update(
    db: &Db,
    current: &Task,
    input: &UpdateTaskInput,
    resolved: &ResolvedUpdate,
    now: DateTime<Utc>,
) -> Result<Option<Task>> {
    let cron_for_spawn = match &input.repeat_cron {
        Some(RepeatCronUpdate::Clear) => None,
        Some(RepeatCronUpdate::Set(expr)) => Some(expr.as_str()),
        None => current.repeat_cron.as_deref(),
    };
    let Some(cron) = cron_for_spawn else {
        return Ok(None);
    };

    let for_spawn = Task {
        id: current.id,
        title: resolved.title.clone(),
        description: resolved.description.clone(),
        status: current.status,
        due_at: current.due_at,
        repeat_cron: Some(cron.to_owned()),
        created_at: current.created_at,
        updated_at: current.updated_at,
        deleted_at: current.deleted_at,
        tags: current.tags.clone(),
    };
    spawn_next_if_recurring(db, &for_spawn, now)
}

fn write_task_update(
    db: &Db,
    current: &Task,
    resolved: &ResolvedUpdate,
    stored_cron: Option<String>,
    tag_names: Option<&Vec<String>>,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = current.id;
    let conn = db.connection();
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "
        UPDATE tasks
        SET title = ?1, description = ?2, status = ?3, due_at = ?4,
            repeat_cron = ?5, updated_at = ?6
        WHERE id = ?7 AND deleted_at IS NULL
        ",
        params![
            resolved.title,
            resolved.description,
            resolved.status.as_str(),
            resolved.due_at.to_rfc3339(),
            stored_cron,
            now.to_rfc3339(),
            id.to_string(),
        ],
    )?;
    if updated == 0 {
        return Err(Error::TaskNotFound(id.to_string()));
    }

    let mut after_tags = current.tags.clone();
    let mut new_tag_names: Option<Vec<String>> = None;
    if let Some(tag_names) = tag_names {
        tx.execute(
            "DELETE FROM task_tags WHERE task_id = ?1",
            params![id.to_string()],
        )?;
        let mut names = Vec::new();
        let mut tags = Vec::new();
        for name in tag_names {
            let tag = ensure_tag_in_tx(&tx, name)?;
            tx.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                params![id.to_string(), tag.id.to_string()],
            )?;
            names.push(tag.name.clone());
            tags.push(tag);
        }
        names.sort_by_key(|a| a.to_lowercase());
        tags.sort_by_key(|a| a.name.to_lowercase());
        new_tag_names = Some(names);
        after_tags = tags;
    }

    let after = Task {
        id,
        title: resolved.title.clone(),
        description: resolved.description.clone(),
        status: resolved.status,
        due_at: resolved.due_at,
        repeat_cron: stored_cron.clone(),
        created_at: current.created_at,
        updated_at: now,
        deleted_at: None,
        tags: after_tags,
    };

    if let Some(payload) = diff_task_update(
        current,
        resolved,
        stored_cron.as_deref(),
        new_tag_names.as_deref(),
        &after,
    ) {
        insert_event(&tx, id, TaskEventType::Updated, payload, now)?;
    }

    tx.commit()?;
    Ok(())
}

fn diff_task_update(
    current: &Task,
    resolved: &ResolvedUpdate,
    stored_cron: Option<&str>,
    new_tag_names: Option<&[String]>,
    after: &Task,
) -> Option<Value> {
    let mut changes = Map::new();

    if current.title != resolved.title {
        changes.insert(
            "title".into(),
            field_change(json!(current.title), json!(resolved.title)),
        );
    }
    if current.description != resolved.description {
        changes.insert(
            "description".into(),
            field_change(json!(current.description), json!(resolved.description)),
        );
    }
    if current.status != resolved.status {
        changes.insert(
            "status".into(),
            field_change(
                json!(current.status.as_str()),
                json!(resolved.status.as_str()),
            ),
        );
    }
    if current.due_at != resolved.due_at {
        changes.insert(
            "due_at".into(),
            field_change(
                json!(current.due_at.to_rfc3339()),
                json!(resolved.due_at.to_rfc3339()),
            ),
        );
    }
    if current.repeat_cron.as_deref() != stored_cron {
        changes.insert(
            "repeat_cron".into(),
            field_change(json!(current.repeat_cron), json!(stored_cron)),
        );
    }
    if let Some(new_tags) = new_tag_names {
        let old_tags: Vec<String> = current.tags.iter().map(|t| t.name.clone()).collect();
        if old_tags != new_tags {
            changes.insert(
                "tags".into(),
                field_change(json!(old_tags), json!(new_tags)),
            );
        }
    }

    updated_payload(changes, after)
}

/// If `task` has a repeat cron, insert the next occurrence (inactive) with the
/// same title/description/tags/cron. Does not mutate the original row.
///
/// Next due = first cron match strictly after `max(due_at, now)` in local time,
/// so overdue completions skip the backlog (Apple Reminders behavior).
fn spawn_next_if_recurring(db: &Db, task: &Task, now: DateTime<Utc>) -> Result<Option<Task>> {
    let Some(cron) = task.repeat_cron.as_deref() else {
        return Ok(None);
    };

    let after = if task.due_at > now { task.due_at } else { now };
    let next_due = next_occurrence(cron, after)?;
    let new_id = Uuid::new_v4();

    {
        let conn = db.connection();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "
            INSERT INTO tasks (id, title, description, status, due_at, repeat_cron, created_at, updated_at, deleted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
            ",
            params![
                new_id.to_string(),
                task.title,
                task.description,
                TaskStatus::Inactive.as_str(),
                next_due.to_rfc3339(),
                cron,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;

        let mut tags = task.tags.clone();
        for tag in &tags {
            tx.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                params![new_id.to_string(), tag.id.to_string()],
            )?;
        }
        tags.sort_by_key(|a| a.name.to_lowercase());

        let snapshot = Task {
            id: new_id,
            title: task.title.clone(),
            description: task.description.clone(),
            status: TaskStatus::Inactive,
            due_at: next_due,
            repeat_cron: Some(cron.to_owned()),
            created_at: now,
            updated_at: now,
            deleted_at: None,
            tags,
        };
        insert_event(
            &tx,
            new_id,
            TaskEventType::Created,
            created_payload(&snapshot, Some(task.id)),
            now,
        )?;

        tx.commit()?;
    }

    Ok(Some(get_task(db, new_id)?))
}

struct TaskRow {
    id: String,
    title: String,
    description: Option<String>,
    status: String,
    due_at: String,
    repeat_cron: Option<String>,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

fn task_from_row(row: TaskRow, tags: Vec<Tag>) -> Result<Task> {
    let id =
        Uuid::parse_str(&row.id).map_err(|e| Error::Message(format!("invalid task id: {e}")))?;
    Ok(Task {
        id,
        title: row.title,
        description: row.description,
        status: TaskStatus::parse(&row.status)?,
        due_at: parse_dt(&row.due_at)?,
        repeat_cron: row.repeat_cron,
        created_at: parse_dt(&row.created_at)?,
        updated_at: parse_dt(&row.updated_at)?,
        deleted_at: row.deleted_at.as_deref().map(parse_dt).transpose()?,
        tags,
    })
}

fn parse_dt(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::InvalidDueAt(format!("{value}: {e}")))
}

fn load_tags_for_task(conn: &rusqlite::Connection, task_id: Uuid) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "
        SELECT t.id, t.name, t.created_at, t.updated_at
        FROM tags t
        INNER JOIN task_tags tt ON tt.tag_id = t.id
        WHERE tt.task_id = ?1
        ORDER BY t.name COLLATE NOCASE ASC
        ",
    )?;

    let rows = stmt.query_map(params![task_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut tags = Vec::new();
    for row in rows {
        let (id, name, created_at, updated_at) = row?;
        tags.push(Tag {
            id: Uuid::parse_str(&id).map_err(|e| Error::Message(format!("invalid tag id: {e}")))?,
            name,
            created_at: parse_dt(&created_at)?,
            updated_at: parse_dt(&updated_at)?,
        });
    }
    Ok(tags)
}

fn ensure_tag_in_tx(tx: &rusqlite::Transaction<'_>, name: &str) -> Result<Tag> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Message("tag name is required".into()));
    }

    let existing: Option<(String, String, String, String)> = tx
        .query_row(
            "
            SELECT id, name, created_at, updated_at
            FROM tags
            WHERE name = ?1 COLLATE NOCASE
            ",
            params![name],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;

    if let Some((id, existing_name, created_at, updated_at)) = existing {
        return Ok(Tag {
            id: Uuid::parse_str(&id).map_err(|e| Error::Message(format!("invalid tag id: {e}")))?,
            name: existing_name,
            created_at: parse_dt(&created_at)?,
            updated_at: parse_dt(&updated_at)?,
        });
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    tx.execute(
        "
        INSERT INTO tags (id, name, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        ",
        params![id.to_string(), name, now.to_rfc3339(), now.to_rfc3339()],
    )?;

    Ok(Tag {
        id,
        name: name.to_owned(),
        created_at: now,
        updated_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;
    use chrono::{Datelike, TimeZone, Timelike};

    fn sample_input(title: &str, due: DateTime<Utc>, tags: &[&str]) -> CreateTaskInput {
        CreateTaskInput {
            title: title.to_owned(),
            description: None,
            due_at: due,
            tag_names: tags.iter().map(|s| (*s).to_owned()).collect(),
            repeat_cron: None,
        }
    }

    fn recurring_input(
        title: &str,
        due: DateTime<Utc>,
        cron: &str,
        tags: &[&str],
    ) -> CreateTaskInput {
        CreateTaskInput {
            title: title.to_owned(),
            description: Some("recurring desc".into()),
            due_at: due,
            tag_names: tags.iter().map(|s| (*s).to_owned()).collect(),
            repeat_cron: Some(cron.to_owned()),
        }
    }

    #[test]
    fn create_lands_in_inactive_and_lists_by_due() {
        let db = open_memory().unwrap();
        let early = Utc.with_ymd_and_hms(2026, 7, 21, 10, 0, 0).unwrap();
        let late = Utc.with_ymd_and_hms(2026, 7, 22, 10, 0, 0).unwrap();

        let later = create_task(&db, sample_input("Later", late, &["work"])).unwrap();
        let sooner = create_task(&db, sample_input("Sooner", early, &[])).unwrap();

        assert_eq!(later.status, TaskStatus::Inactive);
        assert_eq!(sooner.status, TaskStatus::Inactive);
        assert!(later.repeat_cron.is_none());

        let tasks = list_tasks(&db).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Sooner");
        assert_eq!(tasks[1].title, "Later");
        assert_eq!(tasks[1].tags.len(), 1);
        assert_eq!(tasks[1].tags[0].name, "work");
    }

    #[test]
    fn status_transition_and_reschedule() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Move me", due, &[])).unwrap();

        let active = update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Active,
            },
        )
        .unwrap();
        assert_eq!(active.task.status, TaskStatus::Active);
        assert!(active.spawned.is_none());

        let tomorrow = reschedule_task(&db, task.id, RescheduleMode::Tomorrow, None).unwrap();
        assert!(tomorrow.due_at > due);

        let monday = reschedule_task(&db, task.id, RescheduleMode::NextMonday, None).unwrap();
        assert_eq!(
            monday.due_at.with_timezone(&chrono::Local).weekday(),
            chrono::Weekday::Mon
        );

        let custom = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap();
        let customed = reschedule_task(&db, task.id, RescheduleMode::Custom, Some(custom)).unwrap();
        assert_eq!(customed.due_at.hour(), custom.hour());
        assert_eq!(customed.due_at.date_naive(), custom.date_naive());
    }

    #[test]
    fn empty_title_rejected() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let err = create_task(&db, sample_input("   ", due, &[])).unwrap_err();
        assert!(matches!(err, Error::EmptyTitle));
    }

    #[test]
    fn invalid_cron_rejected_on_create() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let err = create_task(
            &db,
            CreateTaskInput {
                title: "Bad cron".into(),
                description: None,
                due_at: due,
                tag_names: vec![],
                repeat_cron: Some("not-a-cron".into()),
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidCron(_)));
    }

    #[test]
    fn soft_delete_hides_from_list() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Gone", due, &[])).unwrap();
        delete_task(&db, task.id).unwrap();
        assert!(list_tasks(&db).unwrap().is_empty());
        let err = delete_task(&db, task.id).unwrap_err();
        assert!(matches!(err, Error::TaskNotFound(_)));
    }

    #[test]
    fn partial_update_multiple_fields() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(
            &db,
            CreateTaskInput {
                title: "Old".into(),
                description: Some("desc".into()),
                due_at: due,
                tag_names: vec![],
                repeat_cron: None,
            },
        )
        .unwrap();

        let custom = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).unwrap();
        let updated = update_task(
            &db,
            UpdateTaskInput {
                id: task.id,
                title: Some("New".into()),
                description: Some(String::new()),
                status: Some(TaskStatus::Active),
                due: Some(DueUpdate::At(custom)),
                tag_names: None,
                repeat_cron: Some(RepeatCronUpdate::Set("0 9 * * *".into())),
            },
        )
        .unwrap();

        assert_eq!(updated.task.title, "New");
        assert_eq!(updated.task.description, None);
        assert_eq!(updated.task.status, TaskStatus::Active);
        assert_eq!(updated.task.due_at, custom);
        assert_eq!(updated.task.repeat_cron.as_deref(), Some("0 9 * * *"));
        assert!(updated.spawned.is_none());
    }

    #[test]
    fn partial_update_replaces_tags() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Tagged", due, &["old", "keep"])).unwrap();

        let updated = update_task(
            &db,
            UpdateTaskInput {
                id: task.id,
                title: None,
                description: None,
                status: None,
                due: None,
                tag_names: Some(vec!["keep".into(), "new".into()]),
                repeat_cron: None,
            },
        )
        .unwrap();

        let names: Vec<_> = updated.task.tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["keep", "new"]);
    }

    #[test]
    fn partial_update_requires_fields() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Keep", due, &[])).unwrap();
        let err = update_task(
            &db,
            UpdateTaskInput {
                id: task.id,
                title: None,
                description: None,
                status: None,
                due: None,
                tag_names: None,
                repeat_cron: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoUpdateFields));
    }

    #[test]
    fn partial_update_missing_task() {
        let db = open_memory().unwrap();
        let err = update_task(
            &db,
            UpdateTaskInput {
                id: Uuid::new_v4(),
                title: Some("Nope".into()),
                description: None,
                status: None,
                due: None,
                tag_names: None,
                repeat_cron: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound(_)));
    }

    #[test]
    fn complete_recurring_spawns_next_and_clears_cron() {
        let db = open_memory().unwrap();
        // Due in the past so max(due_at, now) == now; next daily at 9am local.
        let due = Utc::now() - chrono::Duration::days(3);
        let task = create_task(
            &db,
            recurring_input("Daily chore", due, "0 9 * * *", &["home", "chore"]),
        )
        .unwrap();
        assert_eq!(task.repeat_cron.as_deref(), Some("0 9 * * *"));

        let result = update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Complete,
            },
        )
        .unwrap();

        assert_eq!(result.task.status, TaskStatus::Complete);
        assert!(result.task.repeat_cron.is_none(), "history must clear cron");

        let spawned = result.spawned.expect("should spawn next occurrence");
        assert_eq!(spawned.title, "Daily chore");
        assert_eq!(spawned.description.as_deref(), Some("recurring desc"));
        assert_eq!(spawned.status, TaskStatus::Inactive);
        assert_eq!(spawned.repeat_cron.as_deref(), Some("0 9 * * *"));
        assert!(spawned.due_at > Utc::now() - chrono::Duration::minutes(1));
        let names: Vec<_> = spawned.tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["chore", "home"]);

        // Completing history again must not spawn another copy.
        let again = update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Complete,
            },
        )
        .unwrap();
        assert!(again.spawned.is_none());

        let tasks = list_tasks(&db).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn complete_non_recurring_does_not_spawn() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Once", due, &[])).unwrap();
        let result = update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Complete,
            },
        )
        .unwrap();
        assert!(result.spawned.is_none());
        assert_eq!(list_tasks(&db).unwrap().len(), 1);
    }

    #[test]
    fn update_task_complete_also_spawns() {
        let db = open_memory().unwrap();
        let due = Utc::now() - chrono::Duration::hours(1);
        let task =
            create_task(&db, recurring_input("Via update", due, "0 9 * * 1-5", &[])).unwrap();

        let result = update_task(
            &db,
            UpdateTaskInput {
                id: task.id,
                title: None,
                description: None,
                status: Some(TaskStatus::Complete),
                due: None,
                tag_names: None,
                repeat_cron: None,
            },
        )
        .unwrap();

        assert!(result.task.repeat_cron.is_none());
        let spawned = result.spawned.expect("spawn via update_task");
        assert_eq!(spawned.repeat_cron.as_deref(), Some("0 9 * * 1-5"));
    }
}
