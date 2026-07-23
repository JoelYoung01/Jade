import * as React from "react";

import { parseTagParts, tagKeyBackground } from "@/lib/tags";
import { cn } from "@/lib/utils";

type TagLabelProps = {
  name: string;
  className?: string;
  /** When true, the keyed color block is flush to the left (no outer chip padding). */
  flushKey?: boolean;
};

/** Renders a tag name; keyed tags (`Project:Portal`) style the key with a hashed color. */
export function TagLabel({
  name,
  className,
  flushKey = false,
}: TagLabelProps): React.JSX.Element {
  const parts = parseTagParts(name);

  if (parts.kind === "plain") {
    return <span className={className}>{parts.name}</span>;
  }

  return (
    <span className={cn("inline-flex items-center gap-1.5", className)}>
      <span
        className={cn(
          "px-1.5 font-medium text-white/95",
          flushKey
            ? "self-stretch rounded-l-[inherit] rounded-r-sm flex items-center"
            : "rounded-sm py-px",
        )}
        style={{ backgroundColor: tagKeyBackground(parts.key) }}
      >
        {parts.key}
      </span>
      <span className={cn("opacity-80", flushKey && "py-0.5")}>{parts.value}</span>
    </span>
  );
}
