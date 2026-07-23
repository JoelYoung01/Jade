import { useEffect, useState } from "react";

/**
 * Wall-clock `Date` that re-renders on an interval (default: every minute,
 * aligned to the clock) and when the tab becomes visible again.
 * Use for relative due labels / overdue checks that would otherwise freeze
 * until some unrelated state update.
 */
export function useNow(intervalMs = 60_000): Date {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    let timeoutId = 0;
    let intervalId = 0;

    const tick = () => setNow(new Date());

    const msUntilAligned = intervalMs - (Date.now() % intervalMs);
    timeoutId = window.setTimeout(() => {
      tick();
      intervalId = window.setInterval(tick, intervalMs);
    }, msUntilAligned);

    function onVisibility(): void {
      if (document.visibilityState === "visible") tick();
    }
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      window.clearTimeout(timeoutId);
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [intervalMs]);

  return now;
}
