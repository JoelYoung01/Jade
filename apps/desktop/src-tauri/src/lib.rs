use chrono::{DateTime, Utc};
use jade_core::{
    count_tasks_with_tag, create_task, default_db_path, delete_tag, delete_task, ensure_tag,
    get_settings, list_tags, list_tasks, open_db, reschedule_task, set_lane_visibility, update_task,
    update_task_status, CreateTaskInput, Db, DueUpdate, LaneVisibility, RepeatCronUpdate,
    RescheduleMode, Settings, StatusUpdateResult, Tag, Task, TaskStatus, UpdateTaskInput,
    UpdateTaskStatusInput,
};
use serde::Deserialize;
use tauri::Manager;
use uuid::Uuid;

struct AppState {
    db: Db,
}

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let path = default_db_path().map_err(|e| e.to_string())?;
            let db = open_db(&path).map_err(|e| e.to_string())?;
            app.manage(AppState { db });
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jade");
}
