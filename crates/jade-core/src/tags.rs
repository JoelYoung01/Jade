use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::Tag;

pub fn list_tags(db: &Db) -> Result<Vec<Tag>> {
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT id, name, created_at, updated_at
        FROM tags
        ORDER BY name COLLATE NOCASE ASC
        ",
    )?;

    let rows = stmt.query_map([], |row| {
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

pub fn ensure_tag(db: &Db, name: &str) -> Result<Tag> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Message("tag name is required".into()));
    }

    {
        let conn = db.connection();
        let existing: Option<(String, String, String, String)> = conn
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
                id: Uuid::parse_str(&id)
                    .map_err(|e| Error::Message(format!("invalid tag id: {e}")))?,
                name: existing_name,
                created_at: parse_dt(&created_at)?,
                updated_at: parse_dt(&updated_at)?,
            });
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        conn.execute(
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
}

pub fn count_tasks_with_tag(db: &Db, tag_id: Uuid) -> Result<u64> {
    let conn = db.connection();
    let count: i64 = conn.query_row(
        "
        SELECT COUNT(*)
        FROM task_tags tt
        INNER JOIN tasks t ON t.id = tt.task_id
        WHERE tt.tag_id = ?1 AND t.deleted_at IS NULL
        ",
        params![tag_id.to_string()],
        |row| row.get(0),
    )?;
    u64::try_from(count).map_err(|e| Error::Message(format!("count overflow: {e}")))
}

pub fn delete_tag(db: &Db, tag_id: Uuid) -> Result<()> {
    let conn = db.connection();
    let deleted = conn.execute(
        "DELETE FROM tags WHERE id = ?1",
        params![tag_id.to_string()],
    )?;
    if deleted == 0 {
        return Err(Error::TagNotFound(tag_id.to_string()));
    }
    Ok(())
}

fn parse_dt(value: &str) -> Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::InvalidDueAt(format!("{value}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;
    use crate::models::CreateTaskInput;
    use crate::tasks::{create_task, delete_task};
    use chrono::{TimeZone, Utc};

    #[test]
    fn ensure_tag_is_case_insensitive() {
        let db = open_memory().unwrap();
        let a = ensure_tag(&db, "Work").unwrap();
        let b = ensure_tag(&db, "work").unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(list_tags(&db).unwrap().len(), 1);
    }

    #[test]
    fn count_and_delete_tag() {
        let db = open_memory().unwrap();
        let due = Utc.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let first = create_task(
            &db,
            CreateTaskInput {
                title: "One".into(),
                description: None,
                due_at: due,
                tag_names: vec!["shared".into()],
                repeat_cron: None,
            },
        )
        .unwrap();
        create_task(
            &db,
            CreateTaskInput {
                title: "Two".into(),
                description: None,
                due_at: due,
                tag_names: vec!["shared".into()],
                repeat_cron: None,
            },
        )
        .unwrap();

        let tag_id = first.tags[0].id;
        assert_eq!(count_tasks_with_tag(&db, tag_id).unwrap(), 2);

        delete_task(&db, first.id).unwrap();
        assert_eq!(count_tasks_with_tag(&db, tag_id).unwrap(), 1);

        delete_tag(&db, tag_id).unwrap();
        assert!(list_tags(&db).unwrap().is_empty());
        assert_eq!(count_tasks_with_tag(&db, tag_id).unwrap(), 0);
    }
}
