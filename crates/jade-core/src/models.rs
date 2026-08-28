use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Inactive,
    Active,
    Complete,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Active => "active",
            Self::Complete => "complete",
        }
    }

    pub fn parse(value: &str) -> Result<Self, crate::Error> {
        match value {
            "inactive" => Ok(Self::Inactive),
            "active" => Ok(Self::Active),
            "complete" => Ok(Self::Complete),
            other => Err(crate::Error::InvalidStatus(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub due_at: DateTime<Utc>,
    /// 5-field POSIX cron schedule. Present only on the live occurrence of a series.
    pub repeat_cron: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub tags: Vec<Tag>,
}

/// Result of a status update that may materialize the next recurring occurrence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdateResult {
    pub task: Task,
    /// Next occurrence spawned when completing a recurring task.
    pub spawned: Option<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventType {
    Created,
    Updated,
    Deleted,
}

impl TaskEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
        }
    }

    pub fn parse(value: &str) -> Result<Self, crate::Error> {
        match value {
            "created" => Ok(Self::Created),
            "updated" => Ok(Self::Updated),
            "deleted" => Ok(Self::Deleted),
            other => Err(crate::Error::InvalidEventType(other.to_owned())),
        }
    }
}

/// Default origin stamped on events written by this node.
pub const EVENT_ORIGIN_LOCAL: &str = "local";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    /// Monotonic local sequence (replication cursor).
    pub seq: i64,
    pub id: Uuid,
    pub task_id: Uuid,
    pub event_type: TaskEventType,
    pub payload: serde_json::Value,
    /// Writer attribution (`local`, peer id, agent id, …).
    pub origin: String,
    pub created_at: DateTime<Utc>,
}

/// Query filter for listing task events (newest first).
#[derive(Debug, Clone, Default)]
pub struct ListTaskEventsInput {
    /// When set, only events for this task. When `None`, all tasks.
    pub task_id: Option<Uuid>,
    /// Max rows to return (newest first). Defaults to 50 when `None`.
    pub limit: Option<u32>,
}

/// Query filter for listing task events after a sequence cursor (oldest first).
#[derive(Debug, Clone, Default)]
pub struct ListTaskEventsSinceInput {
    /// Return events with `seq > after_seq`. Use `0` (or omit) to read from the start.
    pub after_seq: i64,
    /// Max rows to return (oldest first). Defaults to 500 when `None`.
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneVisibility {
    pub inactive: bool,
    pub active: bool,
    pub complete: bool,
}

impl Default for LaneVisibility {
    fn default() -> Self {
        Self {
            inactive: true,
            active: true,
            complete: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncthingSettings {
    /// GUI/REST base URL, e.g. `http://127.0.0.1:8384`.
    pub address: String,
    pub api_key: String,
}

impl SyncthingSettings {
    pub fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSyncSettings {
    /// When true, desktop starts the sync listener while the app is open.
    pub enabled: bool,
    /// Bind address for the listener (e.g. `0.0.0.0:7421`).
    pub bind: String,
    /// Shared Bearer token for peer auth.
    pub token: String,
}

impl Default for PeerSyncSettings {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl PeerSyncSettings {
    pub fn with_defaults() -> Self {
        Self {
            enabled: false,
            bind: "0.0.0.0:7421".into(),
            token: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub lane_visibility: LaneVisibility,
    #[serde(default)]
    pub syncthing: SyncthingSettings,
    #[serde(default)]
    pub peer_sync: PeerSyncSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub due_at: DateTime<Utc>,
    pub tag_names: Vec<String>,
    /// Optional 5-field POSIX cron. Empty / whitespace clears to `None`.
    pub repeat_cron: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTaskStatusInput {
    pub id: Uuid,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RescheduleMode {
    Today,
    Tomorrow,
    NextMonday,
    FirstMondayNextMonth,
    Custom,
}

/// How to change `due_at` in a partial task update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueUpdate {
    Tomorrow,
    NextMonday,
    At(DateTime<Utc>),
}

/// How to change `repeat_cron` in a partial task update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatCronUpdate {
    /// Clear the schedule (one-off task).
    Clear,
    /// Set / replace with a validated cron string.
    Set(String),
}

/// Partial update for a task. At least one optional field must be `Some`.
#[derive(Debug, Clone)]
pub struct UpdateTaskInput {
    pub id: Uuid,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub due: Option<DueUpdate>,
    /// When `Some`, replaces the task's tags with the given names (creating tags as needed).
    pub tag_names: Option<Vec<String>>,
    pub repeat_cron: Option<RepeatCronUpdate>,
}
