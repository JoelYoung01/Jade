use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::sync::apply::apply_remote_task_events;
use crate::sync::client::{local_events_since, make_hello, sync_all_peers};
use crate::sync::types::{SyncEventsResponse, SyncPushBody, SyncPushResponse};

pub const DEFAULT_SYNC_PORT: u16 = 7421;
pub const DEFAULT_SYNC_BIND: &str = "0.0.0.0:7421";

#[derive(Clone)]
pub struct SyncServerConfig {
    pub bind: SocketAddr,
    pub token: String,
    /// How often the serve loop dials peers (pull/push).
    pub sync_interval: Duration,
}

impl Default for SyncServerConfig {
    fn default() -> Self {
        Self {
            bind: DEFAULT_SYNC_BIND.parse().expect("default bind"),
            token: String::new(),
            sync_interval: Duration::from_secs(5),
        }
    }
}

struct AppState {
    db: Arc<Mutex<()>>,
    db_path: std::path::PathBuf,
    token: String,
}

/// Open DB per-request from path (avoids holding rusqlite Connection across await).
fn open_state_db(state: &AppState) -> Result<Db> {
    let _guard = state
        .db
        .lock()
        .map_err(|_| Error::Message("sync db lock".into()))?;
    crate::db::open_db(&state.db_path)
}

fn check_auth(headers: &HeaderMap, expected: &str) -> std::result::Result<(), StatusCode> {
    if expected.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Ok(auth) = auth.to_str() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(token) = auth.strip_prefix("Bearer ") else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    if token != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

async fn hello_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> std::result::Result<Json<crate::sync::types::HelloResponse>, StatusCode> {
    check_auth(&headers, &state.token)?;
    let db = open_state_db(&state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hello = make_hello(&db).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(hello))
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after_seq: Option<i64>,
}

async fn get_events_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<EventsQuery>,
) -> std::result::Result<Json<SyncEventsResponse>, StatusCode> {
    check_auth(&headers, &state.token)?;
    let db = open_state_db(&state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let after = q.after_seq.unwrap_or(0);
    let events = local_events_since(&db, after).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(SyncEventsResponse { events }))
}

async fn post_events_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SyncPushBody>,
) -> std::result::Result<Json<SyncPushResponse>, StatusCode> {
    check_auth(&headers, &state.token)?;
    let db = open_state_db(&state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let stats = apply_remote_task_events(&db, &body.events)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(SyncPushResponse {
        accepted: stats.accepted,
        skipped: stats.skipped,
    }))
}

/// Run the sync HTTP server until `shutdown` is signaled.
///
/// Also periodically runs `sync_all_peers` against configured peers.
pub async fn serve_sync(
    db_path: std::path::PathBuf,
    config: SyncServerConfig,
    shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    // Ensure device + open once up front
    {
        let db = crate::db::open_db(&db_path)?;
        let _ = crate::sync::device::ensure_device(&db, None)?;
    }

    let state = Arc::new(AppState {
        db: Arc::new(Mutex::new(())),
        db_path: db_path.clone(),
        token: config.token.clone(),
    });

    let app = Router::new()
        .route("/v1/hello", get(hello_handler))
        .route(
            "/v1/tasks/events",
            get(get_events_handler).post(post_events_handler),
        )
        .with_state(state.clone());

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|e| Error::Message(format!("bind {}: {e}", config.bind)))?;

    let interval = config.sync_interval;
    let db_path_loop = db_path;
    let peer_loop = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let path = db_path_loop.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(db) = crate::db::open_db(&path) {
                    let _ = sync_all_peers(&db);
                }
            })
            .await;
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown.await;
            peer_loop.abort();
        })
        .await
        .map_err(|e| Error::Message(format!("sync server: {e}")))?;

    Ok(())
}

/// Generate a random pairing token.
pub fn generate_token() -> String {
    Uuid::new_v4().simple().to_string()
}
