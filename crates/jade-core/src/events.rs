use chrono::{DateTime, Utc};
use rusqlite::params;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::{
    ListTaskEventsInput, ListTaskEventsSinceInput, Task, TaskEvent, TaskEventType,
    EVENT_ORIGIN_LOCAL,
};

const DEFAULT_LIMIT: u32 = 50;
const DEFAULT_SINCE_LIMIT: u32 = 500;

/// List task events newest-first, optionally filtered by task.
pub fn list_task_events(db: &Db, input: ListTaskEventsInput) -> Result<Vec<TaskEvent>> {
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT);
    let conn = db.connection();

    let mut events = Vec::new();
    if let Some(task_id) = input.task_id {
        let mut stmt = conn.prepare(
            "
            SELECT seq, id, task_id, event_type, payload, origin, created_at
            FROM task_events
            WHERE task_id = ?1
            ORDER BY seq DESC
            LIMIT ?2
            ",
        )?;
        let rows = stmt.query_map(params![task_id.to_string(), limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (seq, id, task_id, event_type, payload, origin, created_at) = row?;
            events.push(task_event_from_parts(
                seq, id, task_id, event_type, payload, origin, created_at,
            )?);
        }
    } else {
        let mut stmt = conn.prepare(
            "
            SELECT seq, id, task_id, event_type, payload, origin, created_at
            FROM task_events
            ORDER BY seq DESC
            LIMIT ?1
            ",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (seq, id, task_id, event_type, payload, origin, created_at) = row?;
            events.push(task_event_from_parts(
                seq, id, task_id, event_type, payload, origin, created_at,
            )?);
        }
    }

    Ok(events)
}

/// List events with `seq > after_seq`, oldest first (for live sync / replication cursors).
pub fn list_task_events_since(db: &Db, input: ListTaskEventsSinceInput) -> Result<Vec<TaskEvent>> {
    let limit = input.limit.unwrap_or(DEFAULT_SINCE_LIMIT);
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT seq, id, task_id, event_type, payload, origin, created_at
        FROM task_events
        WHERE seq > ?1
        ORDER BY seq ASC
        LIMIT ?2
        ",
    )?;
    let rows = stmt.query_map(params![input.after_seq, limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;

    let mut events = Vec::new();
    for row in rows {
        let (seq, id, task_id, event_type, payload, origin, created_at) = row?;
        events.push(task_event_from_parts(
            seq, id, task_id, event_type, payload, origin, created_at,
        )?);
    }
    Ok(events)
}

/// Highest event `seq`, or `0` when the log is empty.
pub fn latest_event_seq(db: &Db) -> Result<i64> {
    let conn = db.connection();
    let seq: Option<i64> =
        conn.query_row("SELECT MAX(seq) FROM task_events", [], |row| row.get(0))?;
    Ok(seq.unwrap_or(0))
}

fn task_event_from_parts(
    seq: i64,
    id: String,
    task_id: String,
    event_type: String,
    payload: String,
    origin: String,
    created_at: String,
) -> Result<TaskEvent> {
    Ok(TaskEvent {
        seq,
        id: Uuid::parse_str(&id).map_err(|e| Error::Message(format!("invalid event id: {e}")))?,
        task_id: Uuid::parse_str(&task_id)
            .map_err(|e| Error::Message(format!("invalid task id: {e}")))?,
        event_type: TaskEventType::parse(&event_type)?,
        payload: serde_json::from_str(&payload)?,
        origin,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::InvalidDueAt(format!("{created_at}: {e}")))?,
    })
}

/// Insert a task event inside an open transaction.
pub fn insert_event(
    tx: &rusqlite::Transaction<'_>,
    task_id: Uuid,
    event_type: TaskEventType,
    payload: Value,
    now: DateTime<Utc>,
) -> Result<()> {
    insert_event_with_origin(tx, task_id, event_type, payload, EVENT_ORIGIN_LOCAL, now)
}

/// Insert a task event with an explicit origin (for future peer/agent writers).
pub fn insert_event_with_origin(
    tx: &rusqlite::Transaction<'_>,
    task_id: Uuid,
    event_type: TaskEventType,
    payload: Value,
    origin: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = Uuid::new_v4();
    tx.execute(
        "
        INSERT INTO task_events (id, task_id, event_type, payload, origin, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ",
        params![
            id.to_string(),
            task_id.to_string(),
            event_type.as_str(),
            payload.to_string(),
            origin,
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Full task snapshot for sync / UI apply (serde shape of [`Task`]).
pub fn task_snapshot(task: &Task) -> Value {
    serde_json::to_value(task).unwrap_or(Value::Null)
}

/// Snapshot payload for a `created` event.
pub fn created_payload(task: &Task, spawned_from: Option<Uuid>) -> Value {
    let tag_names: Vec<&str> = task.tags.iter().map(|t| t.name.as_str()).collect();
    let mut map = Map::new();
    map.insert("title".into(), json!(task.title));
    map.insert("description".into(), json!(task.description));
    map.insert("status".into(), json!(task.status.as_str()));
    map.insert("due_at".into(), json!(task.due_at.to_rfc3339()));
    map.insert("repeat_cron".into(), json!(task.repeat_cron));
    map.insert("tags".into(), json!(tag_names));
    if let Some(from) = spawned_from {
        map.insert("spawned_from".into(), json!(from.to_string()));
    }
    map.insert("task".into(), task_snapshot(task));
    Value::Object(map)
}

/// Build an `updated` payload from field diffs + after-state. Returns `None` if empty.
pub fn updated_payload(changes: Map<String, Value>, after: &Task) -> Option<Value> {
    if changes.is_empty() {
        None
    } else {
        Some(json!({
            "changes": Value::Object(changes),
            "task": task_snapshot(after),
        }))
    }
}

/// Tombstone payload for a `deleted` event.
pub fn deleted_payload(task: &Task) -> Value {
    json!({ "task": task_snapshot(task) })
}

pub fn field_change(old: Value, new: Value) -> Value {
    json!({ "old": old, "new": new })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;
    use crate::models::{
        CreateTaskInput, DueUpdate, RepeatCronUpdate, TaskStatus, UpdateTaskInput,
        UpdateTaskStatusInput,
    };
    use crate::tasks::{create_task, delete_task, update_task, update_task_status};
    use chrono::TimeZone;

    fn sample_input(title: &str, due: DateTime<Utc>, tags: &[&str]) -> CreateTaskInput {
        CreateTaskInput {
            title: title.to_owned(),
            description: None,
            due_at: due,
            tag_names: tags.iter().map(|s| (*s).to_owned()).collect(),
            repeat_cron: None,
        }
    }

    #[test]
    fn create_writes_created_event() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Logged", due, &["work"])).unwrap();

        let events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(task.id),
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, TaskEventType::Created);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[0].origin, EVENT_ORIGIN_LOCAL);
        assert_eq!(events[0].payload["title"], "Logged");
        assert_eq!(events[0].payload["status"], "inactive");
        assert_eq!(events[0].payload["tags"], json!(["work"]));
        assert!(events[0].payload.get("spawned_from").is_none());
        assert_eq!(events[0].payload["task"]["id"], task.id.to_string());
        assert_eq!(events[0].payload["task"]["title"], "Logged");
    }

    #[test]
    fn status_change_writes_updated_event() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Move", due, &[])).unwrap();

        update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Active,
            },
        )
        .unwrap();

        let events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(task.id),
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, TaskEventType::Updated);
        assert_eq!(events[0].payload["changes"]["status"]["old"], "inactive");
        assert_eq!(events[0].payload["changes"]["status"]["new"], "active");
        assert_eq!(events[0].payload["task"]["status"], "active");
    }

    #[test]
    fn noop_status_writes_nothing() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Stay", due, &[])).unwrap();

        update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Inactive,
            },
        )
        .unwrap();

        let events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(task.id),
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, TaskEventType::Created);
    }

    #[test]
    fn multi_field_update_single_event() {
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
        update_task(
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

        let events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(task.id),
                limit: Some(1),
            },
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, TaskEventType::Updated);
        let p = &events[0].payload["changes"];
        assert_eq!(p["title"]["old"], "Old");
        assert_eq!(p["title"]["new"], "New");
        assert_eq!(p["description"]["old"], "desc");
        assert_eq!(p["description"]["new"], Value::Null);
        assert_eq!(p["status"]["old"], "inactive");
        assert_eq!(p["status"]["new"], "active");
        assert_eq!(p["due_at"]["new"], custom.to_rfc3339());
        assert_eq!(p["repeat_cron"]["old"], Value::Null);
        assert_eq!(p["repeat_cron"]["new"], "0 9 * * *");
        assert!(p.get("tags").is_none());
        assert_eq!(events[0].payload["task"]["title"], "New");
    }

    #[test]
    fn delete_writes_deleted_event() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(&db, sample_input("Gone", due, &[])).unwrap();
        delete_task(&db, task.id).unwrap();

        let events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(task.id),
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(events[0].event_type, TaskEventType::Deleted);
        assert_eq!(events[0].payload["task"]["id"], task.id.to_string());
        assert_eq!(events[0].payload["task"]["title"], "Gone");
    }

    #[test]
    fn recurring_complete_logs_update_and_spawn_create() {
        let db = open_memory().unwrap();
        let due = Utc::now() - chrono::Duration::days(3);
        let task = create_task(
            &db,
            CreateTaskInput {
                title: "Daily".into(),
                description: None,
                due_at: due,
                tag_names: vec!["chore".into()],
                repeat_cron: Some("0 9 * * *".into()),
            },
        )
        .unwrap();

        let result = update_task_status(
            &db,
            UpdateTaskStatusInput {
                id: task.id,
                status: TaskStatus::Complete,
            },
        )
        .unwrap();
        let spawned = result.spawned.expect("spawned");

        let original_events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(task.id),
                limit: Some(1),
            },
        )
        .unwrap();
        assert_eq!(original_events[0].event_type, TaskEventType::Updated);
        assert_eq!(
            original_events[0].payload["changes"]["status"]["new"],
            "complete"
        );
        assert_eq!(
            original_events[0].payload["changes"]["repeat_cron"]["old"],
            "0 9 * * *"
        );
        assert_eq!(
            original_events[0].payload["changes"]["repeat_cron"]["new"],
            Value::Null
        );

        let spawn_events = list_task_events(
            &db,
            ListTaskEventsInput {
                task_id: Some(spawned.id),
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(spawn_events.len(), 1);
        assert_eq!(spawn_events[0].event_type, TaskEventType::Created);
        assert_eq!(spawn_events[0].payload["spawned_from"], task.id.to_string());
        assert_eq!(spawn_events[0].payload["tags"], json!(["chore"]));
        assert_eq!(
            spawn_events[0].payload["task"]["id"],
            spawned.id.to_string()
        );
    }

    #[test]
    fn list_since_and_latest_seq() {
        let db = open_memory().unwrap();
        assert_eq!(latest_event_seq(&db).unwrap(), 0);

        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let a = create_task(&db, sample_input("A", due, &[])).unwrap();
        let b = create_task(&db, sample_input("B", due, &[])).unwrap();
        assert_eq!(latest_event_seq(&db).unwrap(), 2);

        let since = list_task_events_since(
            &db,
            ListTaskEventsSinceInput {
                after_seq: 0,
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(since.len(), 2);
        assert_eq!(since[0].task_id, a.id);
        assert_eq!(since[1].task_id, b.id);

        let after_one = list_task_events_since(
            &db,
            ListTaskEventsSinceInput {
                after_seq: 1,
                limit: None,
            },
        )
        .unwrap();
        assert_eq!(after_one.len(), 1);
        assert_eq!(after_one[0].task_id, b.id);
    }
}
