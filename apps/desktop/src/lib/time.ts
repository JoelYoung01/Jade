/** Round local now up to the next whole hour; date stays today unless hour rolls past midnight. */
export function nextHourRounded(now = new Date()): Date {
  const result = new Date(now);
  if (result.getMinutes() === 0 && result.getSeconds() === 0 && result.getMilliseconds() === 0) {
    result.setHours(result.getHours() + 1);
  } else {
    result.setMinutes(0, 0, 0);
    result.setHours(result.getHours() + 1);
  }
  return result;
}

export function toDatetimeLocalValue(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function fromDatetimeLocalValue(value: string): Date {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    throw new Error("Invalid date/time");
  }
  return date;
}

/**
 * Whole local calendar months from `from` toward `to` (signed).
 * A partial month does not count until the day-of-month is reached.
 */
function localMonthDiff(from: Date, to: Date): number {
  let months =
    (to.getFullYear() - from.getFullYear()) * 12 + (to.getMonth() - from.getMonth());
  if (months > 0 && to.getDate() < from.getDate()) months -= 1;
  if (months < 0 && to.getDate() > from.getDate()) months += 1;
  return months;
}

/**
 * Window around `now` shown as "now" in {@link formatDue} and treated as
 * not overdue by {@link isOverdue} (avoids red styling for a few seconds past).
 */
export const DUE_NOW_WINDOW_MS = 60_000;

/** Pretty absolute local datetime (e.g. "Wed, Jul 22, 3:30 PM"). */
export function formatAbsoluteDateTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}

/**
 * Format a due datetime for display. Within ±12 local calendar months of `now`,
 * uses relative phrasing up through months ("now", "in 5 minutes", "in 1 week",
 * "in 4 months", …). Beyond that, falls back to an absolute weekday/date/time.
 */
export function formatDue(iso: string, now = new Date()): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;

  const msPerDay = 24 * 60 * 60 * 1000;
  const dayDiff = Math.round(
    (startOfLocalDay(date).getTime() - startOfLocalDay(now).getTime()) / msPerDay,
  );
  const monthDiff = localMonthDiff(now, date);

  if (Math.abs(monthDiff) <= 12) {
    const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "always" });

    if (dayDiff === 0) {
      const deltaMs = date.getTime() - now.getTime();
      const absMs = Math.abs(deltaMs);
      if (absMs <= DUE_NOW_WINDOW_MS) return "now";
      const sign = deltaMs >= 0 ? 1 : -1;
      const absMinutes = Math.round(absMs / 60_000);
      if (absMinutes < 60) {
        return rtf.format(sign * Math.max(absMinutes, 1), "minute");
      }
      const absHours = Math.round(absMs / 3_600_000);
      return rtf.format(sign * Math.max(absHours, 1), "hour");
    }

    if (Math.abs(dayDiff) < 7) {
      return rtf.format(dayDiff, "day");
    }

    if (Math.abs(monthDiff) < 1) {
      const weeks = Math.trunc(dayDiff / 7) || Math.sign(dayDiff);
      return rtf.format(weeks, "week");
    }

    return rtf.format(monthDiff, "month");
  }

  return formatAbsoluteDateTime(iso);
}

/** True when the ISO datetime falls on today's local calendar date. */
export function isToday(iso: string, now = new Date()): boolean {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return false;
  return (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  );
}

/**
 * True when the due time is past the "now" grace window
 * ({@link DUE_NOW_WINDOW_MS}), so labels that still say "now" are not overdue.
 */
export function isOverdue(iso: string, now = new Date()): boolean {
  const due = new Date(iso);
  if (Number.isNaN(due.getTime())) return false;
  return due.getTime() < now.getTime() - DUE_NOW_WINDOW_MS;
}

/** Board date-range filter presets. */
export type DateRangePreset = "today" | "this_week" | "all_time" | "custom";

/** Format a local date for `<input type="date">`. */
export function toDateInputValue(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** Parse `YYYY-MM-DD` as a local calendar date (midnight). */
export function parseLocalDateInput(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const date = new Date(year, month - 1, day);
  if (
    Number.isNaN(date.getTime()) ||
    date.getFullYear() !== year ||
    date.getMonth() !== month - 1 ||
    date.getDate() !== day
  ) {
    return null;
  }
  return date;
}

export function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0, 0);
}

export function endOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 59, 999);
}

/**
 * Local week bounds: Monday 00:00:00.000 through Sunday 23:59:59.999
 * (Monday-centric to match reschedule presets).
 */
export function localWeekBounds(now = new Date()): { start: Date; end: Date } {
  const start = startOfLocalDay(now);
  const day = start.getDay(); // 0 = Sun … 6 = Sat
  const daysFromMonday = day === 0 ? 6 : day - 1;
  start.setDate(start.getDate() - daysFromMonday);
  const end = endOfLocalDay(start);
  end.setDate(end.getDate() + 6);
  return { start, end };
}

export type DateRangeBounds = { start: Date; end: Date };

/** Resolve inclusive local datetime bounds for a date-range filter. `all_time` has no bounds. */
export function resolveDateRangeBounds(
  preset: DateRangePreset,
  customFrom: string,
  customTo: string,
  now = new Date(),
): DateRangeBounds | null {
  if (preset === "today") {
    return { start: startOfLocalDay(now), end: endOfLocalDay(now) };
  }
  if (preset === "this_week") {
    return localWeekBounds(now);
  }
  if (preset === "all_time") {
    return null;
  }

  const from = parseLocalDateInput(customFrom);
  const to = parseLocalDateInput(customTo);
  if (!from || !to) return null;
  if (from.getTime() <= to.getTime()) {
    return { start: startOfLocalDay(from), end: endOfLocalDay(to) };
  }
  return { start: startOfLocalDay(to), end: endOfLocalDay(from) };
}

/**
 * Whether a task's due datetime falls in the filter range.
 * Incomplete overdue tasks always match when `includeOverdue` is true (default).
 */
export function matchesDueDateRange(
  dueAtIso: string,
  bounds: DateRangeBounds | null,
  options: { includeOverdue?: boolean; now?: Date; isComplete?: boolean } = {},
): boolean {
  const { includeOverdue = true, now = new Date(), isComplete = false } = options;
  if (includeOverdue && !isComplete && isOverdue(dueAtIso, now)) {
    return true;
  }
  if (!bounds) return false;
  const due = new Date(dueAtIso);
  if (Number.isNaN(due.getTime())) return false;
  const t = due.getTime();
  return t >= bounds.start.getTime() && t <= bounds.end.getTime();
}

/** Same presets as the task context-menu reschedule options (excluding custom). */
export type DueQuickPreset = "today" | "tomorrow" | "next_monday" | "first_monday_next_month";

/** True when two dates fall on the same local calendar day. */
export function isSameLocalCalendarDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** Local hour used when the create/edit dialog schedules a non-today date. */
export const DIALOG_NON_TODAY_DUE_HOUR = 17;

export type ApplyDuePresetOptions = {
  /**
   * When true and the resulting calendar day is not today, set local time to
   * {@link DIALOG_NON_TODAY_DUE_HOUR}:00. Used by the create/edit dialog only;
   * context-menu reschedule leaves time-of-day unchanged.
   */
  defaultAfternoonIfNotToday?: boolean;
};

/**
 * Keep local time-of-day from `dueAt`; set the calendar date from `now`
 * according to the quick-select preset (absolute relative to today, not a
 * relative push from the current due date).
 */
export function applyDuePreset(
  dueAt: Date,
  mode: DueQuickPreset,
  now = new Date(),
  options: ApplyDuePresetOptions = {},
): Date {
  const next = new Date(dueAt);
  if (mode === "today") {
    next.setFullYear(now.getFullYear(), now.getMonth(), now.getDate());
  } else if (mode === "tomorrow") {
    next.setFullYear(now.getFullYear(), now.getMonth(), now.getDate());
    next.setDate(next.getDate() + 1);
  } else if (mode === "next_monday") {
    const day = now.getDay();
    const daysUntil = day === 1 ? 7 : (8 - day) % 7 || 7;
    next.setFullYear(now.getFullYear(), now.getMonth(), now.getDate());
    next.setDate(next.getDate() + daysUntil);
  } else {
    next.setFullYear(now.getFullYear(), now.getMonth(), 1);
    next.setMonth(next.getMonth() + 1);
    const day = next.getDay();
    next.setDate(1 + ((8 - day) % 7));
  }

  if (options.defaultAfternoonIfNotToday && !isSameLocalCalendarDay(next, now)) {
    next.setHours(DIALOG_NON_TODAY_DUE_HOUR, 0, 0, 0);
  }
  return next;
}

/** Whether `dueAt`'s local calendar date matches what `mode` would set from `now`. */
export function matchesDueQuickPreset(
  dueAt: Date,
  mode: DueQuickPreset,
  now = new Date(),
): boolean {
  return isSameLocalCalendarDay(dueAt, applyDuePreset(dueAt, mode, now));
}
