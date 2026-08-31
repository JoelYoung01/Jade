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
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  cronFromPreset,
  describeCron,
  inferPreset,
  validateCron,
} from "@/lib/repeat";
import type { RepeatPreset, Tag, Task, TaskFormValues } from "@/lib/types";
import {
  applyDuePreset,
  formatAbsoluteDateTime,
  formatDue,
  fromDatetimeLocalValue,
  matchesDueQuickPreset,
  nextHourRounded,
  toDatetimeLocalValue,
  type DueQuickPreset,
} from "@/lib/time";
import { TagLabel } from "@/components/TagLabel";
import { bestTagAutocomplete, parseTagParts, tagSuggestionPool } from "@/lib/tags";
import { cn } from "@/lib/utils";

const selectClassName = cn(
  "flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-none",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
);

const DUE_QUICK_PRESETS: { mode: DueQuickPreset; label: string }[] = [
  { mode: "today", label: "Today" },
  { mode: "tomorrow", label: "Tomorrow" },
  { mode: "next_monday", label: "Next Monday" },
  { mode: "first_monday_next_month", label: "First Monday next month" },
];

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

const chipButtonClassName =
  "rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring";

type TagChipProps = {
  name: string;
  variant: "selected" | "suggestion";
  highlighted?: boolean;
  chipRef?: React.Ref<HTMLSpanElement>;
  onAdd?: () => void;
  onRemove?: () => void;
  onRequestDelete?: () => void;
};

function TagChip({
  name,
  variant,
  highlighted = false,
  chipRef,
  onAdd,
  onRemove,
  onRequestDelete,
}: TagChipProps): React.JSX.Element {
  const keyed = parseTagParts(name).kind === "keyed";
  const chip = (
    <span
      ref={chipRef}
      className={cn(
        "inline-flex items-center gap-1 rounded-md text-xs transition-colors",
        keyed ? "py-0 pl-0 pr-2" : "px-2 py-0.5",
        variant === "selected"
          ? "bg-accent text-accent-foreground"
          : highlighted
            ? "border border-primary bg-accent text-accent-foreground ring-1 ring-ring"
            : "border border-border text-muted-foreground hover:bg-accent hover:text-accent-foreground",
      )}
    >
      {variant === "suggestion" ? (
        <button
          type="button"
          className={cn(chipButtonClassName, keyed && "min-w-0")}
          onClick={onAdd}
          aria-label={`Add tag ${name}`}
        >
          <TagLabel
            name={name}
            flushKey={keyed}
            {...(keyed ? { className: "rounded-md" } : {})}
          />
        </button>
      ) : (
        <>
          <TagLabel
            name={name}
            flushKey={keyed}
            {...(keyed ? { className: "rounded-md" } : {})}
          />
          {onRemove && (
            <button
              type="button"
              className={cn(chipButtonClassName, "opacity-70 hover:opacity-100")}
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

type DueQuickPresetButtonsProps = {
  dueDate: Date | null;
  onSelect: (mode: DueQuickPreset) => void;
};

/** Exclusive quick-due chips with radiogroup keyboard behavior (no disable-on-select). */
function DueQuickPresetButtons({
  dueDate,
  onSelect,
}: DueQuickPresetButtonsProps): React.JSX.Element {
  const buttonRefs = React.useRef<Array<HTMLButtonElement | null>>([]);
  const selectedIndex = DUE_QUICK_PRESETS.findIndex(
    (preset) => dueDate != null && matchesDueQuickPreset(dueDate, preset.mode),
  );
  const [focusIndex, setFocusIndex] = React.useState(() =>
    selectedIndex >= 0 ? selectedIndex : 0,
  );

  React.useEffect(() => {
    if (selectedIndex >= 0) setFocusIndex(selectedIndex);
  }, [selectedIndex]);

  function moveFocus(nextIndex: number): void {
    setFocusIndex(nextIndex);
    buttonRefs.current[nextIndex]?.focus();
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLButtonElement>, index: number): void {
    const last = DUE_QUICK_PRESETS.length - 1;
    let next: number | null = null;
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown":
        next = index === last ? 0 : index + 1;
        break;
      case "ArrowLeft":
      case "ArrowUp":
        next = index === 0 ? last : index - 1;
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = last;
        break;
      default:
        return;
    }
    event.preventDefault();
    moveFocus(next);
  }

  function selectPreset(mode: DueQuickPreset, index: number): void {
    onSelect(mode);
    setFocusIndex(index);
    // Keep focus on the chip after selection (do not disable selected chips).
    requestAnimationFrame(() => buttonRefs.current[index]?.focus());
  }

  return (
    <div role="radiogroup" aria-label="Quick due date" className="flex flex-wrap gap-1.5">
      {DUE_QUICK_PRESETS.map((preset, index) => {
        const selected =
          dueDate != null && matchesDueQuickPreset(dueDate, preset.mode);
        return (
          <Button
            key={preset.mode}
            ref={(node) => {
              buttonRefs.current[index] = node;
            }}
            type="button"
            role="radio"
            variant="outline"
            size="sm"
            className={cn(
              "h-7 px-2 text-[11px] font-normal",
              selected
                ? "border-primary bg-accent text-accent-foreground"
                : "text-muted-foreground",
            )}
            aria-checked={selected}
            tabIndex={focusIndex === index ? 0 : -1}
            onKeyDown={(event) => handleKeyDown(event, index)}
            onClick={() => selectPreset(preset.mode, index)}
            onFocus={() => setFocusIndex(index)}
          >
            {preset.label}
            {selected ? (
              <span aria-hidden="true"> ✓</span>
            ) : null}
          </Button>
        );
      })}
    </div>
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
  const dueRef = React.useRef<HTMLInputElement>(null);
  const repeatRef = React.useRef<HTMLSelectElement>(null);
  const confirmDeleteRef = React.useRef<HTMLButtonElement>(null);
  const suggestionChipEls = React.useRef(new Map<string, HTMLSpanElement>());
  const selectedChipEls = React.useRef(new Map<string, HTMLSpanElement>());
  const pendingFlipFrom = React.useRef<DOMRect | null>(null);
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

  function applyDueQuickPreset(mode: DueQuickPreset): void {
    let current: Date;
    try {
      current = fromDatetimeLocalValue(dueLocal);
    } catch {
      current = nextHourRounded();
    }
    handleDueChange(
      toDatetimeLocalValue(
        applyDuePreset(current, mode, new Date(), { defaultAfternoonIfNotToday: true }),
      ),
    );
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
  const tabTarget = draft.length > 0 ? bestTagAutocomplete(tagDraft, pool, tagNames) : null;
  const filteredSuggestions = pool
    .filter((name) => !tagNames.some((t) => t.toLowerCase() === name.toLowerCase()))
    .filter((name) => (draft.length === 0 ? true : name.toLowerCase().includes(draft)));
  const suggestions = (() => {
    const limit = draft.length === 0 ? 6 : 8;
    if (!tabTarget) return filteredSuggestions.slice(0, limit);
    const rest = filteredSuggestions.filter(
      (name) => name.toLowerCase() !== tabTarget.toLowerCase(),
    );
    return [tabTarget, ...rest].slice(0, limit);
  })();
  const dueDate = dueFromLocal();

  React.useLayoutEffect(() => {
    const fromRect = pendingFlipFrom.current;
    if (!fromRect) return;
    pendingFlipFrom.current = null;

    const added = tagNames[tagNames.length - 1];
    if (!added) return;
    const toEl = selectedChipEls.current.get(added.toLowerCase());
    if (!toEl) return;

    const toRect = toEl.getBoundingClientRect();
    const dx = fromRect.left - toRect.left;
    const dy = fromRect.top - toRect.top;
    toEl.animate(
      [
        { transform: `translate(${dx}px, ${dy}px) scale(0.96)`, opacity: 0.7 },
        { transform: "translate(0, 0) scale(1)", opacity: 1 },
      ],
      { duration: 240, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
    );
  }, [tagNames]);

  function addTag(name: string): void {
    const trimmed = name.trim();
    if (!trimmed) return;
    if (tagNames.some((t) => t.toLowerCase() === trimmed.toLowerCase())) {
      setTagDraft("");
      return;
    }

    const fromEl = suggestionChipEls.current.get(trimmed.toLowerCase());
    pendingFlipFrom.current = fromEl?.getBoundingClientRect() ?? null;

    setTagNames((prev) => [...prev, trimmed]);
    setTagDraft("");
  }

  function setChipRef(
    map: React.MutableRefObject<Map<string, HTMLSpanElement>>,
    name: string,
  ): (node: HTMLSpanElement | null) => void {
    const key = name.toLowerCase();
    return (node) => {
      if (node) map.current.set(key, node);
      else map.current.delete(key);
    };
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
      titleRef.current?.focus();
      return;
    }
    let due: Date;
    try {
      due = fromDatetimeLocalValue(dueLocal);
    } catch {
      setError("Due date and time are required");
      dueRef.current?.focus();
      return;
    }

    let nextCron: string | null = null;
    if (repeatPreset !== "never") {
      const trimmed = repeatCron.trim();
      const validation = validateCron(trimmed);
      if (!validation.ok) {
        setError(validation.error);
        if (repeatPreset === "custom") {
          document.getElementById("repeat-cron-raw")?.focus();
        } else {
          repeatRef.current?.focus();
        }
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
              ref={dueRef}
              type="datetime-local"
              value={dueLocal}
              onChange={(e) => handleDueChange(e.target.value)}
              required
            />
            <DueQuickPresetButtons dueDate={dueDate} onSelect={applyDueQuickPreset} />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="task-repeat">Repeat</Label>
            <select
              id="task-repeat"
              ref={repeatRef}
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
            {tagNames.length > 0 && (
              <div className="flex flex-wrap gap-1.5" aria-label="Selected tags">
                {tagNames.map((name) => (
                  <TagChip
                    key={name}
                    name={name}
                    variant="selected"
                    chipRef={setChipRef(selectedChipEls, name)}
                    onRemove={() => setTagNames((prev) => prev.filter((t) => t !== name))}
                    onRequestDelete={() => void handleRequestDeleteTag(name)}
                  />
                ))}
              </div>
            )}
            <Input
              id="task-tags"
              value={tagDraft}
              onChange={(e) => setTagDraft(e.target.value)}
              onKeyDown={(e) => {
                if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z" && !e.shiftKey) {
                  if (tagNames.length === 0) return;
                  e.preventDefault();
                  setTagNames((prev) => prev.slice(0, -1));
                  return;
                }
                if (e.key === "Tab") {
                  if (!tabTarget) return;
                  e.preventDefault();
                  addTag(tabTarget);
                  return;
                }
                if (e.key === "Enter" || e.key === ",") {
                  e.preventDefault();
                  addTag(tagDraft);
                }
              }}
              placeholder="Type a tag, Tab to add"
              aria-describedby={
                suggestions.length > 0 ? "task-tag-suggestions" : undefined
              }
              autoComplete="off"
            />
            {suggestions.length > 0 && (
              <div
                id="task-tag-suggestions"
                className="flex flex-wrap gap-1"
                role="group"
                aria-label="Tag suggestions"
              >
                {suggestions.map((name) => {
                  const known = findTagByName(existingTags, name);
                  const highlighted =
                    tabTarget != null && name.toLowerCase() === tabTarget.toLowerCase();
                  return (
                    <TagChip
                      key={name}
                      name={name}
                      variant="suggestion"
                      highlighted={highlighted}
                      chipRef={setChipRef(suggestionChipEls, name)}
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

          {error && (
            <p className="text-sm text-destructive" role="alert">
              {error}
            </p>
          )}

          <div className="flex items-center gap-2">
            {isEdit && task?.created_at ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="cursor-default text-xs text-muted-foreground">
                    Created {formatDue(task.created_at)}
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top">
                  {formatAbsoluteDateTime(task.created_at)}
                </TooltipContent>
              </Tooltip>
            ) : null}
            <div className="ml-auto flex gap-2">
              <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={saving} className="gap-2">
                <span>{saving ? "Saving…" : "Save"}</span>
                {!saving && <ShortcutKeys keys={["Ctrl", "↵"]} className="text-primary-foreground" />}
              </Button>
            </div>
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
