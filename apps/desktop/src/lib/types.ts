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

export type TaskEventType = "created" | "updated" | "deleted";

export type TaskEvent = {
  seq: number;
  id: string;
  task_id: string;
  event_type: TaskEventType;
  payload: unknown;
  origin: string;
  created_at: string;
};

/** Card motion driven by live sync (CLI / peers / local refresh). */
export type TaskMotion = "enter" | "exit" | "flash";

export type StatusUpdateResult = {
  task: Task;
  spawned: Task | null;
};

export type LaneVisibility = {
  inactive: boolean;
  active: boolean;
  complete: boolean;
};

export type SyncthingSettings = {
  address: string;
  api_key: string;
};

export type Settings = {
  lane_visibility: LaneVisibility;
  syncthing: SyncthingSettings;
};

export type WikiRoot = {
  id: string;
  path: string;
  label: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
};

export type WikiPage = {
  id: string;
  root_id: string;
  rel_path: string;
  content_hash: string;
  mtime: string;
  indexed_at: string;
  missing_at: string | null;
  title_cache: string | null;
  tags_cache: string[];
  date_added_cache: string | null;
  summary_cache: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
};

export type WikiFrontMatter = {
  id?: string | null;
  title?: string | null;
  tags?: string[];
  summary?: string | null;
  date?: string | null;
  date_added?: string | null;
  author?: string | null;
  url?: string | null;
  source?: string | null;
  references?: string[];
  [key: string]: unknown;
};

export type WikiPageContent = {
  page: WikiPage;
  absolute_path: string;
  content: string;
  front_matter: WikiFrontMatter | null;
  body: string;
};

export type WikiBacklink = {
  page: WikiPage;
  target_raw: string;
};

export type WikiMatchKind =
  | "body_exact"
  | "title_exact"
  | "tags_exact"
  | "path_exact"
  | "body_related"
  | "title_related"
  | "tags_related"
  | "path_related"
  | "recent";

export type WikiSearchSnippet = {
  before: string;
  matched: string;
  after: string;
};

export type WikiSearchHit = {
  page: WikiPage;
  kind: WikiMatchKind;
  reason: string;
  snippet: WikiSearchSnippet | null;
  score: number;
};

export type SyncthingFolder = {
  id: string;
  label: string;
  path: string;
  paused: boolean;
};

export type SyncthingStatus = {
  underSyncthing: boolean;
  markerDetected: boolean;
  folder: SyncthingFolder | null;
  error: string | null;
};

export type AppView = "tasks" | "wiki";

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
