use jade_core::{list_task_events, Db, ListTaskEventsInput, TaskEvent, TaskEventType};
use serde_json::Value;

use crate::tasks::HistoryArgs;

pub fn run(db: &Db, args: HistoryArgs, json: bool) -> anyhow::Result<()> {
    let events = list_task_events(
        db,
        ListTaskEventsInput {
            task_id: args.id,
            limit: Some(args.limit),
        },
    )?;
    print_events(&events, json)
}

fn print_events(events: &[TaskEvent], json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(events)?);
        return Ok(());
    }

    if events.is_empty() {
        println!("(no events)");
        return Ok(());
    }

    println!("{:<22}  {:<36}  {:<8}  CHANGES", "WHEN", "TASK", "TYPE");
    println!("{}", "-".repeat(120));
    for event in events {
        let when = event.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let summary = summarize_payload(event.event_type, &event.payload);
        println!(
            "{:<22}  {:<36}  {:<8}  {}",
            when,
            event.task_id,
            event.event_type.as_str(),
            summary
        );
    }
    Ok(())
}

fn summarize_payload(event_type: TaskEventType, payload: &Value) -> String {
    match event_type {
        TaskEventType::Created => {
            let title = payload
                .get("title")
                .or_else(|| payload.pointer("/task/title"))
                .and_then(|v| v.as_str())
                .unwrap_or("(untitled)");
            payload
                .get("spawned_from")
                .and_then(|v| v.as_str())
                .map_or_else(
                    || format!("created \"{title}\""),
                    |from| format!("created \"{title}\" (spawned from {from})"),
                )
        }
        TaskEventType::Deleted => payload
            .pointer("/task/title")
            .and_then(|v| v.as_str())
            .map_or_else(
                || "deleted".to_owned(),
                |title| format!("deleted \"{title}\""),
            ),
        TaskEventType::Updated => {
            // Prefer nested `changes` (sync envelope); fall back to flat field map.
            let changes = payload
                .get("changes")
                .and_then(|v| v.as_object())
                .or_else(|| payload.as_object());
            let Some(obj) = changes else {
                return payload.to_string();
            };
            let mut parts = Vec::new();
            for (field, change) in obj {
                if field == "task" || field == "changes" {
                    continue;
                }
                let Some(change_obj) = change.as_object() else {
                    continue;
                };
                if !change_obj.contains_key("old") && !change_obj.contains_key("new") {
                    continue;
                }
                let old = format_value(change.get("old"));
                let new = format_value(change.get("new"));
                parts.push(format!("{field}: {old} -> {new}"));
            }
            if parts.is_empty() {
                "(no field changes)".to_owned()
            } else {
                parts.join("; ")
            }
        }
    }
}

fn format_value(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "null".to_owned(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => {
            let joined = items
                .iter()
                .map(|v| v.as_str().unwrap_or("?"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{joined}]")
        }
        Some(other) => other.to_string(),
    }
}
