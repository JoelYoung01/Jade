import { describe, expect, it } from "vitest";

import {
  applyDuePreset,
  formatDue,
  isOverdue,
  localWeekBounds,
  matchesDueDateRange,
  matchesDueQuickPreset,
  nextHourRounded,
  resolveDateRangeBounds,
  toDatetimeLocalValue,
} from "@/lib/time";

describe("nextHourRounded", () => {
  it("rounds partial hours up", () => {
    const input = new Date(2026, 6, 20, 14, 23, 45);
    const result = nextHourRounded(input);
    expect(result.getHours()).toBe(15);
    expect(result.getMinutes()).toBe(0);
    expect(result.getSeconds()).toBe(0);
  });

  it("advances from an exact hour", () => {
    const input = new Date(2026, 6, 20, 15, 0, 0);
    const result = nextHourRounded(input);
    expect(result.getHours()).toBe(16);
  });
});

describe("toDatetimeLocalValue", () => {
  it("formats for datetime-local inputs", () => {
    const input = new Date(2026, 6, 20, 9, 5, 0);
    expect(toDatetimeLocalValue(input)).toBe("2026-07-20T09:05");
  });
});

describe("isOverdue", () => {
  const now = new Date("2026-07-21T12:00:00.000Z");

  it("is true when due is in the past", () => {
    expect(isOverdue("2026-07-21T11:59:59.000Z", now)).toBe(true);
  });

  it("is false when due is now or in the future", () => {
    expect(isOverdue("2026-07-21T12:00:00.000Z", now)).toBe(false);
    expect(isOverdue("2026-07-21T12:00:01.000Z", now)).toBe(false);
  });
});

describe("formatDue", () => {
  const now = new Date(2026, 6, 22, 12, 0, 0); // Wed Jul 22 2026 noon local

  it("uses hour/minute granularity for due times today", () => {
    expect(formatDue(new Date(2026, 6, 22, 12, 0, 0).toISOString(), now)).toBe("now");
    expect(formatDue(new Date(2026, 6, 22, 12, 0, 45).toISOString(), now)).toBe("now");
    expect(formatDue(new Date(2026, 6, 22, 11, 59, 15).toISOString(), now)).toBe("now");
    expect(formatDue(new Date(2026, 6, 22, 12, 5, 0).toISOString(), now)).toBe("in 5 minutes");
    expect(formatDue(new Date(2026, 6, 22, 11, 45, 0).toISOString(), now)).toBe("15 minutes ago");
    expect(formatDue(new Date(2026, 6, 22, 15, 0, 0).toISOString(), now)).toBe("in 3 hours");
    expect(formatDue(new Date(2026, 6, 22, 9, 0, 0).toISOString(), now)).toBe("3 hours ago");
  });

  it("uses relative phrasing through months", () => {
    expect(formatDue(new Date(2026, 6, 23, 9, 0, 0).toISOString(), now)).toBe("in 1 day");
    expect(formatDue(new Date(2026, 6, 21, 9, 0, 0).toISOString(), now)).toBe("1 day ago");
    expect(formatDue(new Date(2026, 6, 25, 9, 0, 0).toISOString(), now)).toBe("in 3 days");
    expect(formatDue(new Date(2026, 6, 19, 9, 0, 0).toISOString(), now)).toBe("3 days ago");
    expect(formatDue(new Date(2026, 6, 29, 9, 0, 0).toISOString(), now)).toBe("in 1 week");
    expect(formatDue(new Date(2026, 6, 15, 9, 0, 0).toISOString(), now)).toBe("1 week ago");
    expect(formatDue(new Date(2026, 7, 5, 9, 0, 0).toISOString(), now)).toBe("in 2 weeks");
    expect(formatDue(new Date(2026, 7, 22, 9, 0, 0).toISOString(), now)).toBe("in 1 month");
    expect(formatDue(new Date(2026, 5, 22, 9, 0, 0).toISOString(), now)).toBe("1 month ago");
    expect(formatDue(new Date(2026, 10, 22, 9, 0, 0).toISOString(), now)).toBe("in 4 months");
    expect(formatDue(new Date(2026, 2, 22, 9, 0, 0).toISOString(), now)).toBe("4 months ago");
  });

  it("falls back to absolute formatting beyond twelve months", () => {
    const absolute = formatDue(new Date(2027, 7, 22, 15, 30, 0).toISOString(), now);
    expect(absolute).not.toMatch(/ago|in \d|today|now/);
    expect(absolute).toMatch(/22/);
  });
});

describe("localWeekBounds", () => {
  it("spans Monday through Sunday of the current week", () => {
    // Wednesday Jul 22 2026
    const now = new Date(2026, 6, 22, 15, 0, 0);
    const { start, end } = localWeekBounds(now);
    expect(start.getDay()).toBe(1);
    expect(start.getDate()).toBe(20);
    expect(start.getHours()).toBe(0);
    expect(end.getDay()).toBe(0);
    expect(end.getDate()).toBe(26);
    expect(end.getHours()).toBe(23);
  });
});

describe("resolveDateRangeBounds", () => {
  const now = new Date(2026, 6, 22, 12, 0, 0);

  it("resolves today", () => {
    const bounds = resolveDateRangeBounds("today", "", "", now);
    expect(bounds?.start.getDate()).toBe(22);
    expect(bounds?.end.getDate()).toBe(22);
  });

  it("resolves this week", () => {
    const bounds = resolveDateRangeBounds("this_week", "", "", now);
    expect(bounds?.start.getDate()).toBe(20);
    expect(bounds?.end.getDate()).toBe(26);
  });

  it("returns null for all time (unbounded)", () => {
    expect(resolveDateRangeBounds("all_time", "", "", now)).toBeNull();
  });

  it("resolves custom and swaps inverted ranges", () => {
    const bounds = resolveDateRangeBounds("custom", "2026-07-25", "2026-07-20", now);
    expect(bounds?.start.getDate()).toBe(20);
    expect(bounds?.end.getDate()).toBe(25);
  });

  it("returns null for incomplete custom dates", () => {
    expect(resolveDateRangeBounds("custom", "2026-07-20", "", now)).toBeNull();
  });
});

describe("matchesDueDateRange", () => {
  const now = new Date(2026, 6, 22, 12, 0, 0); // Wed noon
  const todayBounds = resolveDateRangeBounds("today", "", "", now)!;

  it("includes tasks due today", () => {
    expect(matchesDueDateRange("2026-07-22T18:00:00", todayBounds, { now })).toBe(true);
  });

  it("excludes future tasks outside the range", () => {
    expect(matchesDueDateRange("2026-07-23T09:00:00", todayBounds, { now })).toBe(false);
  });

  it("always includes incomplete overdue tasks", () => {
    expect(
      matchesDueDateRange("2026-07-20T09:00:00", todayBounds, {
        now,
        isComplete: false,
      }),
    ).toBe(true);
  });

  it("does not force-include completed past-due tasks", () => {
    expect(
      matchesDueDateRange("2026-07-20T09:00:00", todayBounds, {
        now,
        isComplete: true,
      }),
    ).toBe(false);
  });
});

describe("applyDuePreset", () => {
  const due = new Date(2026, 6, 22, 15, 30, 0); // Wed Jul 22 2026 15:30
  const now = new Date(2026, 6, 20, 10, 0, 0); // Mon Jul 20

  it("moves to today keeping time", () => {
    const result = applyDuePreset(due, "today", now);
    expect(result.getFullYear()).toBe(2026);
    expect(result.getMonth()).toBe(6);
    expect(result.getDate()).toBe(20);
    expect(result.getHours()).toBe(15);
    expect(result.getMinutes()).toBe(30);
  });

  it("sets tomorrow relative to now, not the current due", () => {
    const result = applyDuePreset(due, "tomorrow", now);
    expect(result.getDate()).toBe(21);
    expect(result.getHours()).toBe(15);
    // Clicking again stays on tomorrow (idempotent vs advancing).
    const again = applyDuePreset(result, "tomorrow", now);
    expect(again.getDate()).toBe(21);
  });

  it("jumps to the next Monday after now", () => {
    const result = applyDuePreset(due, "next_monday", now);
    expect(result.getDay()).toBe(1);
    expect(result.getDate()).toBe(27);
  });

  it("jumps to first Monday of the month after now", () => {
    const result = applyDuePreset(due, "first_monday_next_month", now);
    expect(result.getMonth()).toBe(7);
    expect(result.getDay()).toBe(1);
    expect(result.getDate()).toBe(3);
  });

  it("with defaultAfternoonIfNotToday, keeps time for today", () => {
    const result = applyDuePreset(due, "today", now, { defaultAfternoonIfNotToday: true });
    expect(result.getDate()).toBe(20);
    expect(result.getHours()).toBe(15);
    expect(result.getMinutes()).toBe(30);
  });

  it("with defaultAfternoonIfNotToday, snaps non-today presets to 5pm local", () => {
    const tomorrow = applyDuePreset(due, "tomorrow", now, { defaultAfternoonIfNotToday: true });
    expect(tomorrow.getDate()).toBe(21);
    expect(tomorrow.getHours()).toBe(17);
    expect(tomorrow.getMinutes()).toBe(0);

    const monday = applyDuePreset(due, "next_monday", now, { defaultAfternoonIfNotToday: true });
    expect(monday.getDate()).toBe(27);
    expect(monday.getHours()).toBe(17);

    const first = applyDuePreset(due, "first_monday_next_month", now, {
      defaultAfternoonIfNotToday: true,
    });
    expect(first.getDate()).toBe(3);
    expect(first.getHours()).toBe(17);
  });
});

describe("matchesDueQuickPreset", () => {
  const now = new Date(2026, 6, 20, 10, 0, 0); // Mon Jul 20

  it("matches today / tomorrow / next monday by calendar day", () => {
    expect(matchesDueQuickPreset(new Date(2026, 6, 20, 9, 0, 0), "today", now)).toBe(true);
    expect(matchesDueQuickPreset(new Date(2026, 6, 21, 15, 30, 0), "tomorrow", now)).toBe(true);
    expect(matchesDueQuickPreset(new Date(2026, 6, 27, 8, 0, 0), "next_monday", now)).toBe(true);
    expect(
      matchesDueQuickPreset(new Date(2026, 7, 3, 12, 0, 0), "first_monday_next_month", now),
    ).toBe(true);
  });

  it("does not match unrelated dates", () => {
    expect(matchesDueQuickPreset(new Date(2026, 6, 22, 15, 30, 0), "tomorrow", now)).toBe(false);
    expect(matchesDueQuickPreset(new Date(2026, 6, 20, 10, 0, 0), "next_monday", now)).toBe(false);
  });
});
