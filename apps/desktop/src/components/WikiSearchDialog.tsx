import * as React from "react";
import { Search } from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { apiSearchWikiPages } from "@/lib/api";
import type { WikiSearchHit } from "@/lib/types";
import { cn } from "@/lib/utils";

type WikiSearchDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (hit: WikiSearchHit, query: string) => void;
};

export function WikiSearchDialog({
  open,
  onOpenChange,
  onSelect,
}: WikiSearchDialogProps): React.JSX.Element {
  const [query, setQuery] = React.useState("");
  const [hits, setHits] = React.useState<WikiSearchHit[]>([]);
  const [activeIndex, setActiveIndex] = React.useState(0);
  const [error, setError] = React.useState<string | null>(null);
  const inputRef = React.useRef<HTMLInputElement>(null);
  const listRef = React.useRef<HTMLUListElement>(null);

  React.useEffect(() => {
    if (!open) return;
    setQuery("");
    setActiveIndex(0);
    setError(null);
    const id = window.setTimeout(() => inputRef.current?.focus(), 0);
    return () => window.clearTimeout(id);
  }, [open]);

  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void (async () => {
      try {
        const next = await apiSearchWikiPages(query);
        if (cancelled) return;
        setHits(next);
        setActiveIndex(0);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        setHits([]);
        setError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, query]);

  React.useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const item = list.querySelector<HTMLElement>(`[data-search-index="${activeIndex}"]`);
    item?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const selectHit = (hit: WikiSearchHit) => {
    onSelect(hit, query.trim());
    onOpenChange(false);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, Math.max(hits.length - 1, 0)));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      const hit = hits[activeIndex];
      if (hit) {
        event.preventDefault();
        selectHit(hit);
      }
    }
  };

  const emptyLabel = query.trim() ? "No matches" : "No recent pages";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="flex w-[min(92vw,36rem)] flex-col gap-0 overflow-hidden p-0"
        onKeyDown={onKeyDown}
      >
        <DialogHeader className="sr-only">
          <DialogTitle>Search wiki</DialogTitle>
          <DialogDescription>Search pages by title, path, tags, or body text.</DialogDescription>
        </DialogHeader>

        <div className="flex items-center gap-2 border-b border-border/60 py-2.5 pr-10 pl-3">
          <Search className="size-4 shrink-0 text-muted-foreground" aria-hidden />
          <Input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search wiki…"
            className="h-9 border-0 bg-transparent px-0 shadow-none focus-visible:ring-0"
            aria-label="Search wiki"
            autoComplete="off"
          />
        </div>

        <div className="max-h-[min(60vh,28rem)] min-h-[12rem] overflow-y-auto">
          {error ? (
            <p className="px-3 py-4 text-sm text-destructive">{error}</p>
          ) : hits.length === 0 ? (
            <p className="px-3 py-4 text-sm text-muted-foreground">{emptyLabel}</p>
          ) : (
            <ul ref={listRef} className="py-1" role="listbox" aria-label="Search results">
              {hits.map((hit, index) => (
                <li key={hit.page.id}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={index === activeIndex}
                    data-search-index={index}
                    className={cn(
                      "flex w-full flex-col gap-0.5 px-3 py-2 text-left",
                      index === activeIndex
                        ? "bg-accent text-accent-foreground"
                        : "hover:bg-accent/50",
                    )}
                    onMouseEnter={() => setActiveIndex(index)}
                    onClick={() => selectHit(hit)}
                  >
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="truncate text-sm font-medium">
                        {hit.page.title_cache || hit.page.rel_path}
                      </span>
                      <span className="shrink-0 text-[11px] text-muted-foreground">
                        {hit.reason}
                      </span>
                    </div>
                    {hit.snippet ? (
                      <p className="line-clamp-2 text-xs leading-relaxed text-muted-foreground">
                        <span>{hit.snippet.before}</span>
                        <mark className="wiki-search-snippet-hit rounded-[0.15em] px-0.5 text-foreground">
                          {hit.snippet.matched}
                        </mark>
                        <span>{hit.snippet.after}</span>
                      </p>
                    ) : (
                      <p className="truncate text-[11px] text-muted-foreground">
                        {hit.kind === "recent"
                          ? hit.page.rel_path
                          : matchDetail(hit)}
                      </p>
                    )}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function matchDetail(hit: WikiSearchHit): string {
  switch (hit.kind) {
    case "title_exact":
    case "title_related":
      return hit.page.title_cache || hit.page.rel_path;
    case "tags_exact":
    case "tags_related":
      return hit.page.tags_cache.length > 0
        ? hit.page.tags_cache.join(", ")
        : hit.page.rel_path;
    case "path_exact":
    case "path_related":
    case "body_related":
    case "body_exact":
    case "recent":
    default:
      return hit.page.rel_path;
  }
}
