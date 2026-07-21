use jade_core::{delete_task, Db};
use serde_json::json;

use crate::tasks::DeleteArgs;

pub fn run(db: &Db, args: DeleteArgs, json: bool) -> anyhow::Result<()> {
    delete_task(db, args.id)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "deleted": true,
                "id": args.id,
            }))?
        );
    } else {
        println!("Deleted task {}", args.id);
    }
    Ok(())
}
