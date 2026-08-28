use rusqlite::{params, OptionalExtension};

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::{LaneVisibility, PeerSyncSettings, Settings, SyncthingSettings};

const LANE_VISIBILITY_KEY: &str = "lane_visibility";
const SYNCTHING_KEY: &str = "syncthing";
const PEER_SYNC_KEY: &str = "peer_sync";

pub fn get_settings(db: &Db) -> Result<Settings> {
    let conn = db.connection();
    let lane_visibility: LaneVisibility = {
        let value: String = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![LANE_VISIBILITY_KEY],
            |row| row.get(0),
        )?;
        serde_json::from_str(&value)?
    };
    let syncthing = load_syncthing(&conn)?;
    let peer_sync = load_peer_sync(&conn)?;
    Ok(Settings {
        lane_visibility,
        syncthing,
        peer_sync,
    })
}

pub fn set_lane_visibility(db: &Db, visibility: LaneVisibility) -> Result<Settings> {
    let conn = db.connection();
    let value = serde_json::to_string(&visibility)?;
    let updated = conn.execute(
        "
        INSERT INTO settings (key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        params![LANE_VISIBILITY_KEY, value],
    )?;
    if updated == 0 {
        return Err(Error::Message("failed to persist lane visibility".into()));
    }
    drop(conn);
    get_settings(db)
}

pub fn set_syncthing_settings(db: &Db, syncthing: SyncthingSettings) -> Result<Settings> {
    let address = if syncthing.address.trim().is_empty() {
        "http://127.0.0.1:8384".to_owned()
    } else {
        syncthing.address.trim().to_owned()
    };
    let stored = SyncthingSettings {
        address,
        api_key: syncthing.api_key,
    };
    let conn = db.connection();
    let value = serde_json::to_string(&stored)?;
    conn.execute(
        "
        INSERT INTO settings (key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        params![SYNCTHING_KEY, value],
    )?;
    drop(conn);
    get_settings(db)
}

pub fn set_peer_sync_settings(db: &Db, peer_sync: PeerSyncSettings) -> Result<Settings> {
    let bind = if peer_sync.bind.trim().is_empty() {
        "0.0.0.0:7421".to_owned()
    } else {
        peer_sync.bind.trim().to_owned()
    };
    let stored = PeerSyncSettings {
        enabled: peer_sync.enabled,
        bind,
        token: peer_sync.token,
    };
    let conn = db.connection();
    let value = serde_json::to_string(&stored)?;
    conn.execute(
        "
        INSERT INTO settings (key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        params![PEER_SYNC_KEY, value],
    )?;
    drop(conn);
    get_settings(db)
}

fn load_syncthing(conn: &rusqlite::Connection) -> Result<SyncthingSettings> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SYNCTHING_KEY],
            |row| row.get(0),
        )
        .optional()?;
    match value {
        Some(raw) => Ok(serde_json::from_str(&raw)?),
        None => Ok(SyncthingSettings {
            address: "http://127.0.0.1:8384".to_owned(),
            api_key: String::new(),
        }),
    }
}

fn load_peer_sync(conn: &rusqlite::Connection) -> Result<PeerSyncSettings> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![PEER_SYNC_KEY],
            |row| row.get(0),
        )
        .optional()?;
    match value {
        Some(raw) => Ok(serde_json::from_str(&raw)?),
        None => Ok(PeerSyncSettings::with_defaults()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    #[test]
    fn defaults_and_persist_lane_visibility() {
        let db = open_memory().unwrap();
        let settings = get_settings(&db).unwrap();
        assert!(settings.lane_visibility.inactive);
        assert!(settings.lane_visibility.active);
        assert!(!settings.lane_visibility.complete);

        let updated = set_lane_visibility(
            &db,
            LaneVisibility {
                inactive: false,
                active: true,
                complete: true,
            },
        )
        .unwrap();
        assert!(!updated.lane_visibility.inactive);
        assert!(updated.lane_visibility.complete);

        let reloaded = get_settings(&db).unwrap();
        assert!(!reloaded.lane_visibility.inactive);
        assert!(!reloaded.syncthing.is_configured());
    }

    #[test]
    fn persist_syncthing_settings() {
        let db = open_memory().unwrap();
        let updated = set_syncthing_settings(
            &db,
            SyncthingSettings {
                address: "http://127.0.0.1:8384".into(),
                api_key: "secret".into(),
            },
        )
        .unwrap();
        assert!(updated.syncthing.is_configured());
        assert_eq!(updated.syncthing.api_key, "secret");
    }
}
