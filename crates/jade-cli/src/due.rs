//! Parse CLI due-date strings into absolute UTC datetimes or reschedule presets.

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use jade_core::{next_hour_rounded, next_monday, push_to_tomorrow, DueUpdate};

/// Parse a due value for `tasks add` (always produces an absolute datetime).
pub fn parse_due_for_create(value: &str) -> anyhow::Result<DateTime<Utc>> {
    let trimmed = value.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "tomorrow" => Ok(push_to_tomorrow(next_hour_rounded(Utc::now()))),
        "next-monday" | "next_monday" => Ok(next_monday(next_hour_rounded(Utc::now()))),
        _ => parse_absolute_datetime(trimmed),
    }
}

/// Parse a due value for `tasks update` (presets relative to the current due).
pub fn parse_due_for_update(value: &str) -> anyhow::Result<DueUpdate> {
    let trimmed = value.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "tomorrow" => Ok(DueUpdate::Tomorrow),
        "next-monday" | "next_monday" => Ok(DueUpdate::NextMonday),
        _ => Ok(DueUpdate::At(parse_absolute_datetime(trimmed)?)),
    }
}

fn parse_absolute_datetime(value: &str) -> anyhow::Result<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Local naive forms: YYYY-MM-DDTHH:MM[:SS] or YYYY-MM-DD HH:MM[:SS]
    let normalized = value.replace(' ', "T");
    if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S") {
        return local_naive_to_utc(naive);
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M") {
        return local_naive_to_utc(naive);
    }

    // Date only → local noon
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(12, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid date: {value}"))?;
        return local_naive_to_utc(naive);
    }

    Err(anyhow::anyhow!(
        "invalid due date '{value}'. Use tomorrow, next-monday, RFC3339, \
         YYYY-MM-DDTHH:MM, or YYYY-MM-DD"
    ))
}

fn local_naive_to_utc(naive: NaiveDateTime) -> anyhow::Result<DateTime<Utc>> {
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| anyhow::anyhow!("ambiguous or invalid local datetime: {naive}"))
}
