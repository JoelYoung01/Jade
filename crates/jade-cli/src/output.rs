use jade_core::Task;

pub fn print_tasks(tasks: &[Task], json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(tasks)?);
        return Ok(());
    }

    if tasks.is_empty() {
        println!("(no tasks)");
        return Ok(());
    }

    println!(
        "{:<36}  {:<10}  {:<22}  {:<30}  {:<16}  TAGS",
        "ID", "STATUS", "DUE", "TITLE", "REPEAT"
    );
    println!("{}", "-".repeat(140));
    for task in tasks {
        print_task_row(task);
    }
    Ok(())
}

pub fn print_task(task: &Task, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(task)?);
        return Ok(());
    }
    println!(
        "{:<36}  {:<10}  {:<22}  {:<30}  {:<16}  TAGS",
        "ID", "STATUS", "DUE", "TITLE", "REPEAT"
    );
    println!("{}", "-".repeat(140));
    print_task_row(task);
    if let Some(description) = &task.description {
        println!();
        println!("Description: {description}");
    }
    if let Some(cron) = &task.repeat_cron {
        println!("Repeat: {cron}");
    }
    Ok(())
}

fn print_task_row(task: &Task) {
    let tags = task
        .tags
        .iter()
        .map(|t| t.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let title = truncate(&task.title, 30);
    let due = task.due_at.format("%Y-%m-%d %H:%M UTC").to_string();
    let repeat = task
        .repeat_cron
        .as_deref()
        .map_or_else(|| "-".into(), |c| truncate(c, 16));
    println!(
        "{:<36}  {:<10}  {:<22}  {:<30}  {:<16}  {}",
        task.id,
        task.status.as_str(),
        due,
        title,
        repeat,
        tags
    );
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
