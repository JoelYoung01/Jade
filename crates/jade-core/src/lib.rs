//! Jade core: task/tag/settings domain and SQLite persistence.

mod cron;
mod db;
mod error;
mod events;
mod models;
mod settings;
mod tags;
mod tasks;
mod time_helpers;

pub use cron::{next_occurrence, normalize_cron, parse_cron};
pub use db::{default_db_path, open_db, open_default_db, Db, APP_DATA_DIR_NAME};
pub use error::{Error, Result};
pub use events::list_task_events;
pub use models::{
    CreateTaskInput, DueUpdate, LaneVisibility, ListTaskEventsInput, RepeatCronUpdate,
    RescheduleMode, Settings, StatusUpdateResult, Tag, Task, TaskEvent, TaskEventType, TaskStatus,
    UpdateTaskInput, UpdateTaskStatusInput,
};
pub use settings::{get_settings, set_lane_visibility};
pub use tags::{count_tasks_with_tag, delete_tag, ensure_tag, list_tags};
pub use tasks::{
    create_task, delete_task, get_task, list_tasks, reschedule_task, update_task,
    update_task_status,
};
pub use time_helpers::{
    first_monday_next_month, next_hour_rounded, next_monday, push_to_today, push_to_tomorrow,
};
