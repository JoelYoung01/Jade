import * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { apiGetSettings, apiSetEndOfDay } from "@/lib/api";
import { DEFAULT_END_OF_DAY } from "@/lib/time";
import type { EndOfDay } from "@/lib/types";

type PreferencesDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called after end of day is saved so the app can refresh create defaults. */
  onEndOfDayChange?: (endOfDay: EndOfDay) => void;
};

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function endOfDayToTimeValue(endOfDay: EndOfDay): string {
  return `${pad2(endOfDay.hour)}:${pad2(endOfDay.minute)}`;
}

function timeValueToEndOfDay(value: string): EndOfDay | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (!Number.isInteger(hour) || !Number.isInteger(minute)) return null;
  if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
  return { hour, minute };
}

export function PreferencesDialog({
  open,
  onOpenChange,
  onEndOfDayChange,
}: PreferencesDialogProps): React.JSX.Element {
  const [timeValue, setTimeValue] = React.useState(() =>
    endOfDayToTimeValue(DEFAULT_END_OF_DAY),
  );
  const [busy, setBusy] = React.useState(false);
  const [message, setMessage] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void apiGetSettings()
      .then((settings) => {
        if (cancelled) return;
        setTimeValue(endOfDayToTimeValue(settings.end_of_day));
        setMessage(null);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  async function handleSave(): Promise<void> {
    const parsed = timeValueToEndOfDay(timeValue);
    if (!parsed) {
      setError("Enter a valid time (HH:MM).");
      return;
    }
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const settings = await apiSetEndOfDay(parsed);
      setTimeValue(endOfDayToTimeValue(settings.end_of_day));
      onEndOfDayChange?.(settings.end_of_day);
      setMessage("Preferences saved.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Preferences</DialogTitle>
          <DialogDescription>
            Local settings for this device. New tasks default to today at end of day.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-1">
          <div className="grid gap-2">
            <Label htmlFor="end-of-day">End of day</Label>
            <Input
              id="end-of-day"
              type="time"
              value={timeValue}
              onChange={(e) => setTimeValue(e.target.value)}
              className="w-40"
            />
            <p className="text-muted-foreground text-xs">
              Used as the default due time when creating a task, and when quick-picking a
              non-today date.
            </p>
          </div>

          {message ? <p className="text-sm text-foreground">{message}</p> : null}
          {error ? <p className="text-sm text-destructive">{error}</p> : null}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
          <Button type="button" disabled={busy} onClick={() => void handleSave()}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
