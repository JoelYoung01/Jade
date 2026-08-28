use jade_core::ensure_device;

use crate::db::open_cli_db;
use crate::output::print_json;
use crate::tasks::Globals;

pub fn run(globals: &Globals, name: Option<&str>) -> anyhow::Result<()> {
    let db = open_cli_db(globals.db.clone())?;
    let device = ensure_device(&db, name)?;
    if globals.json {
        print_json(&device)?;
    } else {
        println!("device_id={}", device.device_id);
        if !device.display_name.is_empty() {
            println!("display_name={}", device.display_name);
        }
    }
    Ok(())
}
