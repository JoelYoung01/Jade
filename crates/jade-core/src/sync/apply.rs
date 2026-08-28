use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::{Task, TaskEventType};
use crate::sync::types::SyncEventEnvelope;

#[derive(Debug, Clone, Default)]
pub struct ApplyStats {
    pub accepted: u32,
    pub skipped: u32,
}

/// Apply remote events: idempotent by event id, LWW on task snapshots.
pub fn apply_remote_task_events(db: &Db, events: &[SyncEventEnvelope]) -> Result<ApplyStats> {
    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.as_bytes().cmp(b.id.as_bytes()))
    });

    let mut stats = ApplyStats::default();
    let conn = db.connection();
    let tx = conn.unchecked_transaction()?;

    for ev in &sorted {
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM task_events WHERE id = ?1",
                params![ev.id.to_string()],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if exists {
            stats.skipped += 1;
            continue;
        }

        let task_exists: bool = tx
            .query_row(
                "SELECT 1 FROM tasks WHERE id = ?1",
                params![ev.task_id.to_string()],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        let apply_state = should_apply_lww(&tx, ev.task_id, ev.id, ev.created_at)?;
        // Task row must exist before event insert (FK). Bootstrap if missing.
        if apply_state || !task_exists {
            materialize_task(&tx, ev)?;
        }

        tx.execute(
            "
            INSERT INTO task_events (id, task_id, event_type, payload, origin, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                ev.id.to_string(),
                ev.task_id.to_string(),
                ev.event_type.as_str(),
                ev.payload.to_string(),
                ev.origin,
                ev.created_at.to_rfc3339(),
            ],
        )?;

        if apply_state {
            tx.execute(
                "
                INSERT INTO sync_applied (task_id, last_event_id, last_event_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(task_id) DO UPDATE SET
                    last_event_id = excluded.last_event_id,
                    last_event_at = excluded.last_event_at
                ",
                params![
                    ev.task_id.to_string(),
                    ev.id.to_string(),
                    ev.created_at.to_rfc3339(),
                ],
            )?;
        }

        stats.accepted += 1;
    }

    tx.commit()?;
    Ok(stats)
}

fn should_apply_lww(
    tx: &rusqlite::Transaction<'_>,
    task_id: Uuid,
    event_id: Uuid,
    event_at: DateTime<Utc>,
) -> Result<bool> {
    let prev: Option<(String, String)> = tx
        .query_row(
            "SELECT last_event_id, last_event_at FROM sync_applied WHERE task_id = ?1",
            params![task_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let Some((prev_id, prev_at)) = prev else {
        return Ok(true);
    };

    let prev_at = DateTime::parse_from_rfc3339(&prev_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::InvalidDueAt(format!("{prev_at}: {e}")))?;
    let prev_id = Uuid::parse_str(&prev_id)
        .map_err(|e| Error::Message(format!("invalid last_event_id: {e}")))?;

    Ok(event_at > prev_at || (event_at == prev_at && event_id > prev_id))
}

fn materialize_task(tx: &rusqlite::Transaction<'_>, ev: &SyncEventEnvelope) -> Result<()> {
    let task_val = ev
        .payload
        .get("task")
        .cloned()
        .ok_or_else(|| Error::Message("sync event missing payload.task".into()))?;
    let task: Task = serde_json::from_value(task_val)
        .map_err(|e| Error::Message(format!("invalid payload.task: {e}")))?;

    if task.id != ev.task_id {
        return Err(Error::Message(
            "payload.task.id does not match event task_id".into(),
        ));
    }

    // Upsert task row (including soft-deleted).
    tx.execute(
        "
        INSERT INTO tasks (
            id, title, description, status, due_at, repeat_cron,
            created_at, updated_at, deleted_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            status = excluded.status,
            due_at = excluded.due_at,
            repeat_cron = excluded.repeat_cron,
            created_at = excluded.created_at,
            updated_at = excluded.updated_at,
            deleted_at = excluded.deleted_at
        ",
        params![
            task.id.to_string(),
            task.title,
            task.description,
            task.status.as_str(),
            task.due_at.to_rfc3339(),
            task.repeat_cron,
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            task.deleted_at.map(|d| d.to_rfc3339()),
        ],
    )?;

    if matches!(ev.event_type, TaskEventType::Deleted) && task.deleted_at.is_none() {
        // Defensive: deleted events should carry deleted_at on snapshot.
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE tasks SET deleted_at = COALESCE(deleted_at, ?1), updated_at = ?1 WHERE id = ?2",
            params![now, task.id.to_string()],
        )?;
    }

    tx.execute(
        "DELETE FROM task_tags WHERE task_id = ?1",
        params![task.id.to_string()],
    )?;

    for tag in &task.tags {
        let tag_id = ensure_tag_named(tx, &tag.name, tag.id)?;
        tx.execute(
            "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
            params![task.id.to_string(), tag_id.to_string()],
        )?;
    }

    Ok(())
}

fn ensure_tag_named(
    tx: &rusqlite::Transaction<'_>,
    name: &str,
    preferred_id: Uuid,
) -> Result<Uuid> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Message("empty tag name in sync payload".into()));
    }

    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
            params![name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Uuid::parse_str(&id).map_err(|e| Error::Message(format!("invalid tag id: {e}")));
    }

    let now = Utc::now().to_rfc3339();
    // Prefer remote tag id when free; otherwise mint new.
    let id_free: bool = tx
        .query_row(
            "SELECT 1 FROM tags WHERE id = ?1",
            params![preferred_id.to_string()],
            |_| Ok(true),
        )
        .optional()?
        .is_none();
    let id = if id_free {
        preferred_id
    } else {
        Uuid::new_v4()
    };

    tx.execute(
        "INSERT INTO tags (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![id.to_string(), name, now, now],
    )?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreateTaskInput, TaskEventType, TaskStatus};
    use crate::sync::device::ensure_device;
    use crate::tasks::{create_task, list_tasks};
    use chrono::TimeZone;
    use serde_json::Value;

    fn envelope_from_local(db: &Db) -> SyncEventEnvelope {
        let conn = db.connection();
        let (seq, id, task_id, event_type, payload, origin, created_at): (
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "
                SELECT seq, id, task_id, event_type, payload, origin, created_at
                FROM task_events ORDER BY seq ASC LIMIT 1
                ",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();
        SyncEventEnvelope {
            seq,
            id: Uuid::parse_str(&id).unwrap(),
            task_id: Uuid::parse_str(&task_id).unwrap(),
            event_type: TaskEventType::parse(&event_type).unwrap(),
            payload: serde_json::from_str(&payload).unwrap(),
            origin,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    #[test]
    fn dual_db_apply_idempotent_and_lww() {
        let a = crate::db::open_memory().unwrap();
        let b = crate::db::open_memory().unwrap();
        ensure_device(&a, Some("A")).unwrap();
        ensure_device(&b, Some("B")).unwrap();

        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let task = create_task(
            &a,
            CreateTaskInput {
                title: "From A".into(),
                description: None,
                due_at: due,
                tag_names: vec!["work".into()],
                repeat_cron: None,
            },
        )
        .unwrap();

        let ev = envelope_from_local(&a);
        let stats = apply_remote_task_events(&b, &[ev.clone()]).unwrap();
        assert_eq!(stats.accepted, 1);
        assert_eq!(stats.skipped, 0);

        let listed = list_tasks(&b).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, task.id);
        assert_eq!(listed[0].title, "From A");
        assert_eq!(listed[0].tags.len(), 1);

        let again = apply_remote_task_events(&b, &[ev]).unwrap();
        assert_eq!(again.accepted, 0);
        assert_eq!(again.skipped, 1);

        // Newer event wins LWW
        let mut newer = envelope_from_local(&a);
        newer.id = Uuid::new_v4();
        newer.created_at = Utc::now() + chrono::Duration::seconds(10);
        newer.event_type = TaskEventType::Updated;
        let mut task_json = newer.payload["task"].clone();
        task_json["title"] = Value::String("Renamed".into());
        task_json["status"] = Value::String(TaskStatus::Active.as_str().into());
        newer.payload = serde_json::json!({
            "changes": {},
            "task": task_json,
        });

        apply_remote_task_events(&b, &[newer]).unwrap();
        let listed = list_tasks(&b).unwrap();
        assert_eq!(listed[0].title, "Renamed");
        assert_eq!(listed[0].status, TaskStatus::Active);
    }
}
