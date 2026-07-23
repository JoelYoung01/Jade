import * as React from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  builderToCron,
  cronToBuilder,
  describeCron,
  nextOccurrences,
  validateCron,
  type BuilderFrequency,
} from "@/lib/repeat";
import { formatDue } from "@/lib/time";
import { cn } from "@/lib/utils";

const WEEKDAYS: { value: number; label: string }[] = [
  { value: 0, label: "Sun" },
  { value: 1, label: "Mon" },
  { value: 2, label: "Tue" },
  { value: 3, label: "Wed" },
  { value: 4, label: "Thu" },
  { value: 5, label: "Fri" },
  { value: 6, label: "Sat" },
];

const selectClassName = cn(
  "flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-none",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
);

type RepeatCronEditorProps = {
  value: string;
  dueLocal: string;
  onChange: (cron: string) => void;
};

export function RepeatCronEditor({
  value,
  dueLocal,
  onChange,
}: RepeatCronEditorProps): React.JSX.Element {
  const due = React.useMemo(() => {
    const parsed = new Date(dueLocal);
    return Number.isNaN(parsed.getTime()) ? new Date() : parsed;
  }, [dueLocal]);

  const initial = React.useMemo(() => cronToBuilder(value || "0 9 * * *", due), [value, due]);

  const [frequency, setFrequency] = React.useState<BuilderFrequency>(initial.frequency);
  const [minute, setMinute] = React.useState(initial.minute);
  const [hour, setHour] = React.useState(initial.hour);
  const [weekdays, setWeekdays] = React.useState<number[]>(initial.weekdays);
  const [dayOfMonth, setDayOfMonth] = React.useState(initial.dayOfMonth);
  const [month, setMonth] = React.useState(initial.month);
  const [raw, setRaw] = React.useState(value);
  const [rawAuthoritative, setRawAuthoritative] = React.useState(false);

  // Keep raw in sync when parent changes (e.g. switching into Custom from a preset).
  React.useEffect(() => {
    setRaw(value);
    setRawAuthoritative(false);
    const next = cronToBuilder(value || "0 9 * * *", due);
    setFrequency(next.frequency);
    setMinute(next.minute);
    setHour(next.hour);
    setWeekdays(next.weekdays);
    setDayOfMonth(next.dayOfMonth);
    setMonth(next.month);
  }, [value, due]);

  function emitFromBuilder(
    nextFreq: BuilderFrequency,
    nextMinute: number,
    nextHour: number,
    nextWeekdays: number[],
    nextDom: number,
    nextMonth: number,
  ): void {
    const cron = builderToCron(nextFreq, {
      minute: nextMinute,
      hour: nextHour,
      weekdays: nextWeekdays,
      dayOfMonth: nextDom,
      month: nextMonth,
    });
    setRaw(cron);
    setRawAuthoritative(false);
    onChange(cron);
  }

  function handleRawChange(next: string): void {
    setRaw(next);
    setRawAuthoritative(true);
    onChange(next.trim());
  }

  const validation = validateCron(raw);
  const description = validation.ok ? describeCron(raw.trim()) : null;
  const upcoming = validation.ok ? nextOccurrences(raw.trim(), new Date(), 3) : [];

  return (
    <div className="grid gap-3 rounded-md border border-border bg-muted/30 p-3">
      <div className="grid grid-cols-2 gap-3">
        <div className="grid gap-1.5">
          <Label htmlFor="repeat-freq">Frequency</Label>
          <select
            id="repeat-freq"
            className={selectClassName}
            value={frequency}
            disabled={rawAuthoritative}
            onChange={(e) => {
              const next = e.target.value as BuilderFrequency;
              setFrequency(next);
              emitFromBuilder(next, minute, hour, weekdays, dayOfMonth, month);
            }}
          >
            <option value="daily">Daily</option>
            <option value="weekly">Weekly</option>
            <option value="monthly">Monthly</option>
            <option value="yearly">Yearly</option>
          </select>
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div className="grid gap-1.5">
            <Label htmlFor="repeat-hour">Hour</Label>
            <Input
              id="repeat-hour"
              type="number"
              min={0}
              max={23}
              value={hour}
              disabled={rawAuthoritative}
              onChange={(e) => {
                const next = Math.min(23, Math.max(0, Number.parseInt(e.target.value, 10) || 0));
                setHour(next);
                emitFromBuilder(frequency, minute, next, weekdays, dayOfMonth, month);
              }}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="repeat-minute">Minute</Label>
            <Input
              id="repeat-minute"
              type="number"
              min={0}
              max={59}
              value={minute}
              disabled={rawAuthoritative}
              onChange={(e) => {
                const next = Math.min(59, Math.max(0, Number.parseInt(e.target.value, 10) || 0));
                setMinute(next);
                emitFromBuilder(frequency, next, hour, weekdays, dayOfMonth, month);
              }}
            />
          </div>
        </div>
      </div>

      {frequency === "weekly" && !rawAuthoritative && (
        <div className="grid gap-1.5">
          <Label>Days of week</Label>
          <div className="flex flex-wrap gap-1" role="group" aria-label="Days of week">
            {WEEKDAYS.map((day) => {
              const active = weekdays.includes(day.value);
              return (
                <button
                  key={day.value}
                  type="button"
                  aria-pressed={active}
                  className={cn(
                    "rounded-md px-2 py-1 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    active
                      ? "bg-primary text-primary-foreground"
                      : "border border-border text-muted-foreground hover:bg-accent",
                  )}
                  onClick={() => {
                    const next = active
                      ? weekdays.filter((d) => d !== day.value)
                      : [...weekdays, day.value];
                    const safe = next.length > 0 ? next : [day.value];
                    setWeekdays(safe);
                    emitFromBuilder(frequency, minute, hour, safe, dayOfMonth, month);
                  }}
                >
                  {day.label}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {(frequency === "monthly" || frequency === "yearly") && !rawAuthoritative && (
        <div className="grid grid-cols-2 gap-3">
          <div className="grid gap-1.5">
            <Label htmlFor="repeat-dom">Day of month</Label>
            <Input
              id="repeat-dom"
              type="number"
              min={1}
              max={31}
              value={dayOfMonth}
              onChange={(e) => {
                const next = Math.min(31, Math.max(1, Number.parseInt(e.target.value, 10) || 1));
                setDayOfMonth(next);
                emitFromBuilder(frequency, minute, hour, weekdays, next, month);
              }}
            />
          </div>
          {frequency === "yearly" && (
            <div className="grid gap-1.5">
              <Label htmlFor="repeat-month">Month</Label>
              <Input
                id="repeat-month"
                type="number"
                min={1}
                max={12}
                value={month}
                onChange={(e) => {
                  const next = Math.min(12, Math.max(1, Number.parseInt(e.target.value, 10) || 1));
                  setMonth(next);
                  emitFromBuilder(frequency, minute, hour, weekdays, dayOfMonth, next);
                }}
              />
            </div>
          )}
        </div>
      )}

      <div className="grid gap-1.5">
        <Label htmlFor="repeat-cron-raw">Cron expression</Label>
        <Input
          id="repeat-cron-raw"
          value={raw}
          spellCheck={false}
          className="font-mono text-xs"
          placeholder="minute hour day month weekday"
          onChange={(e) => handleRawChange(e.target.value)}
        />
        {rawAuthoritative && (
          <button
            type="button"
            className="justify-self-start text-xs text-muted-foreground underline-offset-2 hover:underline"
            onClick={() => {
              setRawAuthoritative(false);
              const next = cronToBuilder(raw.trim() || "0 9 * * *", due);
              setFrequency(next.frequency);
              setMinute(next.minute);
              setHour(next.hour);
              setWeekdays(next.weekdays);
              setDayOfMonth(next.dayOfMonth);
              setMonth(next.month);
            }}
          >
            Sync builder from expression
          </button>
        )}
      </div>

      {validation.ok ? (
        <div className="grid gap-1 text-xs text-muted-foreground">
          {description && <p className="text-foreground/80">{description}</p>}
          {upcoming.length > 0 && (
            <ul className="list-inside list-disc">
              {upcoming.map((date) => (
                <li key={date.toISOString()}>{formatDue(date.toISOString())}</li>
              ))}
            </ul>
          )}
        </div>
      ) : (
        <p className="text-xs text-destructive">{validation.error}</p>
      )}
    </div>
  );
}
