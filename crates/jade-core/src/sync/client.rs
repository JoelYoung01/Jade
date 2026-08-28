use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::{latest_event_seq, list_task_events_since};
use crate::models::{ListTaskEventsSinceInput, TaskEvent};
use crate::sync::apply::apply_remote_task_events;
use crate::sync::device::ensure_device;
use crate::sync::peers::{
    list_peers, set_peer_cursor, set_peer_push_ack, set_peer_sync_result, SyncPeer,
};
use crate::sync::types::{
    HelloResponse, SyncEventEnvelope, SyncEventsResponse, SyncPushBody, SyncPushResponse,
    PROTOCOL_VERSION,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncReport {
    pub peers: Vec<PeerSyncResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSyncResult {
    pub peer_device_id: String,
    pub pulled: u32,
    pub pushed: u32,
    pub skipped: u32,
    pub error: Option<String>,
}

pub fn hello(base_url: &str, token: &str) -> Result<HelloResponse> {
    let url = format!("{}/v1/hello", base_url.trim_end_matches('/'));
    let body = authorized_get(&url, token)?;
    serde_json::from_str(&body).map_err(|e| Error::Message(format!("hello decode: {e}")))
}

pub fn pull_events(
    base_url: &str,
    token: &str,
    after_seq: i64,
) -> Result<Vec<SyncEventEnvelope>> {
    let url = format!(
        "{}/v1/tasks/events?after_seq={after_seq}",
        base_url.trim_end_matches('/')
    );
    let body = authorized_get(&url, token)?;
    let parsed: SyncEventsResponse = serde_json::from_str(&body)
        .map_err(|e| Error::Message(format!("pull decode: {e}")))?;
    Ok(parsed.events)
}

pub fn push_events(
    base_url: &str,
    token: &str,
    events: &[SyncEventEnvelope],
) -> Result<SyncPushResponse> {
    let url = format!("{}/v1/tasks/events", base_url.trim_end_matches('/'));
    let body = SyncPushBody {
        events: events.to_vec(),
    };
    let raw = authorized_post(&url, token, &serde_json::to_string(&body)?)?;
    serde_json::from_str(&raw).map_err(|e| Error::Message(format!("push decode: {e}")))
}

pub fn pull_and_apply_peer(db: &Db, peer: &SyncPeer) -> Result<(u32, u32, i64)> {
    let events = pull_events(&peer.base_url, &peer.token, peer.last_pulled_seq)?;
    let max_seq = events.iter().map(|e| e.seq).max().unwrap_or(peer.last_pulled_seq);
    let stats = apply_remote_task_events(db, &events)?;
    if max_seq > peer.last_pulled_seq {
        set_peer_cursor(db, peer.peer_device_id, max_seq)?;
    }
    Ok((stats.accepted, stats.skipped, max_seq))
}

pub fn push_to_peer(db: &Db, peer: &SyncPeer) -> Result<u32> {
    let _local = ensure_device(db, None)?;
    let events = list_task_events_since(
        db,
        ListTaskEventsSinceInput {
            after_seq: peer.last_push_ack,
            limit: Some(500),
        },
    )?;
    if events.is_empty() {
        return Ok(0);
    }
    let envelopes: Vec<SyncEventEnvelope> = events.iter().map(task_event_to_envelope).collect();
    let max_seq = events.iter().map(|e| e.seq).max().unwrap_or(peer.last_push_ack);
    let resp = push_events(&peer.base_url, &peer.token, &envelopes)?;
    set_peer_push_ack(db, peer.peer_device_id, max_seq)?;
    Ok(resp.accepted)
}

/// Pull then push for every enabled peer.
pub fn sync_all_peers(db: &Db) -> Result<SyncReport> {
    let _ = ensure_device(db, None)?;
    let peers = list_peers(db)?;
    let mut report = SyncReport::default();

    for peer in peers.into_iter().filter(|p| p.enabled) {
        let mut result = PeerSyncResult {
            peer_device_id: peer.peer_device_id.to_string(),
            pulled: 0,
            pushed: 0,
            skipped: 0,
            error: None,
        };
        match pull_and_apply_peer(db, &peer) {
            Ok((accepted, skipped, _)) => {
                result.pulled = accepted;
                result.skipped = skipped;
                match push_to_peer(db, &peer) {
                    Ok(pushed) => {
                        result.pushed = pushed;
                        let _ = set_peer_sync_result(db, peer.peer_device_id, None);
                    }
                    Err(e) => {
                        result.error = Some(e.to_string());
                        let _ = set_peer_sync_result(db, peer.peer_device_id, Some(&e.to_string()));
                    }
                }
            }
            Err(e) => {
                result.error = Some(e.to_string());
                let _ = set_peer_sync_result(db, peer.peer_device_id, Some(&e.to_string()));
            }
        }
        report.peers.push(result);
    }
    Ok(report)
}

pub fn task_event_to_envelope(ev: &TaskEvent) -> SyncEventEnvelope {
    SyncEventEnvelope {
        id: ev.id,
        task_id: ev.task_id,
        event_type: ev.event_type,
        payload: ev.payload.clone(),
        origin: ev.origin.clone(),
        created_at: ev.created_at,
        seq: ev.seq,
    }
}

pub fn make_hello(db: &Db) -> Result<HelloResponse> {
    let device = ensure_device(db, None)?;
    Ok(HelloResponse {
        protocol_version: PROTOCOL_VERSION,
        device_id: device.device_id,
        capabilities: vec!["tasks".into()],
    })
}

pub fn local_events_since(db: &Db, after_seq: i64) -> Result<Vec<SyncEventEnvelope>> {
    let events = list_task_events_since(
        db,
        ListTaskEventsSinceInput {
            after_seq,
            limit: Some(500),
        },
    )?;
    Ok(events.iter().map(task_event_to_envelope).collect())
}

#[allow(dead_code)]
pub fn latest_local_seq(db: &Db) -> Result<i64> {
    latest_event_seq(db)
}

fn authorized_get(url: &str, token: &str) -> Result<String> {
    let response = ureq::get(url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| Error::Message(format!("sync GET {url}: {e}")))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(Error::Message(format!(
            "sync GET {url}: HTTP {}",
            response.status()
        )));
    }
    response
        .into_body()
        .read_to_string()
        .map_err(|e| Error::Message(format!("sync GET body: {e}")))
}

fn authorized_post(url: &str, token: &str, body: &str) -> Result<String> {
    let response = ureq::post(url)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(body)
        .map_err(|e| Error::Message(format!("sync POST {url}: {e}")))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(Error::Message(format!(
            "sync POST {url}: HTTP {}",
            response.status()
        )));
    }
    response
        .into_body()
        .read_to_string()
        .map_err(|e| Error::Message(format!("sync POST body: {e}")))
}
