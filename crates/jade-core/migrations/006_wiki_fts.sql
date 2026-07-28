-- Full-text search over wiki page title/path/tags/body (rebuildable from disk).

CREATE VIRTUAL TABLE wiki_pages_fts USING fts5(
    page_id UNINDEXED,
    title,
    rel_path,
    tags,
    body,
    tokenize = 'porter unicode61'
);
