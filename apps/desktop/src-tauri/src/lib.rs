mod install_context;
mod wiki_watch;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use install_context::{
    detect_install_context, fetch_aur_package_info, latest_appimage_download_url,
    open_aur_update_in_konsole, AurPackageInfo, InstallContext,
};

use chrono::{DateTime, Utc};
use jade_core::{
    add_wiki_root, count_tasks_with_tag, create_task, create_wiki_page, data_version,
    default_db_path, delete_tag, delete_task, ensure_tag, get_settings, get_wiki_page,
    latest_event_seq, list_backlinks, list_tags, list_task_events_since, list_tasks,
    list_wiki_pages, list_wiki_roots, open_db, read_wiki_page, reindex_all, reindex_root,
    remove_wiki_root, reschedule_task, search_wiki_pages, set_lane_visibility,
    set_syncthing_settings, status_for_path, update_task, update_task_status, write_wiki_page,
    AddWikiRootInput, CreateTaskInput, CreateWikiPageInput, Db, DueUpdate, LaneVisibility,
    ListTaskEventsSinceInput, RepeatCronUpdate, RescheduleMode, Settings, StatusUpdateResult,
    SyncthingClientConfig, SyncthingSettings, SyncthingStatus, Tag, Task, TaskEvent, TaskStatus,
    UpdateTaskInput, UpdateTaskStatusInput, WikiBacklink, WikiPage, WikiPageContent, WikiRoot,
    WikiSearchHit, WriteWikiPageInput,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use wiki_watch::{spawn_wiki_watcher, WikiWatchState};

pub(crate) struct AppState {
    pub(crate) db: Db,
    wiki_watch: Arc<WikiWatchState>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DbChangedPayload {
    data_version: i64,
}

const DB_CHANGED_EVENT: &str = "db-changed";
const DATA_VERSION_POLL_MS: u64 = 350;

#[tauri::command]
fn get_install_context_cmd() -> InstallContext {
    detect_install_context()
}

#[tauri::command]
fn fetch_aur_package_info_cmd() -> Result<Option<AurPackageInfo>, String> {
    fetch_aur_package_info()
}

#[tauri::command]
fn open_aur_update_in_konsole_cmd() -> Result<(), String> {
    open_aur_update_in_konsole()
}

#[tauri::command]
fn latest_appimage_download_url_cmd() -> String {
    latest_appimage_download_url()
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

#[tauri::command]
fn set_syncthing_settings_cmd(
    state: tauri::State<'_, AppState>,
    settings: SyncthingSettings,
) -> Result<Settings, String> {
    set_syncthing_settings(&state.db, settings).map_err(|e| e.to_string())
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

// --- Wiki -------------------------------------------------------------------

#[tauri::command]
fn list_wiki_roots_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<WikiRoot>, String> {
    list_wiki_roots(&state.db).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct AddWikiRootArgs {
    path: String,
    label: Option<String>,
}

#[tauri::command]
fn add_wiki_root_cmd(
    state: tauri::State<'_, AppState>,
    args: AddWikiRootArgs,
) -> Result<WikiRoot, String> {
    let root = add_wiki_root(
        &state.db,
        AddWikiRootInput {
            path: args.path,
            label: args.label,
        },
    )
    .map_err(|e| e.to_string())?;
    state.wiki_watch.request_restart();
    Ok(root)
}

#[tauri::command]
fn remove_wiki_root_cmd(state: tauri::State<'_, AppState>, id: Uuid) -> Result<(), String> {
    remove_wiki_root(&state.db, id).map_err(|e| e.to_string())?;
    state.wiki_watch.request_restart();
    Ok(())
}

#[tauri::command]
fn list_wiki_pages_cmd(
    state: tauri::State<'_, AppState>,
    root_id: Option<Uuid>,
) -> Result<Vec<WikiPage>, String> {
    list_wiki_pages(&state.db, root_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_wiki_pages_cmd(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<WikiSearchHit>, String> {
    search_wiki_pages(&state.db, &query).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_wiki_page_cmd(
    state: tauri::State<'_, AppState>,
    id: Uuid,
) -> Result<WikiPageContent, String> {
    read_wiki_page(&state.db, id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_wiki_page_cmd(state: tauri::State<'_, AppState>, id: Uuid) -> Result<WikiPage, String> {
    get_wiki_page(&state.db, id).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct CreateWikiPageArgs {
    root_id: Uuid,
    rel_path: String,
    title: Option<String>,
    body: Option<String>,
    tags: Option<Vec<String>>,
}

#[tauri::command]
fn create_wiki_page_cmd(
    state: tauri::State<'_, AppState>,
    args: CreateWikiPageArgs,
) -> Result<WikiPageContent, String> {
    create_wiki_page(
        &state.db,
        CreateWikiPageInput {
            root_id: args.root_id,
            rel_path: args.rel_path,
            title: args.title,
            body: args.body,
            tags: args.tags,
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct WriteWikiPageArgs {
    id: Uuid,
    content: String,
    ensure_front_matter: Option<bool>,
}

#[tauri::command]
fn write_wiki_page_cmd(
    state: tauri::State<'_, AppState>,
    args: WriteWikiPageArgs,
) -> Result<WikiPageContent, String> {
    write_wiki_page(
        &state.db,
        WriteWikiPageInput {
            id: args.id,
            content: args.content,
            ensure_front_matter: args.ensure_front_matter,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn reindex_wiki_cmd(
    state: tauri::State<'_, AppState>,
    root_id: Option<Uuid>,
) -> Result<(), String> {
    if let Some(id) = root_id {
        reindex_root(&state.db, id).map_err(|e| e.to_string())?;
    } else {
        reindex_all(&state.db).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn list_wiki_backlinks_cmd(
    state: tauri::State<'_, AppState>,
    page_id: Uuid,
) -> Result<Vec<WikiBacklink>, String> {
    list_backlinks(&state.db, page_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn wiki_root_syncthing_status_cmd(
    state: tauri::State<'_, AppState>,
    root_id: Uuid,
) -> Result<SyncthingStatus, String> {
    let roots = list_wiki_roots(&state.db).map_err(|e| e.to_string())?;
    let root = roots
        .into_iter()
        .find(|r| r.id == root_id)
        .ok_or_else(|| format!("wiki root not found: {root_id}"))?;
    let settings = get_settings(&state.db).map_err(|e| e.to_string())?;
    let config = if settings.syncthing.is_configured() {
        Some(SyncthingClientConfig {
            address: settings.syncthing.address,
            api_key: settings.syncthing.api_key,
        })
    } else {
        None
    };
    Ok(status_for_path(
        std::path::Path::new(&root.path),
        config.as_ref(),
    ))
}

#[tauri::command]
fn pick_wiki_folder_cmd(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .set_title("Choose wiki folder")
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

fn spawn_data_version_watcher(app: AppHandle, running: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut last: Option<i64> = None;
        while running.load(Ordering::Relaxed) {
            let version = {
                let state = app.state::<AppState>();
                data_version(&state.db)
            };
            if let Ok(version) = version {
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
            // Err: transient lock / busy — try again next tick.
            thread::sleep(Duration::from_millis(DATA_VERSION_POLL_MS));
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let watcher_running = Arc::new(AtomicBool::new(true));
    let watcher_flag = Arc::clone(&watcher_running);
    let wiki_watch = Arc::new(WikiWatchState::new());
    let wiki_watch_for_setup = Arc::clone(&wiki_watch);
    let wiki_watch_for_exit = Arc::clone(&wiki_watch);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let path = default_db_path().map_err(|e| e.to_string())?;
            let db = open_db(&path).map_err(|e| e.to_string())?;
            app.manage(AppState {
                db,
                wiki_watch: Arc::clone(&wiki_watch_for_setup),
            });
            spawn_data_version_watcher(app.handle().clone(), watcher_flag);
            spawn_wiki_watcher(app.handle().clone(), wiki_watch_for_setup);
            Ok(())
        })
        .on_window_event(move |_window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                watcher_running.store(false, Ordering::Relaxed);
                wiki_watch_for_exit.stop();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_install_context_cmd,
            fetch_aur_package_info_cmd,
            open_aur_update_in_konsole_cmd,
            latest_appimage_download_url_cmd,
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
            set_syncthing_settings_cmd,
            list_task_events_since_cmd,
            latest_event_seq_cmd,
            list_wiki_roots_cmd,
            add_wiki_root_cmd,
            remove_wiki_root_cmd,
            list_wiki_pages_cmd,
            search_wiki_pages_cmd,
            read_wiki_page_cmd,
            get_wiki_page_cmd,
            create_wiki_page_cmd,
            write_wiki_page_cmd,
            reindex_wiki_cmd,
            list_wiki_backlinks_cmd,
            wiki_root_syncthing_status_cmd,
            pick_wiki_folder_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jade");
}
