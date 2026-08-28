mod init;
mod now;
mod pair;
mod serve;
mod status;

use clap::Subcommand;

use crate::tasks::Globals;

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    /// Ensure this database has a sync device id
    Init {
        /// Optional display name for this device
        #[arg(long)]
        name: Option<String>,
    },
    /// Show device id, peers, and last sync status
    Status,
    /// Pair with a remote peer (hello + store peer)
    Pair {
        /// Base URL, e.g. http://192.168.1.10:7421
        url: String,
        /// Shared Bearer token
        #[arg(long)]
        token: String,
    },
    /// One-shot pull/push all enabled peers
    Now,
    /// Listen for peers and periodically sync
    Serve {
        /// Bind address (default 0.0.0.0:7421)
        #[arg(long)]
        bind: Option<String>,
        /// Shared Bearer token (generated and printed if omitted and unset)
        #[arg(long)]
        token: Option<String>,
    },
}

pub fn run(command: SyncCommand, globals: &Globals) -> anyhow::Result<()> {
    match command {
        SyncCommand::Init { name } => init::run(globals, name.as_deref()),
        SyncCommand::Status => status::run(globals),
        SyncCommand::Pair { url, token } => pair::run(globals, &url, &token),
        SyncCommand::Now => now::run(globals),
        SyncCommand::Serve { bind, token } => serve::run(globals, bind.as_deref(), token.as_deref()),
    }
}
