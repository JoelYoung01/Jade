use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncDevice {
    pub device_id: Uuid,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Ensure a singleton device row exists; return it.
pub fn ensure_device(db: &Db, display_name: Option<&str>) -> Result<SyncDevice> {
    if let Some(existing) = get_device(db)? {
        return Ok(existing);
    }

    let device_id = Uuid::new_v4();
    let now = Utc::now();
    let name = display_name.unwrap_or("").trim().to_owned();
    let conn = db.connection();
    conn.execute(
        "
        INSERT INTO sync_device (id, device_id, display_name, created_at)
        VALUES (1, ?1, ?2, ?3)
        ",
        params![device_id.to_string(), name, now.to_rfc3339()],
    )?;
    Ok(SyncDevice {
        device_id,
        display_name: name,
        created_at: now,
    })
}

pub fn get_device(db: &Db) -> Result<Option<SyncDevice>> {
    let conn = db.connection();
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT device_id, display_name, created_at FROM sync_device WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    match row {
        None => Ok(None),
        Some((device_id, display_name, created_at)) => Ok(Some(SyncDevice {
            device_id: Uuid::parse_str(&device_id)
                .map_err(|e| Error::Message(format!("invalid device_id: {e}")))?,
            display_name,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| Error::InvalidDueAt(format!("{created_at}: {e}")))?,
        })),
    }
}

/// Origin for new local writes. Creates the device row inside `tx` if needed.
pub fn local_origin_in_tx(tx: &rusqlite::Transaction<'_>) -> Result<String> {
    let existing: Option<String> = tx
        .query_row(
            "SELECT device_id FROM sync_device WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let device_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "
        INSERT INTO sync_device (id, device_id, display_name, created_at)
        VALUES (1, ?1, '', ?2)
        ",
        params![device_id.to_string(), now],
    )?;
    Ok(device_id.to_string())
}
