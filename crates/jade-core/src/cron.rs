//! Cron schedule helpers for recurring tasks.
//!
//! Schedules are 5-field POSIX cron strings (`minute hour day month weekday`).
//! Evaluation uses the local timezone so `"0 9 * * *"` means 9am local.

use chrono::{DateTime, Local, Utc};
use croner::Cron;

use crate::error::{Error, Result};

/// Parse and validate a 5-field cron expression.
///
/// Note: croner's `FromStr` does not actually parse — always use `Cron::new().parse()`.
pub fn parse_cron(expr: &str) -> Result<Cron> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidCron("schedule is empty".into()));
    }
    Cron::new(trimmed)
        .parse()
        .map_err(|e| Error::InvalidCron(e.to_string()))
}

/// Normalize a cron string (trim). Returns `None` for empty / whitespace-only.
/// Validates when present.
pub fn normalize_cron(expr: Option<&str>) -> Result<Option<String>> {
    match expr {
        None => Ok(None),
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                parse_cron(trimmed)?;
                Ok(Some(trimmed.to_owned()))
            }
        }
    }
}

/// First cron match strictly after `after` (local time), returned as UTC.
///
/// Callers should pass `max(due_at, now)` so overdue completions skip the backlog
/// instead of spawning missed occurrences (Apple Reminders behavior).
pub fn next_occurrence(cron_expr: &str, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let cron = parse_cron(cron_expr)?;
    let after_local = after.with_timezone(&Local);
    let next_local = cron
        .find_next_occurrence(&after_local, false)
        .map_err(|e| Error::InvalidCron(format!("could not compute next occurrence: {e}")))?;
    Ok(next_local.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn rejects_empty_and_invalid() {
        assert!(matches!(parse_cron(""), Err(Error::InvalidCron(_))));
        assert!(matches!(
            parse_cron("not a cron"),
            Err(Error::InvalidCron(_))
        ));
        assert!(normalize_cron(Some("   ")).unwrap().is_none());
    }

    #[test]
    fn next_daily_at_nine() {
        // 2026-07-21 is a Tuesday; local 8:00 → next is same day 09:00 local.
        let after = Local
            .with_ymd_and_hms(2026, 7, 21, 8, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        let next = next_occurrence("0 9 * * *", after).unwrap();
        let next_local = next.with_timezone(&Local);
        assert_eq!(next_local.hour(), 9);
        assert_eq!(next_local.minute(), 0);
        assert_eq!(
            next_local.date_naive(),
            after.with_timezone(&Local).date_naive()
        );
    }

    #[test]
    fn next_skips_when_after_is_past_match() {
        let after = Local
            .with_ymd_and_hms(2026, 7, 21, 10, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        let next = next_occurrence("0 9 * * *", after).unwrap();
        let next_local = next.with_timezone(&Local);
        assert_eq!(
            next_local.date_naive(),
            Local
                .with_ymd_and_hms(2026, 7, 22, 9, 0, 0)
                .unwrap()
                .date_naive()
        );
    }
}
