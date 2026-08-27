-- Cache date_added and summary from front matter for explorer sorting and card previews.

ALTER TABLE wiki_pages ADD COLUMN date_added_cache TEXT;
ALTER TABLE wiki_pages ADD COLUMN summary_cache TEXT;

CREATE INDEX idx_wiki_pages_date_added_cache
    ON wiki_pages (date_added_cache)
    WHERE deleted_at IS NULL AND missing_at IS NULL;
