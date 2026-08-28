use jade_core::sync_all_peers;

use crate::db::open_cli_db;
use crate::output::print_json;
use crate::tasks::Globals;

pub fn run(globals: &Globals) -> anyhow::Result<()> {
    let db = open_cli_db(globals.db.clone())?;
    let report = sync_all_peers(&db)?;
    if globals.json {
        print_json(&report)?;
    } else if report.peers.is_empty() {
        println!("no peers configured (jade sync pair <url> --token …)");
    } else {
        for p in &report.peers {
            match &p.error {
                Some(err) => println!("{} ERROR {err}", p.peer_device_id),
                None => println!(
                    "{} pulled={} skipped={} pushed={}",
                    p.peer_device_id, p.pulled, p.skipped, p.pushed
                ),
            }
        }
    }
    Ok(())
}
