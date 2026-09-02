mod db;
mod due;
mod help;
mod output;
mod sync;
mod tasks;
mod update;
mod wiki;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use sync::SyncCommand;
use tasks::{Globals, TasksCommand};
use wiki::WikiCommand;

#[derive(Debug, Parser)]
#[command(
    name = "jade",
    version,
    about = "Jade — local-first personal toolkit CLI",
    long_about = "Feature-first CLI for Jade. Start with a feature (e.g. tasks), then a verb \
(list, add, update, delete).\n\nRich help: jade help | jade tasks help | jade tasks update status help\n\
Also accepts --help / -h at any command level.",
    disable_help_subcommand = true
)]
struct Cli {
    /// Override the SQLite database path
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    /// Emit JSON instead of human-readable tables
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Task tracking
    Tasks {
        #[command(subcommand)]
        command: TasksCommand,
    },
    /// Filesystem wiki (markdown index)
    Wiki {
        #[command(subcommand)]
        command: WikiCommand,
    },
    /// LAN peer sync for tasks
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
    /// Check for and install Jade updates for this install channel
    Update {
        /// Only report whether an update is available (do not install)
        #[arg(long)]
        check: bool,
        /// Skip confirmation prompts (AUR uses yay --noconfirm)
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();

    if raw.is_empty() {
        return print_help(&[]);
    }

    // clap defaults to -V; also accept -v for convenience.
    if is_version_request(&raw) {
        println!("jade {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if !raw.iter().any(|a| a == "--help" || a == "-h") {
        if let Some(path) = help::extract_help_path(&raw) {
            return print_help(&path);
        }
    }

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            err.print().ok();
            return if err.use_stderr() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            };
        }
    };

    let globals = Globals {
        db: cli.db,
        json: cli.json,
    };

    let result = match cli.command {
        Commands::Tasks { command } => tasks::run(command, &globals),
        Commands::Wiki { command } => wiki::run(command, &globals),
        Commands::Sync { command } => sync::run(command, &globals),
        Commands::Update { check, yes } => update::run(check, yes, globals.json),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn print_help(path: &[String]) -> ExitCode {
    match help::print_topic(path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn is_version_request(args: &[String]) -> bool {
    let has_version = args
        .iter()
        .any(|a| matches!(a.as_str(), "-v" | "-V" | "--version"));
    let has_command = args
        .iter()
        .any(|a| matches!(a.as_str(), "tasks" | "wiki" | "sync" | "update" | "help"));
    has_version && !has_command
}
