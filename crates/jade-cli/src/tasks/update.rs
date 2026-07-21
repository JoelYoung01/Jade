use jade_core::{update_task, Db, RepeatCronUpdate, TaskStatus, UpdateTaskInput};

use crate::due::parse_due_for_update;
use crate::output::print_task;
use crate::tasks::UpdateArgs;

pub fn run(db: &Db, args: UpdateArgs, json: bool) -> anyhow::Result<()> {
    let status = args
        .status
        .as_deref()
        .map(TaskStatus::parse)
        .transpose()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let due = args.due.as_deref().map(parse_due_for_update).transpose()?;

    let repeat_cron = match args.repeat.as_deref() {
        None => None,
        Some(value) if value.eq_ignore_ascii_case("none") => Some(RepeatCronUpdate::Clear),
        Some(value) => Some(RepeatCronUpdate::Set(value.to_owned())),
    };

    let result = update_task(
        db,
        UpdateTaskInput {
            id: args.id,
            title: args.title,
            description: args.description,
            status,
            due,
            tag_names: None,
            repeat_cron,
        },
    )?;
    if let Some(spawned) = &result.spawned {
        if !json {
            println!("Spawned next occurrence:");
            print_task(spawned, false)?;
            println!();
            println!("Completed:");
        }
    }
    print_task(&result.task, json)
}
