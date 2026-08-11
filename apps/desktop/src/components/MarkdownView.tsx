import * as React from "react";
import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import remarkGfm from "remark-gfm";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { rehypeHighlightQuery } from "@/lib/rehypeHighlightQuery";
import { rehypeWikiCitations } from "@/lib/rehypeWikiCitations";
import type { WikiPage } from "@/lib/types";
import { cn } from "@/lib/utils";
import {
  parseWikiLinkHref,
  resolveWikiPage,
  wikiLinkDisplayTitle,
  wikiSafeUrlTransform,
} from "@/lib/wikiLinks";

type MarkdownViewProps = {
  markdown: string;
  /** Indexed wiki pages used to resolve `[[wiki links]]`. */
  pages?: readonly WikiPage[] | undefined;
  /** Navigate to a resolved wiki page when a citation is clicked. */
  onWikiLink?: ((pageId: string) => void) | undefined;
  /** When set, highlight matches and scroll to the first hit. */
  highlightQuery?: string | undefined;
  className?: string | undefined;
};

function wikiTargetFromAnchorProps(
  href: string | undefined,
  props: Record<string, unknown>,
): string | null {
  const fromData = props["data-wiki-target"];
  if (typeof fromData === "string" && fromData.trim()) return fromData;
  if (typeof href === "string") return parseWikiLinkHref(href);
  return null;
}

/** Renders wiki markdown body for read/view mode. */
export function MarkdownView({
  markdown,
  pages = [],
  onWikiLink,
  highlightQuery,
  className,
}: MarkdownViewProps): React.JSX.Element {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const query = highlightQuery?.trim() ?? "";

  const rehypePlugins = React.useMemo(() => {
    const plugins: Array<ReturnType<typeof rehypeWikiCitations>> = [
      rehypeWikiCitations(),
    ];
    if (query) plugins.push(rehypeHighlightQuery(query));
    return plugins;
  }, [query]);

  const components = React.useMemo(
    () => ({
      a: ({
        href,
        title,
        children,
        className: anchorClassName,
        ...props
      }: React.ComponentPropsWithoutRef<"a">) => {
        const target = wikiTargetFromAnchorProps(
          typeof href === "string" ? href : undefined,
          props as Record<string, unknown>,
        );
        if (target == null) {
          return (
            <a href={href} title={title} className={anchorClassName} {...props}>
              {children}
            </a>
          );
        }

        const page = resolveWikiPage(pages, target);
        const displayTitle = wikiLinkDisplayTitle(target, page);
        const missing = page == null;

        return (
          <Tooltip delayDuration={300}>
            <TooltipTrigger asChild>
              <span className="inline">
                <button
                  type="button"
                  className={cn(
                    "wiki-citation",
                    missing && "wiki-citation-missing",
                    anchorClassName,
                  )}
                  aria-label={`Source: ${displayTitle}`}
                  disabled={missing}
                  onClick={() => {
                    if (page) onWikiLink?.(page.id);
                  }}
                >
                  {children}
                </button>
              </span>
            </TooltipTrigger>
            <TooltipContent side="top">{displayTitle}</TooltipContent>
          </Tooltip>
        );
      },
    }),
    [onWikiLink, pages],
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
        urlTransform={(url) => wikiSafeUrlTransform(url, defaultUrlTransform)}
        components={components}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
}
