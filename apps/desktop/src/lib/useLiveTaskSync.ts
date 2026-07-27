import * as React from "react";

import {
  apiLatestEventSeq,
  apiListTaskEventsSince,
  apiSubscribeDbChanged,
} from "@/lib/api";
import type { Task, TaskEvent, TaskMotion } from "@/lib/types";

const ENTER_FLASH_MS = 650;
const EXIT_MS = 280;

export type TaskMotionMap = ReadonlyMap<string, TaskMotion>;

type LiveSyncOptions = {
  /** Current board tasks (used to keep soft-deleted cards for exit animation). */
  tasks: Task[];
  /** Reload tasks/tags/settings from the source of truth. */
  refresh: () => Promise<void>;
  onError?: (message: string) => void;
};

type LiveSyncResult = {
  /** Tasks to render (includes briefly-held exiting cards). */
  displayTasks: Task[];
  motionById: TaskMotionMap;
  /** Call after a successful local refresh so the watcher won't re-play those events. */
  markSynced: () => Promise<void>;
};

/**
 * Keeps the board in sync with external writers (CLI, future peers) by listening
 * for `db-changed`, reading `task_events` since a seq cursor, refreshing state,
 * and tagging cards for enter/exit/flash motion.
 */
export function useLiveTaskSync({
  tasks,
  refresh,
  onError,
}: LiveSyncOptions): LiveSyncResult {
  const tasksRef = React.useRef(tasks);
  tasksRef.current = tasks;

  const lastSeqRef = React.useRef(0);
  const syncingRef = React.useRef(false);
  const pendingRef = React.useRef(false);

  const [exitTasks, setExitTasks] = React.useState<Task[]>([]);
  const [motionById, setMotionById] = React.useState<TaskMotionMap>(() => new Map());

  const clearMotionTimers = React.useRef<number[]>([]);
  React.useEffect(() => {
    return () => {
      for (const id of clearMotionTimers.current) window.clearTimeout(id);
    };
  }, []);

  const applyMotion = React.useCallback((events: TaskEvent[], previous: Task[]) => {
    if (events.length === 0) return;

    const deleted = new Set<string>();
    const created = new Set<string>();
    const updated = new Set<string>();
    for (const event of events) {
      if (event.event_type === "deleted") deleted.add(event.task_id);
      else if (event.event_type === "created") created.add(event.task_id);
      else if (event.event_type === "updated") updated.add(event.task_id);
    }

    // Prefer exit over flash when both appear in one batch.
    for (const id of deleted) {
      created.delete(id);
      updated.delete(id);
    }
    for (const id of created) updated.delete(id);

    const exiting = previous.filter((task) => deleted.has(task.id));
    if (exiting.length > 0) {
      setExitTasks(exiting);
    }

    const nextMotion = new Map<string, TaskMotion>();
    for (const id of created) nextMotion.set(id, "enter");
    for (const id of updated) nextMotion.set(id, "flash");
    for (const id of deleted) nextMotion.set(id, "exit");
    setMotionById(nextMotion);

    for (const id of clearMotionTimers.current) window.clearTimeout(id);
    clearMotionTimers.current = [];

    if (created.size > 0 || updated.size > 0) {
      clearMotionTimers.current.push(
        window.setTimeout(() => {
          setMotionById((prev) => {
            const copy = new Map(prev);
            for (const id of created) {
              if (copy.get(id) === "enter") copy.delete(id);
            }
            for (const id of updated) {
              if (copy.get(id) === "flash") copy.delete(id);
            }
            return copy;
          });
        }, ENTER_FLASH_MS),
      );
    }

    if (exiting.length > 0) {
      clearMotionTimers.current.push(
        window.setTimeout(() => {
          setExitTasks([]);
          setMotionById((prev) => {
            const copy = new Map(prev);
            for (const task of exiting) {
              if (copy.get(task.id) === "exit") copy.delete(task.id);
            }
            return copy;
          });
        }, EXIT_MS),
      );
    }
  }, []);

  const markSynced = React.useCallback(async () => {
    try {
      lastSeqRef.current = await apiLatestEventSeq();
    } catch {
      // ignore — next sync will recover
    }
  }, []);

  const syncFromDb = React.useCallback(async () => {
    if (syncingRef.current) {
      pendingRef.current = true;
      return;
    }
    syncingRef.current = true;
    try {
      do {
        pendingRef.current = false;
        const afterSeq = lastSeqRef.current;
        const events = await apiListTaskEventsSince(afterSeq);
        const previous = tasksRef.current;
        applyMotion(events, previous);
        await refresh();
        const latest = await apiLatestEventSeq();
        const fromEvents =
          events.length > 0 ? Math.max(...events.map((event) => event.seq)) : afterSeq;
        lastSeqRef.current = Math.max(latest, fromEvents);
      } while (pendingRef.current);
    } catch (err) {
      onError?.(err instanceof Error ? err.message : String(err));
    } finally {
      syncingRef.current = false;
    }
  }, [applyMotion, onError, refresh]);

  React.useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let ready = false;

    void (async () => {
      try {
        const seq = await apiLatestEventSeq();
        if (cancelled) return;
        lastSeqRef.current = seq;
        ready = true;
        unlisten = await apiSubscribeDbChanged(() => {
          if (!ready) return;
          void syncFromDb();
        });
      } catch (err) {
        if (!cancelled) {
          onError?.(err instanceof Error ? err.message : String(err));
        }
      }
    })();

    return () => {
      cancelled = true;
      ready = false;
      unlisten?.();
    };
  }, [onError, syncFromDb]);

  const displayTasks = React.useMemo(() => {
    if (exitTasks.length === 0) return tasks;
    const byId = new Map(tasks.map((task) => [task.id, task]));
    for (const task of exitTasks) {
      if (!byId.has(task.id)) byId.set(task.id, task);
    }
    return [...byId.values()];
  }, [tasks, exitTasks]);

  return { displayTasks, motionById, markSynced };
}
