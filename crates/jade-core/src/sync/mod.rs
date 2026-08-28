//! LAN-first peer sync for tasks.

mod apply;
mod client;
mod device;
mod http;
mod peers;
mod types;

pub use apply::{apply_remote_task_events, ApplyStats};
#[allow(unused_imports)]
pub use client::{pull_and_apply_peer, push_to_peer, sync_all_peers, SyncReport};
#[allow(unused_imports)]
pub use device::{ensure_device, get_device, local_origin_in_tx, SyncDevice};
pub use http::{
    generate_token, serve_sync, SyncServerConfig, DEFAULT_SYNC_BIND, DEFAULT_SYNC_PORT,
};
#[allow(unused_imports)]
pub use peers::{
    list_peers, pair_peer, set_peer_cursor, set_peer_sync_result, SyncPeer, UpsertPeerInput,
};
#[allow(unused_imports)]
pub use types::{
    HelloResponse, SyncEventEnvelope, SyncEventsResponse, SyncPushBody, SyncPushResponse,
    PROTOCOL_VERSION,
};
