import * as React from "react";
import { X } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { RepeatCronEditor } from "@/components/RepeatCronEditor";
import { Input } from "@/components/ui/input";
import { ShortcutKeys } from "@/components/ui/kbd";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  cronFromPreset,
  describeCron,
  inferPreset,
  validateCron,
} from "@/lib/repeat";
import type { RepeatPreset, Tag, Task, TaskFormValues } from "@/lib/types";
import { fromDatetimeLocalValue, nextHourRounded, toDatetimeLocalValue } from "@/lib/time";
import { tagSuggestionPool } from "@/lib/tags";
import { cn } from "@/lib/utils";

const selectClassName = cn(
  "flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-none",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
);

const PRESET_OPTIONS: { value: RepeatPreset; label: string }[] = [
  { value: "never", label: "Never" },
  { value: "daily", label: "Daily" },
  { value: "weekdays", label: "Weekdays" },
  { value: "weekly", label: "Weekly" },
  { value: "monthly", label: "Monthly" },
  { value: "yearly", label: "Yearly" },
  { value: "custom", label: "Custom" },
];

type CreateTaskDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** When set, the dialog edits this task instead of creating a new one. */
  task?: Task | null;
  existingTags: Tag[];
  recentTagNames: string[];
  onSubmit: (input: TaskFormValues) => Promise<void>;
  onCountTagUsage: (tagId: string) => Promise<number>;
  onDeleteTag: (tagId: string) => Promise<void>;
};

type TaskFormProps = {
  task?: Task | null;
  existingTags: Tag[];
  recentTagNames: string[];
  onOpenChange: (open: boolean) => void;
  onSubmit: CreateTaskDialogProps["onSubmit"];
  onCountTagUsage: CreateTaskDialogProps["onCountTagUsage"];
  onDeleteTag: CreateTaskDialogProps["onDeleteTag"];
};

type PendingTagDelete = {
  tag: Tag;
  taskCount: number;
};

function findTagByName(tags: Tag[], name: string): Tag | undefined {
  const needle = name.trim().toLowerCase();
  return tags.find((tag) => tag.name.toLowerCase() === needle);
}

type TagChipProps = {
  name: string;
  variant: "selected" | "suggestion";
  onAdd?: () => void;
  onRemove?: () => void;
  onRequestDelete?: () => void;
};

function TagChip({
  name,
  variant,
  onAdd,
  onRemove,
  onRequestDelete,
}: TagChipProps): React.JSX.Element {
  const chip = (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-xs",
        variant === "selected"
          ? "bg-accent text-accent-foreground"
          : "border border-border text-muted-foreground hover:bg-accent hover:text-accent-foreground",
      )}
    >
      {variant === "suggestion" ? (
        <button type="button" onClick={onAdd}>
          {name}
        </button>
      ) : (
        <>
          {name}
          {onRemove && (
            <button
              type="button"
              className="opacity-70 hover:opacity-100"
              onClick={onRemove}
              aria-label={`Remove ${name}`}
            >
              <X className="size-3" />
            </button>
          )}
        </>
      )}
    </span>
  );

  if (!onRequestDelete) {
    return chip;
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{chip}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem
          className="text-destructive focus:bg-destructive/15 focus:text-destructive"
          onSelect={() => onRequestDelete()}
        >
          Delete tag
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

function TaskForm({
  task,
  existingTags,
  recentTagNames,
  onOpenChange,
  onSubmit,
  onCountTagUsage,
  onDeleteTag,
}: TaskFormProps): React.JSX.Element {
  const isEdit = task != null;
  const titleRef = React.useRef<HTMLInputElement>(null);
  const confirmDeleteRef = React.useRef<HTMLButtonElement>(null);
  const [title, setTitle] = React.useState(task?.title ?? "");
  const [description, setDescription] = React.useState(task?.description ?? "");
  const [dueLocal, setDueLocal] = React.useState(() =>
    toDatetimeLocalValue(task ? new Date(task.due_at) : nextHourRounded()),
  );
  const [tagNames, setTagNames] = React.useState<string[]>(() =>
    task ? task.tags.map((tag) => tag.name) : [],
  );
  const [tagDraft, setTagDraft] = React.useState("");
  const [repeatPreset, setRepeatPreset] = React.useState<RepeatPreset>(() =>
    inferPreset(task?.repeat_cron ?? null, task ? new Date(task.due_at) : nextHourRounded()),
  );
  const [repeatCron, setRepeatCron] = React.useState<string>(task?.repeat_cron ?? "");
  const [error, setError] = React.useState<string | null>(null);
  const [saving, setSaving] = React.useState(false);
  const [pendingDelete, setPendingDelete] = React.useState<PendingTagDelete | null>(null);

  function dueFromLocal(): Date | null {
    try {
      return fromDatetimeLocalValue(dueLocal);
    } catch {
      return null;
    }
  }

  function handleDueChange(value: string): void {
    setDueLocal(value);
    if (repeatPreset === "never" || repeatPreset === "custom") return;
    try {
      const due = fromDatetimeLocalValue(value);
      setRepeatCron(cronFromPreset(repeatPreset, due));
    } catch {
      // ignore invalid intermediate datetime-local values
    }
  }

  function handlePresetChange(preset: RepeatPreset): void {
    setRepeatPreset(preset);
    if (preset === "never") {
      setRepeatCron("");
      return;
    }
    const due = dueFromLocal() ?? nextHourRounded();
    if (preset === "custom") {
      const seed =
        repeatCron.trim().length > 0 ? repeatCron.trim() : cronFromPreset("daily", due);
      setRepeatCron(seed);
      return;
    }
    setRepeatCron(cronFromPreset(preset, due));
  }

  React.useEffect(() => {
    const id = window.setTimeout(() => titleRef.current?.focus(), 0);
    return () => window.clearTimeout(id);
  }, []);

  const pool = tagSuggestionPool(recentTagNames, existingTags);
  const draft = tagDraft.trim().toLowerCase();
  const suggestions = pool
    .filter((name) => !tagNames.some((t) => t.toLowerCase() === name.toLowerCase()))
    .filter((name) => (draft.length === 0 ? true : name.toLowerCase().includes(draft)))
    .slice(0, draft.length === 0 ? 6 : 8);

  function addTag(name: string): void {
    const trimmed = name.trim();
    if (!trimmed) return;
    if (tagNames.some((t) => t.toLowerCase() === trimmed.toLowerCase())) {
      setTagDraft("");
      return;
    }
    setTagNames((prev) => [...prev, trimmed]);
    setTagDraft("");
  }

  async function removeTagEntity(tag: Tag): Promise<void> {
    await onDeleteTag(tag.id);
    setTagNames((prev) => prev.filter((name) => name.toLowerCase() !== tag.name.toLowerCase()));
    setPendingDelete(null);
  }

  async function handleRequestDeleteTag(name: string): Promise<void> {
    const tag = findTagByName(existingTags, name);
    if (!tag) {
      setTagNames((prev) => prev.filter((n) => n.toLowerCase() !== name.trim().toLowerCase()));
      return;
    }

    try {
      const taskCount = await onCountTagUsage(tag.id);
      if (taskCount === 0) {
        await removeTagEntity(tag);
        return;
      }
      setPendingDelete({ tag, taskCount });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function confirmPendingDelete(): Promise<void> {
    if (!pendingDelete) return;
    try {
      await removeTagEntity(pendingDelete.tag);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleSubmit(event?: React.FormEvent): Promise<void> {
    event?.preventDefault();
    if (!title.trim()) {
      setError("Title is required");
      return;
    }
    let due: Date;
    try {
      due = fromDatetimeLocalValue(dueLocal);
    } catch {
      setError("Due date and time are required");
      return;
    }

    let nextCron: string | null = null;
    if (repeatPreset !== "never") {
      const trimmed = repeatCron.trim();
      const validation = validateCron(trimmed);
      if (!validation.ok) {
        setError(validation.error);
        return;
      }
      nextCron = trimmed;
    }

    setSaving(true);
    setError(null);
    try {
      const payload: TaskFormValues = {
        title: title.trim(),
        due_at: due.toISOString(),
        tag_names: tagNames,
        repeat_cron: nextCron,
      };
      if (isEdit) {
        payload.description = description.trim();
      } else if (description.trim()) {
        payload.description = description.trim();
      }
      await onSubmit(payload);
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <DialogContent
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
            event.preventDefault();
            void handleSubmit();
          }
        }}
      >
        <DialogHeader>
          <DialogTitle>{isEdit ? "Edit task" : "New task"}</DialogTitle>
        </DialogHeader>

        <form className="grid gap-4" onSubmit={(e) => void handleSubmit(e)}>
          <div className="grid gap-2">
            <Label htmlFor="task-title">Title</Label>
            <Input
              id="task-title"
              ref={titleRef}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="What needs doing?"
              required
            />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="task-description">Description</Label>
            <Textarea
              id="task-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional notes"
            />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="task-due">Due</Label>
            <Input
              id="task-due"
              type="datetime-local"
              value={dueLocal}
              onChange={(e) => handleDueChange(e.target.value)}
              required
            />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="task-repeat">Repeat</Label>
            <select
              id="task-repeat"
              className={selectClassName}
              value={repeatPreset}
              onChange={(e) => handlePresetChange(e.target.value as RepeatPreset)}
            >
              {PRESET_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
            {repeatPreset !== "never" && repeatPreset !== "custom" && (
              <div className="grid gap-1 text-xs text-muted-foreground">
                <code className="rounded bg-muted px-1.5 py-1 font-mono text-[11px] text-foreground/80">
                  {repeatCron}
                </code>
                {describeCron(repeatCron) && <p>{describeCron(repeatCron)}</p>}
              </div>
            )}
            {repeatPreset === "custom" && (
              <RepeatCronEditor
                value={repeatCron}
                dueLocal={dueLocal}
                onChange={setRepeatCron}
              />
            )}
          </div>

          <div className="grid gap-2">
            <Label htmlFor="task-tags">Tags</Label>
            <div className="flex flex-wrap gap-1.5">
              {tagNames.map((name) => (
                <TagChip
                  key={name}
                  name={name}
                  variant="selected"
                  onRemove={() => setTagNames((prev) => prev.filter((t) => t !== name))}
                  onRequestDelete={() => void handleRequestDeleteTag(name)}
                />
              ))}
            </div>
            <Input
              id="task-tags"
              value={tagDraft}
              onChange={(e) => setTagDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === ",") {
                  e.preventDefault();
                  addTag(tagDraft);
                }
              }}
              placeholder="Type a tag, or pick a recent one"
            />
            {suggestions.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {suggestions.map((name) => {
                  const known = findTagByName(existingTags, name);
                  return (
                    <TagChip
                      key={name}
                      name={name}
                      variant="suggestion"
                      onAdd={() => addTag(name)}
                      {...(known
                        ? { onRequestDelete: () => void handleRequestDeleteTag(name) }
                        : {})}
                    />
                  );
                })}
              </div>
            )}
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}

          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving} className="gap-2">
              <span>{saving ? "Saving…" : "Save"}</span>
              {!saving && <ShortcutKeys keys={["Ctrl", "↵"]} className="text-primary-foreground" />}
            </Button>
          </div>
        </form>
      </DialogContent>

      <Dialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) setPendingDelete(null);
        }}
      >
        <DialogContent
          onOpenAutoFocus={(event) => {
            event.preventDefault();
            confirmDeleteRef.current?.focus();
          }}
        >
          <DialogHeader>
            <DialogTitle>Delete tag?</DialogTitle>
            <DialogDescription>
              {pendingDelete
                ? `“${pendingDelete.tag.name}” is used by ${pendingDelete.taskCount} task${pendingDelete.taskCount === 1 ? "" : "s"}. Deleting it removes the tag from those tasks.`
                : null}
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => setPendingDelete(null)}>
              Cancel
            </Button>
            <Button
              ref={confirmDeleteRef}
              type="button"
              variant="destructive"
              onClick={() => void confirmPendingDelete()}
            >
              Delete tag
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

export function CreateTaskDialog({
  open,
  onOpenChange,
  task = null,
  existingTags,
  recentTagNames,
  onSubmit,
  onCountTagUsage,
  onDeleteTag,
}: CreateTaskDialogProps): React.JSX.Element {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {open ? (
        <TaskForm
          key={task?.id ?? "create"}
          task={task}
          existingTags={existingTags}
          recentTagNames={recentTagNames}
          onOpenChange={onOpenChange}
          onSubmit={onSubmit}
          onCountTagUsage={onCountTagUsage}
          onDeleteTag={onDeleteTag}
        />
      ) : null}
    </Dialog>
  );
}
