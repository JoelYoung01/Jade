import { invoke } from "@tauri-apps/api/core";

import { nextOccurrences, validateCron } from "@/lib/repeat";
import { applyDuePreset, type DueQuickPreset } from "@/lib/time";
import type {
  CreateTaskInput,
  LaneVisibility,
  RescheduleMode,
  Settings,
  StatusUpdateResult,
  Tag,
  Task,
  TaskStatus,
  UpdateTaskInput,
} from "@/lib/types";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

type MockStore = {
  tasks: Task[];
  tags: Tag[];
  settings: Settings;
};

const mockStore: MockStore = {
  tasks: [],
  tags: [],
  settings: {
    lane_visibility: { inactive: true, active: true, complete: false },
  },
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
