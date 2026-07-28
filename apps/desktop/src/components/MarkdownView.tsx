import * as React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { rehypeHighlightQuery } from "@/lib/rehypeHighlightQuery";
import { cn } from "@/lib/utils";

type MarkdownViewProps = {
  markdown: string;
  /** When set, highlight matches and scroll to the first hit. */
  highlightQuery?: string | undefined;
  className?: string | undefined;
};

/** Renders wiki markdown body for read/view mode. */
export function MarkdownView({
  markdown,
  highlightQuery,
  className,
}: MarkdownViewProps): React.JSX.Element {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const query = highlightQuery?.trim() ?? "";

  const rehypePlugins = React.useMemo(
    () => (query ? [rehypeHighlightQuery(query)] : []),
    [query],
  );

  React.useEffect(() => {
    if (!query) return;
    const container = containerRef.current;
    if (!container) return;

    // Wait for paint so marks exist in layout.
    const frame = requestAnimationFrame(() => {
      const first = container.querySelector<HTMLElement>(
        "mark.wiki-search-hit-active",
      );
      if (!first) return;

      const cRect = container.getBoundingClientRect();
      const fRect = first.getBoundingClientRect();
      const delta =
        fRect.top - cRect.top - cRect.height / 2 + fRect.height / 2;
      container.scrollTo({
        top: container.scrollTop + delta,
        behavior: "smooth",
      });
    });
    return () => cancelAnimationFrame(frame);
  }, [markdown, query]);

  return (
    <div
      ref={containerRef}
      className={cn(
        "wiki-prose min-h-0 flex-1 overflow-y-auto px-5 py-4",
        className,
      )}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={rehypePlugins}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
}
