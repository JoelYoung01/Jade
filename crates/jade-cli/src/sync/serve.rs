use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use jade_core::{
    ensure_device, generate_token, get_settings, serve_sync, set_peer_sync_settings,
    PeerSyncSettings, SyncServerConfig, DEFAULT_SYNC_BIND,
};
use tokio::sync::oneshot;

use crate::db::open_cli_db;
use crate::tasks::Globals;

pub fn run(
    globals: &Globals,
    bind: Option<&str>,
    token: Option<&str>,
) -> anyhow::Result<()> {
    let db_path = resolve_db_path(globals.db.clone())?;
    let db = open_cli_db(Some(db_path.clone()))?;
    let _ = ensure_device(&db, None)?;

    let settings = get_settings(&db)?;
    let bind_str = bind.map(str::to_owned).unwrap_or_else(|| {
        if settings.peer_sync.bind.is_empty() {
            DEFAULT_SYNC_BIND.to_owned()
        } else {
            settings.peer_sync.bind.clone()
        }
    });
    let token = match token {
        Some(t) if !t.is_empty() => t.to_owned(),
        _ if !settings.peer_sync.token.is_empty() => settings.peer_sync.token.clone(),
        _ => {
            let t = generate_token();
            eprintln!("generated token (save for pairing): {t}");
            t
        }
    };

    let _ = set_peer_sync_settings(
        &db,
        PeerSyncSettings {
            enabled: settings.peer_sync.enabled,
            bind: bind_str.clone(),
            token: token.clone(),
        },
    )?;
    drop(db);

    let addr: SocketAddr = bind_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid --bind {bind_str}: {e}"))?;

    eprintln!("jade sync serve on http://{addr}");
    eprintln!("token={token}");
    eprintln!(
        "pair from another peer: jade sync pair http://<this-host>:{} --token {token}",
        addr.port()
    );

    let config = SyncServerConfig {
        bind: addr,
        token,
        sync_interval: Duration::from_secs(5),
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = tx.send(());
        });
        serve_sync(db_path, config, rx)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    })?;
    Ok(())
}

fn resolve_db_path(override_path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    jade_core::default_db_path().map_err(|e| anyhow::anyhow!("{e}"))
}
