import { describe, expect, it } from "vitest";

import { isOverdue, nextHourRounded, toDatetimeLocalValue } from "@/lib/time";

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
