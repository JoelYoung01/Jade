import * as React from "react";
import { BookOpen, Folder, FolderPlus, Pencil, Plus, RefreshCw, Save, Search } from "lucide-react";

import { MarkdownView } from "@/components/MarkdownView";
import { WikiSearchDialog } from "@/components/WikiSearchDialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  apiAddWikiRoot,
  apiCreateWikiPage,
  apiListWikiBacklinks,
  apiListWikiPages,
  apiListWikiRoots,
  apiPickWikiFolder,
  apiReadWikiPage,
  apiReindexWiki,
  apiRemoveWikiRoot,
  apiSetSyncthingSettings,
  apiSubscribeDbChanged,
  apiWikiRootSyncthingStatus,
  apiWriteWikiPage,
  apiGetSettings,
} from "@/lib/api";
import type {
  SyncthingSettings,
  SyncthingStatus,
  WikiBacklink,
  WikiPage,
  WikiPageContent,
  WikiRoot,
  WikiSearchHit,
} from "@/lib/types";
import { cn } from "@/lib/utils";

export function WikiView(): React.JSX.Element {
  const [roots, setRoots] = React.useState<WikiRoot[]>([]);
  const [pages, setPages] = React.useState<WikiPage[]>([]);
  const [highlightQuery, setHighlightQuery] = React.useState("");
  const [searchOpen, setSearchOpen] = React.useState(false);
  const [selectedRootId, setSelectedRootId] = React.useState<string | null>(null);
  const [selectedPageId, setSelectedPageId] = React.useState<string | null>(null);
  const [content, setContent] = React.useState<WikiPageContent | null>(null);
  const [draft, setDraft] = React.useState("");
  const [editing, setEditing] = React.useState(false);
  const [backlinks, setBacklinks] = React.useState<WikiBacklink[]>([]);
  const [syncStatus, setSyncStatus] = React.useState<SyncthingStatus | null>(null);
  const [syncthing, setSyncthing] = React.useState<SyncthingSettings>({
    address: "http://127.0.0.1:8384",
    api_key: "",
  });
  const [error, setError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const [newRelPath, setNewRelPath] = React.useState("");
  const [foldersOpen, setFoldersOpen] = React.useState(false);
  const [showSyncSettings, setShowSyncSettings] = React.useState(false);

  const refreshRootsAndPages = React.useCallback(async () => {
    const [nextRoots, settings] = await Promise.all([
      apiListWikiRoots(),
      apiGetSettings(),
    ]);
    setRoots(nextRoots);
    setSyncthing(settings.syncthing);
    const nextPages = await apiListWikiPages(selectedRootId ?? undefined);
    setPages(nextPages);
  }, [selectedRootId]);

  React.useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await refreshRootsAndPages();
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refreshRootsAndPages]);

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    void apiSubscribeDbChanged(() => {
      void refreshRootsAndPages().catch(() => {});
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [refreshRootsAndPages]);

  React.useEffect(() => {
    if (!selectedPageId) {
      setContent(null);
      setDraft("");
      setEditing(false);
      setBacklinks([]);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const [page, links] = await Promise.all([
          apiReadWikiPage(selectedPageId),
          apiListWikiBacklinks(selectedPageId),
        ]);
        if (cancelled) return;
        setContent(page);
        setDraft(page.content);
        setEditing(false);
        setBacklinks(links);
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedPageId]);

  React.useEffect(() => {
    const rootId = selectedRootId ?? roots[0]?.id;
    if (!rootId) {
      setSyncStatus(null);
      return;
    }
    let cancelled = false;
    void apiWikiRootSyncthingStatus(rootId)
      .then((status) => {
        if (!cancelled) setSyncStatus(status);
      })
      .catch(() => {
        if (!cancelled) setSyncStatus(null);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedRootId, roots]);

  React.useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || !event.shiftKey) return;
      if (event.key.toLowerCase() !== "f") return;
      event.preventDefault();
      setSearchOpen(true);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  function selectPage(pageId: string, nextHighlight = ""): void {
    setSelectedPageId(pageId);
    setHighlightQuery(nextHighlight);
  }

  function handleSearchSelect(hit: WikiSearchHit, query: string): void {
    const highlight =
      hit.kind === "body_exact" && query
        ? query
        : hit.snippet?.matched ?? "";
    selectPage(hit.page.id, highlight);
  }

  async function handleAddRoot(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      const path = await apiPickWikiFolder();
      if (!path) return;
      const root = await apiAddWikiRoot(path);
      setSelectedRootId(root.id);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleRemoveRoot(id: string): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      await apiRemoveWikiRoot(id);
      if (selectedRootId === id) setSelectedRootId(null);
      setSelectedPageId(null);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleReindex(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      await apiReindexWiki(selectedRootId ?? undefined);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  function handleStartEdit(): void {
    if (!content) return;
    setDraft(content.content);
    setEditing(true);
  }

  function handleCancelEdit(): void {
    if (!content) return;
    setDraft(content.content);
    setEditing(false);
    setError(null);
  }

  async function handleSave(): Promise<void> {
    if (!selectedPageId || !editing) return;
    setBusy(true);
    setError(null);
    try {
      const next = await apiWriteWikiPage(selectedPageId, draft, true);
      setContent(next);
      setDraft(next.content);
      setEditing(false);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleCreatePage(): Promise<void> {
    const rootId = selectedRootId ?? roots[0]?.id;
    if (!rootId) {
      setError("Add a wiki folder first.");
      return;
    }
    const rel = newRelPath.trim();
    if (!rel) {
      setError("Enter a relative path for the new page.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const title = rel.replace(/\.md$/i, "").split("/").pop();
      const created = await apiCreateWikiPage({
        root_id: rootId,
        rel_path: rel,
        ...(title ? { title } : {}),
      });
      setNewRelPath("");
      setSelectedPageId(created.page.id);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleSaveSyncthing(): Promise<void> {
    setBusy(true);
    setError(null);
    try {
      const settings = await apiSetSyncthingSettings(syncthing);
      setSyncthing(settings.syncthing);
      setShowSyncSettings(false);
      const rootId = selectedRootId ?? roots[0]?.id;
      if (rootId) {
        setSyncStatus(await apiWikiRootSyncthingStatus(rootId));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const activeRoot = roots.find((r) => r.id === (selectedRootId ?? roots[0]?.id));

  return (
    <div className="flex min-h-0 flex-1">
      <aside className="flex w-72 shrink-0 flex-col border-r border-border/60">
        <div className="flex items-center gap-1 border-b border-border/60 p-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setFoldersOpen(true)}
                aria-label="Folders"
              >
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
                onClick={() => void handleReindex()}
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
                onClick={() => setSearchOpen(true)}
                aria-label="Search wiki"
                className="ml-auto"
              >
                <Search />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Search · Ctrl+Shift+F</TooltipContent>
          </Tooltip>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          <p className="mb-1 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
            Pages
          </p>
          {pages.length === 0 ? (
            <p className="px-1 text-xs text-muted-foreground">No pages indexed.</p>
          ) : (
            <ul className="space-y-0.5">
              {pages.map((page) => (
                <li key={page.id}>
                  <button
                    type="button"
                    className={cn(
                      "w-full rounded-md px-2 py-1.5 text-left",
                      selectedPageId === page.id
                        ? "bg-accent text-accent-foreground"
                        : "hover:bg-accent/50",
                    )}
                    onClick={() => selectPage(page.id)}
                  >
                    <div className="truncate text-sm font-medium">
                      {page.title_cache || page.rel_path}
                    </div>
                    <div className="truncate text-[11px] text-muted-foreground">
                      {page.rel_path}
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex gap-1 border-t border-border/60 p-2">
          <Input
            value={newRelPath}
            onChange={(e) => setNewRelPath(e.target.value)}
            placeholder="notes/new.md"
            className="h-8"
          />
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="icon"
                variant="ghost"
                onClick={() => void handleCreatePage()}
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

      <WikiSearchDialog
        open={searchOpen}
        onOpenChange={setSearchOpen}
        onSelect={handleSearchSelect}
      />

      <Dialog open={foldersOpen} onOpenChange={setFoldersOpen}>
        <DialogContent className="w-[min(92vw,32rem)] gap-3">
          <DialogHeader>
            <DialogTitle>Wiki folders</DialogTitle>
            <DialogDescription>
              Choose which folders to show in the page list, or add another local
              directory.
            </DialogDescription>
          </DialogHeader>

          <Button
            className="w-full justify-start gap-2"
            variant="secondary"
            onClick={() => void handleAddRoot()}
            disabled={busy}
          >
            <FolderPlus className="size-4" />
            Add folder…
          </Button>

          <div className="max-h-64 space-y-0.5 overflow-y-auto rounded-md border border-border/60 p-1">
            <button
              type="button"
              className={cn(
                "w-full rounded-md px-2.5 py-2 text-left text-sm",
                selectedRootId == null
                  ? "bg-accent text-accent-foreground"
                  : "hover:bg-accent/50",
              )}
              onClick={() => setSelectedRootId(null)}
            >
              All folders
            </button>
            {roots.length === 0 ? (
              <p className="px-2.5 py-3 text-xs text-muted-foreground">
                No folders yet. Add a local directory to start indexing markdown.
              </p>
            ) : (
              roots.map((root) => (
                <div
                  key={root.id}
                  className={cn(
                    "group flex items-start gap-1 rounded-md",
                    selectedRootId === root.id && "bg-accent text-accent-foreground",
                  )}
                >
                  <button
                    type="button"
                    className={cn(
                      "min-w-0 flex-1 px-2.5 py-2 text-left",
                      selectedRootId !== root.id && "hover:bg-accent/50 rounded-md",
                    )}
                    onClick={() => setSelectedRootId(root.id)}
                    title={root.path}
                  >
                    <div className="truncate text-sm font-medium">{root.label}</div>
                    <div className="truncate text-[11px] text-muted-foreground">
                      {root.path}
                    </div>
                  </button>
                  <button
                    type="button"
                    className="mr-1 mt-2 shrink-0 rounded px-1.5 py-0.5 text-xs text-muted-foreground opacity-0 hover:text-destructive group-hover:opacity-100"
                    onClick={() => void handleRemoveRoot(root.id)}
                  >
                    Remove
                  </button>
                </div>
              ))
            )}
          </div>

          {syncStatus?.underSyncthing ? (
            <p className="text-[11px] text-primary">
              Syncthing
              {syncStatus.folder
                ? `: ${syncStatus.folder.label || syncStatus.folder.id}`
                : syncStatus.markerDetected
                  ? " (marker detected)"
                  : ""}
            </p>
          ) : null}

          <button
            type="button"
            className="text-left text-[11px] text-muted-foreground underline-offset-2 hover:underline"
            onClick={() => setShowSyncSettings((v) => !v)}
          >
            Syncthing settings
          </button>
          {showSyncSettings ? (
            <div className="space-y-1.5">
              <Input
                value={syncthing.address}
                onChange={(e) =>
                  setSyncthing((s) => ({ ...s, address: e.target.value }))
                }
                placeholder="http://127.0.0.1:8384"
                className="h-8 text-xs"
              />
              <Input
                value={syncthing.api_key}
                onChange={(e) =>
                  setSyncthing((s) => ({ ...s, api_key: e.target.value }))
                }
                placeholder="API key (optional)"
                className="h-8 text-xs"
              />
              <Button
                size="sm"
                variant="secondary"
                className="h-7"
                onClick={() => void handleSaveSyncthing()}
                disabled={busy}
              >
                Save Syncthing settings
              </Button>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>

      <section className="flex min-w-0 flex-1 flex-col">
        {error ? (
          <div className="border-b border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </div>
        ) : null}

        {!content ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 text-muted-foreground">
            <BookOpen className="size-8 opacity-50" />
            <p className="text-sm">
              {roots.length === 0
                ? "Add a wiki folder to get started."
                : "Select a page to read and edit."}
            </p>
            {roots.length === 0 ? (
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setFoldersOpen(true)}
              >
                <Folder className="size-3.5" />
                Open folders
              </Button>
            ) : activeRoot && selectedRootId ? (
              <p className="max-w-md truncate text-xs opacity-70" title={activeRoot.path}>
                {activeRoot.path}
              </p>
            ) : null}
          </div>
        ) : (
          <>
            <div className="flex items-center justify-between gap-2 border-b border-border/60 px-3 py-2">
              <div className="min-w-0">
                <h2 className="truncate font-display text-sm font-semibold tracking-wide">
                  {content.page.title_cache || content.page.rel_path}
                </h2>
                <p className="truncate text-[11px] text-muted-foreground">
                  {content.absolute_path}
                </p>
              </div>
              {editing ? (
                <div className="flex shrink-0 items-center gap-1.5">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={handleCancelEdit}
                    disabled={busy}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    onClick={() => void handleSave()}
                    disabled={busy}
                  >
                    <Save className="size-3.5" />
                    Save
                  </Button>
                </div>
              ) : (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={handleStartEdit}
                      aria-label="Edit"
                    >
                      <Pencil className="size-3.5" />
                      Edit
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="bottom">Edit markdown</TooltipContent>
                </Tooltip>
              )}
            </div>
            {editing ? (
              <Textarea
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                className="min-h-0 flex-1 resize-none rounded-none border-0 bg-transparent px-3 py-3 font-mono text-sm leading-relaxed focus-visible:ring-0"
                spellCheck={false}
              />
            ) : (
              <MarkdownView
                markdown={content.body}
                pages={pages}
                highlightQuery={highlightQuery}
                onWikiLink={(pageId) => selectPage(pageId)}
              />
            )}
            {!editing && backlinks.length > 0 ? (
              <div className="border-t border-border/60 px-3 py-2">
                <p className="mb-1 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  Backlinks
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {backlinks.map((link) => (
                    <button
                      key={`${link.page.id}-${link.target_raw}`}
                      type="button"
                      className="rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground hover:bg-accent"
                      onClick={() => selectPage(link.page.id)}
                    >
                      {link.page.title_cache || link.page.rel_path}
                    </button>
                  ))}
                </div>
              </div>
            ) : null}
          </>
        )}
      </section>
    </div>
  );
}
