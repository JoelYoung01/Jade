//! YAML front matter parse / serialize for wiki markdown files.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml::Mapping;
use uuid::Uuid;

use crate::error::{Error, Result};

const FENCE: &str = "---";

const LIST_FIELDS: &[&str] = &["tags", "references"];
const STRING_FIELDS: &[&str] = &[
    "title",
    "summary",
    "date",
    "date_added",
    "author",
    "url",
    "source",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiFrontMatterIssueKind {
    /// `tags` or `references` was a single string (or other scalar) instead of a list.
    StringAsList,
    /// A string field was a number/bool and was converted to text.
    ScalarAsString,
    /// The YAML document is not valid YAML.
    InvalidYaml,
    /// A field had a structure Jade cannot use (mapping, nested list, etc.).
    UnsupportedType,
    /// `id` was present but not a UUID.
    InvalidId,
    /// The markdown file could not be read during indexing.
    ReadFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontMatterIssue {
    pub kind: WikiFrontMatterIssueKind,
    pub field: Option<String>,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub repairable: bool,
    pub repair_label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrontMatter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_added: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    /// Preserve unknown keys when round-tripping via Value merge is handled separately.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct ParsedMarkdown {
    pub front_matter: Option<FrontMatter>,
    pub body: String,
    #[allow(dead_code)]
    pub had_front_matter: bool,
    pub issues: Vec<FrontMatterIssue>,
}

/// Split optional `---` YAML front matter from a markdown document.
///
/// Malformed headers do not fail the parse: the body is still returned and
/// problems are listed in [`ParsedMarkdown::issues`]. Known mismatches (a
/// string where a list is required, and similar) are coerced in memory so
/// indexing can continue.
pub fn parse_markdown(raw: &str) -> ParsedMarkdown {
    let normalized = raw.replace("\r\n", "\n");
    let trimmed_start = normalized.trim_start_matches('\u{feff}');

    if !trimmed_start.starts_with(FENCE) {
        return ParsedMarkdown {
            front_matter: None,
            body: normalized,
            had_front_matter: false,
            issues: Vec::new(),
        };
    }

    let after_open = &trimmed_start[FENCE.len()..];
    let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

    let Some(close_rel) = after_open.find(&format!("\n{FENCE}")) else {
        // Opening fence without close — treat whole file as body.
        return ParsedMarkdown {
            front_matter: None,
            body: normalized,
            had_front_matter: false,
            issues: Vec::new(),
        };
    };

    let yaml = &after_open[..close_rel];
    let rest = &after_open[close_rel + 1 + FENCE.len()..];
    let body = rest.strip_prefix('\n').unwrap_or(rest).to_owned();

    let (front_matter, issues) = parse_front_matter_yaml(yaml);

    ParsedMarkdown {
        front_matter,
        body,
        had_front_matter: true,
        issues,
    }
}

/// Rewrite a document's YAML header into Jade's canonical form when issues
/// were detected. Keeps the article body. Assigns `id` / `title` if missing.
pub fn repair_markdown_front_matter(raw: &str) -> Result<String> {
    let parsed = parse_markdown(raw);
    if parsed.issues.is_empty() {
        return Ok(raw.replace("\r\n", "\n"));
    }
    if !parsed.issues.iter().any(|issue| issue.repairable) {
        return Err(Error::Message(
            "front matter cannot be repaired automatically".into(),
        ));
    }

    let mut fm = parsed.front_matter.unwrap_or_default();
    let title_hint = first_heading(&parsed.body);
    ensure_identity(&mut fm, title_hint.as_deref());
    render_markdown(&fm, &parsed.body)
}

/// Serialize front matter + body into a markdown file string.
pub fn render_markdown(fm: &FrontMatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(fm)
        .map_err(|e| Error::Message(format!("failed to serialize front matter: {e}")))?;
    // serde_yaml adds a trailing newline; strip a leading `---` if present (it shouldn't).
    let yaml = yaml.trim_end();
    let body = body.trim_start_matches('\n');
    if body.is_empty() {
        Ok(format!("{FENCE}\n{yaml}\n{FENCE}\n"))
    } else {
        Ok(format!("{FENCE}\n{yaml}\n{FENCE}\n{body}"))
    }
}

/// Ensure `id` (and optional title) exist on front matter for Jade writes.
pub fn ensure_identity(fm: &mut FrontMatter, title_hint: Option<&str>) {
    if fm.id.is_none() {
        fm.id = Some(Uuid::new_v4());
    }
    if fm.title.as_ref().is_none_or(|t| t.trim().is_empty()) {
        if let Some(title) = title_hint.map(str::trim).filter(|t| !t.is_empty()) {
            fm.title = Some(title.to_owned());
        }
    }
}

/// First markdown AT1 heading in body, if any.
pub fn first_heading(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return Some(title.to_owned());
            }
        }
    }
    None
}

/// Best date for "recently added" sorting: `date_added`, then `date`.
pub fn resolve_date_added(fm: Option<&FrontMatter>) -> Option<String> {
    fm.and_then(|f| {
        f.date_added
            .as_ref()
            .or(f.date.as_ref())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    })
}

/// Title from front matter, else first heading, else file stem.
pub fn resolve_title(fm: Option<&FrontMatter>, body: &str, file_stem: &str) -> String {
    if let Some(title) = fm
        .and_then(|f| f.title.as_ref())
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
    {
        return title.to_owned();
    }
    if let Some(heading) = first_heading(body) {
        return heading;
    }
    file_stem.to_owned()
}

fn parse_front_matter_yaml(yaml: &str) -> (Option<FrontMatter>, Vec<FrontMatterIssue>) {
    if yaml.trim().is_empty() {
        return (Some(FrontMatter::default()), Vec::new());
    }

    let mut value: serde_yaml::Value = match serde_yaml::from_str(yaml) {
        Ok(v) => v,
        Err(err) => return (None, vec![yaml_syntax_issue(&err)]),
    };

    let issues = coerce_front_matter_value(&mut value);
    if !matches!(value, serde_yaml::Value::Mapping(_)) {
        return (None, issues);
    }

    match serde_yaml::from_value(value) {
        Ok(fm) => (Some(fm), issues),
        Err(err) => {
            let mut issues = issues;
            issues.push(FrontMatterIssue {
                kind: WikiFrontMatterIssueKind::UnsupportedType,
                field: None,
                message: format!(
                    "Jade couldn't use this YAML header: {err}. A fresh header can be written; the article body is kept."
                ),
                line: err.location().map(|loc| loc.line().saturating_add(1)),
                column: err.location().map(|loc| loc.column()),
                repairable: true,
                repair_label: Some("Replace with a fresh header".into()),
            });
            (None, issues)
        }
    }
}

fn yaml_syntax_issue(err: &serde_yaml::Error) -> FrontMatterIssue {
    FrontMatterIssue {
        kind: WikiFrontMatterIssueKind::InvalidYaml,
        field: None,
        message: format!(
            "The YAML header isn't valid YAML ({err}). Jade can replace it with a fresh header and keep the article body."
        ),
        line: err.location().map(|loc| loc.line().saturating_add(1)),
        column: err.location().map(|loc| loc.column()),
        repairable: true,
        repair_label: Some("Replace with a fresh header".into()),
    }
}

fn coerce_front_matter_value(value: &mut serde_yaml::Value) -> Vec<FrontMatterIssue> {
    match value {
        serde_yaml::Value::Mapping(map) => coerce_mapping(map),
        other => vec![FrontMatterIssue {
            kind: WikiFrontMatterIssueKind::InvalidYaml,
            field: None,
            message: format!(
                "Front matter must be a YAML mapping, not {}. Jade can replace it with a fresh header and keep the article body.",
                yaml_kind_name(other)
            ),
            line: None,
            column: None,
            repairable: true,
            repair_label: Some("Replace with a fresh header".into()),
        }],
    }
}

fn coerce_mapping(map: &mut Mapping) -> Vec<FrontMatterIssue> {
    let mut issues = Vec::new();

    for field in LIST_FIELDS {
        let key = serde_yaml::Value::String((*field).to_owned());
        let Some(current) = map.get(&key).cloned() else {
            continue;
        };
        match coerce_list_field(field, &current) {
            CoerceAction::Unchanged => {}
            CoerceAction::Replace(next, issue) => {
                map.insert(key, next);
                issues.push(issue);
            }
            CoerceAction::Remove(issue) => {
                map.remove(&key);
                issues.push(issue);
            }
        }
    }

    for field in STRING_FIELDS {
        let key = serde_yaml::Value::String((*field).to_owned());
        let Some(current) = map.get(&key).cloned() else {
            continue;
        };
        match coerce_string_field(field, &current) {
            CoerceAction::Unchanged => {}
            CoerceAction::Replace(next, issue) => {
                map.insert(key, next);
                issues.push(issue);
            }
            CoerceAction::Remove(issue) => {
                map.remove(&key);
                issues.push(issue);
            }
        }
    }

    let id_key = serde_yaml::Value::String("id".into());
    if let Some(current) = map.get(&id_key).cloned() {
        match coerce_id_field(&current) {
            CoerceAction::Unchanged => {}
            CoerceAction::Replace(next, issue) => {
                map.insert(id_key, next);
                issues.push(issue);
            }
            CoerceAction::Remove(issue) => {
                map.remove(&id_key);
                issues.push(issue);
            }
        }
    }

    issues
}

enum CoerceAction {
    Unchanged,
    Replace(serde_yaml::Value, FrontMatterIssue),
    Remove(FrontMatterIssue),
}

fn coerce_list_field(field: &str, value: &serde_yaml::Value) -> CoerceAction {
    match value {
        serde_yaml::Value::Null => CoerceAction::Replace(
            serde_yaml::Value::Sequence(Vec::new()),
            FrontMatterIssue {
                kind: WikiFrontMatterIssueKind::StringAsList,
                field: Some(field.to_owned()),
                message: format!(
                    "`{field}` is empty. Jade can write it as an empty list."
                ),
                line: None,
                column: None,
                repairable: true,
                repair_label: Some("Write as an empty list".into()),
            },
        ),
        serde_yaml::Value::String(text) => {
            let preview = preview_scalar(text);
            CoerceAction::Replace(
                serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(text.clone())]),
                FrontMatterIssue {
                    kind: WikiFrontMatterIssueKind::StringAsList,
                    field: Some(field.to_owned()),
                    message: format!(
                        "`{field}` is a single value ({preview}), but Jade expects a list. It can be wrapped as a one-item list."
                    ),
                    line: None,
                    column: None,
                    repairable: true,
                    repair_label: Some("Wrap as a list".into()),
                },
            )
        }
        serde_yaml::Value::Number(num) => {
            let text = num.to_string();
            CoerceAction::Replace(
                serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(text.clone())]),
                FrontMatterIssue {
                    kind: WikiFrontMatterIssueKind::StringAsList,
                    field: Some(field.to_owned()),
                    message: format!(
                        "`{field}` is the number {text}, but Jade expects a list of strings. It can be wrapped as a one-item list."
                    ),
                    line: None,
                    column: None,
                    repairable: true,
                    repair_label: Some("Wrap as a list".into()),
                },
            )
        }
        serde_yaml::Value::Bool(flag) => {
            let text = flag.to_string();
            CoerceAction::Replace(
                serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(text.clone())]),
                FrontMatterIssue {
                    kind: WikiFrontMatterIssueKind::StringAsList,
                    field: Some(field.to_owned()),
                    message: format!(
                        "`{field}` is `{text}`, but Jade expects a list of strings. It can be wrapped as a one-item list."
                    ),
                    line: None,
                    column: None,
                    repairable: true,
                    repair_label: Some("Wrap as a list".into()),
                },
            )
        }
        serde_yaml::Value::Sequence(items) => coerce_list_items(field, items),
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Tagged(_) => CoerceAction::Remove(
            FrontMatterIssue {
                kind: WikiFrontMatterIssueKind::UnsupportedType,
                field: Some(field.to_owned()),
                message: format!(
                    "`{field}` is not a list of strings, so Jade can't use it. The field can be removed so the rest of the header still works."
                ),
                line: None,
                column: None,
                repairable: true,
                repair_label: Some(format!("Remove `{field}`")),
            },
        ),
    }
}

fn coerce_list_items(field: &str, items: &[serde_yaml::Value]) -> CoerceAction {
    let mut next = Vec::with_capacity(items.len());
    let mut coerced = false;
    let mut dropped = 0usize;
    for item in items {
        match value_as_string(item) {
            Some(text) => {
                if !item.is_string() {
                    coerced = true;
                }
                next.push(serde_yaml::Value::String(text));
            }
            None => {
                dropped += 1;
            }
        }
    }
    if !coerced && dropped == 0 {
        return CoerceAction::Unchanged;
    }
    let message = if dropped > 0 {
        format!(
            "`{field}` has {dropped} value(s) that aren't text. Jade can keep the text items and drop the rest."
        )
    } else {
        format!("`{field}` has non-text items. Jade can store each item as text.")
    };
    CoerceAction::Replace(
        serde_yaml::Value::Sequence(next),
        FrontMatterIssue {
            kind: WikiFrontMatterIssueKind::ScalarAsString,
            field: Some(field.to_owned()),
            message,
            line: None,
            column: None,
            repairable: true,
            repair_label: Some("Normalize list items".into()),
        },
    )
}

fn coerce_string_field(field: &str, value: &serde_yaml::Value) -> CoerceAction {
    match value {
        serde_yaml::Value::Null | serde_yaml::Value::String(_) => CoerceAction::Unchanged,
        serde_yaml::Value::Number(num) => {
            let text = num.to_string();
            CoerceAction::Replace(
                serde_yaml::Value::String(text.clone()),
                FrontMatterIssue {
                    kind: WikiFrontMatterIssueKind::ScalarAsString,
                    field: Some(field.to_owned()),
                    message: format!(
                        "`{field}` is the number {text}. Jade can store it as text."
                    ),
                    line: None,
                    column: None,
                    repairable: true,
                    repair_label: Some("Store as text".into()),
                },
            )
        }
        serde_yaml::Value::Bool(flag) => {
            let text = flag.to_string();
            CoerceAction::Replace(
                serde_yaml::Value::String(text.clone()),
                FrontMatterIssue {
                    kind: WikiFrontMatterIssueKind::ScalarAsString,
                    field: Some(field.to_owned()),
                    message: format!("`{field}` is `{text}`. Jade can store it as text."),
                    line: None,
                    column: None,
                    repairable: true,
                    repair_label: Some("Store as text".into()),
                },
            )
        }
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => CoerceAction::Remove(FrontMatterIssue {
            kind: WikiFrontMatterIssueKind::UnsupportedType,
            field: Some(field.to_owned()),
            message: format!(
                "`{field}` is not a single text value, so Jade can't use it. The field can be removed so the rest of the header still works."
            ),
            line: None,
            column: None,
            repairable: true,
            repair_label: Some(format!("Remove `{field}`")),
        }),
    }
}

fn coerce_id_field(value: &serde_yaml::Value) -> CoerceAction {
    match value {
        serde_yaml::Value::Null => CoerceAction::Unchanged,
        serde_yaml::Value::String(text) if Uuid::parse_str(text.trim()).is_ok() => {
            CoerceAction::Unchanged
        }
        other => {
            let preview = value_as_string(other).map_or_else(
                || yaml_kind_name(other).to_owned(),
                |text| preview_scalar(&text),
            );
            CoerceAction::Remove(FrontMatterIssue {
                kind: WikiFrontMatterIssueKind::InvalidId,
                field: Some("id".into()),
                message: format!(
                    "`id` is {preview}, which is not a UUID. Jade can drop it and assign a new one."
                ),
                line: None,
                column: None,
                repairable: true,
                repair_label: Some("Assign a new id".into()),
            })
        }
    }
}

fn value_as_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(text) => Some(text.clone()),
        serde_yaml::Value::Number(num) => Some(num.to_string()),
        serde_yaml::Value::Bool(flag) => Some(flag.to_string()),
        serde_yaml::Value::Null
        | serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => None,
    }
}

fn yaml_kind_name(value: &serde_yaml::Value) -> &'static str {
    match value {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "a boolean",
        serde_yaml::Value::Number(_) => "a number",
        serde_yaml::Value::String(_) => "a string",
        serde_yaml::Value::Sequence(_) => "a list",
        serde_yaml::Value::Mapping(_) => "a mapping",
        serde_yaml::Value::Tagged(_) => "a tagged value",
    }
}

fn preview_scalar(value: &str) -> String {
    const MAX: usize = 64;
    if value.chars().count() <= MAX {
        format!("\"{value}\"")
    } else {
        let prefix: String = value.chars().take(MAX.saturating_sub(1)).collect();
        format!("\"{prefix}…\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_without_front_matter() {
        let parsed = parse_markdown("# Hello\n\nWorld");
        assert!(!parsed.had_front_matter);
        assert!(parsed.front_matter.is_none());
        assert!(parsed.issues.is_empty());
        assert!(parsed.body.contains("Hello"));
    }

    #[test]
    fn parse_and_roundtrip() {
        let raw = "---\nid: 11111111-1111-1111-1111-111111111111\ntitle: Note\ntags:\n  - a\n  - b\n---\n# Note\n\nBody\n";
        let parsed = parse_markdown(raw);
        assert!(parsed.had_front_matter);
        assert!(parsed.issues.is_empty());
        let fm = parsed.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Note"));
        assert_eq!(fm.tags, vec!["a".to_owned(), "b".to_owned()]);
        assert!(parsed.body.starts_with("# Note"));

        let rendered = render_markdown(&fm, &parsed.body).unwrap();
        let again = parse_markdown(&rendered);
        assert_eq!(again.front_matter.unwrap().title.as_deref(), Some("Note"));
    }

    #[test]
    fn string_references_are_coerced_and_flagged() {
        let raw =
            "---\ntitle: Wigolo\nreferences: https://github.com/KnockOutEZ/wigolo\n---\n# Wigolo\n";
        let parsed = parse_markdown(raw);
        let fm = parsed.front_matter.expect("coerced front matter");
        assert_eq!(
            fm.references,
            vec!["https://github.com/KnockOutEZ/wigolo".to_owned()]
        );
        assert_eq!(parsed.issues.len(), 1);
        assert_eq!(
            parsed.issues[0].kind,
            WikiFrontMatterIssueKind::StringAsList
        );
        assert_eq!(parsed.issues[0].field.as_deref(), Some("references"));
        assert!(parsed.issues[0].repairable);
    }

    #[test]
    fn string_tags_are_coerced() {
        let raw = "---\ntags: rust\n---\nBody\n";
        let parsed = parse_markdown(raw);
        assert_eq!(parsed.front_matter.unwrap().tags, vec!["rust".to_owned()]);
        assert_eq!(
            parsed.issues[0].kind,
            WikiFrontMatterIssueKind::StringAsList
        );
    }

    #[test]
    fn invalid_yaml_keeps_body_and_is_repairable() {
        let raw = "---\ntitle: [unterminated\n---\n# Still here\n\nBody\n";
        let parsed = parse_markdown(raw);
        assert!(parsed.front_matter.is_none());
        assert!(parsed.body.starts_with("# Still here"));
        assert_eq!(parsed.issues[0].kind, WikiFrontMatterIssueKind::InvalidYaml);
        assert!(parsed.issues[0].repairable);

        let repaired = repair_markdown_front_matter(raw).unwrap();
        let again = parse_markdown(&repaired);
        assert!(again.issues.is_empty());
        assert!(again.front_matter.unwrap().id.is_some());
        assert!(again.body.contains("Still here"));
    }

    #[test]
    fn repair_wraps_string_references() {
        let raw = "---\ntitle: Wigolo\nreferences: https://github.com/KnockOutEZ/wigolo\n---\n# Wigolo\n\nNotes.\n";
        let repaired = repair_markdown_front_matter(raw).unwrap();
        let parsed = parse_markdown(&repaired);
        assert!(parsed.issues.is_empty());
        assert_eq!(
            parsed.front_matter.unwrap().references,
            vec!["https://github.com/KnockOutEZ/wigolo".to_owned()]
        );
        assert!(repaired.contains("references:\n"));
        assert!(repaired.contains("- https://github.com/KnockOutEZ/wigolo"));
    }

    #[test]
    fn resolve_date_added_prefers_date_added() {
        let fm = FrontMatter {
            date_added: Some("2026-08-27".into()),
            date: Some("2026-01-01".into()),
            ..FrontMatter::default()
        };
        assert_eq!(resolve_date_added(Some(&fm)).as_deref(), Some("2026-08-27"));
    }

    #[test]
    fn ensure_identity_fills_id() {
        let mut fm = FrontMatter::default();
        ensure_identity(&mut fm, Some("From Hint"));
        assert!(fm.id.is_some());
        assert_eq!(fm.title.as_deref(), Some("From Hint"));
    }
}
