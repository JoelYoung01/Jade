use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Timelike, Utc, Weekday};

/// Round "now" up to the next whole hour (local time), returned as UTC.
/// If already on an exact hour, returns the following hour.
#[must_use]
pub fn next_hour_rounded(now: DateTime<Utc>) -> DateTime<Utc> {
    let local = now.with_timezone(&Local);
    let next = if local.minute() == 0 && local.second() == 0 && local.nanosecond() == 0 {
        local + Duration::hours(1)
    } else {
        let truncated = local
            .with_minute(0)
            .and_then(|dt| dt.with_second(0))
            .and_then(|dt| dt.with_nanosecond(0))
            .expect("valid local datetime");
        truncated + Duration::hours(1)
    };
    next.with_timezone(&Utc)
}

/// Keep the local time-of-day from `local`, place it on `date`.
/// Falls back to local noon if the combination is ambiguous or invalid (DST).
fn on_local_date(date: chrono::NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    Local
        .from_local_datetime(&date.and_time(time))
        .single()
        .unwrap_or_else(|| {
            Local
                .from_local_datetime(
                    &date.and_time(NaiveTime::from_hms_opt(12, 0, 0).expect("noon")),
                )
                .single()
                .expect("valid local noon")
        })
        .with_timezone(&Utc)
}

/// Keep the same local time-of-day, move the calendar date to today.
#[must_use]
pub fn push_to_today(due_at: DateTime<Utc>) -> DateTime<Utc> {
    push_to_today_at(due_at, Utc::now())
}

/// Like [`push_to_today`], but with an explicit `now` (for tests).
#[must_use]
pub fn push_to_today_at(due_at: DateTime<Utc>, now: DateTime<Utc>) -> DateTime<Utc> {
    let local = due_at.with_timezone(&Local);
    on_local_date(now.with_timezone(&Local).date_naive(), local.time())
}

/// Keep the same local time-of-day, move the calendar date to tomorrow (relative to now).
#[must_use]
pub fn push_to_tomorrow(due_at: DateTime<Utc>) -> DateTime<Utc> {
    push_to_tomorrow_at(due_at, Utc::now())
}

/// Like [`push_to_tomorrow`], but with an explicit `now` (for tests).
#[must_use]
pub fn push_to_tomorrow_at(due_at: DateTime<Utc>, now: DateTime<Utc>) -> DateTime<Utc> {
    let local = due_at.with_timezone(&Local);
    let tomorrow = now.with_timezone(&Local).date_naive() + Duration::days(1);
    on_local_date(tomorrow, local.time())
}

/// Move to the next Monday after today, preserving local time-of-day.
/// If today is Monday, jumps to the following Monday.
#[must_use]
pub fn next_monday(due_at: DateTime<Utc>) -> DateTime<Utc> {
    next_monday_at(due_at, Utc::now())
}

/// Like [`next_monday`], but with an explicit `now` (for tests).
#[must_use]
pub fn next_monday_at(due_at: DateTime<Utc>, now: DateTime<Utc>) -> DateTime<Utc> {
    let local = due_at.with_timezone(&Local);
    let now_local = now.with_timezone(&Local);
    let days_until = match now_local.weekday() {
        Weekday::Mon => 7,
        Weekday::Tue => 6,
        Weekday::Wed => 5,
        Weekday::Thu => 4,
        Weekday::Fri => 3,
        Weekday::Sat => 2,
        Weekday::Sun => 1,
    };
    on_local_date(
        now_local.date_naive() + Duration::days(days_until),
        local.time(),
    )
}

/// Move to the first Monday of the month after today, preserving local time-of-day.
#[must_use]
pub fn first_monday_next_month(due_at: DateTime<Utc>) -> DateTime<Utc> {
    first_monday_next_month_at(due_at, Utc::now())
}

/// Like [`first_monday_next_month`], but with an explicit `now` (for tests).
#[must_use]
pub fn first_monday_next_month_at(due_at: DateTime<Utc>, now: DateTime<Utc>) -> DateTime<Utc> {
    let local = due_at.with_timezone(&Local);
    let now_local = now.with_timezone(&Local);
    let (year, month) = if now_local.month() == 12 {
        (now_local.year() + 1, 1)
    } else {
        (now_local.year(), now_local.month() + 1)
    };
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("valid first of month");
    let days_until = (7 - first.weekday().num_days_from_monday()) % 7;
    on_local_date(first + Duration::days(i64::from(days_until)), local.time())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn next_hour_rounds_up_from_partial_hour() {
        let now = Utc.with_ymd_and_hms(2026, 7, 20, 14, 23, 45).unwrap();
        let result = next_hour_rounded(now);
        let local = result.with_timezone(&Local);
        assert_eq!(local.minute(), 0);
        assert_eq!(local.second(), 0);
        // Should be one hour ahead of the truncated local hour
        let expected_hour = (now.with_timezone(&Local).hour() + 1) % 24;
        assert_eq!(local.hour(), expected_hour);
    }

    #[test]
    fn next_hour_from_exact_hour_advances() {
        let local = Local.with_ymd_and_hms(2026, 7, 20, 15, 0, 0).unwrap();
        let now = local.with_timezone(&Utc);
        let result = next_hour_rounded(now);
        assert_eq!(
            result.with_timezone(&Local),
            (local + Duration::hours(1)).with_timezone(&Local)
        );
    }

    #[test]
    fn push_to_today_preserves_time() {
        let now = Local.with_ymd_and_hms(2026, 7, 20, 10, 0, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 7, 22, 9, 30, 0).unwrap();
        let result = push_to_today_at(due.with_timezone(&Utc), now.with_timezone(&Utc))
            .with_timezone(&Local);
        assert_eq!(result.date_naive(), now.date_naive());
        assert_eq!(result.time(), due.time());
    }

    #[test]
    fn push_to_tomorrow_is_relative_to_now_not_due() {
        // Due is Wednesday; now is Monday → tomorrow is Tuesday the 21st.
        let now = Local.with_ymd_and_hms(2026, 7, 20, 10, 0, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 7, 22, 9, 30, 0).unwrap();
        let result = push_to_tomorrow_at(due.with_timezone(&Utc), now.with_timezone(&Utc))
            .with_timezone(&Local);
        assert_eq!(
            result.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 21).unwrap()
        );
        assert_eq!(result.time(), due.time());

        // Applying again stays on tomorrow (idempotent vs advancing from due).
        let again = push_to_tomorrow_at(result.with_timezone(&Utc), now.with_timezone(&Utc))
            .with_timezone(&Local);
        assert_eq!(again.date_naive(), result.date_naive());
    }

    #[test]
    fn next_monday_from_wednesday_now() {
        // now 2026-07-22 Wednesday → next Monday is 2026-07-27
        let now = Local.with_ymd_and_hms(2026, 7, 22, 8, 0, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 7, 15, 10, 0, 0).unwrap();
        let result =
            next_monday_at(due.with_timezone(&Utc), now.with_timezone(&Utc)).with_timezone(&Local);
        assert_eq!(result.weekday(), Weekday::Mon);
        assert_eq!(
            result.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()
        );
        assert_eq!(result.time(), due.time());
    }

    #[test]
    fn first_monday_next_month_from_now_not_due() {
        // Due in December, but now is July → first Monday of August
        let now = Local.with_ymd_and_hms(2026, 7, 15, 10, 30, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 12, 10, 10, 30, 0).unwrap();
        let result = first_monday_next_month_at(due.with_timezone(&Utc), now.with_timezone(&Utc))
            .with_timezone(&Local);
        assert_eq!(result.weekday(), Weekday::Mon);
        assert_eq!(
            result.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 8, 3).unwrap()
        );
        assert_eq!(result.time(), due.time());
    }

    #[test]
    fn first_monday_next_month_rolls_year() {
        let now = Local.with_ymd_and_hms(2026, 12, 10, 8, 0, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 7, 1, 8, 0, 0).unwrap();
        let result = first_monday_next_month_at(due.with_timezone(&Utc), now.with_timezone(&Utc))
            .with_timezone(&Local);
        assert_eq!(
            result.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2027, 1, 4).unwrap()
        );
    }

    #[test]
    fn first_monday_next_month_when_first_is_monday() {
        let now = Local.with_ymd_and_hms(2027, 1, 15, 9, 0, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 7, 1, 9, 0, 0).unwrap();
        let result = first_monday_next_month_at(due.with_timezone(&Utc), now.with_timezone(&Utc))
            .with_timezone(&Local);
        assert_eq!(
            result.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2027, 2, 1).unwrap()
        );
    }

    #[test]
    fn next_monday_from_monday_skips_to_following() {
        // now 2026-07-20 Monday → following Monday is 2026-07-27
        let now = Local.with_ymd_and_hms(2026, 7, 20, 10, 0, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 7, 20, 10, 0, 0).unwrap();
        let result =
            next_monday_at(due.with_timezone(&Utc), now.with_timezone(&Utc)).with_timezone(&Local);
        assert_eq!(result.weekday(), Weekday::Mon);
        assert_eq!(
            result.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()
        );
    }
}
