//! Config hot reload: polls the file's modification time and applies changes live.
//!
//! Everything applies live. Gesture thresholds and panel mapping go to the running devices; a
//! change to the panel list, a panel's device or monitor, or `dev_mode` rebuilds the panels.

use crate::config::{self, AppConfig, Issue, Level, Loaded, UiConfig};
use crate::panels::Panels;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, Manager};

const POLL: Duration = Duration::from_millis(500);

/// Set by [`request`]; the next poll reloads even if the file is unchanged.
static FORCE: AtomicBool = AtomicBool::new(false);

/// Reloads the config on the next poll (tray "Reload config").
pub fn request() {
    FORCE.store(true, Ordering::Relaxed);
}

pub fn spawn(app: AppHandle, path: PathBuf, mut current: AppConfig) {
    std::thread::Builder::new()
        .name("config-reload".into())
        .spawn(move || {
            let mut seen = modified(&path);
            loop {
                std::thread::sleep(POLL);
                let m = modified(&path);
                if m == seen && !FORCE.swap(false, Ordering::Relaxed) {
                    continue;
                }
                seen = m;
                let Loaded { config, issues, .. } = config::load(&path);
                log_issues(&path, &issues, "keeping the previous config");
                match config {
                    Some(next) => {
                        apply(&app, &current, &next, issues);
                        current = next;
                    }
                    None => publish(&app, UiConfig::new(&current, issues)),
                }
            }
        })
        .expect("spawning config-reload thread");
}

/// Logs each issue, saying what happens instead when one of them is an error.
pub fn log_issues(path: &Path, issues: &[Issue], fallback: &str) {
    for i in issues {
        match i.level {
            Level::Error => log::error!("config {}: {}; {fallback}", path.display(), i.message),
            Level::Warning => log::warn!("config {}: {}", path.display(), i.message),
        }
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn apply(app: &AppHandle, old: &AppConfig, new: &AppConfig, mut issues: Vec<Issue>) {
    issues.extend(app.state::<Panels>().apply(app, old, new));
    *app.state::<Mutex<AppConfig>>().lock().unwrap() = new.clone();
    publish(app, UiConfig::new(new, issues));
    log::info!("config reloaded");
}

/// Stores the webview's config and sends it, so the UI shows the file's current issues too.
fn publish(app: &AppHandle, ui: UiConfig) {
    *app.state::<Mutex<UiConfig>>().lock().unwrap() = ui.clone();
    let _ = app.emit("ui-config", ui);
}
