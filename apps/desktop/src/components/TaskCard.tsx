import * as React from "react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
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
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { describeCron } from "@/lib/repeat";
import type { RescheduleMode, Task, TaskStatus } from "@/lib/types";
import { formatDue, isOverdue, isToday, toDatetimeLocalValue } from "@/lib/time";
import { cn } from "@/lib/utils";

const STATUS_ORDER: TaskStatus[] = ["inactive", "active", "complete"];

const STATUS_LABEL: Record<TaskStatus, string> = {
  inactive: "Inactive",
  active: "Active",
  complete: "Complete",
};

function nextStatus(status: TaskStatus): TaskStatus | null {
  const index = STATUS_ORDER.indexOf(status);
  if (index < 0 || index >= STATUS_ORDER.length - 1) return null;
  return STATUS_ORDER[index + 1] ?? null;
}

export type ReschedulePreset = Exclude<RescheduleMode, "custom">;

type TaskCardProps = {
  task: Task;
  onEdit: (task: Task) => void;
  onUpdateStatus: (id: string, status: TaskStatus) => void;
  onReschedule: (id: string, mode: ReschedulePreset) => void;
  onRescheduleCustom: (id: string, dueAt: string) => void;
  onDelete: (id: string) => void;
};

export function TaskCard({
  task,
  onEdit,
  onUpdateStatus,
  onReschedule,
  onRescheduleCustom,
  onDelete,
}: TaskCardProps): React.JSX.Element {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: task.id,
    data: { status: task.status },
  });
  const [customOpen, setCustomOpen] = React.useState(false);
  const [deleteOpen, setDeleteOpen] = React.useState(false);
  const [customDue, setCustomDue] = React.useState(() =>
    toDatetimeLocalValue(new Date(task.due_at)),
  );
  const confirmDeleteRef = React.useRef<HTMLButtonElement>(null);

  const style: React.CSSProperties = {
    transform: CSS.Translate.toString(transform),
    opacity: isDragging ? 0.55 : 1,
  };

  function confirmDelete(): void {
    onDelete(task.id);
    setDeleteOpen(false);
  }

  const advanceTo = nextStatus(task.status);
  const dueToday = isToday(task.due_at);
  const overdue = task.status !== "complete" && isOverdue(task.due_at);
  const repeatLabel = task.repeat_cron ? describeCron(task.repeat_cron) : null;

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <article
            ref={setNodeRef}
            style={style}
            className={cn(
              "cursor-grab rounded-md border border-transparent bg-card/70 px-3 py-2.5 transition-colors hover:border-border active:cursor-grabbing",
              isDragging && "shadow-lg ring-1 ring-primary/40",
            )}
            onDoubleClick={(event) => {
              event.stopPropagation();
              onEdit(task);
            }}
            {...listeners}
            {...attributes}
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
              <span>{formatDue(task.due_at)}</span>
              {overdue ? <span className="sr-only">Overdue</span> : null}
            </p>
            {task.tags.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1">
                {task.tags.map((tag) => (
                  <span
                    key={tag.id}
                    className="rounded bg-secondary px-1.5 py-0.5 text-[10px] tracking-wide text-secondary-foreground uppercase"
                  >
                    {tag.name}
                  </span>
                ))}
              </div>
            )}
          </article>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem onSelect={() => onEdit(task)}>Edit…</ContextMenuItem>
          {advanceTo ? (
            <ContextMenuItem onSelect={() => onUpdateStatus(task.id, advanceTo)}>
              Advance to {STATUS_LABEL[advanceTo]}
            </ContextMenuItem>
          ) : null}
          <ContextMenuSub>
            <ContextMenuSubTrigger>Update status</ContextMenuSubTrigger>
            <ContextMenuSubContent>
              {STATUS_ORDER.map((status) => (
                <ContextMenuItem
                  key={status}
                  disabled={status === task.status}
                  onSelect={() => onUpdateStatus(task.id, status)}
                >
                  {STATUS_LABEL[status]}
                  {status === task.status ? " ✓" : ""}
                </ContextMenuItem>
              ))}
            </ContextMenuSubContent>
          </ContextMenuSub>
          <ContextMenuSeparator />
          <ContextMenuSub>
            <ContextMenuSubTrigger>Reschedule</ContextMenuSubTrigger>
            <ContextMenuSubContent>
              <ContextMenuItem
                disabled={dueToday}
                onSelect={() => onReschedule(task.id, "today")}
              >
                Today
                {dueToday ? " ✓" : ""}
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => onReschedule(task.id, "tomorrow")}>
                Tomorrow
              </ContextMenuItem>
              <ContextMenuItem onSelect={() => onReschedule(task.id, "next_monday")}>
                Next Monday
              </ContextMenuItem>
              <ContextMenuItem
                onSelect={() => onReschedule(task.id, "first_monday_next_month")}
              >
                First Monday of Next Month
              </ContextMenuItem>
              <ContextMenuItem
                onSelect={() => {
                  setCustomDue(toDatetimeLocalValue(new Date(task.due_at)));
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
            <DialogTitle>Delete task?</DialogTitle>
            <DialogDescription>
              “{task.title}” will be removed from your board.
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
          <div className="w-full max-w-sm rounded-lg border border-border bg-popover p-4 shadow-xl">
            <h4 className="font-display text-sm font-semibold">Reschedule</h4>
            <p className="mt-1 text-xs text-muted-foreground">{task.title}</p>
            <Input
              className="mt-3"
              type="datetime-local"
              value={customDue}
              onChange={(e) => setCustomDue(e.target.value)}
            />
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                className="rounded-md px-3 py-1.5 text-sm text-muted-foreground hover:bg-accent"
                onClick={() => setCustomOpen(false)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground"
                onClick={() => {
                  const date = new Date(customDue);
                  if (!Number.isNaN(date.getTime())) {
                    onRescheduleCustom(task.id, date.toISOString());
                    setCustomOpen(false);
                  }
                }}
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
