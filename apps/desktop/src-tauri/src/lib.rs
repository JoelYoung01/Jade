use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use jade_core::{
    count_tasks_with_tag, create_task, data_version, default_db_path, delete_tag, delete_task,
    ensure_tag, get_settings, latest_event_seq, list_tags, list_task_events_since, list_tasks,
    open_db, reschedule_task, set_lane_visibility, update_task, update_task_status, CreateTaskInput,
    Db, DueUpdate, LaneVisibility, ListTaskEventsSinceInput, RepeatCronUpdate, RescheduleMode,
    Settings, StatusUpdateResult, Tag, Task, TaskEvent, TaskStatus, UpdateTaskInput,
    UpdateTaskStatusInput,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

struct AppState {
    db: Db,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DbChangedPayload {
    data_version: i64,
}

const DB_CHANGED_EVENT: &str = "db-changed";
const DATA_VERSION_POLL_MS: u64 = 350;

#[tauri::command]
fn list_tasks_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<Task>, String> {
    list_tasks(&state.db).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct CreateTaskArgs {
    title: String,
    description: Option<String>,
    due_at: DateTime<Utc>,
    tag_names: Vec<String>,
    repeat_cron: Option<String>,
}

#[tauri::command]
fn create_task_cmd(
    state: tauri::State<'_, AppState>,
    args: CreateTaskArgs,
) -> Result<Task, String> {
    create_task(
        &state.db,
        CreateTaskInput {
            title: args.title,
            description: args.description,
            due_at: args.due_at,
            tag_names: args.tag_names,
            repeat_cron: args.repeat_cron,
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct UpdateStatusArgs {
    id: Uuid,
    status: TaskStatus,
}

#[tauri::command]
fn update_task_status_cmd(
    state: tauri::State<'_, AppState>,
    args: UpdateStatusArgs,
) -> Result<StatusUpdateResult, String> {
    update_task_status(
        &state.db,
        UpdateTaskStatusInput {
            id: args.id,
            status: args.status,
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct UpdateTaskArgs {
    id: Uuid,
    title: Option<String>,
    description: Option<String>,
    due_at: Option<DateTime<Utc>>,
    tag_names: Option<Vec<String>>,
    /// Desktop form always sends this: `null`/empty clears, string sets.
    repeat_cron: Option<String>,
}

#[tauri::command]
fn update_task_cmd(
    state: tauri::State<'_, AppState>,
    args: UpdateTaskArgs,
) -> Result<StatusUpdateResult, String> {
    let repeat_cron = Some(match args.repeat_cron {
        None => RepeatCronUpdate::Clear,
        Some(expr) if expr.trim().is_empty() => RepeatCronUpdate::Clear,
        Some(expr) => RepeatCronUpdate::Set(expr),
    });

    update_task(
        &state.db,
        UpdateTaskInput {
            id: args.id,
            title: args.title,
            description: args.description,
            status: None,
            due: args.due_at.map(DueUpdate::At),
            tag_names: args.tag_names,
            repeat_cron,
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct RescheduleArgs {
    id: Uuid,
    mode: RescheduleMode,
    due_at: Option<DateTime<Utc>>,
}

#[tauri::command]
fn reschedule_task_cmd(
    state: tauri::State<'_, AppState>,
    args: RescheduleArgs,
) -> Result<Task, String> {
    reschedule_task(&state.db, args.id, args.mode, args.due_at).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task_cmd(state: tauri::State<'_, AppState>, id: Uuid) -> Result<(), String> {
    delete_task(&state.db, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tags_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<Tag>, String> {
    list_tags(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
fn ensure_tag_cmd(state: tauri::State<'_, AppState>, name: String) -> Result<Tag, String> {
    ensure_tag(&state.db, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn count_tasks_with_tag_cmd(state: tauri::State<'_, AppState>, id: Uuid) -> Result<u64, String> {
    count_tasks_with_tag(&state.db, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_tag_cmd(state: tauri::State<'_, AppState>, id: Uuid) -> Result<(), String> {
    delete_tag(&state.db, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings_cmd(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    get_settings(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_lane_visibility_cmd(
    state: tauri::State<'_, AppState>,
    visibility: LaneVisibility,
) -> Result<Settings, String> {
    set_lane_visibility(&state.db, visibility).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct ListEventsSinceArgs {
    after_seq: i64,
    limit: Option<u32>,
}

#[tauri::command]
fn list_task_events_since_cmd(
    state: tauri::State<'_, AppState>,
    args: ListEventsSinceArgs,
) -> Result<Vec<TaskEvent>, String> {
    list_task_events_since(
        &state.db,
        ListTaskEventsSinceInput {
            after_seq: args.after_seq,
            limit: args.limit,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn latest_event_seq_cmd(state: tauri::State<'_, AppState>) -> Result<i64, String> {
    latest_event_seq(&state.db).map_err(|e| e.to_string())
}

fn spawn_data_version_watcher(app: AppHandle, running: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut last: Option<i64> = None;
        while running.load(Ordering::Relaxed) {
            let version = {
                let state = app.state::<AppState>();
                data_version(&state.db)
            };
            match version {
                Ok(version) => {
                    if last.is_some_and(|prev| prev != version) {
                        let _ = app.emit(
                            DB_CHANGED_EVENT,
                            DbChangedPayload {
                                data_version: version,
                            },
                        );
                    }
                    last = Some(version);
                }
                Err(_) => {
                    // Transient lock / busy — try again next tick.
                }
            }
            thread::sleep(Duration::from_millis(DATA_VERSION_POLL_MS));
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let watcher_running = Arc::new(AtomicBool::new(true));
    let watcher_flag = Arc::clone(&watcher_running);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let path = default_db_path().map_err(|e| e.to_string())?;
            let db = open_db(&path).map_err(|e| e.to_string())?;
            app.manage(AppState { db });
            spawn_data_version_watcher(app.handle().clone(), watcher_flag);
            Ok(())
        })
        .on_window_event(move |_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                watcher_running.store(false, Ordering::Relaxed);
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_tasks_cmd,
            create_task_cmd,
            update_task_status_cmd,
            update_task_cmd,
            reschedule_task_cmd,
            delete_task_cmd,
            list_tags_cmd,
            ensure_tag_cmd,
            count_tasks_with_tag_cmd,
            delete_tag_cmd,
            get_settings_cmd,
            set_lane_visibility_cmd,
            list_task_events_since_cmd,
            latest_event_seq_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jade");
}
