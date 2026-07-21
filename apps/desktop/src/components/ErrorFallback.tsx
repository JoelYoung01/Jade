import * as React from "react";
import { Check, ChevronDown, Copy } from "lucide-react";

import { Button } from "@/components/ui/button";

function formatErrorDetails(error: Error, componentStack: string | null): string {
  const parts = [
    `${error.name}: ${error.message}`,
    error.stack ? `\nStack:\n${error.stack}` : null,
    componentStack ? `\nComponent stack:\n${componentStack.trim()}` : null,
  ];
  return parts.filter(Boolean).join("\n");
}

type ErrorFallbackProps = {
  error: Error;
  componentStack: string | null;
  onReset: () => void;
};

export function ErrorFallback({
  error,
  componentStack,
  onReset,
}: ErrorFallbackProps): React.JSX.Element {
  const [copied, setCopied] = React.useState(false);
  const details = formatErrorDetails(error, componentStack);

  async function copyDetails(): Promise<void> {
    try {
      await navigator.clipboard.writeText(details);
    } catch {
      const area = document.createElement("textarea");
      area.value = details;
      area.setAttribute("readonly", "");
      area.style.position = "fixed";
      area.style.left = "-9999px";
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      document.body.removeChild(area);
    }
    setCopied(true);
    window.setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="flex h-full min-h-0 items-center justify-center p-6">
      <div className="w-full max-w-lg rounded-lg border border-border bg-card/80 p-5 shadow-xl">
        <h1 className="font-display text-lg font-semibold tracking-tight text-foreground">
          Something went wrong
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Jade hit an unexpected error. You can try again, or copy the details below if you want to
          debug or report it.
        </p>

        <div className="mt-4 flex flex-wrap gap-2">
          <Button type="button" onClick={onReset}>
            Try again
          </Button>
          <Button type="button" variant="outline" onClick={() => void copyDetails()}>
            {copied ? <Check /> : <Copy />}
            {copied ? "Copied" : "Copy details"}
          </Button>
        </div>

        <details className="group mt-5 rounded-md border border-border/70 bg-background/40">
          <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-sm text-muted-foreground marker:content-none [&::-webkit-details-marker]:hidden">
            <ChevronDown className="size-4 shrink-0 transition-transform group-open:rotate-180" />
            Technical details
          </summary>
          <div className="border-t border-border/70 px-3 py-3">
            <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-muted-foreground">
              {details}
            </pre>
          </div>
        </details>
      </div>
    </div>
  );
}
