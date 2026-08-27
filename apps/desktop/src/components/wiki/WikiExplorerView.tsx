import * as React from "react";
import { BookOpen, Folder, Plus, RefreshCw, Search } from "lucide-react";

import { WikiArticleGrid } from "@/components/wiki/WikiArticleGrid";
import { WikiTopicSidebar } from "@/components/wiki/WikiTopicSidebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { WikiPage, WikiRoot } from "@/lib/types";
import {
  collectWikiTopics,
  filterPagesByTag,
} from "@/lib/wikiTopics";

type WikiExplorerViewProps = {
  roots: WikiRoot[];
  pages: WikiPage[];
  selectedRootId: string | null;
  selectedTag: string | null;
  activeRoot: WikiRoot | undefined;
  busy: boolean;
  newRelPath: string;
  onNewRelPathChange: (value: string) => void;
  onSelectTag: (tag: string | null) => void;
  onSelectPage: (pageId: string) => void;
  onOpenFolders: () => void;
  onReindex: () => void;
  onOpenSearch: () => void;
  onCreatePage: () => void;
};

export function WikiExplorerView({
  roots,
  pages,
  selectedRootId,
  selectedTag,
  activeRoot,
  busy,
  newRelPath,
  onNewRelPathChange,
  onSelectTag,
  onSelectPage,
  onOpenFolders,
  onReindex,
  onOpenSearch,
  onCreatePage,
}: WikiExplorerViewProps): React.JSX.Element {
  const topics = React.useMemo(() => collectWikiTopics(pages), [pages]);
  const visiblePages = React.useMemo(
    () => filterPagesByTag(pages, selectedTag),
    [pages, selectedTag],
  );

  const gridHeading =
    selectedTag == null ? "All articles" : selectedTag;
  const gridDescription =
    selectedTag == null
      ? "Recently added articles across your wiki."
      : `${visiblePages.length} article${visiblePages.length === 1 ? "" : "s"} tagged “${selectedTag}”.`;

  return (
    <div className="flex min-h-0 flex-1">
      <aside className="flex w-72 shrink-0 flex-col border-r border-border/60">
        <div className="flex items-center gap-1 border-b border-border/60 p-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" onClick={onOpenFolders} aria-label="Folders">
                <Folder />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              {roots.length === 0
                ? "Folders"
                : selectedRootId
                  ? `Folders · ${activeRoot?.label ?? "1 selected"}`
                  : `Folders · ${roots.length}`}
            </TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={onReindex}
                disabled={busy}
                aria-label="Reindex wiki"
              >
                <RefreshCw />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Reindex wiki</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={onOpenSearch}
                aria-label="Search wiki"
                className="ml-auto"
              >
                <Search />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Search · Ctrl+Shift+F</TooltipContent>
          </Tooltip>
        </div>

        <WikiTopicSidebar
          topics={topics}
          selectedTag={selectedTag}
          totalArticles={pages.length}
          onSelectTag={onSelectTag}
        />

        <div className="flex gap-1 border-t border-border/60 p-2">
          <Input
            value={newRelPath}
            onChange={(e) => onNewRelPathChange(e.target.value)}
            placeholder="notes/new.md"
            className="h-8"
          />
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="icon"
                variant="ghost"
                onClick={onCreatePage}
                disabled={busy || roots.length === 0}
                aria-label="Create page"
              >
                <Plus />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">Create page</TooltipContent>
          </Tooltip>
        </div>
      </aside>

      <section className="flex min-w-0 flex-1 flex-col">
        {roots.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 text-muted-foreground">
            <BookOpen className="size-8 opacity-50" />
            <p className="text-sm">Add a wiki folder to get started.</p>
            <Button size="sm" variant="secondary" onClick={onOpenFolders}>
              <Folder className="size-3.5" />
              Open folders
            </Button>
          </div>
        ) : pages.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 text-muted-foreground">
            <BookOpen className="size-8 opacity-50" />
            <p className="text-sm">No pages indexed yet.</p>
            {activeRoot && selectedRootId ? (
              <p className="max-w-md truncate text-xs opacity-70" title={activeRoot.path}>
                {activeRoot.path}
              </p>
            ) : null}
          </div>
        ) : (
          <WikiArticleGrid
            pages={visiblePages}
            heading={gridHeading}
            description={gridDescription}
            onSelectPage={onSelectPage}
          />
        )}
      </section>
    </div>
  );
}
