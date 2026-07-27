import * as React from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragCancelEvent,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";

import { AppShell } from "@/components/AppShell";
import { CreateTaskDialog } from "@/components/CreateTaskDialog";
import { TaskBoard } from "@/components/TaskBoard";
import { TaskDragPreview, type ReschedulePreset } from "@/components/TaskCard";
import { ShortcutKeys } from "@/components/ui/kbd";
import { TooltipProvider } from "@/components/ui/tooltip";
import {
  apiCountTasksWithTag,
  apiCreateTask,
  apiDeleteTag,
  apiDeleteTask,
  apiGetSettings,
  apiListTags,
  apiListTasks,
  apiRescheduleTask,
  apiSetLaneVisibility,
  apiUpdateTask,
  apiUpdateTaskStatus,
} from "@/lib/api";
import type { LaneVisibility, Tag, Task, TaskFormValues, TaskStatus } from "@/lib/types";
import { recentTagNames } from "@/lib/tags";
import { useLiveTaskSync } from "@/lib/useLiveTaskSync";
import { useNow } from "@/lib/useNow";

const STATUS_LANES: TaskStatus[] = ["inactive", "active", "complete"];

function isStatus(value: string): value is TaskStatus {
  return STATUS_LANES.includes(value as TaskStatus);
}

export default function App(): React.JSX.Element {
  const now = useNow();
  const [tasks, setTasks] = React.useState<Task[]>([]);
  const [tags, setTags] = React.useState<Tag[]>([]);
  const [visibility, setVisibility] = React.useState<LaneVisibility>({
    inactive: true,
    active: true,
    complete: false,
  });
  const [createOpen, setCreateOpen] = React.useState(false);
  const [editingTask, setEditingTask] = React.useState<Task | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [activeId, setActiveId] = React.useState<string | null>(null);
  const [draggingIds, setDraggingIds] = React.useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const draggingIdsRef = React.useRef<ReadonlySet<string>>(new Set());
  const [selectedIds, setSelectedIds] = React.useState<ReadonlySet<string>>(
    () => new Set(),
  );

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 6 },
    }),
  );

  const refresh = React.useCallback(async () => {
    const [nextTasks, nextTags, settings] = await Promise.all([
      apiListTasks(),
      apiListTags(),
      apiGetSettings(),
    ]);
    setTasks(nextTasks);
    setTags(nextTags);
    setVisibility(settings.lane_visibility);
  }, []);

  const onSyncError = React.useCallback((message: string) => {
    setError(message);
  }, []);

  const { displayTasks, motionById, markSynced } = useLiveTaskSync({
    tasks,
    refresh,
    onError: onSyncError,
  });

  const refreshAndAck = React.useCallback(async () => {
    await refresh();
    await markSynced();
  }, [refresh, markSynced]);

  React.useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await refreshAndAck();
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refreshAndAck]);

  React.useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      const target = event.target as HTMLElement | null;
      const inField =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable);

      if (event.key === "Escape" && selectedIds.size > 0 && !inField) {
        setSelectedIds(new Set());
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") {
        if (inField) return;
        event.preventDefault();
        openCreate();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedIds]);

  async function handleCreate(input: TaskFormValues): Promise<void> {
    await apiCreateTask({
      title: input.title,
      ...(input.description !== undefined ? { description: input.description } : {}),
      due_at: input.due_at,
      tag_names: input.tag_names,
      repeat_cron: input.repeat_cron ?? null,
    });
    await refreshAndAck();
  }

  async function handleUpdate(input: TaskFormValues): Promise<void> {
    if (!editingTask) return;
    await apiUpdateTask({
      id: editingTask.id,
      title: input.title,
      description: input.description ?? "",
      due_at: input.due_at,
      tag_names: input.tag_names,
      repeat_cron: input.repeat_cron ?? null,
    });
    await refreshAndAck();
  }

  function openCreate(): void {
    setEditingTask(null);
    setCreateOpen(true);
  }

  function openEdit(task: Task): void {
    setEditingTask(task);
    setCreateOpen(true);
  }

  function handleDialogOpenChange(open: boolean): void {
    setCreateOpen(open);
    if (!open) setEditingTask(null);
  }

  async function handleToggleLane(status: TaskStatus): Promise<void> {
    const next = { ...visibility, [status]: !visibility[status] };
    setVisibility(next);
    try {
      const settings = await apiSetLaneVisibility(next);
      setVisibility(settings.lane_visibility);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refreshAndAck();
    }
  }

  function toggleSelect(id: string): void {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function clearSelection(): void {
    setSelectedIds(new Set());
  }

  /** Right-click on an unselected card while others are selected scopes to that card only. */
  function prepareContextSelection(id: string): void {
    setSelectedIds((prev) => {
      if (prev.size === 0 || prev.has(id)) return prev;
      return new Set([id]);
    });
  }

  async function handleReschedule(ids: string[], mode: ReschedulePreset): Promise<void> {
    const unique = [...new Set(ids)];
    if (unique.length === 0) return;
    try {
      const updated = await Promise.all(unique.map((id) => apiRescheduleTask(id, mode)));
      const byId = new Map(updated.map((task) => [task.id, task]));
      setTasks((prev) => prev.map((t) => byId.get(t.id) ?? t));
      await markSynced();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refreshAndAck();
    }
  }

  async function handleRescheduleCustom(ids: string[], dueAt: string): Promise<void> {
    const unique = [...new Set(ids)];
    if (unique.length === 0) return;
    try {
      const updated = await Promise.all(
        unique.map((id) => apiRescheduleTask(id, "custom", dueAt)),
      );
      const byId = new Map(updated.map((task) => [task.id, task]));
      setTasks((prev) => prev.map((t) => byId.get(t.id) ?? t));
      await markSynced();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refreshAndAck();
    }
  }

  async function handleDelete(ids: string[]): Promise<void> {
    const unique = [...new Set(ids)];
    if (unique.length === 0) return;
    try {
      await Promise.all(unique.map((id) => apiDeleteTask(id)));
      setSelectedIds(new Set());
      await refreshAndAck();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleUpdateStatus(ids: string[], status: TaskStatus): Promise<void> {
    const unique = [...new Set(ids)];
    const toUpdate = unique.filter((id) => {
      const task = tasks.find((t) => t.id === id);
      return task != null && task.status !== status;
    });
    if (toUpdate.length === 0) return;

    setTasks((prev) =>
      prev.map((t) => (toUpdate.includes(t.id) ? { ...t, status } : t)),
    );
    try {
      const results = await Promise.all(
        toUpdate.map((id) => apiUpdateTaskStatus(id, status)),
      );
      setTasks((prev) => {
        let next = prev;
        for (const result of results) {
          next = next.map((t) => (t.id === result.task.id ? result.task : t));
          if (result.spawned && !next.some((t) => t.id === result.spawned!.id)) {
            next = [...next, result.spawned];
          }
        }
        return next;
      });
      setSelectedIds(new Set());
      await refreshAndAck();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refreshAndAck();
    }
  }

  function onDragStart(event: DragStartEvent): void {
    const id = String(event.active.id);
    setActiveId(id);
    // Dragging a selected card moves the whole selection; otherwise just that card.
    const ids =
      selectedIds.has(id) && selectedIds.size > 1
        ? new Set(selectedIds)
        : new Set([id]);
    draggingIdsRef.current = ids;
    setDraggingIds(ids);
  }

  function clearDragState(): void {
    draggingIdsRef.current = new Set();
    setActiveId(null);
    setDraggingIds(new Set());
  }

  function onDragCancel(_event: DragCancelEvent): void {
    clearDragState();
  }

  async function onDragEnd(event: DragEndEvent): Promise<void> {
    const ids = [...draggingIdsRef.current];
    clearDragState();

    const overId = event.over?.id;
    if (!overId || ids.length === 0) return;
    const status = String(overId);
    if (!isStatus(status)) return;

    await handleUpdateStatus(ids, status);
  }

  const dragTasks = React.useMemo(() => {
    if (draggingIds.size === 0) return [];
    const byId = new Map(displayTasks.map((task) => [task.id, task]));
    const ordered: Task[] = [];
    // Prefer the grabbed card first in the overlay stack.
    if (activeId) {
      const active = byId.get(activeId);
      if (active) ordered.push(active);
    }
    for (const id of draggingIds) {
      if (id === activeId) continue;
      const task = byId.get(id);
      if (task) ordered.push(task);
    }
    return ordered;
  }, [displayTasks, draggingIds, activeId]);

  return (
    <TooltipProvider delayDuration={200}>
      <AppShell onCreateTask={openCreate}>
        {error && (
          <div className="mx-4 mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
            <button
              type="button"
              className="ml-3 underline"
              onClick={() => setError(null)}
            >
              dismiss
            </button>
          </div>
        )}

        {loading ? (
          <p className="px-4 py-16 text-center text-sm text-muted-foreground">Loading…</p>
        ) : (
          <DndContext
            sensors={sensors}
            onDragStart={onDragStart}
            onDragCancel={onDragCancel}
            onDragEnd={(e) => void onDragEnd(e)}
          >
            <div className="flex min-h-0 flex-1 flex-col">
              <TaskBoard
                tasks={displayTasks}
                now={now}
                visible={visibility}
                animateLayout={activeId === null}
                motionById={motionById}
                selectedIds={selectedIds}
                draggingIds={draggingIds}
                onToggleSelect={toggleSelect}
                onClearSelection={clearSelection}
                onPrepareContextSelection={prepareContextSelection}
                onToggleLane={(status) => void handleToggleLane(status)}
                onEdit={openEdit}
                onUpdateStatus={(ids, status) => void handleUpdateStatus(ids, status)}
                onReschedule={(ids, mode) => void handleReschedule(ids, mode)}
                onRescheduleCustom={(ids, dueAt) => void handleRescheduleCustom(ids, dueAt)}
                onDelete={(ids) => void handleDelete(ids)}
              />
            </div>
            <DragOverlay dropAnimation={null}>
              {dragTasks.length > 0 ? (
                <TaskDragPreview tasks={dragTasks} now={now} />
              ) : null}
            </DragOverlay>
          </DndContext>
        )}

        {!loading && displayTasks.length === 0 && !error && (
          <p className="pointer-events-none fixed inset-x-0 bottom-10 flex items-center justify-center gap-1.5 text-sm text-muted-foreground">
            <span>No tasks yet — press</span>
            <ShortcutKeys keys={["Ctrl", "N"]} />
            <span>or use +</span>
          </p>
        )}
      </AppShell>

      <CreateTaskDialog
        open={createOpen}
        onOpenChange={handleDialogOpenChange}
        task={editingTask}
        existingTags={tags}
        recentTagNames={recentTagNames(tasks)}
        onSubmit={editingTask ? handleUpdate : handleCreate}
        onCountTagUsage={apiCountTasksWithTag}
        onDeleteTag={async (tagId) => {
          await apiDeleteTag(tagId);
          await refreshAndAck();
        }}
      />
    </TooltipProvider>
  );
}
