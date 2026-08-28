use jade_core::{ensure_device, list_peers};

use crate::db::open_cli_db;
use crate::output::print_json;
use crate::tasks::Globals;

pub fn run(globals: &Globals) -> anyhow::Result<()> {
    let db = open_cli_db(globals.db.clone())?;
    let device = ensure_device(&db, None)?;
    let peers = list_peers(&db)?;
    if globals.json {
        print_json(&serde_json::json!({
            "device": device,
            "peers": peers,
        }))?;
    } else {
        println!("device_id={}", device.device_id);
        if peers.is_empty() {
            println!("peers: (none)");
        } else {
            println!("peers:");
            for p in peers {
                let err = p.last_error.as_deref().unwrap_or("-");
                let when = p
                    .last_sync_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| "-".into());
                println!(
                    "  {}  {}  pulled_seq={}  last_sync={}  error={}",
                    p.peer_device_id, p.base_url, p.last_pulled_seq, when, err
                );
            }
        }
    }
    Ok(())
}
