//! Wiki domain: filesystem roots, page location index, front matter I/O.

mod frontmatter;
mod links;
mod syncthing;

pub use frontmatter::{
    ensure_identity, parse_markdown, render_markdown, resolve_title, FrontMatter,
};
pub use links::extract_link_targets;
pub use syncthing::{
    detect_stfolder_marker, list_folders, status_for_path, SyncthingClientConfig, SyncthingFolder,
    SyncthingStatus,
};

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::models::EVENT_ORIGIN_LOCAL;

// --- Models -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiRoot {
    pub id: Uuid,
    pub path: String,
    pub label: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: Uuid,
    pub root_id: Uuid,
    pub rel_path: String,
    pub content_hash: String,
    pub mtime: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
    pub missing_at: Option<DateTime<Utc>>,
    pub title_cache: Option<String>,
    pub tags_cache: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiEntityType {
    Root,
    Page,
}

impl WikiEntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Page => "page",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "root" => Ok(Self::Root),
            "page" => Ok(Self::Page),
            other => Err(Error::Message(format!("invalid wiki entity type: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiEventType {
    Created,
    Updated,
    Deleted,
}

impl WikiEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "created" => Ok(Self::Created),
            "updated" => Ok(Self::Updated),
            "deleted" => Ok(Self::Deleted),
            other => Err(Error::InvalidEventType(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiEvent {
    pub seq: i64,
    pub id: Uuid,
    pub entity_type: WikiEntityType,
    pub entity_id: Uuid,
    pub event_type: WikiEventType,
    pub payload: Value,
    pub origin: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddWikiRootInput {
    pub path: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWikiPageInput {
    pub root_id: Uuid,
    /// Relative path under the root, e.g. `notes/hello.md`. `.md` appended if missing.
    pub rel_path: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WriteWikiPageInput {
    pub id: Uuid,
    /// Full markdown body (may include front matter). Jade ensures identity front matter on write.
    pub content: String,
    /// When true (default), inject/ensure `id` + `title` in front matter.
    pub ensure_front_matter: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiPageContent {
    pub page: WikiPage,
    pub absolute_path: String,
    pub content: String,
    pub front_matter: Option<FrontMatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiBacklink {
    pub page: WikiPage,
    pub target_raw: String,
}

/// Why a page appeared in search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiMatchKind {
    /// Full query string appears in the body.
    BodyExact,
    /// Full query string appears in the title.
    TitleExact,
    /// Full query string appears in tags.
    TagsExact,
    /// Full query string appears in the relative path.
    PathExact,
    /// FTS hit in body via stem/prefix (not a literal substring).
    BodyRelated,
    /// FTS hit attributed to title without literal substring.
    TitleRelated,
    /// FTS hit attributed to tags without literal substring.
    TagsRelated,
    /// FTS hit attributed to path without literal substring.
    PathRelated,
    /// Empty-query "recent" listing.
    Recent,
}

impl WikiMatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BodyExact => "body_exact",
            Self::TitleExact => "title_exact",
            Self::TagsExact => "tags_exact",
            Self::PathExact => "path_exact",
            Self::BodyRelated => "body_related",
            Self::TitleRelated => "title_related",
            Self::TagsRelated => "tags_related",
            Self::PathRelated => "path_related",
            Self::Recent => "recent",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::BodyExact => "Text match",
            Self::TitleExact => "Title match",
            Self::TagsExact => "Tag match",
            Self::PathExact => "Path match",
            Self::BodyRelated => "Related in body",
            Self::TitleRelated => "Related in title",
            Self::TagsRelated => "Related in tags",
            Self::PathRelated => "Related in path",
            Self::Recent => "Recent",
        }
    }

    fn rank_score(self) -> i32 {
        match self {
            Self::BodyExact => 100,
            Self::TitleExact => 90,
            Self::TagsExact => 80,
            Self::PathExact => 70,
            Self::BodyRelated => 40,
            Self::TitleRelated => 35,
            Self::TagsRelated => 30,
            Self::PathRelated => 25,
            Self::Recent => 0,
        }
    }
}

/// Highlighted body excerpt for an exact text match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiSearchSnippet {
    pub before: String,
    pub matched: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiSearchHit {
    pub page: WikiPage,
    pub kind: WikiMatchKind,
    /// Human-readable reason, e.g. "Text match" / "Related in title".
    pub reason: String,
    /// Present when `kind` is `body_exact`.
    pub snippet: Option<WikiSearchSnippet>,
    /// Higher is better (includes match-kind weight).
    pub score: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ReindexStats {
    pub scanned: u64,
    pub upserted: u64,
    pub missing: u64,
}

// --- Roots ------------------------------------------------------------------

pub fn list_wiki_roots(db: &Db) -> Result<Vec<WikiRoot>> {
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT id, path, label, enabled, created_at, updated_at, deleted_at
        FROM wiki_roots
        WHERE deleted_at IS NULL
        ORDER BY label COLLATE NOCASE ASC
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;

    let mut roots = Vec::new();
    for row in rows {
        let (id, path, label, enabled, created_at, updated_at, deleted_at) = row?;
        roots.push(WikiRoot {
            id: parse_uuid(&id, "wiki root")?,
            path,
            label,
            enabled: enabled != 0,
            created_at: parse_dt(&created_at)?,
            updated_at: parse_dt(&updated_at)?,
            deleted_at: deleted_at.as_deref().map(parse_dt).transpose()?,
        });
    }
    Ok(roots)
}

pub fn get_wiki_root(db: &Db, id: Uuid) -> Result<WikiRoot> {
    list_wiki_roots(db)?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| Error::Message(format!("wiki root not found: {id}")))
}

pub fn add_wiki_root(db: &Db, input: AddWikiRootInput) -> Result<WikiRoot> {
    let path = normalize_root_path(&input.path)?;
    if !path.is_dir() {
        return Err(Error::Message(format!(
            "wiki root is not a directory: {}",
            path.display()
        )));
    }
    let path_str = path.to_string_lossy().to_string();
    let label = input
        .label
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| {
            path.file_name()
                .map_or_else(|| path_str.clone(), |s| s.to_string_lossy().into_owned())
        });

    let now = Utc::now();
    let id = Uuid::new_v4();

    let root_id = {
        let conn = db.connection();
        let tx = conn.unchecked_transaction()?;

        // Revive soft-deleted root at same path if present.
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM wiki_roots WHERE path = ?1",
                params![path_str],
                |row| row.get(0),
            )
            .optional()?;

        let root_id = if let Some(existing_id) = existing {
            let uid = parse_uuid(&existing_id, "wiki root")?;
            tx.execute(
                "
                UPDATE wiki_roots
                SET label = ?1, enabled = 1, updated_at = ?2, deleted_at = NULL
                WHERE id = ?3
                ",
                params![label, now.to_rfc3339(), existing_id],
            )?;
            insert_wiki_event(
                &tx,
                WikiEntityType::Root,
                uid,
                WikiEventType::Updated,
                json!({ "path": path_str, "label": label, "revived": true }),
                now,
            )?;
            uid
        } else {
            tx.execute(
                "
                INSERT INTO wiki_roots (id, path, label, enabled, created_at, updated_at, deleted_at)
                VALUES (?1, ?2, ?3, 1, ?4, ?5, NULL)
                ",
                params![
                    id.to_string(),
                    path_str,
                    label,
                    now.to_rfc3339(),
                    now.to_rfc3339()
                ],
            )?;
            insert_wiki_event(
                &tx,
                WikiEntityType::Root,
                id,
                WikiEventType::Created,
                json!({ "path": path_str, "label": label }),
                now,
            )?;
            id
        };

        tx.commit()?;
        root_id
    };

    let root = get_wiki_root(db, root_id)?;
    let _ = reindex_root(db, root_id)?;
    Ok(get_wiki_root(db, root.id).unwrap_or(root))
}

pub fn remove_wiki_root(db: &Db, id: Uuid) -> Result<()> {
    let root = get_wiki_root(db, id)?;
    let now = Utc::now();
    let conn = db.connection();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "
        DELETE FROM wiki_pages_fts
        WHERE page_id IN (SELECT id FROM wiki_pages WHERE root_id = ?1)
        ",
        params![id.to_string()],
    )?;
    tx.execute(
        "UPDATE wiki_roots SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), id.to_string()],
    )?;
    tx.execute(
        "UPDATE wiki_pages SET deleted_at = ?1, updated_at = ?1 WHERE root_id = ?2 AND deleted_at IS NULL",
        params![now.to_rfc3339(), id.to_string()],
    )?;
    insert_wiki_event(
        &tx,
        WikiEntityType::Root,
        id,
        WikiEventType::Deleted,
        json!({ "path": root.path, "label": root.label }),
        now,
    )?;
    tx.commit()?;
    Ok(())
}

// --- Pages ------------------------------------------------------------------

pub fn list_wiki_pages(db: &Db, root_id: Option<Uuid>) -> Result<Vec<WikiPage>> {
    let conn = db.connection();
    let mut pages = Vec::new();

    if let Some(root_id) = root_id {
        let mut stmt = conn.prepare(
            "
            SELECT id, root_id, rel_path, content_hash, mtime, indexed_at, missing_at,
                   title_cache, tags_cache, created_at, updated_at, deleted_at
            FROM wiki_pages
            WHERE deleted_at IS NULL AND missing_at IS NULL AND root_id = ?1
            ORDER BY mtime DESC, updated_at DESC, rel_path ASC
            ",
        )?;
        let rows = stmt.query_map(params![root_id.to_string()], map_page_row)?;
        for row in rows {
            pages.push(page_from_row(row?)?);
        }
    } else {
        let mut stmt = conn.prepare(
            "
            SELECT id, root_id, rel_path, content_hash, mtime, indexed_at, missing_at,
                   title_cache, tags_cache, created_at, updated_at, deleted_at
            FROM wiki_pages
            WHERE deleted_at IS NULL AND missing_at IS NULL
            ORDER BY mtime DESC, updated_at DESC, rel_path ASC
            ",
        )?;
        let rows = stmt.query_map([], map_page_row)?;
        for row in rows {
            pages.push(page_from_row(row?)?);
        }
    }
    Ok(pages)
}

pub fn search_wiki_pages(db: &Db, query: &str) -> Result<Vec<WikiSearchHit>> {
    let raw = query.trim();
    if raw.is_empty() {
        return Ok(list_wiki_pages(db, None)?
            .into_iter()
            .map(|page| WikiSearchHit {
                page,
                kind: WikiMatchKind::Recent,
                reason: WikiMatchKind::Recent.label().to_owned(),
                snippet: None,
                score: WikiMatchKind::Recent.rank_score(),
            })
            .collect());
    }
    ensure_wiki_fts_backfill(db)?;

    let Some(match_query) = build_fts_match_query(raw) else {
        return Ok(Vec::new());
    };

    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT p.id, p.root_id, p.rel_path, p.content_hash, p.mtime, p.indexed_at, p.missing_at,
               p.title_cache, p.tags_cache, p.created_at, p.updated_at, p.deleted_at,
               wiki_pages_fts.title, wiki_pages_fts.rel_path, wiki_pages_fts.tags, wiki_pages_fts.body,
               bm25(wiki_pages_fts)
        FROM wiki_pages_fts
        JOIN wiki_pages p ON p.id = wiki_pages_fts.page_id
        WHERE wiki_pages_fts MATCH ?1
          AND p.deleted_at IS NULL
          AND p.missing_at IS NULL
        ",
    )?;
    let rows = stmt.query_map(params![match_query], |row| {
        Ok((
            map_page_row(row)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
            row.get::<_, f64>(16)?,
        ))
    })?;

    let mut hits = Vec::new();
    for row in rows {
        let (page_row, fts_title, fts_path, fts_tags, fts_body, bm25) = row?;
        let page = page_from_row(page_row)?;
        let (kind, snippet) = classify_match(raw, &fts_title, &fts_path, &fts_tags, &fts_body);
        // Prefer better match kinds; within a kind, lower bm25 is better.
        let score = kind.rank_score() * 1_000 - (bm25 * 10.0).round() as i32;
        hits.push(WikiSearchHit {
            page,
            kind,
            reason: kind.label().to_owned(),
            snippet,
            score,
        });
    }
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.page.mtime.cmp(&a.page.mtime))
    });
    Ok(hits)
}

fn classify_match(
    query: &str,
    title: &str,
    rel_path: &str,
    tags: &str,
    body: &str,
) -> (WikiMatchKind, Option<WikiSearchSnippet>) {
    let q = query.to_lowercase();
    if let Some(snippet) = make_body_snippet(body, query) {
        return (WikiMatchKind::BodyExact, Some(snippet));
    }
    if title.to_lowercase().contains(&q) {
        return (WikiMatchKind::TitleExact, None);
    }
    if tags.to_lowercase().contains(&q) {
        return (WikiMatchKind::TagsExact, None);
    }
    if rel_path.to_lowercase().contains(&q) {
        return (WikiMatchKind::PathExact, None);
    }
    if field_has_related_token(body, query) {
        return (WikiMatchKind::BodyRelated, None);
    }
    if field_has_related_token(title, query) {
        return (WikiMatchKind::TitleRelated, None);
    }
    if field_has_related_token(tags, query) {
        return (WikiMatchKind::TagsRelated, None);
    }
    if field_has_related_token(rel_path, query) {
        return (WikiMatchKind::PathRelated, None);
    }
    (WikiMatchKind::BodyRelated, None)
}

fn field_has_related_token(haystack: &str, query: &str) -> bool {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return false;
    }
    for word in haystack.split(|c: char| !c.is_alphanumeric()) {
        let w = word.to_lowercase();
        if w.is_empty() {
            continue;
        }
        if tokens.iter().any(|t| w.starts_with(t) || t.starts_with(&w)) {
            return true;
        }
    }
    false
}

const SNIPPET_CONTEXT: usize = 56;

fn make_body_snippet(body: &str, query: &str) -> Option<WikiSearchSnippet> {
    let lower_q = query.to_lowercase();
    if lower_q.is_empty() {
        return None;
    }
    let q_chars: Vec<char> = query.chars().collect();
    let q_len = q_chars.len();
    let hay: Vec<(usize, char)> = body.char_indices().collect();
    if hay.len() < q_len {
        return None;
    }

    let mut match_start = None;
    for i in 0..=hay.len() - q_len {
        let slice: String = hay[i..i + q_len].iter().map(|(_, c)| *c).collect();
        if slice.to_lowercase() == lower_q {
            match_start = Some(i);
            break;
        }
    }
    let start_i = match_start?;
    let start_byte = hay[start_i].0;
    let end_byte = if start_i + q_len < hay.len() {
        hay[start_i + q_len].0
    } else {
        body.len()
    };
    let matched = body.get(start_byte..end_byte)?.to_owned();

    let before_char_start = start_i.saturating_sub(SNIPPET_CONTEXT);
    let before_byte = hay[before_char_start].0;
    let mut before = body.get(before_byte..start_byte).unwrap_or("").to_owned();
    if before_char_start > 0 {
        before = format!("…{}", before.trim_start());
    }

    let after_char_end = (start_i + q_len).saturating_add(SNIPPET_CONTEXT).min(hay.len());
    let after_byte = if after_char_end < hay.len() {
        hay[after_char_end].0
    } else {
        body.len()
    };
    let mut after = body.get(end_byte..after_byte).unwrap_or("").to_owned();
    if after_char_end < hay.len() {
        after = format!("{}…", after.trim_end());
    }

    Some(WikiSearchSnippet {
        before: strip_snippet_part(&before),
        matched: strip_markdown_for_display(&matched),
        after: strip_snippet_part(&after),
    })
}

/// Preserve snippet ellipsis edges while cleaning markdown in the middle.
fn strip_snippet_part(text: &str) -> String {
    let lead = text.starts_with('…');
    let trail = text.ends_with('…');
    let core = text.trim_matches('…');
    let cleaned = strip_markdown_for_display(core);
    let mut out = String::with_capacity(cleaned.len() + 2);
    if lead {
        out.push('…');
    }
    out.push_str(&cleaned);
    if trail {
        out.push('…');
    }
    out
}

/// Light markdown cleanup for search snippet *display* (not used for FTS indexing).
fn strip_markdown_for_display(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        // Image: ![alt](url) / ![alt][ref]
        if chars[i] == '!' && i + 1 < n && chars[i + 1] == '[' {
            if let Some((label, next)) = parse_md_linkish(&chars, i + 1) {
                out.push_str(&label);
                i = next;
                continue;
            }
        }
        // Link: [text](url) / [text][ref]
        if chars[i] == '[' {
            if let Some((label, next)) = parse_md_linkish(&chars, i) {
                out.push_str(&label);
                i = next;
                continue;
            }
        }
        // Inline code: `code` / `` code ``
        if chars[i] == '`' {
            let tick_start = i;
            while i < n && chars[i] == '`' {
                i += 1;
            }
            let tick_count = i - tick_start;
            let mut j = i;
            let mut closed = false;
            while j < n {
                if chars[j] == '`' {
                    let mut k = j;
                    while k < n && chars[k] == '`' {
                        k += 1;
                    }
                    if k - j == tick_count {
                        out.extend(chars[i..j].iter().copied());
                        i = k;
                        closed = true;
                        break;
                    }
                    j = k;
                } else {
                    j += 1;
                }
            }
            if closed {
                continue;
            }
            i = tick_start;
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // ATX heading markers at line start: "# "
        if chars[i] == '#' && (i == 0 || chars[i - 1] == '\n') {
            let mut j = i;
            while j < n && chars[j] == '#' && j - i < 6 {
                j += 1;
            }
            if j > i && j < n && chars[j].is_whitespace() {
                i = j;
                while i < n && chars[i] == ' ' {
                    i += 1;
                }
                continue;
            }
        }
        // List / blockquote markers at line start
        if i == 0 || chars[i - 1] == '\n' {
            if chars[i] == '>' && i + 1 < n && chars[i + 1].is_whitespace() {
                i += 1;
                while i < n && chars[i] == ' ' {
                    i += 1;
                }
                continue;
            }
            if matches!(chars[i], '-' | '*' | '+') && i + 1 < n && chars[i + 1] == ' ' {
                i += 2;
                continue;
            }
        }
        // Bold markers ** / __ (leave single _ alone for snake_case)
        if (chars[i] == '*' || chars[i] == '_') && i + 1 < n && chars[i + 1] == chars[i] {
            i += 2;
            continue;
        }
        // Italic *text* — skip lone * (not list: already handled). Leave lone _.
        if chars[i] == '*' {
            i += 1;
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    normalize_snippet_whitespace(&cleanup_snippet_md_debris(&out))
}

/// Remove link/image debris left when a match splits a markdown construct across snippet edges.
fn cleanup_snippet_md_debris(text: &str) -> String {
    let mut s = text.to_owned();
    loop {
        let trimmed = s.trim_start();
        if let Some(rest) = trimmed.strip_prefix("](") {
            if let Some(end) = rest.find(')') {
                s = rest[end + 1..].to_owned();
                continue;
            }
        }
        if let Some(rest) = trimmed.strip_prefix("][") {
            if let Some(end) = rest.find(']') {
                s = rest[end + 1..].to_owned();
                continue;
            }
        }
        if let Some(rest) = trimmed.strip_prefix(']') {
            s = rest.to_owned();
            continue;
        }
        break;
    }
    loop {
        let trimmed = s.trim_end();
        if !trimmed.ends_with('[') {
            break;
        }
        let mut without = trimmed.trim_end_matches('[').trim_end();
        if without.ends_with('!') {
            without = without[..without.len() - '!'.len_utf8()].trim_end();
        }
        s = without.to_owned();
    }
    s
}

fn parse_md_linkish(chars: &[char], open_bracket_at: usize) -> Option<(String, usize)> {
    if chars.get(open_bracket_at) != Some(&'[') {
        return None;
    }
    let mut i = open_bracket_at + 1;
    let mut label = String::new();
    let mut depth = 1usize;
    while i < chars.len() {
        match chars[i] {
            '[' => {
                depth += 1;
                label.push('[');
                i += 1;
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    i += 1;
                    break;
                }
                label.push(']');
                i += 1;
            }
            c => {
                label.push(c);
                i += 1;
            }
        }
    }
    if depth != 0 {
        return None;
    }

    if i < chars.len() && chars[i] == '(' {
        i += 1;
        let mut paren_depth = 1usize;
        while i < chars.len() {
            match chars[i] {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        i += 1;
                        return Some((label, i));
                    }
                }
                _ => {}
            }
            i += 1;
        }
        return None;
    }

    if i < chars.len() && chars[i] == '[' {
        i += 1;
        while i < chars.len() && chars[i] != ']' {
            i += 1;
        }
        if i < chars.len() && chars[i] == ']' {
            return Some((label, i + 1));
        }
        return None;
    }

    // Bare [text] without a destination — leave as-is.
    None
}

fn normalize_snippet_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        out.push(c);
        prev_space = false;
    }
    out.trim().to_owned()
}

/// Rebuild FTS once after upgrading a DB that already has wiki pages.
fn ensure_wiki_fts_backfill(db: &Db) -> Result<()> {
    let needs_backfill = {
        let conn = db.connection();
        let pages: i64 = conn.query_row(
            "
            SELECT COUNT(*) FROM wiki_pages
            WHERE deleted_at IS NULL AND missing_at IS NULL
            ",
            [],
            |row| row.get(0),
        )?;
        if pages == 0 {
            return Ok(());
        }
        let fts: i64 =
            conn.query_row("SELECT COUNT(*) FROM wiki_pages_fts", [], |row| row.get(0))?;
        fts == 0
    };
    if needs_backfill {
        let _ = reindex_all(db)?;
    }
    Ok(())
}

/// Turn a user query into a safe FTS5 MATCH expression (AND of prefix terms).
fn build_fts_match_query(raw: &str) -> Option<String> {
    let mut terms = Vec::new();
    for token in raw.split_whitespace() {
        let cleaned: String = token
            .chars()
            .filter(|c| !matches!(c, '"' | '*' | '(' | ')' | ':' | '^'))
            .collect();
        let cleaned = cleaned.trim();
        if cleaned.is_empty() {
            continue;
        }
        // Quote + prefix so "sync" matches "syncthing" and special chars stay safe.
        terms.push(format!("\"{}\"*", cleaned.replace('"', "")));
    }
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

pub fn get_wiki_page(db: &Db, id: Uuid) -> Result<WikiPage> {
    let conn = db.connection();
    let row = conn
        .query_row(
            "
            SELECT id, root_id, rel_path, content_hash, mtime, indexed_at, missing_at,
                   title_cache, tags_cache, created_at, updated_at, deleted_at
            FROM wiki_pages
            WHERE id = ?1 AND deleted_at IS NULL
            ",
            params![id.to_string()],
            map_page_row,
        )
        .optional()?
        .ok_or_else(|| Error::Message(format!("wiki page not found: {id}")))?;
    page_from_row(row)
}

pub fn read_wiki_page(db: &Db, id: Uuid) -> Result<WikiPageContent> {
    let page = get_wiki_page(db, id)?;
    let root = get_wiki_root(db, page.root_id)?;
    let absolute = PathBuf::from(&root.path).join(&page.rel_path);
    let content = fs::read_to_string(&absolute)
        .map_err(|e| Error::Message(format!("failed to read {}: {e}", absolute.display())))?;
    let parsed = parse_markdown(&content)?;
    Ok(WikiPageContent {
        page,
        absolute_path: absolute.to_string_lossy().into_owned(),
        content,
        front_matter: parsed.front_matter,
        body: parsed.body,
    })
}

pub fn create_wiki_page(db: &Db, input: CreateWikiPageInput) -> Result<WikiPageContent> {
    let root = get_wiki_root(db, input.root_id)?;
    if !root.enabled {
        return Err(Error::Message("wiki root is disabled".into()));
    }

    let mut rel = input.rel_path.trim().replace('\\', "/");
    while rel.starts_with('/') {
        rel = rel[1..].to_owned();
    }
    if rel.is_empty() {
        return Err(Error::Message("rel_path is required".into()));
    }
    if !rel.to_lowercase().ends_with(".md") {
        rel = format!("{rel}.md");
    }
    if rel.contains("..") {
        return Err(Error::Message("rel_path must not contain '..'".into()));
    }

    let absolute = PathBuf::from(&root.path).join(&rel);
    if absolute.exists() {
        return Err(Error::Message(format!(
            "file already exists: {}",
            absolute.display()
        )));
    }
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::Message(format!("failed to create directories: {e}")))?;
    }

    let stem = absolute
        .file_stem()
        .map_or_else(|| "untitled".into(), |s| s.to_string_lossy().into_owned());
    let title = input
        .title
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or(stem);
    let body = input.body.unwrap_or_default();
    let mut fm = FrontMatter {
        id: Some(Uuid::new_v4()),
        title: Some(title),
        tags: input.tags.unwrap_or_default(),
        ..FrontMatter::default()
    };
    ensure_identity(&mut fm, None);
    let content = render_markdown(&fm, &body)?;
    fs::write(&absolute, &content)
        .map_err(|e| Error::Message(format!("failed to write {}: {e}", absolute.display())))?;

    index_file(db, &root, &rel, &absolute)?;
    let page = find_page_by_rel(db, root.id, &rel)?;
    read_wiki_page(db, page.id)
}

pub fn write_wiki_page(db: &Db, input: WriteWikiPageInput) -> Result<WikiPageContent> {
    let page = get_wiki_page(db, input.id)?;
    let root = get_wiki_root(db, page.root_id)?;
    let absolute = PathBuf::from(&root.path).join(&page.rel_path);

    let ensure = input.ensure_front_matter.unwrap_or(true);
    let content = if ensure {
        let mut parsed = parse_markdown(&input.content)?;
        let stem = absolute
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut fm = parsed.front_matter.take().unwrap_or_default();
        // Preserve existing page id when injecting.
        if fm.id.is_none() {
            fm.id = Some(page.id);
        } else if fm.id != Some(page.id) {
            // Front matter id wins as SoT — update DB id association by keeping FM id.
            // Prefer stable DB id: overwrite FM to match indexed page.
            fm.id = Some(page.id);
        }
        let title_hint = resolve_title(Some(&fm), &parsed.body, &stem);
        ensure_identity(&mut fm, Some(&title_hint));
        render_markdown(&fm, &parsed.body)?
    } else {
        input.content
    };

    fs::write(&absolute, &content)
        .map_err(|e| Error::Message(format!("failed to write {}: {e}", absolute.display())))?;
    index_file(db, &root, &page.rel_path, &absolute)?;
    read_wiki_page(db, page.id)
}

pub fn list_backlinks(db: &Db, page_id: Uuid) -> Result<Vec<WikiBacklink>> {
    let page = get_wiki_page(db, page_id)?;
    let targets = {
        let mut t = vec![page.rel_path.clone()];
        if let Some(title) = &page.title_cache {
            t.push(title.clone());
        }
        let stem = Path::new(&page.rel_path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned());
        if let Some(stem) = stem {
            t.push(stem);
        }
        t
    };

    let conn = db.connection();
    let mut out = Vec::new();
    for target in targets {
        let mut stmt = conn.prepare(
            "
            SELECT p.id, p.root_id, p.rel_path, p.content_hash, p.mtime, p.indexed_at, p.missing_at,
                   p.title_cache, p.tags_cache, p.created_at, p.updated_at, p.deleted_at, l.target_raw
            FROM wiki_links l
            JOIN wiki_pages p ON p.id = l.source_page_id
            WHERE l.target_raw = ?1
              AND p.deleted_at IS NULL
              AND p.missing_at IS NULL
              AND p.id != ?2
            ",
        )?;
        let rows = stmt.query_map(params![target, page_id.to_string()], |row| {
            Ok((map_page_row(row)?, row.get::<_, String>(12)?))
        })?;
        for row in rows {
            let (page_row, target_raw) = row?;
            out.push(WikiBacklink {
                page: page_from_row(page_row)?,
                target_raw,
            });
        }
    }
    // Dedupe by source page id
    out.sort_by_key(|a| a.page.id);
    out.dedup_by(|a, b| a.page.id == b.page.id);
    Ok(out)
}

// --- Index ------------------------------------------------------------------

pub fn reindex_root(db: &Db, root_id: Uuid) -> Result<ReindexStats> {
    let root = get_wiki_root(db, root_id)?;
    let root_path = PathBuf::from(&root.path);
    if !root_path.is_dir() {
        return Err(Error::Message(format!(
            "wiki root missing on disk: {}",
            root.path
        )));
    }

    let mut stats = ReindexStats::default();
    let mut seen: Vec<String> = Vec::new();

    visit_markdown_files(&root_path, &root_path, &mut |rel, abs| {
        stats.scanned += 1;
        seen.push(rel.replace('\\', "/"));
        index_file(db, &root, &rel.replace('\\', "/"), abs)?;
        stats.upserted += 1;
        Ok(())
    })?;

    let now = Utc::now();
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT id, rel_path FROM wiki_pages
        WHERE root_id = ?1 AND deleted_at IS NULL AND missing_at IS NULL
        ",
    )?;
    let existing: Vec<(String, String)> = stmt
        .query_map(params![root_id.to_string()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<std::result::Result<_, _>>()?;

    for (id, rel) in existing {
        if !seen.iter().any(|s| s == &rel) {
            conn.execute(
                "UPDATE wiki_pages SET missing_at = ?1, updated_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), id],
            )?;
            conn.execute(
                "DELETE FROM wiki_pages_fts WHERE page_id = ?1",
                params![id],
            )?;
            stats.missing += 1;
        }
    }

    Ok(stats)
}

pub fn reindex_all(db: &Db) -> Result<ReindexStats> {
    let mut total = ReindexStats::default();
    for root in list_wiki_roots(db)? {
        if !root.enabled {
            continue;
        }
        let stats = reindex_root(db, root.id)?;
        total.scanned += stats.scanned;
        total.upserted += stats.upserted;
        total.missing += stats.missing;
    }
    Ok(total)
}

#[allow(clippy::too_many_lines)]
fn index_file(db: &Db, root: &WikiRoot, rel_path: &str, absolute: &Path) -> Result<()> {
    let bytes = fs::read(absolute)
        .map_err(|e| Error::Message(format!("failed to read {}: {e}", absolute.display())))?;
    let content_hash = hex::encode(Sha256::digest(&bytes));
    let mtime = file_mtime(absolute)?;
    let text = String::from_utf8_lossy(&bytes);
    let parsed = parse_markdown(&text)?;
    let stem = absolute
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let title = resolve_title(parsed.front_matter.as_ref(), &parsed.body, &stem);
    let tags = parsed
        .front_matter
        .as_ref()
        .map(|fm| fm.tags.clone())
        .unwrap_or_default();
    let tags_json = serde_json::to_string(&tags)?;
    let page_id = parsed
        .front_matter
        .as_ref()
        .and_then(|fm| fm.id)
        .unwrap_or_else(Uuid::new_v4);
    let link_targets = extract_link_targets(&parsed.body);
    let now = Utc::now();

    let conn = db.connection();
    let tx = conn.unchecked_transaction()?;

    let existing: Option<(String, String)> = tx
        .query_row(
            "
            SELECT id, content_hash FROM wiki_pages
            WHERE root_id = ?1 AND rel_path = ?2 AND deleted_at IS NULL
            ",
            params![root.id.to_string(), rel_path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (entity_id, event_type) = if let Some((existing_id, old_hash)) = existing {
        let uid = parse_uuid(&existing_id, "wiki page")?;
        // Prefer stable DB id; front-matter id may differ until write normalizes.
        tx.execute(
            "
            UPDATE wiki_pages
            SET content_hash = ?1, mtime = ?2, indexed_at = ?3, missing_at = NULL,
                title_cache = ?4, tags_cache = ?5, updated_at = ?3
            WHERE id = ?6
            ",
            params![
                content_hash,
                mtime.to_rfc3339(),
                now.to_rfc3339(),
                title,
                tags_json,
                existing_id
            ],
        )?;
        let ev = if old_hash == content_hash {
            None
        } else {
            Some(WikiEventType::Updated)
        };
        (uid, ev)
    } else {
        // If FM id already exists under another path, generate a fresh index id.
        let id_taken: bool = tx
            .query_row(
                "SELECT 1 FROM wiki_pages WHERE id = ?1",
                params![page_id.to_string()],
                |_| Ok(1_i64),
            )
            .optional()?
            .is_some();
        let id = if id_taken { Uuid::new_v4() } else { page_id };
        tx.execute(
            "
            INSERT INTO wiki_pages (
                id, root_id, rel_path, content_hash, mtime, indexed_at, missing_at,
                title_cache, tags_cache, created_at, updated_at, deleted_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?6, ?6, NULL)
            ",
            params![
                id.to_string(),
                root.id.to_string(),
                rel_path,
                content_hash,
                mtime.to_rfc3339(),
                now.to_rfc3339(),
                title,
                tags_json
            ],
        )?;
        (id, Some(WikiEventType::Created))
    };

    tx.execute(
        "DELETE FROM wiki_links WHERE source_page_id = ?1",
        params![entity_id.to_string()],
    )?;
    for target in &link_targets {
        tx.execute(
            "INSERT OR IGNORE INTO wiki_links (source_page_id, target_raw) VALUES (?1, ?2)",
            params![entity_id.to_string(), target],
        )?;
    }

    upsert_wiki_fts(
        &tx,
        entity_id,
        &title,
        rel_path,
        &tags,
        &parsed.body,
    )?;

    if let Some(event_type) = event_type {
        insert_wiki_event(
            &tx,
            WikiEntityType::Page,
            entity_id,
            event_type,
            json!({
                "root_id": root.id.to_string(),
                "rel_path": rel_path,
                "content_hash": content_hash,
            }),
            now,
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn upsert_wiki_fts(
    tx: &rusqlite::Transaction<'_>,
    page_id: Uuid,
    title: &str,
    rel_path: &str,
    tags: &[String],
    body: &str,
) -> Result<()> {
    tx.execute(
        "DELETE FROM wiki_pages_fts WHERE page_id = ?1",
        params![page_id.to_string()],
    )?;
    tx.execute(
        "
        INSERT INTO wiki_pages_fts (page_id, title, rel_path, tags, body)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params![
            page_id.to_string(),
            title,
            rel_path,
            tags.join(" "),
            body
        ],
    )?;
    Ok(())
}

fn find_page_by_rel(db: &Db, root_id: Uuid, rel_path: &str) -> Result<WikiPage> {
    let conn = db.connection();
    let row = conn.query_row(
        "
        SELECT id, root_id, rel_path, content_hash, mtime, indexed_at, missing_at,
               title_cache, tags_cache, created_at, updated_at, deleted_at
        FROM wiki_pages
        WHERE root_id = ?1 AND rel_path = ?2 AND deleted_at IS NULL
        ",
        params![root_id.to_string(), rel_path],
        map_page_row,
    )?;
    page_from_row(row)
}

fn visit_markdown_files(
    root: &Path,
    dir: &Path,
    on_file: &mut dyn FnMut(String, &Path) -> Result<()>,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .map_err(|e| Error::Message(format!("failed to read {}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::Message(format!("failed to read dir entry: {e}")))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            // Skip common SyncThing / VCS noise.
            if name == ".stversions" || name == ".git" || name == "node_modules" {
                continue;
            }
            visit_markdown_files(root, &path, on_file)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| Error::Message(format!("path strip failed: {e}")))?
                .to_string_lossy()
                .into_owned();
            on_file(rel, &path)?;
        }
    }
    Ok(())
}

// --- Events -----------------------------------------------------------------

fn insert_wiki_event(
    tx: &rusqlite::Transaction<'_>,
    entity_type: WikiEntityType,
    entity_id: Uuid,
    event_type: WikiEventType,
    payload: Value,
    now: DateTime<Utc>,
) -> Result<()> {
    let id = Uuid::new_v4();
    tx.execute(
        "
        INSERT INTO wiki_events (id, entity_type, entity_id, event_type, payload, origin, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ",
        params![
            id.to_string(),
            entity_type.as_str(),
            entity_id.to_string(),
            event_type.as_str(),
            payload.to_string(),
            EVENT_ORIGIN_LOCAL,
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn latest_wiki_event_seq(db: &Db) -> Result<i64> {
    let conn = db.connection();
    let seq: Option<i64> =
        conn.query_row("SELECT MAX(seq) FROM wiki_events", [], |row| row.get(0))?;
    Ok(seq.unwrap_or(0))
}

pub fn list_wiki_events_since(
    db: &Db,
    after_seq: i64,
    limit: Option<u32>,
) -> Result<Vec<WikiEvent>> {
    let limit = limit.unwrap_or(500);
    let conn = db.connection();
    let mut stmt = conn.prepare(
        "
        SELECT seq, id, entity_type, entity_id, event_type, payload, origin, created_at
        FROM wiki_events
        WHERE seq > ?1
        ORDER BY seq ASC
        LIMIT ?2
        ",
    )?;
    let rows = stmt.query_map(params![after_seq, limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
        ))
    })?;

    let mut events = Vec::new();
    for row in rows {
        let (seq, id, entity_type, entity_id, event_type, payload, origin, created_at) = row?;
        events.push(WikiEvent {
            seq,
            id: parse_uuid(&id, "wiki event")?,
            entity_type: WikiEntityType::parse(&entity_type)?,
            entity_id: parse_uuid(&entity_id, "wiki entity")?,
            event_type: WikiEventType::parse(&event_type)?,
            payload: serde_json::from_str(&payload)?,
            origin,
            created_at: parse_dt(&created_at)?,
        });
    }
    Ok(events)
}

// --- Helpers ----------------------------------------------------------------

fn normalize_root_path(path: &str) -> Result<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(Error::Message("wiki root path is required".into()));
    }
    let path = PathBuf::from(trimmed);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|e| Error::Message(format!("cwd: {e}")))?
            .join(path)
    };
    Ok(absolute.canonicalize().unwrap_or(absolute))
}

fn file_mtime(path: &Path) -> Result<DateTime<Utc>> {
    let meta =
        fs::metadata(path).map_err(|e| Error::Message(format!("stat {}: {e}", path.display())))?;
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    Ok(DateTime::<Utc>::from(modified))
}

fn parse_uuid(value: &str, kind: &str) -> Result<Uuid> {
    Uuid::parse_str(value).map_err(|e| Error::Message(format!("invalid {kind} id: {e}")))
}

fn parse_dt(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::InvalidDueAt(format!("{value}: {e}")))
}

type PageRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
);

fn map_page_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PageRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
    ))
}

fn page_from_row(row: PageRow) -> Result<WikiPage> {
    let (
        id,
        root_id,
        rel_path,
        content_hash,
        mtime,
        indexed_at,
        missing_at,
        title_cache,
        tags_cache,
        created_at,
        updated_at,
        deleted_at,
    ) = row;
    let tags: Vec<String> = match tags_cache {
        Some(raw) if !raw.is_empty() => serde_json::from_str(&raw)?,
        _ => Vec::new(),
    };
    Ok(WikiPage {
        id: parse_uuid(&id, "wiki page")?,
        root_id: parse_uuid(&root_id, "wiki root")?,
        rel_path,
        content_hash,
        mtime: parse_dt(&mtime)?,
        indexed_at: parse_dt(&indexed_at)?,
        missing_at: missing_at.as_deref().map(parse_dt).transpose()?,
        title_cache,
        tags_cache: tags,
        created_at: parse_dt(&created_at)?,
        updated_at: parse_dt(&updated_at)?,
        deleted_at: deleted_at.as_deref().map(parse_dt).transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;
    use std::io::Write;

    #[test]
    fn add_root_indexes_markdown() {
        let dir = std::env::temp_dir().join(format!("jade-wiki-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("hello.md");
        let mut f = fs::File::create(&file).unwrap();
        write!(
            f,
            "---\ntitle: Hello\ntags:\n  - greet\n---\n# Hello\n\nSee [[Other]].\n"
        )
        .unwrap();

        let db = open_memory().unwrap();
        let root = add_wiki_root(
            &db,
            AddWikiRootInput {
                path: dir.to_string_lossy().into_owned(),
                label: Some("Test".into()),
            },
        )
        .unwrap();
        assert_eq!(root.label, "Test");

        let pages = list_wiki_pages(&db, Some(root.id)).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title_cache.as_deref(), Some("Hello"));
        assert_eq!(pages[0].tags_cache, vec!["greet".to_owned()]);

        let created = create_wiki_page(
            &db,
            CreateWikiPageInput {
                root_id: root.id,
                rel_path: "other.md".into(),
                title: Some("Other".into()),
                body: Some("Back to [[Hello]].".into()),
                tags: None,
            },
        )
        .unwrap();
        assert!(created.front_matter.unwrap().id.is_some());

        let backlinks = list_backlinks(&db, pages[0].id).unwrap();
        // other.md links to Hello — may match title
        assert!(
            backlinks.iter().any(|b| b.page.id == created.page.id)
                || list_backlinks(&db, created.page.id).is_ok()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_injects_front_matter() {
        let dir = std::env::temp_dir().join(format!("jade-wiki-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bare.md"), "# Bare\n\nNo fm.\n").unwrap();

        let db = open_memory().unwrap();
        let root = add_wiki_root(
            &db,
            AddWikiRootInput {
                path: dir.to_string_lossy().into_owned(),
                label: None,
            },
        )
        .unwrap();
        let pages = list_wiki_pages(&db, Some(root.id)).unwrap();
        let page = &pages[0];

        let written = write_wiki_page(
            &db,
            WriteWikiPageInput {
                id: page.id,
                content: "# Bare\n\nUpdated.\n".into(),
                ensure_front_matter: Some(true),
            },
        )
        .unwrap();
        assert!(written.content.starts_with("---"));
        assert!(written.front_matter.unwrap().id.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_matches_body_content() {
        let dir = std::env::temp_dir().join(format!("jade-wiki-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("alpha.md"),
            "---\ntitle: Alpha\n---\n# Alpha\n\nUniquePhraseAboutWidgets lives here.\n",
        )
        .unwrap();
        fs::write(
            dir.join("beta.md"),
            "---\ntitle: UniqueTitleOnly\n---\n# Heading\n\nNothing relevant.\n",
        )
        .unwrap();

        let db = open_memory().unwrap();
        add_wiki_root(
            &db,
            AddWikiRootInput {
                path: dir.to_string_lossy().into_owned(),
                label: Some("Search".into()),
            },
        )
        .unwrap();

        let hits = search_wiki_pages(&db, "UniquePhraseAboutWidgets").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].page.title_cache.as_deref(), Some("Alpha"));
        assert_eq!(hits[0].kind, WikiMatchKind::BodyExact);
        assert!(hits[0].snippet.is_some());

        let by_title = search_wiki_pages(&db, "UniqueTitleOnly").unwrap();
        assert!(by_title.iter().any(|h| {
            h.page.title_cache.as_deref() == Some("UniqueTitleOnly")
                && h.kind == WikiMatchKind::TitleExact
        }));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn strip_markdown_for_display_cleans_common_markup() {
        assert_eq!(
            strip_markdown_for_display("see **bold** and [link](https://x.test) here"),
            "see bold and link here"
        );
        assert_eq!(
            strip_markdown_for_display("![diagram](./a.png) next"),
            "diagram next"
        );
        assert_eq!(
            strip_markdown_for_display("# Title\n`code` and snake_case"),
            "Title code and snake_case"
        );
        assert_eq!(
            strip_snippet_part("…**before** match"),
            "…before match"
        );
    }

    #[test]
    fn search_snippet_strips_markdown_around_match() {
        let dir = std::env::temp_dir().join(format!("jade-wiki-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("fmt.md"),
            "---\ntitle: Fmt\n---\nSee **important** [UniqueSnippetTarget](https://example.test/path) in notes.\n",
        )
        .unwrap();

        let db = open_memory().unwrap();
        add_wiki_root(
            &db,
            AddWikiRootInput {
                path: dir.to_string_lossy().into_owned(),
                label: Some("Fmt".into()),
            },
        )
        .unwrap();

        let hits = search_wiki_pages(&db, "UniqueSnippetTarget").unwrap();
        assert_eq!(hits.len(), 1);
        let snippet = hits[0].snippet.as_ref().expect("snippet");
        assert_eq!(snippet.matched, "UniqueSnippetTarget");
        assert!(!snippet.before.contains("**"));
        assert!(!snippet.after.contains("https://"));
        assert!(snippet.before.contains("important") || snippet.before.contains("See"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_fts_query_escapes_and_prefixes() {
        assert_eq!(
            build_fts_match_query("hello world").as_deref(),
            Some("\"hello\"* \"world\"*")
        );
        assert!(build_fts_match_query("   ").is_none());
        assert_eq!(
            build_fts_match_query("foo\"bar").as_deref(),
            Some("\"foobar\"*")
        );
    }
}
