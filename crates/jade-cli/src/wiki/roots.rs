use jade_core::{add_wiki_root, list_wiki_roots, remove_wiki_root, AddWikiRootInput, Db, WikiRoot};

use crate::output::ListFormat;
use crate::wiki::{RootsArgs, RootsCommand};

pub fn run(db: &Db, args: RootsArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        None => {
            let format = if json && args.format == ListFormat::Plain {
                ListFormat::Json
            } else {
                args.format
            };
            print_roots(&list_wiki_roots(db)?, format)
        }
        Some(RootsCommand::Add(add)) => {
            let root = add_wiki_root(
                db,
                AddWikiRootInput {
                    path: add.path.to_string_lossy().into_owned(),
                    label: add.label,
                },
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&root)?);
            } else {
                println!("added wiki root {}  {}  {}", root.id, root.label, root.path);
            }
            Ok(())
        }
        Some(RootsCommand::Remove(remove)) => {
            remove_wiki_root(db, remove.id)?;
            if json {
                println!(r#"{{"removed":"{}"}}"#, remove.id);
            } else {
                println!("removed wiki root {}", remove.id);
            }
            Ok(())
        }
    }
}

fn print_roots(roots: &[WikiRoot], format: ListFormat) -> anyhow::Result<()> {
    match format {
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(roots)?);
        }
        ListFormat::Csv => {
            println!("id,label,path,enabled");
            for root in roots {
                println!(
                    "{},{},{},{}",
                    csv_escape(&root.id.to_string()),
                    csv_escape(&root.label),
                    csv_escape(&root.path),
                    root.enabled
                );
            }
        }
        ListFormat::Plain => {
            if roots.is_empty() {
                println!("(no wiki roots)");
                return Ok(());
            }
            println!("{:<36}  {:<20}  PATH", "ID", "LABEL");
            println!("{}", "-".repeat(100));
            for root in roots {
                println!(
                    "{:<36}  {:<20}  {}",
                    root.id,
                    truncate(&root.label, 20),
                    root.path
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
