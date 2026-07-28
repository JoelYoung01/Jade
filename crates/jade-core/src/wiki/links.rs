//! Extract wiki-style and markdown links from page bodies (rebuildable cache).

use std::collections::BTreeSet;

/// Collect `[[target]]` wiki links and markdown `[text](target)` link targets.
pub fn extract_link_targets(body: &str) -> Vec<String> {
    let mut targets = BTreeSet::new();

    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let raw = after[..end].trim();
            // `[[target|label]]` → target
            let target = raw.split('|').next().unwrap_or(raw).trim();
            if !target.is_empty() {
                targets.insert(target.to_owned());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }

    // Simple markdown links: [label](target) — skip http(s) and anchors-only for wiki graph.
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(label_end) = body[i + 1..].find(']') {
                let after_label = i + 1 + label_end + 1;
                if body[after_label..].starts_with('(') {
                    if let Some(url_end) = body[after_label + 1..].find(')') {
                        let target = body[after_label + 1..after_label + 1 + url_end].trim();
                        if !target.is_empty()
                            && !target.starts_with("http://")
                            && !target.starts_with("https://")
                            && !target.starts_with('#')
                            && !target.starts_with("mailto:")
                        {
                            targets.insert(target.to_owned());
                        }
                        i = after_label + 1 + url_end + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    targets.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_wiki_and_md_links() {
        let body = "See [[Other Page|label]] and [x](notes/a.md). Skip [web](https://example.com).";
        let links = extract_link_targets(body);
        assert!(links.contains(&"Other Page".to_owned()));
        assert!(links.contains(&"notes/a.md".to_owned()));
        assert!(!links.iter().any(|l| l.contains("example.com")));
    }
}
