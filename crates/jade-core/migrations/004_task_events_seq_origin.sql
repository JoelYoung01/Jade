-- Make task_events a sync-friendly replication log:
-- monotonic seq for cursors, origin for multi-peer attribution.

CREATE TABLE task_events_new (
    seq INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    id TEXT NOT NULL UNIQUE,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('created', 'updated', 'deleted')),
    payload TEXT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'local',
    created_at TEXT NOT NULL
);

INSERT INTO task_events_new (id, task_id, event_type, payload, origin, created_at)
SELECT id, task_id, event_type, payload, 'local', created_at
FROM task_events
ORDER BY created_at ASC, id ASC;

DROP TABLE task_events;
ALTER TABLE task_events_new RENAME TO task_events;

CREATE INDEX idx_task_events_task_time ON task_events (task_id, created_at);
CREATE INDEX idx_task_events_seq ON task_events (seq);
