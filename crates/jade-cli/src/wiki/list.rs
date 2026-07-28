use jade_core::{list_wiki_pages, Db, WikiPage};
use uuid::Uuid;

use crate::output::ListFormat;

pub fn run(db: &Db, root_id: Option<Uuid>, format: ListFormat) -> anyhow::Result<()> {
    let pages = list_wiki_pages(db, root_id)?;
    print_pages(&pages, format)
}

pub fn print_pages(pages: &[WikiPage], format: ListFormat) -> anyhow::Result<()> {
    match format {
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(pages)?);
        }
        ListFormat::Csv => {
            println!("id,root_id,title,rel_path,tags");
            for page in pages {
                let tags = page.tags_cache.join(";");
                println!(
                    "{},{},{},{},{}",
                    csv_escape(&page.id.to_string()),
                    csv_escape(&page.root_id.to_string()),
                    csv_escape(page.title_cache.as_deref().unwrap_or("")),
                    csv_escape(&page.rel_path),
                    csv_escape(&tags),
                );
            }
        }
        ListFormat::Plain => {
            if pages.is_empty() {
                println!("(no wiki pages)");
                return Ok(());
            }
            println!("{:<36}  {:<28}  REL_PATH", "ID", "TITLE");
            println!("{}", "-".repeat(100));
            for page in pages {
                let title = page.title_cache.as_deref().unwrap_or("");
                println!(
                    "{:<36}  {:<28}  {}",
                    page.id,
                    truncate(title, 28),
                    page.rel_path
                );
            }
        }
    }
    Ok(())
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_owned()
    } else {
        let mut s: String = value.chars().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
