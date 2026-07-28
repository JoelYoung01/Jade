//! Optional Syncthing status overlay (never used for content discovery).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncthingFolder {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncthingStatus {
    /// Whether this wiki root sits under a configured Syncthing folder.
    pub under_syncthing: bool,
    /// True when a `.stfolder` marker was found walking parents (no API needed).
    pub marker_detected: bool,
    pub folder: Option<SyncthingFolder>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyncthingClientConfig {
    /// e.g. `http://127.0.0.1:8384`
    pub address: String,
    pub api_key: String,
}

impl Default for SyncthingClientConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:8384".to_owned(),
            api_key: String::new(),
        }
    }
}

/// Walk parents looking for a `.stfolder` (or custom marker) directory/file.
pub fn detect_stfolder_marker(path: &Path) -> bool {
    let mut current = path.to_path_buf();
    loop {
        if current.join(".stfolder").exists() {
            return true;
        }
        if !current.pop() {
            break;
        }
    }
    false
}

/// List Syncthing folders via REST (`GET /rest/config/folders`).
pub fn list_folders(config: &SyncthingClientConfig) -> Result<Vec<SyncthingFolder>> {
    if config.api_key.trim().is_empty() {
        return Err(Error::Message("syncthing api key is required".into()));
    }
    let base = config.address.trim_end_matches('/');
    let url = format!("{base}/rest/config/folders");
    let response = ureq::get(&url)
        .header("X-API-Key", config.api_key.trim())
        .call()
        .map_err(|e| Error::Message(format!("syncthing request failed: {e}")))?;
    let body = response
        .into_body()
        .read_to_string()
        .map_err(|e| Error::Message(format!("syncthing response read failed: {e}")))?;
    let folders: Vec<SyncthingFolder> = serde_json::from_str(&body)
        .map_err(|e| Error::Message(format!("syncthing response decode failed: {e}")))?;
    Ok(folders)
}

fn normalize_for_prefix(path: &Path) -> PathBuf {
    // Best-effort absolute path without requiring the path to exist.
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
    }
}

fn path_is_under(child: &Path, parent: &Path) -> bool {
    let child = child
        .canonicalize()
        .unwrap_or_else(|_| normalize_for_prefix(child));
    let parent = parent
        .canonicalize()
        .unwrap_or_else(|_| normalize_for_prefix(parent));
    child.starts_with(&parent)
}

/// Resolve Syncthing status for a wiki root path.
///
/// Matches the Syncthing folder whose path is a **prefix** of the wiki root
/// (so a subdirectory of a wide sync share still counts).
pub fn status_for_path(path: &Path, config: Option<&SyncthingClientConfig>) -> SyncthingStatus {
    let marker_detected = detect_stfolder_marker(path);

    let Some(config) = config.filter(|c| !c.api_key.trim().is_empty()) else {
        return SyncthingStatus {
            under_syncthing: marker_detected,
            marker_detected,
            folder: None,
            error: None,
        };
    };

    match list_folders(config) {
        Ok(folders) => {
            let mut best: Option<SyncthingFolder> = None;
            let mut best_len = 0usize;
            for folder in folders {
                let folder_path = PathBuf::from(&folder.path);
                if path_is_under(path, &folder_path) {
                    let len = folder.path.len();
                    if len >= best_len {
                        best_len = len;
                        best = Some(folder);
                    }
                }
            }
            SyncthingStatus {
                under_syncthing: best.is_some() || marker_detected,
                marker_detected,
                folder: best,
                error: None,
            }
        }
        Err(err) => SyncthingStatus {
            under_syncthing: marker_detected,
            marker_detected,
            folder: None,
            error: Some(err.to_string()),
        },
    }
}
