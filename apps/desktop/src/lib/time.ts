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

export function formatDue(iso: string): string {
  const date = new Date(iso);
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
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

/** True when the due time is strictly before `now`. */
export function isOverdue(iso: string, now = new Date()): boolean {
  const due = new Date(iso);
  if (Number.isNaN(due.getTime())) return false;
  return due.getTime() < now.getTime();
}
