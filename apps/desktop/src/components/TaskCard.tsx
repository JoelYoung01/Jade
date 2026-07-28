import * as React from "react";
import { useDraggable } from "@dnd-kit/core";
import { Repeat, TriangleAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ShortcutKeys } from "@/components/ui/kbd";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { TagLabel } from "@/components/TagLabel";
import { describeCron } from "@/lib/repeat";
import { parseTagParts } from "@/lib/tags";
import type { RescheduleMode, Task, TaskMotion, TaskStatus } from "@/lib/types";
import {
  formatDue,
  isOverdue,
  matchesDueQuickPreset,
  toDatetimeLocalValue,
  type DueQuickPreset,
} from "@/lib/time";
import { cn } from "@/lib/utils";

const STATUS_ORDER: TaskStatus[] = ["inactive", "active", "complete"];

const STATUS_LABEL: Record<TaskStatus, string> = {
  inactive: "Inactive",
  active: "Active",
  complete: "Complete",
};

const RESCHEDULE_PRESETS: { mode: DueQuickPreset; label: string }[] = [
  { mode: "today", label: "Today" },
  { mode: "tomorrow", label: "Tomorrow" },
  { mode: "next_monday", label: "Next Monday" },
  { mode: "first_monday_next_month", label: "First Monday of Next Month" },
];

function nextStatus(status: TaskStatus): TaskStatus | null {
  const index = STATUS_ORDER.indexOf(status);
  if (index < 0 || index >= STATUS_ORDER.length - 1) return null;
  return STATUS_ORDER[index + 1] ?? null;
}

/** Shared next status when every target has the same status; otherwise null. */
function sharedNextStatus(tasks: Task[]): TaskStatus | null {
  if (tasks.length === 0) return null;
  const first = tasks[0]!.status;
  if (!tasks.every((t) => t.status === first)) return null;
  return nextStatus(first);
}

export type ReschedulePreset = Exclude<RescheduleMode, "custom">;

type TaskCardProps = {
  task: Task;
  now: Date;
  selected: boolean;
  /** True while this card is part of the active multi/single drag. */
  dragging: boolean;
  /** Live-sync enter / exit / flash motion. */
  motion?: TaskMotion | undefined;
  selectedTasks: Task[];
  onToggleSelect: (id: string) => void;
  onClearSelection: () => void;
  onPrepareContextSelection: (id: string) => void;
  onEdit: (task: Task) => void;
  onUpdateStatus: (ids: string[], status: TaskStatus) => void;
  onReschedule: (ids: string[], mode: ReschedulePreset) => void;
  onRescheduleCustom: (ids: string[], dueAt: string) => void;
  onDelete: (ids: string[]) => void;
};

const DRAG_STACK_LIMIT = 3;

/** Floating preview shown under the pointer while dragging one or more tasks. */
export function TaskDragPreview({
  tasks,
  now,
}: {
  tasks: Task[];
  now: Date;
}): React.JSX.Element | null {
  if (tasks.length === 0) return null;

  const visible = tasks.slice(0, DRAG_STACK_LIMIT);
  const extra = tasks.length - visible.length;

  return (
    <div className="relative w-[min(100vw-2rem,18rem)]">
      {/* Back-to-front so the grabbed card sits on top of the stack. */}
      {[...visible].reverse().map((task, reverseIndex) => {
        const index = visible.length - 1 - reverseIndex;
        const overdue = task.status !== "complete" && isOverdue(task.due_at, now);
        const isFront = index === 0;
        return (
          <div
            key={task.id}
            className={cn(
              "rounded-md border border-primary/40 bg-card px-3 py-2.5 shadow-xl",
              !isFront && "pointer-events-none absolute inset-x-0 top-0",
            )}
            style={
              isFront
                ? { position: "relative", zIndex: visible.length }
                : {
                    transform: `translate(${index * 6}px, ${index * 6}px)`,
                    zIndex: visible.length - index,
                    opacity: Math.max(0.55, 1 - index * 0.15),
                  }
            }
          >
            {isFront ? (
              <>
                <div className="flex items-start justify-between gap-2">
                  <p className="text-sm font-medium leading-snug">{task.title}</p>
                  {tasks.length > 1 ? (
                    <span className="shrink-0 rounded-md bg-primary px-1.5 py-0.5 text-[10px] font-semibold text-primary-foreground tabular-nums">
                      {tasks.length}
                    </span>
                  ) : null}
                </div>
                <p
                  className={cn(
                    "mt-1 flex items-center gap-1 text-xs",
                    overdue ? "font-medium text-red-400" : "text-muted-foreground",
                  )}
                >
                  {overdue ? (
                    <TriangleAlert className="size-3.5 shrink-0" aria-hidden />
                  ) : null}
                  <span>{formatDue(task.due_at, now)}</span>
                </p>
                {extra > 0 ? (
                  <p className="mt-1 text-[10px] text-muted-foreground">
                    +{extra} more
                  </p>
                ) : null}
              </>
            ) : (
              <p className="truncate text-sm font-medium text-muted-foreground">
                {task.title}
              </p>
            )}
          </div>
        );
      })}
    </div>
  );
}

export function TaskCard({
  task,
  now,
  selected,
  dragging,
  motion,
  selectedTasks,
  onToggleSelect,
  onClearSelection,
  onPrepareContextSelection,
  onEdit,
  onUpdateStatus,
  onReschedule,
  onRescheduleCustom,
  onDelete,
}: TaskCardProps): React.JSX.Element {
  const { attributes, listeners, setNodeRef } = useDraggable({
    id: task.id,
    data: { status: task.status },
  });
  const [customOpen, setCustomOpen] = React.useState(false);
  const [deleteOpen, setDeleteOpen] = React.useState(false);
  const [menuTargets, setMenuTargets] = React.useState<Task[]>([task]);
  const [customDue, setCustomDue] = React.useState(() =>
    toDatetimeLocalValue(new Date(task.due_at)),
  );
  const confirmDeleteRef = React.useRef<HTMLButtonElement>(null);

  const style: React.CSSProperties = {
    // DragOverlay carries the pointer preview; lane cards stay put as ghosts.
    opacity: dragging ? 0.4 : 1,
  };

  const targetIds = menuTargets.map((t) => t.id);
  const multi = menuTargets.length > 1;
  const moveTo = sharedNextStatus(menuTargets);

  function confirmDelete(): void {
    onDelete(targetIds);
    setDeleteOpen(false);
  }

  function saveCustomReschedule(): void {
    const date = new Date(customDue);
    if (Number.isNaN(date.getTime())) return;
    onRescheduleCustom(targetIds, date.toISOString());
    setCustomOpen(false);
  }

  const overdue = task.status !== "complete" && isOverdue(task.due_at, now);
  const repeatLabel = task.repeat_cron ? describeCron(task.repeat_cron) : null;

  return (
    <>
      <ContextMenu
        onOpenChange={(open) => {
          if (!open) return;
          const targets =
            selected && selectedTasks.length > 1 ? selectedTasks : [task];
          setMenuTargets(targets);
          onPrepareContextSelection(task.id);
        }}
      >
        <ContextMenuTrigger asChild>
          <div
            data-flip-id={task.id}
            className={cn(
              "w-full shrink-0 will-change-transform",
              motion === "enter" && "jade-card-enter",
              motion === "exit" && "jade-card-exit",
            )}
          >
            <article
              ref={setNodeRef}
              style={style}
              aria-selected={selected}
              className={cn(
                "cursor-grab rounded-md border border-transparent bg-card/70 px-3 py-2.5 transition-colors hover:border-border active:cursor-grabbing",
                selected && "border-primary/50 bg-primary/10 ring-1 ring-primary/35",
                dragging && "shadow-lg ring-1 ring-primary/40",
                motion === "flash" && "jade-card-flash",
              )}
              onClick={(event) => {
                if (event.ctrlKey || event.metaKey) {
                  event.preventDefault();
                  event.stopPropagation();
                  onToggleSelect(task.id);
                  return;
                }
                if (selectedTasks.length > 0) {
                  onClearSelection();
                }
              }}
              onDoubleClick={(event) => {
                event.stopPropagation();
                onEdit(task);
              }}
              {...listeners}
              {...attributes}
              onPointerDown={(event) => {
                if (event.ctrlKey || event.metaKey) {
                  // Keep dnd-kit from starting a drag on modifier-click select.
                  return;
                }
                listeners?.onPointerDown?.(event);
              }}
            >
              <div className="flex items-start justify-between gap-2">
                <h3 className="text-sm font-medium leading-snug text-foreground">{task.title}</h3>
                {task.repeat_cron ? (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <span
                        className="mt-0.5 shrink-0 text-muted-foreground"
                        aria-label={repeatLabel ?? `Repeats: ${task.repeat_cron}`}
                      >
                        <Repeat className="size-3.5" aria-hidden />
                      </span>
                    </TooltipTrigger>
                    <TooltipContent>
                      {repeatLabel ?? task.repeat_cron}
                    </TooltipContent>
                  </Tooltip>
                ) : null}
              </div>
              <p
                className={cn(
                  "mt-1 flex items-center gap-1 text-xs",
                  overdue ? "font-medium text-red-400" : "text-muted-foreground",
                )}
              >
                {overdue ? (
                  <TriangleAlert className="size-3.5 shrink-0" aria-hidden />
                ) : null}
                <span>{formatDue(task.due_at, now)}</span>
                {overdue ? <span className="sr-only">Overdue</span> : null}
              </p>
              {task.tags.length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {task.tags.map((tag) => {
                    const keyed = parseTagParts(tag.name).kind === "keyed";
                    return (
                      <span
                        key={tag.id}
                        className={cn(
                          "rounded bg-secondary text-[10px] text-secondary-foreground",
                          keyed ? "py-0 pl-0 pr-1.5" : "px-1.5 py-0.5",
                        )}
                      >
                        <TagLabel
                          name={tag.name}
                          flushKey={keyed}
                          {...(keyed ? { className: "rounded" } : {})}
                        />
                      </span>
                    );
                  })}
                </div>
              )}
            </article>
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          {!multi ? (
            <ContextMenuItem onSelect={() => onEdit(task)}>Edit…</ContextMenuItem>
          ) : null}
          {moveTo ? (
            <ContextMenuItem onSelect={() => onUpdateStatus(targetIds, moveTo)}>
              Move to {STATUS_LABEL[moveTo]}
              {multi ? ` (${menuTargets.length})` : ""}
            </ContextMenuItem>
          ) : null}
          <ContextMenuSub>
            <ContextMenuSubTrigger>
              Update status
              {multi ? ` (${menuTargets.length})` : ""}
            </ContextMenuSubTrigger>
            <ContextMenuSubContent>
              {STATUS_ORDER.map((status) => {
                const allHave = menuTargets.every((t) => t.status === status);
                return (
                  <ContextMenuItem
                    key={status}
                    disabled={allHave}
                    onSelect={() => onUpdateStatus(targetIds, status)}
                  >
                    {STATUS_LABEL[status]}
                    {allHave ? " ✓" : ""}
                  </ContextMenuItem>
                );
              })}
            </ContextMenuSubContent>
          </ContextMenuSub>
          <ContextMenuSeparator />
          <ContextMenuSub>
            <ContextMenuSubTrigger>
              Reschedule
              {multi ? ` (${menuTargets.length})` : ""}
            </ContextMenuSubTrigger>
            <ContextMenuSubContent>
              {RESCHEDULE_PRESETS.map(({ mode, label }) => {
                const allMatch = menuTargets.every((t) =>
                  matchesDueQuickPreset(new Date(t.due_at), mode, now),
                );
                return (
                  <ContextMenuItem
                    key={mode}
                    disabled={allMatch}
                    onSelect={() => onReschedule(targetIds, mode)}
                  >
                    {label}
                    {allMatch ? " ✓" : ""}
                  </ContextMenuItem>
                );
              })}
              <ContextMenuItem
                onSelect={() => {
                  setCustomDue(
                    toDatetimeLocalValue(
                      new Date(menuTargets[0]?.due_at ?? task.due_at),
                    ),
                  );
                  setCustomOpen(true);
                }}
              >
                Custom…
              </ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuSub>
          <ContextMenuSeparator />
          <ContextMenuItem
            className="text-destructive focus:bg-destructive/15 focus:text-destructive"
            onSelect={() => setDeleteOpen(true)}
          >
            Delete
            {multi ? ` (${menuTargets.length})` : ""}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      <Dialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <DialogContent
          onOpenAutoFocus={(event) => {
            event.preventDefault();
            confirmDeleteRef.current?.focus();
          }}
        >
          <DialogHeader>
            <DialogTitle>
              {multi ? `Delete ${menuTargets.length} tasks?` : "Delete task?"}
            </DialogTitle>
            <DialogDescription>
              {multi
                ? "The selected tasks will be removed from your board."
                : `“${task.title}” will be removed from your board.`}
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => setDeleteOpen(false)}>
              Cancel
            </Button>
            <Button
              ref={confirmDeleteRef}
              type="button"
              variant="destructive"
              onClick={confirmDelete}
            >
              Delete
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {customOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div
            className="w-full max-w-sm rounded-lg border border-border bg-popover p-4 shadow-xl"
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
                event.preventDefault();
                saveCustomReschedule();
              }
            }}
          >
            <h4 className="font-display text-sm font-semibold">Reschedule</h4>
            <p className="mt-1 text-xs text-muted-foreground">
              {multi
                ? `${menuTargets.length} selected tasks`
                : task.title}
            </p>
            <Input
              className="mt-3"
              type="datetime-local"
              value={customDue}
              onChange={(e) => setCustomDue(e.target.value)}
              autoFocus
            />
            <div className="mt-4 flex justify-end gap-2">
              <Button type="button" variant="ghost" onClick={() => setCustomOpen(false)}>
                Cancel
              </Button>
              <Button type="button" className="gap-2" onClick={saveCustomReschedule}>
                <span>Save</span>
                <ShortcutKeys keys={["Ctrl", "↵"]} className="text-primary-foreground" />
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
