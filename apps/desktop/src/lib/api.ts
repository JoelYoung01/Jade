import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { nextOccurrences, validateCron } from "@/lib/repeat";
import { applyDuePreset, type DueQuickPreset } from "@/lib/time";
import type {
  CreateTaskInput,
  LaneVisibility,
  RescheduleMode,
  Settings,
  StatusUpdateResult,
  SyncthingSettings,
  SyncthingStatus,
  Tag,
  Task,
  TaskEvent,
  TaskStatus,
  UpdateTaskInput,
  WikiBacklink,
  WikiMatchKind,
  WikiPage,
  WikiPageContent,
  WikiRoot,
  WikiSearchHit,
  WikiSearchSnippet,
} from "@/lib/types";

export const DB_CHANGED_EVENT = "db-changed";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

type MockStore = {
  tasks: Task[];
  tags: Tag[];
  settings: Settings;
  wikiRoots: WikiRoot[];
  wikiPages: WikiPage[];
  wikiContent: Record<string, string>;
};

const mockStore: MockStore = {
  tasks: [],
  tags: [],
  settings: {
    lane_visibility: { inactive: true, active: true, complete: false },
    syncthing: { address: "http://127.0.0.1:8384", api_key: "" },
  },
  wikiRoots: [],
  wikiPages: [],
  wikiContent: {},
};

function nowIso(): string {
  return new Date().toISOString();
}

function sortTasks(tasks: Task[]): Task[] {
  return [...tasks].sort(
    (a, b) => new Date(a.due_at).getTime() - new Date(b.due_at).getTime(),
  );
}

function ensureMockTag(name: string): Tag {
  const existing = mockStore.tags.find(
    (t) => t.name.toLowerCase() === name.trim().toLowerCase(),
  );
  if (existing) return existing;
  const tag: Tag = {
    id: crypto.randomUUID(),
    name: name.trim(),
    created_at: nowIso(),
    updated_at: nowIso(),
  };
  mockStore.tags.push(tag);
  return tag;
}

function normalizeRepeatCron(value: string | null | undefined): string | null {
  if (value == null) return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  const result = validateCron(trimmed);
  if (!result.ok) throw new Error(result.error);
  return trimmed;
}

function spawnNextMock(task: Task): Task | null {
  if (!task.repeat_cron) return null;
  const now = new Date();
  const due = new Date(task.due_at);
  const after = due > now ? due : now;
  const next = nextOccurrences(task.repeat_cron, after, 1)[0];
  if (!next) throw new Error("Could not compute next occurrence");
  const spawned: Task = {
    id: crypto.randomUUID(),
    title: task.title,
    description: task.description,
    status: "inactive",
    due_at: next.toISOString(),
    repeat_cron: task.repeat_cron,
    created_at: nowIso(),
    updated_at: nowIso(),
    deleted_at: null,
    tags: [...task.tags],
  };
  mockStore.tasks.push(spawned);
  return spawned;
}

export async function apiListTasks(): Promise<Task[]> {
  if (!isTauri()) return sortTasks(mockStore.tasks);
  return invoke<Task[]>("list_tasks_cmd");
}

export async function apiCreateTask(input: CreateTaskInput): Promise<Task> {
  const repeat_cron = normalizeRepeatCron(input.repeat_cron);
  if (!isTauri()) {
    const description = input.description?.trim();
    const tags = input.tag_names.map(ensureMockTag);
    const task: Task = {
      id: crypto.randomUUID(),
      title: input.title.trim(),
      description: description && description.length > 0 ? description : null,
      status: "inactive",
      due_at: input.due_at,
      repeat_cron,
      created_at: nowIso(),
      updated_at: nowIso(),
      deleted_at: null,
      tags,
    };
    mockStore.tasks.push(task);
    return task;
  }
  return invoke<Task>("create_task_cmd", {
    args: {
      title: input.title,
      description: input.description ?? null,
      due_at: input.due_at,
      tag_names: input.tag_names,
      repeat_cron,
    },
  });
}

export async function apiUpdateTaskStatus(
  id: string,
  status: TaskStatus,
): Promise<StatusUpdateResult> {
  if (!isTauri()) {
    const task = mockStore.tasks.find((t) => t.id === id);
    if (!task) throw new Error("Task not found");
    let spawned: Task | null = null;
    if (status === "complete" && task.repeat_cron) {
      spawned = spawnNextMock(task);
      task.repeat_cron = null;
    }
    task.status = status;
    task.updated_at = nowIso();
    return { task: { ...task }, spawned };
  }
  return invoke<StatusUpdateResult>("update_task_status_cmd", {
    args: { id, status },
  });
}

export async function apiUpdateTask(input: UpdateTaskInput): Promise<StatusUpdateResult> {
  const repeat_cron = normalizeRepeatCron(input.repeat_cron);
  if (!isTauri()) {
    const task = mockStore.tasks.find((t) => t.id === input.id);
    if (!task) throw new Error("Task not found");
    const title = input.title.trim();
    if (!title) throw new Error("Title is required");
    task.title = title;
    const description = input.description.trim();
    task.description = description.length > 0 ? description : null;
    task.due_at = input.due_at;
    task.tags = input.tag_names.map(ensureMockTag);
    task.repeat_cron = repeat_cron;
    task.updated_at = nowIso();
    return { task: { ...task }, spawned: null };
  }
  return invoke<StatusUpdateResult>("update_task_cmd", {
    args: {
      id: input.id,
      title: input.title,
      description: input.description,
      due_at: input.due_at,
      tag_names: input.tag_names,
      repeat_cron,
    },
  });
}

export async function apiRescheduleTask(
  id: string,
  mode: RescheduleMode,
  dueAt?: string,
): Promise<Task> {
  if (!isTauri()) {
    const task = mockStore.tasks.find((t) => t.id === id);
    if (!task) throw new Error("Task not found");
    let next: Date;
    if (mode === "custom") {
      if (!dueAt) throw new Error("Custom reschedule requires due date");
      next = new Date(dueAt);
    } else {
      next = applyDuePreset(new Date(task.due_at), mode as DueQuickPreset);
    }
    task.due_at = next.toISOString();
    task.updated_at = nowIso();
    return task;
  }
  return invoke<Task>("reschedule_task_cmd", {
    args: {
      id,
      mode,
      due_at: dueAt ?? null,
    },
  });
}

export async function apiDeleteTask(id: string): Promise<void> {
  if (!isTauri()) {
    const index = mockStore.tasks.findIndex((t) => t.id === id);
    if (index < 0) throw new Error("Task not found");
    mockStore.tasks.splice(index, 1);
    return;
  }
  await invoke<null>("delete_task_cmd", { id });
}

export async function apiListTags(): Promise<Tag[]> {
  if (!isTauri()) return [...mockStore.tags].sort((a, b) => a.name.localeCompare(b.name));
  return invoke<Tag[]>("list_tags_cmd");
}

export async function apiEnsureTag(name: string): Promise<Tag> {
  if (!isTauri()) return ensureMockTag(name);
  return invoke<Tag>("ensure_tag_cmd", { name });
}

export async function apiCountTasksWithTag(id: string): Promise<number> {
  if (!isTauri()) {
    return mockStore.tasks.filter((task) => task.tags.some((tag) => tag.id === id)).length;
  }
  return invoke<number>("count_tasks_with_tag_cmd", { id });
}

export async function apiDeleteTag(id: string): Promise<void> {
  if (!isTauri()) {
    const index = mockStore.tags.findIndex((tag) => tag.id === id);
    if (index < 0) throw new Error("Tag not found");
    mockStore.tags.splice(index, 1);
    for (const task of mockStore.tasks) {
      task.tags = task.tags.filter((tag) => tag.id !== id);
    }
    return;
  }
  await invoke<null>("delete_tag_cmd", { id });
}

export async function apiGetSettings(): Promise<Settings> {
  if (!isTauri()) return structuredClone(mockStore.settings);
  return invoke<Settings>("get_settings_cmd");
}

export async function apiSetLaneVisibility(visibility: LaneVisibility): Promise<Settings> {
  if (!isTauri()) {
    mockStore.settings.lane_visibility = visibility;
    return structuredClone(mockStore.settings);
  }
  return invoke<Settings>("set_lane_visibility_cmd", { visibility });
}

export async function apiSetSyncthingSettings(
  settings: SyncthingSettings,
): Promise<Settings> {
  if (!isTauri()) {
    mockStore.settings.syncthing = settings;
    return structuredClone(mockStore.settings);
  }
  return invoke<Settings>("set_syncthing_settings_cmd", { settings });
}

export async function apiListWikiRoots(): Promise<WikiRoot[]> {
  if (!isTauri()) return [...mockStore.wikiRoots];
  return invoke<WikiRoot[]>("list_wiki_roots_cmd");
}

export async function apiAddWikiRoot(
  path: string,
  label?: string,
): Promise<WikiRoot> {
  if (!isTauri()) {
    const root: WikiRoot = {
      id: crypto.randomUUID(),
      path,
      label: label?.trim() || path.split(/[/\\]/).filter(Boolean).pop() || path,
      enabled: true,
      created_at: nowIso(),
      updated_at: nowIso(),
      deleted_at: null,
    };
    mockStore.wikiRoots.push(root);
    return root;
  }
  return invoke<WikiRoot>("add_wiki_root_cmd", {
    args: { path, label: label ?? null },
  });
}

export async function apiRemoveWikiRoot(id: string): Promise<void> {
  if (!isTauri()) {
    mockStore.wikiRoots = mockStore.wikiRoots.filter((r) => r.id !== id);
    mockStore.wikiPages = mockStore.wikiPages.filter((p) => p.root_id !== id);
    return;
  }
  await invoke<null>("remove_wiki_root_cmd", { id });
}

export async function apiListWikiPages(rootId?: string): Promise<WikiPage[]> {
  if (!isTauri()) {
    return mockStore.wikiPages.filter((p) =>
      rootId ? p.root_id === rootId : true,
    );
  }
  return invoke<WikiPage[]>("list_wiki_pages_cmd", {
    rootId: rootId ?? null,
  });
}

export async function apiSearchWikiPages(query: string): Promise<WikiSearchHit[]> {
  if (!isTauri()) {
    return mockSearchWikiPages(query);
  }
  return invoke<WikiSearchHit[]>("search_wiki_pages_cmd", { query });
}

function mockSearchWikiPages(query: string): WikiSearchHit[] {
  const raw = query.trim();
  const pages = [...mockStore.wikiPages].sort(
    (a, b) => new Date(b.mtime).getTime() - new Date(a.mtime).getTime(),
  );
  if (!raw) {
    return pages.map((page) => ({
      page,
      kind: "recent" as const,
      reason: "Recent",
      snippet: null,
      score: 0,
    }));
  }

  const q = raw.toLowerCase();
  const hits: WikiSearchHit[] = [];
  for (const page of pages) {
    const title = page.title_cache ?? "";
    const body = mockStore.wikiContent[page.id] ?? "";
    const tags = page.tags_cache.join(" ");
    const classified = mockClassifyMatch(raw, q, title, page.rel_path, tags, body);
    if (!classified) continue;
    hits.push({
      page,
      kind: classified.kind,
      reason: classified.reason,
      snippet: classified.snippet,
      score: classified.score,
    });
  }
  hits.sort((a, b) => b.score - a.score || new Date(b.page.mtime).getTime() - new Date(a.page.mtime).getTime());
  return hits;
}

function mockClassifyMatch(
  raw: string,
  q: string,
  title: string,
  relPath: string,
  tags: string,
  body: string,
): { kind: WikiMatchKind; reason: string; snippet: WikiSearchSnippet | null; score: number } | null {
  const snippet = mockBodySnippet(body, raw);
  if (snippet) {
    return { kind: "body_exact", reason: "Text match", snippet, score: 100 };
  }
  if (title.toLowerCase().includes(q)) {
    return { kind: "title_exact", reason: "Title match", snippet: null, score: 90 };
  }
  if (tags.toLowerCase().includes(q)) {
    return { kind: "tags_exact", reason: "Tag match", snippet: null, score: 80 };
  }
  if (relPath.toLowerCase().includes(q)) {
    return { kind: "path_exact", reason: "Path match", snippet: null, score: 70 };
  }
  return null;
}

function mockBodySnippet(body: string, query: string): WikiSearchSnippet | null {
  const lower = body.toLowerCase();
  const q = query.toLowerCase();
  const idx = lower.indexOf(q);
  if (idx < 0) return null;
  const context = 56;
  const start = Math.max(0, idx - context);
  const end = Math.min(body.length, idx + query.length + context);
  const beforeRaw = `${start > 0 ? "…" : ""}${body.slice(start, idx)}`;
  const afterRaw = `${body.slice(idx + query.length, end)}${end < body.length ? "…" : ""}`;
  return {
    before: stripSnippetPart(beforeRaw),
    matched: stripMarkdownForDisplay(body.slice(idx, idx + query.length)),
    after: stripSnippetPart(afterRaw),
  };
}

function stripSnippetPart(text: string): string {
  const lead = text.startsWith("…");
  const trail = text.endsWith("…");
  const core = text.replace(/^…+/, "").replace(/…+$/, "");
  const cleaned = stripMarkdownForDisplay(core);
  return `${lead ? "…" : ""}${cleaned}${trail ? "…" : ""}`;
}

/** Light markdown cleanup for search snippet display (mirrors jade-core). */
function stripMarkdownForDisplay(text: string): string {
  let s = text;
  // Images then links: keep label, drop destination.
  s = s.replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1");
  s = s.replace(/!\[([^\]]*)\]\[[^\]]*\]/g, "$1");
  s = s.replace(/\[([^\]]+)\]\([^)]*\)/g, "$1");
  s = s.replace(/\[([^\]]+)\]\[[^\]]*\]/g, "$1");
  // Inline code ticks
  s = s.replace(/`([^`]+)`/g, "$1");
  // ATX headings / list / quote markers at line starts
  s = s.replace(/(^|\n)\s{0,3}#{1,6}\s+/g, "$1");
  s = s.replace(/(^|\n)\s{0,3}>+\s?/g, "$1");
  s = s.replace(/(^|\n)\s{0,3}[-*+]\s+/g, "$1");
  // Bold markers; lone * used as italic
  s = s.replace(/\*\*|__/g, "");
  s = s.replace(/\*/g, "");
  // Debris when a match splits a link/image across snippet edges
  s = s.replace(/^\]\([^)]*\)/, "");
  s = s.replace(/^\]\[[^\]]*\]/, "");
  s = s.replace(/^\]/, "");
  s = s.replace(/!?\[+$/, "");
  return s.replace(/\s+/g, " ").trim();
}

export async function apiReadWikiPage(id: string): Promise<WikiPageContent> {
  if (!isTauri()) {
    const page = mockStore.wikiPages.find((p) => p.id === id);
    if (!page) throw new Error("Wiki page not found");
    const content = mockStore.wikiContent[id] ?? `# ${page.title_cache ?? page.rel_path}\n`;
    return {
      page,
      absolute_path: page.rel_path,
      content,
      front_matter: { id: page.id, title: page.title_cache, tags: page.tags_cache },
      body: content,
    };
  }
  return invoke<WikiPageContent>("read_wiki_page_cmd", { id });
}

export async function apiCreateWikiPage(input: {
  root_id: string;
  rel_path: string;
  title?: string;
  body?: string;
  tags?: string[];
}): Promise<WikiPageContent> {
  if (!isTauri()) {
    const page: WikiPage = {
      id: crypto.randomUUID(),
      root_id: input.root_id,
      rel_path: input.rel_path.endsWith(".md")
        ? input.rel_path
        : `${input.rel_path}.md`,
      content_hash: "mock",
      mtime: nowIso(),
      indexed_at: nowIso(),
      missing_at: null,
      title_cache: input.title ?? input.rel_path,
      tags_cache: input.tags ?? [],
      created_at: nowIso(),
      updated_at: nowIso(),
      deleted_at: null,
    };
    mockStore.wikiPages.push(page);
    const content = `---\nid: ${page.id}\ntitle: ${page.title_cache}\n---\n${input.body ?? ""}\n`;
    mockStore.wikiContent[page.id] = content;
    return {
      page,
      absolute_path: page.rel_path,
      content,
      front_matter: { id: page.id, title: page.title_cache, tags: page.tags_cache },
      body: input.body ?? "",
    };
  }
  return invoke<WikiPageContent>("create_wiki_page_cmd", {
    args: {
      root_id: input.root_id,
      rel_path: input.rel_path,
      title: input.title ?? null,
      body: input.body ?? null,
      tags: input.tags ?? null,
    },
  });
}

export async function apiWriteWikiPage(
  id: string,
  content: string,
  ensureFrontMatter = true,
): Promise<WikiPageContent> {
  if (!isTauri()) {
    const page = mockStore.wikiPages.find((p) => p.id === id);
    if (!page) throw new Error("Wiki page not found");
    mockStore.wikiContent[id] = content;
    page.updated_at = nowIso();
    return apiReadWikiPage(id);
  }
  return invoke<WikiPageContent>("write_wiki_page_cmd", {
    args: {
      id,
      content,
      ensure_front_matter: ensureFrontMatter,
    },
  });
}

export async function apiReindexWiki(rootId?: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<null>("reindex_wiki_cmd", { rootId: rootId ?? null });
}

export async function apiListWikiBacklinks(pageId: string): Promise<WikiBacklink[]> {
  if (!isTauri()) return [];
  return invoke<WikiBacklink[]>("list_wiki_backlinks_cmd", { pageId });
}

export async function apiWikiRootSyncthingStatus(
  rootId: string,
): Promise<SyncthingStatus> {
  if (!isTauri()) {
    return {
      underSyncthing: false,
      markerDetected: false,
      folder: null,
      error: null,
    };
  }
  return invoke<SyncthingStatus>("wiki_root_syncthing_status_cmd", { rootId });
}

export async function apiPickWikiFolder(): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string | null>("pick_wiki_folder_cmd");
}

export async function apiListTaskEventsSince(
  afterSeq: number,
  limit?: number,
): Promise<TaskEvent[]> {
  if (!isTauri()) return [];
  return invoke<TaskEvent[]>("list_task_events_since_cmd", {
    args: { after_seq: afterSeq, limit: limit ?? null },
  });
}

export async function apiLatestEventSeq(): Promise<number> {
  if (!isTauri()) return 0;
  return invoke<number>("latest_event_seq_cmd");
}

/** Subscribe to SQLite change notifications from the Tauri host. */
export async function apiSubscribeDbChanged(
  onChanged: () => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return listen(DB_CHANGED_EVENT, () => {
    onChanged();
  });
}
