use jade_core::pair_peer;

use crate::db::open_cli_db;
use crate::output::print_json;
use crate::tasks::Globals;

pub fn run(globals: &Globals, url: &str, token: &str) -> anyhow::Result<()> {
    let db = open_cli_db(globals.db.clone())?;
    let peer = pair_peer(&db, url, token)?;
    if globals.json {
        print_json(&peer)?;
    } else {
        println!("paired peer_device_id={}", peer.peer_device_id);
        println!("base_url={}", peer.base_url);
    }
    Ok(())
}
