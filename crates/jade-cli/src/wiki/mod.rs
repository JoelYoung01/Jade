mod list;
mod roots;
mod search;

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::db::open_cli_db;
use crate::output::ListFormat;
use crate::tasks::Globals;

#[derive(Debug, Subcommand)]
pub enum WikiCommand {
    /// List configured wiki roots
    Roots(RootsArgs),
    /// List indexed wiki pages
    List(ListArgs),
    /// Search indexed wiki pages by title, path, or tags
    Search(SearchArgs),
}

#[derive(Debug, Args)]
pub struct RootsArgs {
    #[command(subcommand)]
    pub command: Option<RootsCommand>,

    /// Output format: plain (default), csv, or json
    #[arg(long, value_enum, default_value_t = ListFormat::Plain)]
    pub format: ListFormat,
}

#[derive(Debug, Subcommand)]
pub enum RootsCommand {
    /// Add a wiki root directory
    Add(RootsAddArgs),
    /// Remove a wiki root (soft-delete)
    Remove(RootsRemoveArgs),
}

#[derive(Debug, Args)]
pub struct RootsAddArgs {
    /// Absolute or relative path to the wiki folder
    pub path: PathBuf,

    /// Optional display label
    #[arg(long)]
    pub label: Option<String>,
}

#[derive(Debug, Args)]
pub struct RootsRemoveArgs {
    /// Wiki root id (UUID)
    #[arg(long)]
    pub id: uuid::Uuid,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Filter to a single root id
    #[arg(long)]
    pub root: Option<uuid::Uuid>,

    /// Output format: plain (default), csv, or json
    #[arg(long, value_enum, default_value_t = ListFormat::Plain)]
    pub format: ListFormat,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Output format: plain (default), csv, or json
    #[arg(long, value_enum, default_value_t = ListFormat::Plain)]
    pub format: ListFormat,
}

pub fn run(command: WikiCommand, globals: &Globals) -> anyhow::Result<()> {
    let db = open_cli_db(globals.db.clone())?;
    match command {
        WikiCommand::Roots(args) => roots::run(&db, args, globals.json),
        WikiCommand::List(args) => {
            let format = if globals.json && args.format == ListFormat::Plain {
                ListFormat::Json
            } else {
                args.format
            };
            list::run(&db, args.root, format)
        }
        WikiCommand::Search(args) => {
            let format = if globals.json && args.format == ListFormat::Plain {
                ListFormat::Json
            } else {
                args.format
            };
            search::run(&db, &args.query, format)
        }
    }
}
