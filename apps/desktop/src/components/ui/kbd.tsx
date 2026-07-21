import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

type KbdProps = {
  children: ReactNode;
  className?: string;
};

export function Kbd({ children, className }: KbdProps): React.JSX.Element {
  return (
    <kbd
      className={cn(
        "rounded border border-current/25 bg-current/10 px-1 py-px font-sans text-[9px] leading-none",
        className,
      )}
    >
      {children}
    </kbd>
  );
}

type ShortcutKeysProps = {
  keys: string[];
  className?: string;
  label?: string;
};

export function ShortcutKeys({ keys, className, label }: ShortcutKeysProps): React.JSX.Element {
  return (
    <span
      className={cn("inline-flex items-center gap-0.5", className)}
      aria-label={label ?? keys.join("+")}
    >
      {keys.map((key) => (
        <Kbd key={key}>{key}</Kbd>
      ))}
    </span>
  );
}
