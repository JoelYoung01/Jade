//! YAML front matter parse / serialize for wiki markdown files.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{Error, Result};

const FENCE: &str = "---";

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
}

/// Split optional `---` YAML front matter from a markdown document.
pub fn parse_markdown(raw: &str) -> Result<ParsedMarkdown> {
    let normalized = raw.replace("\r\n", "\n");
    let trimmed_start = normalized.trim_start_matches('\u{feff}');

    if !trimmed_start.starts_with(FENCE) {
        return Ok(ParsedMarkdown {
            front_matter: None,
            body: normalized,
            had_front_matter: false,
        });
    }

    let after_open = &trimmed_start[FENCE.len()..];
    let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

    let Some(close_rel) = after_open.find(&format!("\n{FENCE}")) else {
        // Opening fence without close — treat whole file as body.
        return Ok(ParsedMarkdown {
            front_matter: None,
            body: normalized,
            had_front_matter: false,
        });
    };

    let yaml = &after_open[..close_rel];
    let rest = &after_open[close_rel + 1 + FENCE.len()..];
    let body = rest.strip_prefix('\n').unwrap_or(rest).to_owned();

    let front_matter: FrontMatter = if yaml.trim().is_empty() {
        FrontMatter::default()
    } else {
        serde_yaml::from_str(yaml)
            .map_err(|e| Error::Message(format!("invalid front matter: {e}")))?
    };

    Ok(ParsedMarkdown {
        front_matter: Some(front_matter),
        body,
        had_front_matter: true,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_without_front_matter() {
        let parsed = parse_markdown("# Hello\n\nWorld").unwrap();
        assert!(!parsed.had_front_matter);
        assert!(parsed.front_matter.is_none());
        assert!(parsed.body.contains("Hello"));
    }

    #[test]
    fn parse_and_roundtrip() {
        let raw = "---\nid: 11111111-1111-1111-1111-111111111111\ntitle: Note\ntags:\n  - a\n  - b\n---\n# Note\n\nBody\n";
        let parsed = parse_markdown(raw).unwrap();
        assert!(parsed.had_front_matter);
        let fm = parsed.front_matter.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Note"));
        assert_eq!(fm.tags, vec!["a".to_owned(), "b".to_owned()]);
        assert!(parsed.body.starts_with("# Note"));

        let rendered = render_markdown(&fm, &parsed.body).unwrap();
        let again = parse_markdown(&rendered).unwrap();
        assert_eq!(again.front_matter.unwrap().title.as_deref(), Some("Note"));
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
