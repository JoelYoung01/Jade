import * as React from "react";

import { WikiExplorerView } from "@/components/wiki/WikiExplorerView";
import { WikiFoldersDialog } from "@/components/wiki/WikiFoldersDialog";
import { WikiFrontMatterIssuesDialog } from "@/components/wiki/WikiFrontMatterIssuesDialog";
import {
  groupWikiIndexIssues,
  type WikiFrontMatterFileGroup,
} from "@/lib/wikiFrontMatterIssues";
import { WikiReaderView } from "@/components/wiki/WikiReaderView";
import { WikiSearchDialog } from "@/components/WikiSearchDialog";
import {
  apiAddWikiRoot,
  apiCreateWikiPage,
  apiGetSettings,
  apiListWikiBacklinks,
  apiListWikiPages,
  apiListWikiRoots,
  apiPickWikiFolder,
  apiReadWikiPage,
  apiReindexWiki,
  apiRemoveWikiRoot,
  apiRepairWikiFrontMatter,
  apiSetSyncthingSettings,
  apiSubscribeDbChanged,
  apiSubscribeWikiIndexIssues,
  apiWikiRootSyncthingStatus,
  apiWriteWikiPage,
} from "@/lib/api";
import type {
  SyncthingSettings,
  SyncthingStatus,
  WikiBacklink,
  WikiIndexIssue,
  WikiPage,
  WikiPageContent,
  WikiRoot,
  WikiSearchHit,
} from "@/lib/types";

type WikiViewMode = "explorer" | "reader";

export function WikiView(): React.JSX.Element {
  const [roots, setRoots] = React.useState<WikiRoot[]>([]);
  const [pages, setPages] = React.useState<WikiPage[]>([]);
  const [viewMode, setViewMode] = React.useState<WikiViewMode>("explorer");
  const [selectedTag, setSelectedTag] = React.useState<string | null>(null);
  const [highlightQuery, setHighlightQuery] = React.useState("");
  const [searchOpen, setSearchOpen] = React.useState(false);
  const [selectedRootId, setSelectedRootId] = React.useState<string | null>(null);
  const [selectedPageId, setSelectedPageId] = React.useState<string | null>(null);
  const [content, setContent] = React.useState<WikiPageContent | null>(null);
  const [draft, setDraft] = React.useState("");
  const [editing, setEditing] = React.useState(false);
  const [metadataOpen, setMetadataOpen] = React.useState(false);
  const [backlinks, setBacklinks] = React.useState<WikiBacklink[]>([]);
  const [syncStatus, setSyncStatus] = React.useState<SyncthingStatus | null>(null);
  const [syncthing, setSyncthing] = React.useState<SyncthingSettings>({
    address: "http://127.0.0.1:8384",
    api_key: "",
  });
  const [error, setError] = React.useState<string | null>(null);
  const [indexIssues, setIndexIssues] = React.useState<WikiIndexIssue[]>([]);
  const [issuesOpen, setIssuesOpen] = React.useState(false);
  const [repairingKey, setRepairingKey] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const [newRelPath, setNewRelPath] = React.useState("");
  const [foldersOpen, setFoldersOpen] = React.useState(false);
  const [showSyncSettings, setShowSyncSettings] = React.useState(false);
  const [copied, setCopied] = React.useState(false);

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
    let unlisten: (() => void) | undefined;
    void apiSubscribeWikiIndexIssues((rootIds, issues) => {
      setIndexIssues((prev) => {
        const drop = new Set(rootIds);
        return [...prev.filter((issue) => !drop.has(issue.root_id)), ...issues];
      });
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  React.useEffect(() => {
    if (!selectedPageId) {
      setContent(null);
      setDraft("");
      setEditing(false);
      setBacklinks([]);
      setMetadataOpen(false);
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

  React.useEffect(() => {
    setCopied(false);
  }, [selectedPageId]);

  const openReader = React.useCallback((pageId: string, nextHighlight = ""): void => {
    setSelectedPageId(pageId);
    setHighlightQuery(nextHighlight);
    setViewMode("reader");
    setMetadataOpen(false);
  }, []);

  const backToExplorer = React.useCallback((): void => {
    setViewMode("explorer");
    setSelectedPageId(null);
    setHighlightQuery("");
    setEditing(false);
    setMetadataOpen(false);
  }, []);

  function handleSearchSelect(hit: WikiSearchHit, query: string): void {
    const highlight =
      hit.kind === "body_exact" && query ? query : hit.snippet?.matched ?? "";
    openReader(hit.page.id, highlight);
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
      if (viewMode === "reader") backToExplorer();
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
      const stats = await apiReindexWiki(selectedRootId ?? undefined);
      setIndexIssues((prev) => {
        if (!selectedRootId) return stats.issues;
        return [
          ...prev.filter((issue) => issue.root_id !== selectedRootId),
          ...stats.issues,
        ];
      });
      if (stats.issues.length > 0) {
        setIssuesOpen(true);
      }
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  function dropIssuesForFile(rootId: string, relPath: string): void {
    setIndexIssues((prev) =>
      prev.filter((issue) => !(issue.root_id === rootId && issue.rel_path === relPath)),
    );
  }

  async function applyFrontMatterRepair(
    rootId: string,
    relPath: string,
  ): Promise<void> {
    const next = await apiRepairWikiFrontMatter(rootId, relPath);
    dropIssuesForFile(rootId, relPath);
    if (selectedPageId === next.page.id) {
      setContent(next);
      if (!editing) {
        setDraft(next.content);
      }
    }
  }

  async function handleRepairFile(group: WikiFrontMatterFileGroup): Promise<void> {
    setBusy(true);
    setRepairingKey(group.key);
    setError(null);
    try {
      await applyFrontMatterRepair(group.root_id, group.rel_path);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
      setRepairingKey(null);
    }
  }

  async function handleRepairAll(): Promise<void> {
    const groups = groupWikiIndexIssues(indexIssues).filter((group) => group.repairable);
    setBusy(true);
    setRepairingKey("*");
    setError(null);
    try {
      for (const group of groups) {
        await applyFrontMatterRepair(group.root_id, group.rel_path);
      }
      await refreshRootsAndPages();
      const hadUnrepairable = groupWikiIndexIssues(indexIssues).some(
        (group) => !group.repairable,
      );
      if (!hadUnrepairable) {
        setIssuesOpen(false);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
      setRepairingKey(null);
    }
  }

  async function handleRepairCurrentPage(): Promise<void> {
    if (!content) return;
    setBusy(true);
    setRepairingKey(`${content.page.root_id}:${content.page.rel_path}`);
    setError(null);
    try {
      await applyFrontMatterRepair(content.page.root_id, content.page.rel_path);
      await refreshRootsAndPages();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
      setRepairingKey(null);
    }
  }

  async function handleCopyBody(): Promise<void> {
    if (!content) return;
    const text = content.body;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
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

  function handleStartEdit(): void {
    if (!content) return;
    setDraft(content.content);
    setEditing(true);
    setMetadataOpen(false);
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
        tags: [],
      });
      setNewRelPath("");
      openReader(created.page.id);
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
    <div className="flex min-h-0 flex-1 flex-col">
      {error ? (
        <div className="border-b border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {indexIssues.length > 0 ? (
        <div className="flex items-center gap-3 border-b border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-950 dark:text-amber-100">
          <p className="min-w-0 flex-1">
            {indexIssues.length === 1
              ? "1 wiki file has a front matter problem."
              : `${indexIssues.length} wiki files have front matter problems.`}
          </p>
          <button
            type="button"
            className="shrink-0 underline underline-offset-2"
            onClick={() => setIssuesOpen(true)}
          >
            Review and fix
          </button>
        </div>
      ) : null}

      {viewMode === "explorer" ? (
        <WikiExplorerView
          roots={roots}
          pages={pages}
          selectedRootId={selectedRootId}
          selectedTag={selectedTag}
          activeRoot={activeRoot}
          busy={busy}
          newRelPath={newRelPath}
          onNewRelPathChange={setNewRelPath}
          onSelectTag={setSelectedTag}
          onSelectPage={openReader}
          onOpenFolders={() => setFoldersOpen(true)}
          onReindex={() => void handleReindex()}
          onOpenSearch={() => setSearchOpen(true)}
          onCreatePage={() => void handleCreatePage()}
        />
      ) : content ? (
        <WikiReaderView
          content={content}
          pages={pages}
          backlinks={backlinks}
          highlightQuery={highlightQuery}
          editing={editing}
          draft={draft}
          busy={busy}
          copied={copied}
          metadataOpen={metadataOpen}
          onBack={backToExplorer}
          onToggleMetadata={() => setMetadataOpen((open) => !open)}
          onWikiLink={openReader}
          onStartEdit={handleStartEdit}
          onCancelEdit={handleCancelEdit}
          onSave={() => void handleSave()}
          onCopyBody={() => void handleCopyBody()}
          onDraftChange={setDraft}
          onCloseMetadata={() => setMetadataOpen(false)}
          onRepairFrontMatter={() => void handleRepairCurrentPage()}
        />
      ) : (
        <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
          Loading article…
        </div>
      )}

      <WikiSearchDialog
        open={searchOpen}
        onOpenChange={setSearchOpen}
        onSelect={handleSearchSelect}
      />

      <WikiFoldersDialog
        open={foldersOpen}
        onOpenChange={setFoldersOpen}
        roots={roots}
        selectedRootId={selectedRootId}
        busy={busy}
        syncStatus={syncStatus}
        syncthing={syncthing}
        showSyncSettings={showSyncSettings}
        onSelectRoot={setSelectedRootId}
        onAddRoot={() => void handleAddRoot()}
        onRemoveRoot={(id) => void handleRemoveRoot(id)}
        onToggleSyncSettings={() => setShowSyncSettings((v) => !v)}
        onSyncthingChange={setSyncthing}
        onSaveSyncthing={() => void handleSaveSyncthing()}
      />

      <WikiFrontMatterIssuesDialog
        open={issuesOpen}
        onOpenChange={setIssuesOpen}
        issues={indexIssues}
        busy={busy}
        repairingKey={repairingKey}
        onRepairFile={(group) => void handleRepairFile(group)}
        onRepairAll={() => void handleRepairAll()}
      />
    </div>
  );
}
