-- Wiki: filesystem roots + page location index + P2P-friendly event log.
-- Markdown files (with YAML front matter) remain the content source of truth.

CREATE TABLE wiki_roots (
    id TEXT PRIMARY KEY NOT NULL,
    path TEXT NOT NULL,
    label TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX idx_wiki_roots_path_active
    ON wiki_roots (path)
    WHERE deleted_at IS NULL;

CREATE TABLE wiki_pages (
    id TEXT PRIMARY KEY NOT NULL,
    root_id TEXT NOT NULL REFERENCES wiki_roots(id),
    rel_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    mtime TEXT NOT NULL,
    indexed_at TEXT NOT NULL,
    missing_at TEXT,
    title_cache TEXT,
    tags_cache TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    UNIQUE (root_id, rel_path)
);

CREATE INDEX idx_wiki_pages_root_active
    ON wiki_pages (root_id)
    WHERE deleted_at IS NULL AND missing_at IS NULL;

CREATE INDEX idx_wiki_pages_title_cache
    ON wiki_pages (title_cache)
    WHERE deleted_at IS NULL AND missing_at IS NULL;

CREATE TABLE wiki_events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    id TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('root', 'page')),
    entity_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('created', 'updated', 'deleted')),
    payload TEXT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'local',
    created_at TEXT NOT NULL
);

CREATE INDEX idx_wiki_events_entity ON wiki_events (entity_type, entity_id, created_at);
CREATE INDEX idx_wiki_events_seq ON wiki_events (seq);

-- Outgoing wiki-links extracted at index time (rebuildable cache).
CREATE TABLE wiki_links (
    source_page_id TEXT NOT NULL REFERENCES wiki_pages(id) ON DELETE CASCADE,
    target_raw TEXT NOT NULL,
    PRIMARY KEY (source_page_id, target_raw)
);

CREATE INDEX idx_wiki_links_target ON wiki_links (target_raw);
