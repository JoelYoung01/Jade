import * as React from "react";

import type { WikiPage } from "@/lib/types";
import { formatWikiDate, pageDisplayTitle, pageSortDate } from "@/lib/wikiTopics";
import { cn } from "@/lib/utils";

type WikiArticleCardProps = {
  page: WikiPage;
  onClick: () => void;
  className?: string | undefined;
};

export function WikiArticleCard({
  page,
  onClick,
  className,
}: WikiArticleCardProps): React.JSX.Element {
  const title = pageDisplayTitle(page);
  const summary = page.summary_cache?.trim();

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex flex-col rounded-lg border border-border/60 bg-card/50 p-4 text-left",
        "transition-colors hover:border-primary/40 hover:bg-card",
        className,
      )}
    >
      <h3 className="line-clamp-2 font-display text-sm font-semibold leading-snug tracking-wide">
        {title}
      </h3>
      {summary ? (
        <p className="mt-2 line-clamp-3 text-xs leading-relaxed text-muted-foreground">
          {summary}
        </p>
      ) : null}
      <div className="mt-auto flex flex-wrap items-center gap-1.5 pt-3">
        {page.tags_cache.slice(0, 4).map((tag) => (
          <span
            key={tag}
            className="rounded-md bg-secondary px-1.5 py-0.5 text-[10px] text-secondary-foreground"
          >
            {tag}
          </span>
        ))}
        {page.tags_cache.length > 4 ? (
          <span className="text-[10px] text-muted-foreground">
            +{page.tags_cache.length - 4}
          </span>
        ) : null}
        <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
          {formatWikiDate(pageSortDate(page)) ?? pageSortDate(page)}
        </span>
      </div>
    </button>
  );
}
