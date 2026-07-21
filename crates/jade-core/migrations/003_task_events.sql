-- Append-only event log for task mutations (create / update / delete).

CREATE TABLE task_events (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('created', 'updated', 'deleted')),
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_task_events_task_time ON task_events (task_id, created_at);
