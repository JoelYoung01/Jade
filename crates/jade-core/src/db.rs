use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};

use crate::error::Result;

/// App data folder name shared by the desktop shell and CLI.
/// Must stay aligned with Tauri `identifier` in `tauri.conf.json`.
pub const APP_DATA_DIR_NAME: &str = "app.jade.desktop";

/// Default SQLite path: `{data_dir}/app.jade.desktop/jade.db`.
///
/// On Windows this is `%APPDATA%\app.jade.desktop\jade.db`, matching Tauri's
/// `app.path().app_data_dir()` for identifier `app.jade.desktop`.
pub fn default_db_path() -> Result<PathBuf> {
    let data = dirs::data_dir()
        .ok_or_else(|| crate::Error::Message("could not resolve platform data directory".into()))?;
    Ok(data.join(APP_DATA_DIR_NAME).join("jade.db"))
}

/// Open the default on-disk database (creates parent dirs + migrates).
pub fn open_default_db() -> Result<Db> {
    open_db(default_db_path()?)
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("db mutex poisoned")
    }
}

pub fn open_db(path: impl AsRef<Path>) -> Result<Db> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::Error::Message(format!("failed to create db directory: {e}")))?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 5000;
        ",
    )?;
    migrate(&conn)?;
    Ok(Db {
        conn: Mutex::new(conn),
    })
}

/// Open an in-memory database (for tests).
#[cfg(test)]
pub fn open_memory() -> Result<Db> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        ",
    )?;
    migrate(&conn)?;
    Ok(Db {
        conn: Mutex::new(conn),
    })
}

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/001_init.sql"),
    include_str!("../migrations/002_add_repeat_cron.sql"),
    include_str!("../migrations/003_task_events.sql"),
];

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL
        );
        ",
    )?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);

    for (idx, sql) in MIGRATIONS.iter().enumerate() {
        let version = i64::try_from(idx + 1).expect("migration index fits i64");
        if version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![version, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_cleanly() {
        let db = open_memory().expect("open memory db");
        let conn = db.connection();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 3);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
