//! Filesystem watcher that reindexes wiki roots when markdown changes.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use jade_core::{list_wiki_roots, reindex_root, Db};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

const DEBOUNCE_MS: u64 = 400;

pub struct WikiWatchState {
    restart: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl WikiWatchState {
    pub fn new() -> Self {
        Self {
            restart: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn request_restart(&self) {
        self.restart.store(true, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn spawn_wiki_watcher(app: AppHandle, watch: Arc<WikiWatchState>) {
    thread::spawn(move || {
        let mut watcher: Option<RecommendedWatcher> = None;
        let pending: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
        let mut last_fire = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);

        loop {
            if watch.stop.load(Ordering::Relaxed) {
                break;
            }

            if watcher.is_none() || watch.restart.swap(false, Ordering::Relaxed) {
                watcher = build_watcher(app.clone(), Arc::clone(&pending)).ok();
                if let Some(w) = watcher.as_mut() {
                    let roots = {
                        let state = app.state::<crate::AppState>();
                        list_wiki_roots(&state.db).unwrap_or_default()
                    };
                    for root in roots {
                        if !root.enabled {
                            continue;
                        }
                        let path = PathBuf::from(&root.path);
                        if path.is_dir() {
                            let _ = w.watch(&path, RecursiveMode::Recursive);
                        }
                    }
                }
            }

            // Debounced reindex of dirty roots.
            let due = last_fire.elapsed() >= Duration::from_millis(DEBOUNCE_MS);
            if due {
                let dirty: Vec<PathBuf> = {
                    let mut guard = pending.lock().expect("wiki watch pending poisoned");
                    guard.drain().collect()
                };
                if !dirty.is_empty() {
                    let state = app.state::<crate::AppState>();
                    reindex_dirty_roots(&state.db, &dirty);
                    last_fire = Instant::now();
                }
            }

            thread::sleep(Duration::from_millis(150));
        }
    });
}

fn build_watcher(
    _app: AppHandle,
    pending: Arc<Mutex<HashSet<PathBuf>>>,
) -> notify::Result<RecommendedWatcher> {
    RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_)
                    | EventKind::Modify(_)
                    | EventKind::Remove(_)
                    | EventKind::Any => {
                        let mut guard = pending.lock().expect("wiki watch pending poisoned");
                        for path in event.paths {
                            if is_markdown(&path) || path.is_dir() {
                                guard.insert(path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        },
        notify::Config::default(),
    )
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

fn reindex_dirty_roots(db: &Db, dirty: &[PathBuf]) {
    let Ok(roots) = list_wiki_roots(db) else {
        return;
    };
    let mut touched = HashSet::new();
    for root in roots {
        if !root.enabled {
            continue;
        }
        let root_path = PathBuf::from(&root.path);
        for path in dirty {
            if path.starts_with(&root_path) || root_path.starts_with(path) {
                touched.insert(root.id);
                break;
            }
        }
    }
    for id in touched {
        let _ = reindex_root(db, id);
    }
}
