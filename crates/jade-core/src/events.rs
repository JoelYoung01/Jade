use chrono::{DateTime, Utc};
use rusqlite::params;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::{ListTaskEventsInput, TaskEvent, TaskEventType, TaskStatus};

const DEFAULT_LIMIT: u32 = 50;

/// List task events newest-first, optionally filtered by task.
pub fn list_task_events(db: &Db, input: ListTaskEventsInput) -> Result<Vec<TaskEvent>> {
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT);
    let conn = db.connection();

    let mut events = Vec::new();
    if let Some(task_id) = input.task_id {
        let mut stmt = conn.prepare(
            "
            SELECT id, task_id, event_type, payload, created_at
            FROM task_events
            WHERE task_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT ?2
            ",
        )?;
        let rows = stmt.query_map(params![task_id.to_string(), limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let (id, task_id, event_type, payload, created_at) = row?;
            events.push(task_event_from_parts(
                id, task_id, event_type, payload, created_at,
            )?);
        }
    } else {
        let mut stmt = conn.prepare(
            "
            SELECT id, task_id, event_type, payload, created_at
            FROM task_events
            ORDER BY created_at DESC, id DESC
            LIMIT ?1
            ",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let (id, task_id, event_type, payload, created_at) = row?;
            events.push(task_event_from_parts(
                id, task_id, event_type, payload, created_at,
            )?);
        }
    }

    Ok(events)
}

fn task_event_from_parts(
    id: String,
    task_id: String,
    event_type: String,
    payload: String,
    created_at: String,
) -> Result<TaskEvent> {
    Ok(TaskEvent {
        id: Uuid::parse_str(&id).map_err(|e| Error::Message(format!("invalid event id: {e}")))?,
        task_id: Uuid::parse_str(&task_id)
            .map_err(|e| Error::Message(format!("invalid task id: {e}")))?,
        event_type: TaskEventType::parse(&event_type)?,
        payload: serde_json::from_str(&payload)?,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::InvalidDueAt(format!("{created_at}: {e}")))?,
    })
}

/// Insert a task event inside an open transaction.
pub(crate) fn insert_event(
    tx: &rusqlite::Transaction<'_>,
    task_id: Uuid,
    event_type: TaskEventType,
    payload: Value,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = Uuid::new_v4();
    tx.execute(
        "
        INSERT INTO task_events (id, task_id, event_type, payload, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params![
            id.to_string(),
            task_id.to_string(),
            event_type.as_str(),
            payload.to_string(),
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Snapshot payload for a `created` event.
pub(crate) fn created_payload(
    title: &str,
    description: Option<&str>,
    status: TaskStatus,
    due_at: DateTime<Utc>,
    repeat_cron: Option<&str>,
    tags: &[String],
    spawned_from: Option<Uuid>,
) -> Value {
    let mut map = Map::new();
    map.insert("title".into(), json!(title));
    map.insert("description".into(), json!(description));
    map.insert("status".into(), json!(status.as_str()));
    map.insert("due_at".into(), json!(due_at.to_rfc3339()));
    map.insert("repeat_cron".into(), json!(repeat_cron));
    map.insert("tags".into(), json!(tags));
    if let Some(from) = spawned_from {
        map.insert("spawned_from".into(), json!(from.to_string()));
    }
    Value::Object(map)
}

/// Build an `updated` payload from old/new field pairs. Returns `None` if empty.
pub(crate) fn updated_payload(changes: Map<String, Value>) -> Option<Value> {
    if changes.is_empty() {
        None
    } else {
        Some(Value::Object(changes))
    }
}

pub(crate) fn field_change(old: Value, new: Value) -> Value {
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
        assert_eq!(events[0].payload["title"], "Logged");
        assert_eq!(events[0].payload["status"], "inactive");
        assert_eq!(events[0].payload["tags"], json!(["work"]));
        assert!(events[0].payload.get("spawned_from").is_none());
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
        assert_eq!(events[0].payload["status"]["old"], "inactive");
        assert_eq!(events[0].payload["status"]["new"], "active");
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
        let p = &events[0].payload;
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
        assert_eq!(events[0].payload, json!({}));
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
        assert_eq!(original_events[0].payload["status"]["new"], "complete");
        assert_eq!(original_events[0].payload["repeat_cron"]["old"], "0 9 * * *");
        assert_eq!(original_events[0].payload["repeat_cron"]["new"], Value::Null);

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
        assert_eq!(
            spawn_events[0].payload["spawned_from"],
            task.id.to_string()
        );
        assert_eq!(spawn_events[0].payload["tags"], json!(["chore"]));
    }
}
