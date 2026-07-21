use std::path::PathBuf;

use jade_core::{default_db_path, open_db, Db};

pub fn open_cli_db(db_override: Option<PathBuf>) -> anyhow::Result<Db> {
    let path = match db_override {
        Some(path) => path,
        None => default_db_path()?,
    };
    Ok(open_db(path)?)
}
