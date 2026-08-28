//! Jade core: task/tag/settings/wiki domain and SQLite persistence.

mod cron;
mod db;
mod error;
mod events;
mod install;
mod models;
mod settings;
mod sync;
mod tags;
mod tasks;
mod time_helpers;
mod wiki;

pub use cron::{next_occurrence, normalize_cron, parse_cron};
pub use db::{data_version, default_db_path, open_db, open_default_db, Db, APP_DATA_DIR_NAME};
pub use error::{Error, Result};
pub use events::{latest_event_seq, list_task_events, list_task_events_since};
pub use install::{
    appimage_download_url, cli_script_marker_path, command_exists, deb_download_url,
    detect_install_context, detect_install_context_for, download_to_temp, fetch_aur_package_info,
    fetch_latest_release, is_newer_version, latest_appimage_download_url, open_aur_update_in_konsole,
    prefix_from_jade_exe, read_cli_script_marker, strip_pkgrel, windows_setup_download_url,
    write_cli_script_marker, AurPackageInfo, CliScriptMarker, InstallContext, InstallKind,
    LatestReleaseInfo, AUR_PACKAGE_NAME, CLI_SCRIPT_CHANNEL, CLI_SCRIPT_MARKER_REL,
    GITHUB_RELEASES_URL, LATEST_JSON_URL,
};
pub use models::{
    CreateTaskInput, DueUpdate, LaneVisibility, ListTaskEventsInput, ListTaskEventsSinceInput,
    PeerSyncSettings, RepeatCronUpdate, RescheduleMode, Settings, StatusUpdateResult,
    SyncthingSettings, Tag, Task, TaskEvent, TaskEventType, TaskStatus, UpdateTaskInput,
    UpdateTaskStatusInput, EVENT_ORIGIN_LOCAL,
};
pub use settings::{
    get_settings, set_lane_visibility, set_peer_sync_settings, set_syncthing_settings,
};
pub use sync::{
    apply_remote_task_events, ensure_device, generate_token, list_peers, pair_peer, serve_sync,
    sync_all_peers, ApplyStats, HelloResponse, SyncDevice, SyncPeer, SyncReport,
    SyncServerConfig, DEFAULT_SYNC_BIND, DEFAULT_SYNC_PORT, PROTOCOL_VERSION,
};
pub use tags::{count_tasks_with_tag, delete_tag, ensure_tag, list_tags};
pub use tasks::{
    create_task, delete_task, get_task, list_tasks, reschedule_task, update_task,
    update_task_status,
};
pub use time_helpers::{
    first_monday_next_month, next_hour_rounded, next_monday, push_to_today, push_to_tomorrow,
};
pub use wiki::{
    add_wiki_root, create_wiki_page, detect_stfolder_marker, get_wiki_page, get_wiki_root,
    latest_wiki_event_seq, list_backlinks, list_folders, list_wiki_events_since, list_wiki_pages,
    list_wiki_roots, read_wiki_page, reindex_all, reindex_root, remove_wiki_root,
    search_wiki_pages, status_for_path, write_wiki_page, AddWikiRootInput, CreateWikiPageInput,
    FrontMatter, ReindexStats, SyncthingClientConfig, SyncthingFolder, SyncthingStatus,
    WikiBacklink, WikiEntityType, WikiEvent, WikiEventType, WikiMatchKind, WikiPage,
    WikiPageContent, WikiRoot, WikiSearchHit, WikiSearchSnippet, WriteWikiPageInput,
};
