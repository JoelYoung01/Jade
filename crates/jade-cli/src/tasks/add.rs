use jade_core::{create_task, end_of_day_today, get_settings, CreateTaskInput, Db};

use crate::due::parse_due_for_create;
use crate::output::print_task;
use crate::tasks::AddArgs;

pub fn run(db: &Db, args: AddArgs, json: bool) -> anyhow::Result<()> {
    let due_at = match args.due.as_deref() {
        Some(value) => parse_due_for_create(value)?,
        None => end_of_day_today(get_settings(db)?.end_of_day),
    };

    let task = create_task(
        db,
        CreateTaskInput {
            title: args.title,
            description: args.description,
            due_at,
            tag_names: args.tags,
            repeat_cron: args.repeat,
        },
    )?;
    print_task(&task, json)
}
