//! In-process peer sync listener for the desktop shell.

use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;

use jade_core::{
    ensure_device, generate_token, get_settings, list_peers, pair_peer, serve_sync,
    set_peer_sync_settings, sync_all_peers, PeerSyncSettings, SyncDevice, SyncPeer, SyncReport,
    SyncServerConfig,
};
use serde::Serialize;
use tokio::sync::oneshot;

pub struct PeerSyncRuntime {
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

impl PeerSyncRuntime {
    pub fn new() -> Self {
        Self {
            shutdown: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        self.shutdown
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.shutdown.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
    }

    pub fn start(&self, settings: &PeerSyncSettings) -> Result<(), String> {
        self.stop();
        if !settings.enabled {
            return Ok(());
        }
        if settings.token.trim().is_empty() {
            return Err("peer sync token is required".into());
        }
        let bind: SocketAddr = settings
            .bind
            .parse()
            .map_err(|e| format!("invalid sync bind {}: {e}", settings.bind))?;
        let db_path = jade_core::default_db_path().map_err(|e| e.to_string())?;
        let config = SyncServerConfig {
            bind,
            token: settings.token.clone(),
            sync_interval: Duration::from_secs(5),
        };
        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.shutdown.lock().map_err(|_| "sync lock")?;
            *guard = Some(tx);
        }
        std::thread::Builder::new()
            .name("jade-peer-sync".into())
            .spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("jade peer sync runtime: {e}");
                        return;
                    }
                };
                if let Err(e) = rt.block_on(serve_sync(db_path, config, rx)) {
                    eprintln!("jade peer sync stopped: {e}");
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct PeerSyncStatus {
    pub device: SyncDevice,
    pub peers: Vec<SyncPeer>,
    pub settings: PeerSyncSettings,
    pub listening: bool,
}

pub fn status(db: &jade_core::Db, runtime: &PeerSyncRuntime) -> Result<PeerSyncStatus, String> {
    let device = ensure_device(db, None).map_err(|e| e.to_string())?;
    let peers = list_peers(db).map_err(|e| e.to_string())?;
    let settings = get_settings(db).map_err(|e| e.to_string())?.peer_sync;
    Ok(PeerSyncStatus {
        device,
        peers,
        settings,
        listening: runtime.is_running(),
    })
}

pub fn apply_settings(
    db: &jade_core::Db,
    runtime: &PeerSyncRuntime,
    mut settings: PeerSyncSettings,
) -> Result<PeerSyncStatus, String> {
    if settings.bind.trim().is_empty() {
        settings.bind = "0.0.0.0:7421".into();
    }
    if settings.enabled && settings.token.trim().is_empty() {
        settings.token = generate_token();
    }
    let _ = set_peer_sync_settings(db, settings).map_err(|e| e.to_string())?;
    let next = get_settings(db).map_err(|e| e.to_string())?.peer_sync;
    if next.enabled {
        runtime.start(&next)?;
    } else {
        runtime.stop();
    }
    status(db, runtime)
}

pub fn pair(db: &jade_core::Db, url: &str, token: &str) -> Result<SyncPeer, String> {
    pair_peer(db, url, token).map_err(|e| e.to_string())
}

pub fn sync_now(db: &jade_core::Db) -> Result<SyncReport, String> {
    sync_all_peers(db).map_err(|e| e.to_string())
}
