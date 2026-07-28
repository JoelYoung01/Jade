use jade_core::{search_wiki_pages, Db};

use crate::output::ListFormat;

pub fn run(db: &Db, query: &str, format: ListFormat) -> anyhow::Result<()> {
    let hits = search_wiki_pages(db, query)?;
    match format {
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&hits)?);
        }
        ListFormat::Csv => {
            println!("score,kind,title,rel_path,reason,snippet");
            for hit in &hits {
                let title = hit.page.title_cache.as_deref().unwrap_or("");
                let snippet = hit
                    .snippet
                    .as_ref()
                    .map(|s| format!("{}{}{}", s.before, s.matched, s.after))
                    .unwrap_or_default();
                println!(
                    "{},{},{},{},{},{}",
                    hit.score,
                    hit.kind.as_str(),
                    csv_escape(title),
                    csv_escape(&hit.page.rel_path),
                    csv_escape(&hit.reason),
                    csv_escape(&snippet),
                );
            }
        }
        ListFormat::Plain => {
            if hits.is_empty() {
                println!("(no matches)");
                return Ok(());
            }
            for hit in &hits {
                let title = hit.page.title_cache.as_deref().unwrap_or(&hit.page.rel_path);
                println!("{title}");
                println!("  {}  ·  {}", hit.reason, hit.page.rel_path);
                if let Some(s) = &hit.snippet {
                    println!("  …{}[{}]{}…", trim_edges(&s.before), s.matched, trim_edges(&s.after));
                }
                println!();
            }
        }
    }
    Ok(())
}

fn trim_edges(value: &str) -> &str {
    value.trim_matches('…').trim()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
