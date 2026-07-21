use rusqlite::params;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::{LaneVisibility, Settings};

const LANE_VISIBILITY_KEY: &str = "lane_visibility";

pub fn get_settings(db: &Db) -> Result<Settings> {
    let conn = db.connection();
    let value: String = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![LANE_VISIBILITY_KEY],
        |row| row.get(0),
    )?;

    let lane_visibility: LaneVisibility = serde_json::from_str(&value)?;
    Ok(Settings { lane_visibility })
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
    Ok(Settings {
        lane_visibility: visibility,
    })
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
        assert!(reloaded.lane_visibility.complete);
    }
}
