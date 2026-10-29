//! Calibration: four targets on the panel itself. While it runs, the panel's device maps without
//! its calibration, so the webview sees positions that convert back to raw 0..1 touches
//! (`u = x / (width − 1)`, after rotation and flip). The webview collects the taps, asks for the fit
//! error, and saves or cancels; saving writes `panels.<i>.panel.calibration`.

use crate::config::AppConfig;
use crate::panels::Panels;
use crate::settings::{edit_config, ConfigPath};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use touch_core::CalPoint;

#[derive(Clone, Serialize)]
struct Start {
    index: usize,
    name: String,
    width_px: u32,
    height_px: u32,
}

#[tauri::command]
pub fn calibrate_start(
    app: AppHandle,
    panels: State<'_, Panels>,
    current: State<'_, Mutex<AppConfig>>,
    index: usize,
) -> Result<(), String> {
    let cfg = current.lock().unwrap().clone();
    let entry = cfg.panels.get(index).ok_or("no such panel")?;
    let raw = touch_core::PanelConfig { calibration: None, ..entry.panel.clone() };
    if !panels.reconfigure_one(index, raw, cfg.gestures.clone()) {
        return Err("this panel isn't running; give it a monitor first".into());
    }
    let start = Start { index, name: entry.name.clone(), width_px: entry.panel.width_px, height_px: entry.panel.height_px };
    panels.emit_to_panel(&app, index, "calibrate", start);
    log::info!("calibrating {}", entry.name);
    Ok(())
}

/// RMS distance in panel px between the targets and where the fit puts the taps.
#[tauri::command]
pub fn calibrate_check(points: [CalPoint; 4]) -> Result<f32, String> {
    touch_core::mapper::calibration_error(&points).map_err(|e| e.to_string())
}

/// Tells the settings window a calibration ended, so it drops its "tap the targets" hint.
fn ended(app: &AppHandle, index: usize) {
    let _ = app.emit_to(crate::settings::LABEL, "calibrate-end", index);
}

#[tauri::command]
pub fn calibrate_save(
    app: AppHandle,
    panels: State<'_, Panels>,
    path: State<'_, ConfigPath>,
    current: State<'_, Mutex<AppConfig>>,
    index: usize,
    points: [CalPoint; 4],
) -> Result<(), String> {
    let value = serde_json::to_value(points).map_err(|e| e.to_string())?;
    edit_config(&path.0, &[(&format!("panels.{index}.panel.calibration"), Some(value))]).map_err(|e| e.join("; "))?;
    // The hot reload applies it too; this covers saving the same calibration again (no file change).
    let cfg = current.lock().unwrap().clone();
    if let Some(entry) = cfg.panels.get(index) {
        panels.reconfigure_one(index, touch_core::PanelConfig { calibration: Some(points), ..entry.panel.clone() }, cfg.gestures);
    }
    ended(&app, index);
    Ok(())
}

/// Back to the panel's configured mapping.
#[tauri::command]
pub fn calibrate_cancel(app: AppHandle, panels: State<'_, Panels>, current: State<'_, Mutex<AppConfig>>, index: usize) {
    let cfg = current.lock().unwrap().clone();
    if let Some(entry) = cfg.panels.get(index) {
        panels.reconfigure_one(index, entry.panel.clone(), cfg.gestures);
    }
    ended(&app, index);
}
