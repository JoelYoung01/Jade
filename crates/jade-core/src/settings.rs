use rusqlite::{params, OptionalExtension};

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::{EndOfDay, LaneVisibility, PeerSyncSettings, Settings, SyncthingSettings};

const LANE_VISIBILITY_KEY: &str = "lane_visibility";
const SYNCTHING_KEY: &str = "syncthing";
const PEER_SYNC_KEY: &str = "peer_sync";
const END_OF_DAY_KEY: &str = "end_of_day";

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
    let end_of_day = load_end_of_day(&conn)?;
    Ok(Settings {
        lane_visibility,
        syncthing,
        peer_sync,
        end_of_day,
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

pub fn set_end_of_day(db: &Db, end_of_day: EndOfDay) -> Result<Settings> {
    if !end_of_day.is_valid() {
        return Err(Error::Message(format!(
            "invalid end of day time: {:02}:{:02}",
            end_of_day.hour, end_of_day.minute
        )));
    }
    let conn = db.connection();
    let value = serde_json::to_string(&end_of_day)?;
    conn.execute(
        "
        INSERT INTO settings (key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        params![END_OF_DAY_KEY, value],
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

fn load_end_of_day(conn: &rusqlite::Connection) -> Result<EndOfDay> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![END_OF_DAY_KEY],
            |row| row.get(0),
        )
        .optional()?;
    match value {
        Some(raw) => {
            let parsed: EndOfDay = serde_json::from_str(&raw)?;
            if parsed.is_valid() {
                Ok(parsed)
            } else {
                Ok(EndOfDay::DEFAULT)
            }
        }
        None => Ok(EndOfDay::DEFAULT),
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

    #[test]
    fn defaults_and_persist_end_of_day() {
        let db = open_memory().unwrap();
        let settings = get_settings(&db).unwrap();
        assert_eq!(settings.end_of_day, EndOfDay::DEFAULT);
        assert_eq!(settings.end_of_day.hour, 16);
        assert_eq!(settings.end_of_day.minute, 0);

        let updated = set_end_of_day(
            &db,
            EndOfDay {
                hour: 17,
                minute: 30,
            },
        )
        .unwrap();
        assert_eq!(updated.end_of_day.hour, 17);
        assert_eq!(updated.end_of_day.minute, 30);

        let reloaded = get_settings(&db).unwrap();
        assert_eq!(reloaded.end_of_day.hour, 17);
        assert_eq!(reloaded.end_of_day.minute, 30);
    }

    #[test]
    fn rejects_invalid_end_of_day() {
        let db = open_memory().unwrap();
        let err = set_end_of_day(
            &db,
            EndOfDay {
                hour: 24,
                minute: 0,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid end of day"));
    }
}
