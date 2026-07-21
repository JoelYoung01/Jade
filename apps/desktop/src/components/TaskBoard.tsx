import * as React from "react";
import { useDroppable } from "@dnd-kit/core";
import { Search } from "lucide-react";

import { Input } from "@/components/ui/input";
import { TaskCard, type ReschedulePreset } from "@/components/TaskCard";
import type { Tag, Task, TaskStatus } from "@/lib/types";
import { cn } from "@/lib/utils";

const LANE_META: Record<TaskStatus, { title: string; hint: string }> = {
  inactive: { title: "Inactive", hint: "Not started" },
  active: { title: "Active", hint: "In progress" },
  complete: { title: "Complete", hint: "Done" },
};

type LaneProps = {
  status: TaskStatus;
  tasks: Task[];
  onEdit: (task: Task) => void;
  onUpdateStatus: (id: string, status: TaskStatus) => void;
  onReschedule: (id: string, mode: ReschedulePreset) => void;
  onRescheduleCustom: (id: string, dueAt: string) => void;
  onDelete: (id: string) => void;
};

function Lane({
  status,
  tasks,
  onEdit,
  onUpdateStatus,
  onReschedule,
  onRescheduleCustom,
  onDelete,
}: LaneProps): React.JSX.Element {
  const { setNodeRef, isOver } = useDroppable({ id: status });
  const meta = LANE_META[status];

  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
      <div className="flex shrink-0 items-baseline justify-between gap-2 px-1">
        <h2 className="font-display text-xs font-semibold tracking-[0.16em] text-muted-foreground uppercase">
          {meta.title}
        </h2>
        <span className="truncate text-[11px] text-muted-foreground/80">
          {tasks.length} · {meta.hint}
        </span>
      </div>
      <div
        ref={setNodeRef}
        className={cn(
          "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto rounded-lg border border-dashed border-border/50 p-2 transition-colors",
          isOver && "border-primary/50 bg-primary/5",
        )}
      >
        {tasks.length === 0 ? (
          <p className="px-1 py-8 text-center text-xs text-muted-foreground/70">No tasks</p>
        ) : (
          tasks.map((task) => (
            <TaskCard
              key={task.id}
              task={task}
              onEdit={onEdit}
              onUpdateStatus={onUpdateStatus}
              onReschedule={onReschedule}
              onRescheduleCustom={onRescheduleCustom}
              onDelete={onDelete}
            />
          ))
        )}
      </div>
    </section>
  );
}

type TaskBoardProps = {
  tasks: Task[];
  tags: Tag[];
  visible: Record<TaskStatus, boolean>;
  onToggleLane: (status: TaskStatus) => void;
  onEdit: (task: Task) => void;
  onUpdateStatus: (id: string, status: TaskStatus) => void;
  onReschedule: (id: string, mode: ReschedulePreset) => void;
  onRescheduleCustom: (id: string, dueAt: string) => void;
  onDelete: (id: string) => void;
};

const ORDER: TaskStatus[] = ["inactive", "active", "complete"];

function matchesFilters(task: Task, textQuery: string, tagId: string): boolean {
  if (tagId && !task.tags.some((tag) => tag.id === tagId)) {
    return false;
  }

  const query = textQuery.trim().toLowerCase();
  if (!query) return true;

  const haystack = [task.title, task.description ?? "", ...task.tags.map((t) => t.name)]
    .join(" ")
    .toLowerCase();
  return haystack.includes(query);
}

export function TaskBoard({
  tasks,
  tags,
  visible,
  onToggleLane,
  onEdit,
  onUpdateStatus,
  onReschedule,
  onRescheduleCustom,
  onDelete,
}: TaskBoardProps): React.JSX.Element {
  const [textQuery, setTextQuery] = React.useState("");
  const [tagId, setTagId] = React.useState("");

  const filtered = tasks.filter((task) => matchesFilters(task, textQuery, tagId));

  const byStatus = (status: TaskStatus) =>
    filtered
      .filter((t) => t.status === status)
      .sort((a, b) => new Date(a.due_at).getTime() - new Date(b.due_at).getTime());

  const visibleLanes = ORDER.filter((s) => visible[s]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 px-4 py-4">
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <div className="relative min-w-40 flex-1 basis-40">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={textQuery}
            onChange={(e) => setTextQuery(e.target.value)}
            placeholder="Filter tasks…"
            aria-label="Filter tasks"
            className="h-8 pl-8 text-xs"
          />
        </div>

        <select
          value={tagId}
          onChange={(e) => setTagId(e.target.value)}
          aria-label="Filter by tag"
          className="h-8 rounded-md border border-input bg-background px-2 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="">All tags</option>
          {tags.map((tag) => (
            <option key={tag.id} value={tag.id}>
              {tag.name}
            </option>
          ))}
        </select>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <span className="text-xs text-muted-foreground">Lanes</span>
          {ORDER.map((status) => (
            <button
              key={status}
              type="button"
              onClick={() => onToggleLane(status)}
              className={cn(
                "rounded-md px-2.5 py-1 text-xs transition-colors",
                visible[status]
                  ? "bg-primary/15 text-primary"
                  : "bg-secondary text-muted-foreground hover:text-foreground",
              )}
              aria-pressed={visible[status]}
            >
              {LANE_META[status].title}
            </button>
          ))}
        </div>
      </div>

      {visibleLanes.length > 0 ? (
        <div className="flex min-h-0 flex-1 gap-3">
          {visibleLanes.map((status) => (
            <Lane
              key={status}
              status={status}
              tasks={byStatus(status)}
              onEdit={onEdit}
              onUpdateStatus={onUpdateStatus}
              onReschedule={onReschedule}
              onRescheduleCustom={onRescheduleCustom}
              onDelete={onDelete}
            />
          ))}
        </div>
      ) : (
        <p className="py-16 text-center text-sm text-muted-foreground">
          All lanes hidden — toggle one above to focus.
        </p>
      )}
    </div>
  );
}
