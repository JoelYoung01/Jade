-- Peer sync: device identity, peers, LWW apply cursors

CREATE TABLE sync_device (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    device_id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE TABLE sync_peers (
    peer_device_id TEXT PRIMARY KEY NOT NULL,
    base_url TEXT NOT NULL,
    token TEXT NOT NULL,
    last_pulled_seq INTEGER NOT NULL DEFAULT 0,
    last_push_ack INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    last_sync_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE sync_applied (
    task_id TEXT PRIMARY KEY NOT NULL,
    last_event_id TEXT NOT NULL,
    last_event_at TEXT NOT NULL
);
