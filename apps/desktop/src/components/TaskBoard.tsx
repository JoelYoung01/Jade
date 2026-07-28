import * as React from "react";
import { useDroppable } from "@dnd-kit/core";
import { Search } from "lucide-react";

import { Input } from "@/components/ui/input";
import { TaskCard, type ReschedulePreset } from "@/components/TaskCard";
import {
  matchesTextFilter,
  toggleTagInTextFilter,
} from "@/lib/taskFilter";
import type { Task, TaskMotion, TaskStatus } from "@/lib/types";
import {
  type DateRangePreset,
  matchesDueDateRange,
  resolveDateRangeBounds,
  toDateInputValue,
} from "@/lib/time";
import { useFlipLayout } from "@/lib/useFlipLayout";
import { cn } from "@/lib/utils";

const LANE_META: Record<TaskStatus, { title: string; hint: string }> = {
  inactive: { title: "Inactive", hint: "Not started" },
  active: { title: "Active", hint: "In progress" },
  complete: { title: "Complete", hint: "Done" },
};

type LaneProps = {
  status: TaskStatus;
  tasks: Task[];
  /** Total tasks in this status before board filters are applied. */
  unfilteredCount: number;
  now: Date;
  textQuery: string;
  selectedIds: ReadonlySet<string>;
  draggingIds: ReadonlySet<string>;
  selectedTasks: Task[];
  motionById?: ReadonlyMap<string, TaskMotion> | undefined;
  onToggleSelect: (id: string) => void;
  onClearSelection: () => void;
  onPrepareContextSelection: (id: string) => void;
  onFilterTag: (tagName: string) => void;
  onEdit: (task: Task) => void;
  onUpdateStatus: (ids: string[], status: TaskStatus) => void;
  onReschedule: (ids: string[], mode: ReschedulePreset) => void;
  onRescheduleCustom: (ids: string[], dueAt: string) => void;
  onDelete: (ids: string[]) => void;
};

function Lane({
  status,
  tasks,
  unfilteredCount,
  now,
  textQuery,
  selectedIds,
  draggingIds,
  selectedTasks,
  motionById,
  onToggleSelect,
  onClearSelection,
  onPrepareContextSelection,
  onFilterTag,
  onEdit,
  onUpdateStatus,
  onReschedule,
  onRescheduleCustom,
  onDelete,
}: LaneProps): React.JSX.Element {
  const { setNodeRef, isOver } = useDroppable({ id: status });
  const meta = LANE_META[status];
  const emptyMessage =
    unfilteredCount > 0 ? "No tasks match your filters" : "No tasks";

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
        data-lane-scroll
        className={cn(
          "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto rounded-lg border border-dashed border-border/50 p-2 transition-colors",
          isOver && "border-primary/50 bg-primary/5",
        )}
        onPointerDown={(event) => {
          const target = event.target as HTMLElement | null;
          if (target?.closest("[data-flip-id]")) return;
          onClearSelection();
        }}
      >
        {tasks.length === 0 ? (
          <p className="px-1 py-8 text-center text-xs text-muted-foreground/70">
            {emptyMessage}
          </p>
        ) : (
          tasks.map((task) => (
            <TaskCard
              key={task.id}
              task={task}
              now={now}
              textQuery={textQuery}
              selected={selectedIds.has(task.id)}
              dragging={draggingIds.has(task.id)}
              motion={motionById?.get(task.id)}
              selectedTasks={selectedTasks}
              onToggleSelect={onToggleSelect}
              onClearSelection={onClearSelection}
              onPrepareContextSelection={onPrepareContextSelection}
              onFilterTag={onFilterTag}
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
  now: Date;
  visible: Record<TaskStatus, boolean>;
  /** When false, skip FLIP (e.g. while a card is being dragged). */
  animateLayout?: boolean;
  motionById?: ReadonlyMap<string, TaskMotion> | undefined;
  selectedIds: ReadonlySet<string>;
  draggingIds: ReadonlySet<string>;
  onToggleSelect: (id: string) => void;
  onClearSelection: () => void;
  onPrepareContextSelection: (id: string) => void;
  onToggleLane: (status: TaskStatus) => void;
  onEdit: (task: Task) => void;
  onUpdateStatus: (ids: string[], status: TaskStatus) => void;
  onReschedule: (ids: string[], mode: ReschedulePreset) => void;
  onRescheduleCustom: (ids: string[], dueAt: string) => void;
  onDelete: (ids: string[]) => void;
};

const ORDER: TaskStatus[] = ["inactive", "active", "complete"];

const DATE_RANGE_OPTIONS: { value: DateRangePreset; label: string }[] = [
  { value: "today", label: "Today" },
  { value: "this_week", label: "This week" },
  { value: "all_time", label: "All time" },
  { value: "custom", label: "Custom" },
];

type BoardFilters = {
  textQuery: string;
  datePreset: DateRangePreset;
  customFrom: string;
  customTo: string;
  now?: Date;
};

function matchesFilters(task: Task, filters: BoardFilters): boolean {
  const { textQuery, datePreset, customFrom, customTo, now } = filters;

  if (datePreset !== "all_time") {
    const bounds = resolveDateRangeBounds(datePreset, customFrom, customTo, now);
    if (
      !matchesDueDateRange(task.due_at, bounds, {
        ...(now ? { now } : {}),
        isComplete: task.status === "complete",
      })
    ) {
      return false;
    }
  }

  return matchesTextFilter(task, textQuery);
}

export function TaskBoard({
  tasks,
  now,
  visible,
  animateLayout = true,
  motionById,
  selectedIds,
  draggingIds,
  onToggleSelect,
  onClearSelection,
  onPrepareContextSelection,
  onToggleLane,
  onEdit,
  onUpdateStatus,
  onReschedule,
  onRescheduleCustom,
  onDelete,
}: TaskBoardProps): React.JSX.Element {
  const [textQuery, setTextQuery] = React.useState("");
  const [datePreset, setDatePreset] = React.useState<DateRangePreset>("today");
  const [customFrom, setCustomFrom] = React.useState(() => toDateInputValue(new Date()));
  const [customTo, setCustomTo] = React.useState(() => toDateInputValue(new Date()));
  const lanesRef = React.useRef<HTMLDivElement>(null);
  const filterInputRef = React.useRef<HTMLInputElement>(null);

  const selectClassName =
    "h-8 rounded-md border border-input bg-background px-2 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring";

  React.useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "f") {
        return;
      }

      const target = event.target as HTMLElement | null;
      const inField =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable);
      if (inField && target !== filterInputRef.current) return;

      event.preventDefault();
      const input = filterInputRef.current;
      if (!input) return;
      input.focus();
      input.select();
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const filtered = tasks.filter((task) =>
    matchesFilters(task, { textQuery, datePreset, customFrom, customTo, now }),
  );

  const tasksByLane = React.useMemo(() => {
    const groups: Record<TaskStatus, Task[]> = {
      inactive: [],
      active: [],
      complete: [],
    };
    for (const task of filtered) {
      groups[task.status].push(task);
    }
    for (const status of ORDER) {
      groups[status].sort(
        (a, b) => new Date(a.due_at).getTime() - new Date(b.due_at).getTime(),
      );
    }
    return groups;
  }, [filtered]);

  const unfilteredCountByLane = React.useMemo(() => {
    const counts: Record<TaskStatus, number> = {
      inactive: 0,
      active: 0,
      complete: 0,
    };
    for (const task of tasks) {
      counts[task.status] += 1;
    }
    return counts;
  }, [tasks]);

  const selectedTasks = React.useMemo(
    () => tasks.filter((task) => selectedIds.has(task.id)),
    [tasks, selectedIds],
  );

  const visibleLanes = ORDER.filter((s) => visible[s]);

  const layoutKey = React.useMemo(() => {
    const lanes = visibleLanes
      .map((status) =>
        tasksByLane[status]
          .map((task) => `${task.id}:${task.status}:${task.due_at}`)
          .join(","),
      )
      .join("|");
    return `${lanes}#q:${textQuery}#date:${datePreset}:${customFrom}:${customTo}`;
  }, [tasksByLane, visibleLanes, textQuery, datePreset, customFrom, customTo]);

  useFlipLayout(lanesRef, layoutKey, animateLayout);

  function handleDatePresetChange(next: DateRangePreset): void {
    setDatePreset(next);
    if (next === "custom") {
      const today = toDateInputValue(new Date());
      setCustomFrom((prev) => prev || today);
      setCustomTo((prev) => prev || today);
    }
  }

  function handleFilterTag(tagName: string): void {
    setTextQuery((prev) => toggleTagInTextFilter(prev, tagName));
  }

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-4 px-4 py-4">
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <div className="relative min-w-40 flex-1 basis-40">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={filterInputRef}
            data-jade-board-filter=""
            value={textQuery}
            onChange={(e) => setTextQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.currentTarget.blur();
              }
            }}
            placeholder="Filter tasks… ( | = OR)"
            aria-label="Filter tasks"
            title="Match any term separated by | (OR)"
            className="h-8 pl-8 text-xs"
          />
        </div>

        <select
          value={datePreset}
          onChange={(e) => handleDatePresetChange(e.target.value as DateRangePreset)}
          aria-label="Filter by due date range"
          className={selectClassName}
        >
          {DATE_RANGE_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>

        {datePreset === "custom" ? (
          <>
            <Input
              type="date"
              value={customFrom}
              onChange={(e) => setCustomFrom(e.target.value)}
              aria-label="Custom range start"
              className="h-8 w-auto text-xs"
            />
            <span className="text-xs text-muted-foreground">to</span>
            <Input
              type="date"
              value={customTo}
              onChange={(e) => setCustomTo(e.target.value)}
              aria-label="Custom range end"
              className="h-8 w-auto text-xs"
            />
          </>
        ) : null}

        <div className="ml-auto flex flex-wrap items-center gap-2">
          {selectedIds.size > 0 ? (
            <span className="text-xs text-muted-foreground">
              {selectedIds.size} selected
            </span>
          ) : null}
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
        <div
          ref={lanesRef}
          className="flex min-h-0 flex-1 gap-3"
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) onClearSelection();
          }}
        >
          {visibleLanes.map((status) => (
            <Lane
              key={status}
              status={status}
              tasks={tasksByLane[status]}
              unfilteredCount={unfilteredCountByLane[status]}
              now={now}
              textQuery={textQuery}
              selectedIds={selectedIds}
              draggingIds={draggingIds}
              selectedTasks={selectedTasks}
              motionById={motionById}
              onToggleSelect={onToggleSelect}
              onClearSelection={onClearSelection}
              onPrepareContextSelection={onPrepareContextSelection}
              onFilterTag={handleFilterTag}
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
