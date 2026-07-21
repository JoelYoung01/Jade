use jade_core::{list_tasks, Db};

use crate::output::print_tasks;

pub fn run(db: &Db, json: bool) -> anyhow::Result<()> {
    let tasks = list_tasks(db)?;
    print_tasks(&tasks, json)
}
