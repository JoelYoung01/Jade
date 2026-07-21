import { describe, expect, it } from "vitest";

import {
  cronFromPreset,
  describeCron,
  inferPreset,
  nextOccurrences,
  validateCron,
} from "@/lib/repeat";

describe("repeat helpers", () => {
  const tuesdayNine = new Date(2026, 6, 21, 9, 0, 0); // local Jul 21 2026 09:00 (Tue)

  it("builds presets from due time", () => {
    expect(cronFromPreset("daily", tuesdayNine)).toBe("0 9 * * *");
    expect(cronFromPreset("weekdays", tuesdayNine)).toBe("0 9 * * 1-5");
    expect(cronFromPreset("weekly", tuesdayNine)).toBe("0 9 * * 2");
    expect(cronFromPreset("monthly", tuesdayNine)).toBe("0 9 21 * *");
    expect(cronFromPreset("yearly", tuesdayNine)).toBe("0 9 21 7 *");
  });

  it("infers presets and falls back to custom", () => {
    expect(inferPreset(null, tuesdayNine)).toBe("never");
    expect(inferPreset("0 9 * * *", tuesdayNine)).toBe("daily");
    expect(inferPreset("0 9 * * 1-5", tuesdayNine)).toBe("weekdays");
    expect(inferPreset("15 8 * * 3", tuesdayNine)).toBe("custom");
  });

  it("validates cron expressions", () => {
    expect(validateCron("0 9 * * *").ok).toBe(true);
    expect(validateCron("not a cron").ok).toBe(false);
  });

  it("describes and projects next occurrences", () => {
    const description = describeCron("0 9 * * 1-5");
    expect(description).toBeTruthy();
    const next = nextOccurrences("0 9 * * *", new Date(2026, 6, 21, 8, 0, 0), 2);
    expect(next).toHaveLength(2);
    expect(next[0]?.getHours()).toBe(9);
  });
});
