export type TaskStatus = "inactive" | "active" | "complete";

export type Tag = {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
};

export type Task = {
  id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  due_at: string;
  /** 5-field POSIX cron; present only on the live occurrence of a series. */
  repeat_cron: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
  tags: Tag[];
};

export type StatusUpdateResult = {
  task: Task;
  spawned: Task | null;
};

export type LaneVisibility = {
  inactive: boolean;
  active: boolean;
  complete: boolean;
};

export type Settings = {
  lane_visibility: LaneVisibility;
};

export type RescheduleMode =
  | "today"
  | "tomorrow"
  | "next_monday"
  | "first_monday_next_month"
  | "custom";

export type CreateTaskInput = {
  title: string;
  description?: string;
  due_at: string;
  tag_names: string[];
  repeat_cron?: string | null;
};

export type UpdateTaskInput = {
  id: string;
  title: string;
  /** Pass empty string to clear. */
  description: string;
  due_at: string;
  tag_names: string[];
  /** `null` clears the schedule. */
  repeat_cron: string | null;
};

export type TaskFormValues = {
  title: string;
  description?: string;
  due_at: string;
  tag_names: string[];
  /** `null` / omit = never repeats. */
  repeat_cron?: string | null;
};

export type RepeatPreset =
  | "never"
  | "daily"
  | "weekdays"
  | "weekly"
  | "monthly"
  | "yearly"
  | "custom";
