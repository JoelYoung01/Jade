use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::TaskEventType;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub protocol_version: u32,
    pub device_id: Uuid,
    pub capabilities: Vec<String>,
}

/// Wire event. Sender `seq` is only used as a pull cursor for that peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEventEnvelope {
    pub id: Uuid,
    pub task_id: Uuid,
    pub event_type: TaskEventType,
    pub payload: serde_json::Value,
    pub origin: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEventsResponse {
    pub events: Vec<SyncEventEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPushBody {
    pub events: Vec<SyncEventEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPushResponse {
    pub accepted: u32,
    pub skipped: u32,
}
