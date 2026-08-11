import * as React from "react";

import { cn } from "@/lib/utils";

type ProgressProps = {
  /** Percent from 0-100. Null renders an indeterminate bar. */
  value: number | null;
  className?: string;
};

export function Progress({ value, className }: ProgressProps): React.JSX.Element {
  const percent = value === null ? null : Math.min(100, Math.max(0, value));

  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent === null ? undefined : Math.round(percent)}
      className={cn(
        "relative h-1.5 w-full overflow-hidden rounded-full bg-muted",
        className,
      )}
    >
      {percent === null ? (
        <div className="jade-progress-indeterminate absolute inset-y-0 w-1/3 rounded-full bg-primary" />
      ) : (
        <div
          className="h-full rounded-full bg-primary transition-[width] duration-150 ease-out"
          style={{ width: `${percent}%` }}
        />
      )}
    </div>
  );
}
