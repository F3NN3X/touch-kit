//! The settings window (main monitor, normal focusable window) and the commands it uses.
//!
//! Edits go through [`config_set`]: one key changes in the file, `null` deletes it (= back to its
//! default), and a change that would make the file invalid is refused before anything is written.
//! The hot reload then applies the file as usual.

use crate::config::{self, AppConfig, Level};
use crate::panels::{PanelStatus, Panels};
use crate::window::{self, MonitorInfo};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

/// Where the config file lives; managed state.
pub struct ConfigPath(pub PathBuf);

/// Opens the settings window, or brings it to the front, from a thread of its own: building a
/// window in an event handler or a synchronous command deadlocks on Windows (WebView2).
pub fn open_async(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || open(&app, None));
}

/// Opens the settings window, or brings it to the front. Not from an event handler or a
/// synchronous command; use [`open_async`] there.
/// `page`: open on that settings page (`"driver"`, ...) instead of Panels.
pub fn open(app: &AppHandle, page: Option<&str>) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(format!("index.html#settings{}", page.map(|p| format!("/{p}")).unwrap_or_default()).into()))
        .title("Touch Kit settings")
        .inner_size(980.0, 720.0)
        .min_inner_size(720.0, 480.0)
        .center()
        .build();
    if let Err(e) = built {
        log::error!("opening settings: {e}");
    }
}

#[derive(Serialize)]
pub struct ConfigView {
    /// The file as written, or null if it can't be read or parsed (then `file_error` says why).
    file: Value,
    file_error: Option<String>,
    /// The config in effect (the file's if it applied, else the previous one).
    effective: AppConfig,
    defaults: AppConfig,
}

#[tauri::command]
pub fn config_get(path: State<'_, ConfigPath>, current: State<'_, Mutex<AppConfig>>) -> ConfigView {
    let (file, file_error) = match std::fs::read_to_string(&path.0).map_err(|e| e.to_string()).and_then(|t| {
        serde_json::from_str::<Value>(&t).map_err(|e| e.to_string())
    }) {
        Ok(v) => (v, None),
        Err(e) => (Value::Null, Some(e)),
    };
    ConfigView { file, file_error, effective: current.lock().unwrap().clone(), defaults: AppConfig::default() }
}

/// Sets `key` (dotted, array indices as numbers: `panels.0.panel.rotation`) to `value`, or deletes
/// it when `value` is null. Refused, with the errors, if the result would not apply.
#[tauri::command]
pub fn config_set(path: State<'_, ConfigPath>, key: String, value: Value) -> Result<(), Vec<String>> {
    edit_config(&path.0, &[(key.as_str(), (!value.is_null()).then_some(value))])
}

/// Applies `edits` (key, value or `None` to delete) to the config file in one write. Nothing is
/// written if the result would not apply; the errors come back instead.
pub fn edit_config(path: &std::path::Path, edits: &[(&str, Option<Value>)]) -> Result<(), Vec<String>> {
    let text = std::fs::read_to_string(path).map_err(|e| vec![format!("reading the config: {e}")])?;
    let mut root: Value =
        serde_json::from_str(&text).map_err(|e| vec![format!("the config file doesn't parse ({e}); fix it first")])?;
    for (key, value) in edits {
        set_path(&mut root, key, value.clone()).map_err(|e| vec![e])?;
    }
    let next = serde_json::to_string_pretty(&root).expect("a Value serialises") + "
";
    let loaded = config::check(&next);
    if loaded.config.is_none() {
        return Err(loaded.issues.into_iter().filter(|i| i.level == Level::Error).map(|i| i.message).collect());
    }
    // Write-then-rename, so the reload never reads a half-written file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, next).and_then(|()| std::fs::rename(&tmp, path)).map_err(|e| vec![format!("writing the config: {e}")])?;
    let keys: Vec<&str> = edits.iter().map(|(k, _)| *k).collect();
    log::info!("settings: {} changed", keys.join(", "));
    Ok(())
}

#[tauri::command]
pub fn panels_status(panels: State<'_, Panels>) -> Vec<PanelStatus> {
    panels.status()
}

#[derive(Serialize)]
pub struct Monitor {
    name: Option<String>,
    position: [i32; 2],
    size: [u32; 2],
}

#[tauri::command]
pub fn monitors(app: AppHandle) -> Result<Vec<Monitor>, String> {
    let ms = window::monitors(&app).map_err(|e| e.to_string())?;
    Ok(ms.into_iter().map(|MonitorInfo { name, position, size }| Monitor { name, position, size }).collect())
}

/// Shows the panel's name big on its own screen for a moment.
#[tauri::command]
pub fn identify(app: AppHandle, panels: State<'_, Panels>, current: State<'_, Mutex<AppConfig>>, index: usize) {
    let name = current.lock().unwrap().panels.get(index).map(|p| p.name.clone()).unwrap_or_default();
    panels.identify(&app, index, &name);
}

/// Sets (or with `None` deletes) a dotted path in a JSON document, creating objects on the way.
pub fn set_path(root: &mut Value, path: &str, value: Option<Value>) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    let (last, parents) = parts.split_last().ok_or("empty key")?;
    let mut node = root;
    for p in parents {
        node = match node {
            Value::Array(items) => {
                let i: usize = p.parse().map_err(|_| format!("\"{p}\" in {path} is not an index"))?;
                items.get_mut(i).ok_or(format!("{path}: no element {i}"))?
            }
            _ => {
                if !node.is_object() {
                    *node = Value::Object(Default::default());
                }
                node.as_object_mut().unwrap().entry(p.to_string()).or_insert_with(|| Value::Object(Default::default()))
            }
        };
    }
    match (node, value) {
        (Value::Array(items), v) => {
            let i: usize = last.parse().map_err(|_| format!("\"{last}\" in {path} is not an index"))?;
            let slot = items.get_mut(i).ok_or(format!("{path}: no element {i}"))?;
            *slot = v.ok_or("array elements can't be deleted by key; set the whole array")?;
        }
        (node, Some(v)) => {
            if !node.is_object() {
                *node = Value::Object(Default::default());
            }
            node.as_object_mut().unwrap().insert(last.to_string(), v);
        }
        (Value::Object(map), None) => {
            map.remove(*last);
        }
        (_, None) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sets_creates_and_deletes_only_the_named_key() {
        let mut v = json!({"dev_mode": false, "panels": [{"name": "A"}]});
        set_path(&mut v, "gestures.slop_px", Some(json!(14))).unwrap();
        set_path(&mut v, "panels.0.panel.rotation", Some(json!("90"))).unwrap();
        set_path(&mut v, "panels.0.name", None).unwrap();
        set_path(&mut v, "dev_mode", None).unwrap();
        assert_eq!(v, json!({"gestures": {"slop_px": 14}, "panels": [{"panel": {"rotation": "90"}}]}));
        set_path(&mut v, "gestures.nope", None).unwrap();
        assert!(set_path(&mut v, "panels.3.name", Some(json!("x"))).is_err());
        assert!(set_path(&mut v, "panels.x.name", Some(json!("x"))).is_err());
    }
}
