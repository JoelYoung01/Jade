import * as React from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { TriangleAlert } from "lucide-react";

import { AppShell } from "@/components/AppShell";
import { CreateTaskDialog } from "@/components/CreateTaskDialog";
import { TaskBoard } from "@/components/TaskBoard";
import type { ReschedulePreset } from "@/components/TaskCard";
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
import { formatDue, isOverdue } from "@/lib/time";
import { recentTagNames } from "@/lib/tags";
import { cn } from "@/lib/utils";

const STATUS_LANES: TaskStatus[] = ["inactive", "active", "complete"];

function isStatus(value: string): value is TaskStatus {
  return STATUS_LANES.includes(value as TaskStatus);
}

export default function App(): React.JSX.Element {
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

  React.useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await refresh();
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
  }, [refresh]);

  React.useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") {
        const target = event.target as HTMLElement | null;
        if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) {
          return;
        }
        event.preventDefault();
        openCreate();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  async function handleCreate(input: TaskFormValues): Promise<void> {
    await apiCreateTask({
      title: input.title,
      ...(input.description !== undefined ? { description: input.description } : {}),
      due_at: input.due_at,
      tag_names: input.tag_names,
      repeat_cron: input.repeat_cron ?? null,
    });
    await refresh();
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
    await refresh();
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
      await refresh();
    }
  }

  async function handleReschedule(id: string, mode: ReschedulePreset): Promise<void> {
    try {
      await apiRescheduleTask(id, mode);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleRescheduleCustom(id: string, dueAt: string): Promise<void> {
    try {
      await apiRescheduleTask(id, "custom", dueAt);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleDelete(id: string): Promise<void> {
    try {
      await apiDeleteTask(id);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleUpdateStatus(id: string, status: TaskStatus): Promise<void> {
    const task = tasks.find((t) => t.id === id);
    if (!task || task.status === status) return;

    setTasks((prev) => prev.map((t) => (t.id === id ? { ...t, status } : t)));
    try {
      const result = await apiUpdateTaskStatus(id, status);
      setTasks((prev) => {
        const withoutOld = prev.map((t) => (t.id === id ? result.task : t));
        if (!result.spawned) return withoutOld;
        if (withoutOld.some((t) => t.id === result.spawned!.id)) return withoutOld;
        return [...withoutOld, result.spawned];
      });
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refresh();
    }
  }

  function onDragStart(event: DragStartEvent): void {
    setActiveId(String(event.active.id));
  }

  async function onDragEnd(event: DragEndEvent): Promise<void> {
    setActiveId(null);
    const overId = event.over?.id;
    if (!overId) return;
    const status = String(overId);
    if (!isStatus(status)) return;

    const taskId = String(event.active.id);
    const task = tasks.find((t) => t.id === taskId);
    if (!task || task.status === status) return;

    setTasks((prev) => prev.map((t) => (t.id === taskId ? { ...t, status } : t)));
    try {
      await apiUpdateTaskStatus(taskId, status);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await refresh();
    }
  }

  const activeTask = activeId ? tasks.find((t) => t.id === activeId) : undefined;

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
          <DndContext sensors={sensors} onDragStart={onDragStart} onDragEnd={(e) => void onDragEnd(e)}>
            <div className="flex min-h-0 flex-1 flex-col">
              <TaskBoard
                tasks={tasks}
                tags={tags}
                visible={visibility}
                onToggleLane={(status) => void handleToggleLane(status)}
                onEdit={openEdit}
                onUpdateStatus={(id, status) => void handleUpdateStatus(id, status)}
                onReschedule={(id, mode) => void handleReschedule(id, mode)}
                onRescheduleCustom={(id, dueAt) => void handleRescheduleCustom(id, dueAt)}
                onDelete={(id) => void handleDelete(id)}
              />
            </div>
            <DragOverlay>
              {activeTask ? (
                <div className="rounded-md border border-primary/40 bg-card px-3 py-2.5 shadow-xl">
                  <p className="text-sm font-medium">{activeTask.title}</p>
                  <p
                    className={cn(
                      "mt-1 flex items-center gap-1 text-xs",
                      activeTask.status !== "complete" && isOverdue(activeTask.due_at)
                        ? "font-medium text-red-400"
                        : "text-muted-foreground",
                    )}
                  >
                    {activeTask.status !== "complete" && isOverdue(activeTask.due_at) ? (
                      <TriangleAlert className="size-3.5 shrink-0" aria-hidden />
                    ) : null}
                    <span>{formatDue(activeTask.due_at)}</span>
                  </p>
                </div>
              ) : null}
            </DragOverlay>
          </DndContext>
        )}

        {!loading && tasks.length === 0 && !error && (
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
          await refresh();
        }}
      />
    </TooltipProvider>
  );
}
