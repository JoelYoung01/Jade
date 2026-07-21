mod add;
mod delete;
mod history;
mod list;
mod update;

use std::path::PathBuf;

use clap::{Args, Subcommand};
use uuid::Uuid;

use crate::db::open_cli_db;

#[derive(Debug, Subcommand)]
pub enum TasksCommand {
    /// List non-deleted tasks ordered by due date
    List,
    /// Create a new task
    Add(AddArgs),
    /// Partially update an existing task
    Update(UpdateArgs),
    /// Soft-delete a task
    Delete(DeleteArgs),
    /// Show the task event log (newest first)
    History(HistoryArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Task title
    pub title: String,

    /// Optional description
    #[arg(long, short = 'd')]
    pub description: Option<String>,

    /// Due date: tomorrow, next-monday, RFC3339, YYYY-MM-DDTHH:MM, or YYYY-MM-DD
    #[arg(long)]
    pub due: Option<String>,

    /// Tag name (repeatable)
    #[arg(long = "tag", short = 't')]
    pub tags: Vec<String>,

    /// 5-field POSIX cron schedule (e.g. "0 9 * * 1-5")
    #[arg(long = "repeat")]
    pub repeat: Option<String>,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Task id (UUID)
    #[arg(long)]
    pub id: Uuid,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New description (empty string clears it)
    #[arg(long, short = 'd')]
    pub description: Option<String>,

    /// New status: inactive, active, or complete
    #[arg(long)]
    pub status: Option<String>,

    /// New due: tomorrow, next-monday, RFC3339, YYYY-MM-DDTHH:MM, or YYYY-MM-DD
    #[arg(long)]
    pub due: Option<String>,

    /// 5-field POSIX cron schedule, or `none` to clear
    #[arg(long = "repeat")]
    pub repeat: Option<String>,
}

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Task id (UUID)
    #[arg(long)]
    pub id: Uuid,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Filter to a single task id (UUID)
    #[arg(long)]
    pub id: Option<Uuid>,

    /// Max events to return (newest first)
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
}

pub struct Globals {
    pub db: Option<PathBuf>,
    pub json: bool,
}

pub fn run(command: TasksCommand, globals: &Globals) -> anyhow::Result<()> {
    let db = open_cli_db(globals.db.clone())?;
    match command {
        TasksCommand::List => list::run(&db, globals.json),
        TasksCommand::Add(args) => add::run(&db, args, globals.json),
        TasksCommand::Update(args) => update::run(&db, args, globals.json),
        TasksCommand::Delete(args) => delete::run(&db, args, globals.json),
        TasksCommand::History(args) => history::run(&db, args, globals.json),
    }
}
