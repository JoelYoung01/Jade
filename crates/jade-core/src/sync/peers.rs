use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeer {
    pub peer_device_id: Uuid,
    pub base_url: String,
    pub token: String,
    pub last_pulled_seq: i64,
    pub last_push_ack: i64,
    pub enabled: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertPeerInput {
    pub peer_device_id: Uuid,
    pub base_url: String,
    pub token: String,
}

pub fn list_peers(db: &Db) -> Result<Vec<SyncPeer>> {
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT peer_device_id, base_url, token, last_pulled_seq, last_push_ack, enabled,
               last_sync_at, last_error, created_at, updated_at
        FROM sync_peers
        ORDER BY created_at ASC
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
        ))
    })?;

    let mut peers = Vec::new();
    for row in rows {
        let (
            peer_device_id,
            base_url,
            token,
            last_pulled_seq,
            last_push_ack,
            enabled,
            last_sync_at,
            last_error,
            created_at,
            updated_at,
        ) = row?;
        peers.push(SyncPeer {
            peer_device_id: Uuid::parse_str(&peer_device_id)
                .map_err(|e| Error::Message(format!("invalid peer_device_id: {e}")))?,
            base_url,
            token,
            last_pulled_seq,
            last_push_ack,
            enabled: enabled != 0,
            last_sync_at: parse_opt_dt(last_sync_at)?,
            last_error,
            created_at: parse_dt(&created_at)?,
            updated_at: parse_dt(&updated_at)?,
        });
    }
    Ok(peers)
}

/// Hello a remote URL and upsert the peer row.
pub fn pair_peer(db: &Db, base_url: &str, token: &str) -> Result<SyncPeer> {
    let hello = crate::sync::client::hello(base_url, token)?;
    let local = crate::sync::device::ensure_device(db, None)?;
    if hello.device_id == local.device_id {
        return Err(Error::Message("cannot pair with self".into()));
    }

    upsert_peer(
        db,
        UpsertPeerInput {
            peer_device_id: hello.device_id,
            base_url: base_url.trim_end_matches('/').to_owned(),
            token: token.to_owned(),
        },
    )
}

pub fn upsert_peer(db: &Db, input: UpsertPeerInput) -> Result<SyncPeer> {
    let now = Utc::now();
    let base_url = input.base_url.trim_end_matches('/').to_owned();
    let conn = db.connection();
    conn.execute(
        "
        INSERT INTO sync_peers (
            peer_device_id, base_url, token, last_pulled_seq, last_push_ack, enabled,
            last_sync_at, last_error, created_at, updated_at
        ) VALUES (?1, ?2, ?3, 0, 0, 1, NULL, NULL, ?4, ?4)
        ON CONFLICT(peer_device_id) DO UPDATE SET
            base_url = excluded.base_url,
            token = excluded.token,
            enabled = 1,
            updated_at = excluded.updated_at,
            last_error = NULL
        ",
        params![
            input.peer_device_id.to_string(),
            base_url,
            input.token,
            now.to_rfc3339(),
        ],
    )?;
    drop(conn);
    get_peer(db, input.peer_device_id)?.ok_or_else(|| Error::Message("peer missing after upsert".into()))
}

pub fn get_peer(db: &Db, peer_device_id: Uuid) -> Result<Option<SyncPeer>> {
    list_peers(db).map(|peers| peers.into_iter().find(|p| p.peer_device_id == peer_device_id))
}

pub fn set_peer_cursor(db: &Db, peer_device_id: Uuid, last_pulled_seq: i64) -> Result<()> {
    let now = Utc::now();
    let conn = db.connection();
    let n = conn.execute(
        "
        UPDATE sync_peers
        SET last_pulled_seq = ?1, updated_at = ?2
        WHERE peer_device_id = ?3
        ",
        params![last_pulled_seq, now.to_rfc3339(), peer_device_id.to_string()],
    )?;
    if n == 0 {
        return Err(Error::Message(format!("unknown peer {peer_device_id}")));
    }
    Ok(())
}

pub fn set_peer_push_ack(db: &Db, peer_device_id: Uuid, last_push_ack: i64) -> Result<()> {
    let now = Utc::now();
    let conn = db.connection();
    conn.execute(
        "
        UPDATE sync_peers
        SET last_push_ack = ?1, updated_at = ?2
        WHERE peer_device_id = ?3
        ",
        params![last_push_ack, now.to_rfc3339(), peer_device_id.to_string()],
    )?;
    Ok(())
}

pub fn set_peer_sync_result(
    db: &Db,
    peer_device_id: Uuid,
    error: Option<&str>,
) -> Result<()> {
    let now = Utc::now();
    let conn = db.connection();
    conn.execute(
        "
        UPDATE sync_peers
        SET last_sync_at = ?1, last_error = ?2, updated_at = ?1
        WHERE peer_device_id = ?3
        ",
        params![now.to_rfc3339(), error, peer_device_id.to_string()],
    )?;
    Ok(())
}

fn parse_dt(raw: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::InvalidDueAt(format!("{raw}: {e}")))
}

fn parse_opt_dt(raw: Option<String>) -> Result<Option<DateTime<Utc>>> {
    match raw {
        None => Ok(None),
        Some(s) => Ok(Some(parse_dt(&s)?)),
    }
}
