import { CronExpressionParser } from "cron-parser";
import cronstrue from "cronstrue";

import type { RepeatPreset } from "@/lib/types";

/** Minute / hour extracted from a due Date (local). */
export function dueTimeParts(due: Date): { minute: number; hour: number } {
  return { minute: due.getMinutes(), hour: due.getHours() };
}

/** Cron weekday: 0 = Sunday … 6 = Saturday (POSIX). */
export function dueCronWeekday(due: Date): number {
  return due.getDay();
}

export function cronFromPreset(preset: Exclude<RepeatPreset, "never" | "custom">, due: Date): string {
  const { minute, hour } = dueTimeParts(due);
  switch (preset) {
    case "daily":
      return `${minute} ${hour} * * *`;
    case "weekdays":
      return `${minute} ${hour} * * 1-5`;
    case "weekly":
      return `${minute} ${hour} * * ${dueCronWeekday(due)}`;
    case "monthly":
      return `${minute} ${hour} ${due.getDate()} * *`;
    case "yearly":
      return `${minute} ${hour} ${due.getDate()} ${due.getMonth() + 1} *`;
  }
}

/**
 * Infer which preset (if any) matches `cron` given the task's due datetime.
 * Falls back to `custom` when the expression doesn't match a regenerated preset,
 * or `never` when cron is null/empty.
 */
export function inferPreset(cron: string | null | undefined, due: Date): RepeatPreset {
  if (!cron || cron.trim().length === 0) return "never";
  const normalized = cron.trim().replace(/\s+/g, " ");
  const presets: Exclude<RepeatPreset, "never" | "custom">[] = [
    "daily",
    "weekdays",
    "weekly",
    "monthly",
    "yearly",
  ];
  for (const preset of presets) {
    if (cronFromPreset(preset, due) === normalized) return preset;
  }
  return "custom";
}

export function describeCron(cron: string): string | null {
  try {
    return cronstrue.toString(cron, { use24HourTimeFormat: false, verbose: false });
  } catch {
    return null;
  }
}

export function validateCron(cron: string): { ok: true } | { ok: false; error: string } {
  const trimmed = cron.trim();
  if (!trimmed) return { ok: false, error: "Schedule is empty" };
  // Prefer 5-field POSIX (no seconds). cron-parser's `strict` mode requires 6 fields.
  const parts = trimmed.split(/\s+/);
  if (parts.length !== 5) {
    return {
      ok: false,
      error: "Expected a 5-field cron: minute hour day month weekday",
    };
  }
  try {
    CronExpressionParser.parse(trimmed);
    return { ok: true };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "Invalid cron expression",
    };
  }
}

/** Next N occurrences after `after` (local evaluation via cron-parser). */
export function nextOccurrences(cron: string, after: Date, count = 3): Date[] {
  const trimmed = cron.trim();
  if (!trimmed) return [];
  try {
    const interval = CronExpressionParser.parse(trimmed, {
      currentDate: after,
    });
    const out: Date[] = [];
    for (let i = 0; i < count; i += 1) {
      out.push(interval.next().toDate());
    }
    return out;
  } catch {
    return [];
  }
}

/** Fields for the custom cron builder. */
export type CronFields = {
  minute: string;
  hour: string;
  dayOfMonth: string;
  month: string;
  dayOfWeek: string;
};

export function parseCronFields(cron: string): CronFields | null {
  const parts = cron.trim().split(/\s+/);
  if (parts.length !== 5) return null;
  const [minute, hour, dayOfMonth, month, dayOfWeek] = parts;
  if (!minute || !hour || !dayOfMonth || !month || !dayOfWeek) return null;
  return { minute, hour, dayOfMonth, month, dayOfWeek };
}

export function joinCronFields(fields: CronFields): string {
  return `${fields.minute} ${fields.hour} ${fields.dayOfMonth} ${fields.month} ${fields.dayOfWeek}`;
}

export type BuilderFrequency = "daily" | "weekly" | "monthly" | "yearly";

export function builderToCron(
  frequency: BuilderFrequency,
  opts: {
    minute: number;
    hour: number;
    weekdays: number[];
    dayOfMonth: number;
    month: number;
  },
): string {
  const { minute, hour, weekdays, dayOfMonth, month } = opts;
  switch (frequency) {
    case "daily":
      return `${minute} ${hour} * * *`;
    case "weekly": {
      const days = (weekdays.length > 0 ? [...weekdays] : [0]).sort((a, b) => a - b);
      return `${minute} ${hour} * * ${days.join(",")}`;
    }
    case "monthly":
      return `${minute} ${hour} ${dayOfMonth} * *`;
    case "yearly":
      return `${minute} ${hour} ${dayOfMonth} ${month} *`;
  }
}

/** Best-effort reverse of builderToCron for initializing the builder from a cron string. */
export function cronToBuilder(cron: string, fallbackDue: Date): {
  frequency: BuilderFrequency;
  minute: number;
  hour: number;
  weekdays: number[];
  dayOfMonth: number;
  month: number;
} {
  const fields = parseCronFields(cron);
  const { minute: dueMin, hour: dueHour } = dueTimeParts(fallbackDue);
  if (!fields) {
    return {
      frequency: "daily",
      minute: dueMin,
      hour: dueHour,
      weekdays: [dueCronWeekday(fallbackDue)],
      dayOfMonth: fallbackDue.getDate(),
      month: fallbackDue.getMonth() + 1,
    };
  }

  const minute = Number.parseInt(fields.minute, 10);
  const hour = Number.parseInt(fields.hour, 10);
  const safeMinute = Number.isFinite(minute) ? minute : dueMin;
  const safeHour = Number.isFinite(hour) ? hour : dueHour;

  if (fields.dayOfMonth === "*" && fields.month === "*" && fields.dayOfWeek === "*") {
    return {
      frequency: "daily",
      minute: safeMinute,
      hour: safeHour,
      weekdays: [dueCronWeekday(fallbackDue)],
      dayOfMonth: fallbackDue.getDate(),
      month: fallbackDue.getMonth() + 1,
    };
  }

  if (fields.dayOfMonth === "*" && fields.month === "*" && fields.dayOfWeek !== "*") {
    const weekdays = fields.dayOfWeek
      .split(",")
      .flatMap((part) => {
        if (part.includes("-")) {
          const bounds = part.split("-").map((n) => Number.parseInt(n, 10));
          const start = bounds[0];
          const end = bounds[1];
          if (start === undefined || end === undefined) return [];
          if (!Number.isFinite(start) || !Number.isFinite(end)) return [];
          const out: number[] = [];
          for (let d = start; d <= end; d += 1) out.push(d);
          return out;
        }
        const n = Number.parseInt(part, 10);
        return Number.isFinite(n) ? [n] : [];
      })
      .filter((d) => d >= 0 && d <= 7)
      .map((d) => (d === 7 ? 0 : d));
    return {
      frequency: "weekly",
      minute: safeMinute,
      hour: safeHour,
      weekdays: weekdays.length > 0 ? weekdays : [dueCronWeekday(fallbackDue)],
      dayOfMonth: fallbackDue.getDate(),
      month: fallbackDue.getMonth() + 1,
    };
  }

  if (fields.month === "*" && fields.dayOfWeek === "*" && fields.dayOfMonth !== "*") {
    const dayOfMonth = Number.parseInt(fields.dayOfMonth, 10);
    return {
      frequency: "monthly",
      minute: safeMinute,
      hour: safeHour,
      weekdays: [dueCronWeekday(fallbackDue)],
      dayOfMonth: Number.isFinite(dayOfMonth) ? dayOfMonth : fallbackDue.getDate(),
      month: fallbackDue.getMonth() + 1,
    };
  }

  if (fields.dayOfWeek === "*" && fields.dayOfMonth !== "*" && fields.month !== "*") {
    const dayOfMonth = Number.parseInt(fields.dayOfMonth, 10);
    const month = Number.parseInt(fields.month, 10);
    return {
      frequency: "yearly",
      minute: safeMinute,
      hour: safeHour,
      weekdays: [dueCronWeekday(fallbackDue)],
      dayOfMonth: Number.isFinite(dayOfMonth) ? dayOfMonth : fallbackDue.getDate(),
      month: Number.isFinite(month) ? month : fallbackDue.getMonth() + 1,
    };
  }

  return {
    frequency: "daily",
    minute: safeMinute,
    hour: safeHour,
    weekdays: [dueCronWeekday(fallbackDue)],
    dayOfMonth: fallbackDue.getDate(),
    month: fallbackDue.getMonth() + 1,
  };
}
