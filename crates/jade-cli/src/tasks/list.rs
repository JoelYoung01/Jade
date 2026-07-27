use jade_core::{list_tasks, Db};

use crate::output::{print_tasks, ListFormat};

pub fn run(db: &Db, format: ListFormat) -> anyhow::Result<()> {
    let tasks = list_tasks(db)?;
    print_tasks(&tasks, format)
}
