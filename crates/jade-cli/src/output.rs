use clap::ValueEnum;
use jade_core::Task;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ListFormat {
    /// Human-readable table
    Plain,
    /// CSV with header row
    Csv,
    /// Pretty-printed JSON array
    Json,
}

pub fn print_tasks(tasks: &[Task], format: ListFormat) -> anyhow::Result<()> {
    match format {
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(tasks)?);
        }
        ListFormat::Csv => print_tasks_csv(tasks),
        ListFormat::Plain => print_tasks_plain(tasks),
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

fn print_tasks_plain(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("(no tasks)");
        return;
    }

    println!(
        "{:<36}  {:<10}  {:<22}  {:<30}  {:<16}  TAGS",
        "ID", "STATUS", "DUE", "TITLE", "REPEAT"
    );
    println!("{}", "-".repeat(140));
    for task in tasks {
        print_task_row(task);
    }
}

fn print_tasks_csv(tasks: &[Task]) {
    println!("id,status,due_at,title,description,repeat_cron,tags");
    for task in tasks {
        let tags = task
            .tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(";");
        let description = task.description.as_deref().unwrap_or("");
        let repeat = task.repeat_cron.as_deref().unwrap_or("");
        println!(
            "{},{},{},{},{},{},{}",
            csv_escape(&task.id.to_string()),
            csv_escape(task.status.as_str()),
            csv_escape(&task.due_at.to_rfc3339()),
            csv_escape(&task.title),
            csv_escape(description),
            csv_escape(repeat),
            csv_escape(&tags),
        );
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        let escaped = value.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        value.to_owned()
    }
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

pub fn print_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
