use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid status: {0}")]
    InvalidStatus(String),

    #[error("invalid event type: {0}")]
    InvalidEventType(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("invalid due date: {0}")]
    InvalidDueAt(String),

    #[error("tag not found: {0}")]
    TagNotFound(String),

    #[error("title is required")]
    EmptyTitle,

    #[error("no fields to update")]
    NoUpdateFields,

    #[error("invalid cron schedule: {0}")]
    InvalidCron(String),

    #[error("{0}")]
    Message(String),
}
