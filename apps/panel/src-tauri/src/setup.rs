//! One-touch setup: which touchscreen belongs to which monitor, for any number of panels at once.
//!
//! Every monitor but the main one shows a target, each at a different distance from its centre, and
//! every ready touchscreen is opened. A tap tells both halves: the device that reports it, and — by
//! how far from the centre it lands — the monitor it was shown on. Distance from the centre doesn't
//! change when a digitizer is mounted rotated or mirrored against its display, so this works before
//! calibration. Each tap binds one device to one monitor; the bindings are written as panels (port,
//! monitor and the monitor's size), reusing a panel that already has the device or the monitor.

use crate::config::{AppConfig, PanelEntry};
use crate::panels::Panels;
use crate::settings::{edit_config, ConfigPath};
use crate::window::{self, MonitorInfo};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use touch_core::{DeviceConfig, DeviceMsg, Output, PanelConfig, PointerKind, TouchDevice};

/// Gives up when no tap binds anything for this long.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// Farthest target from the centre, as a fraction of the screen (0.5 is the edge).
const MAX_RADIUS: f32 = 0.4;
/// More targets than this would sit too close together for an uncalibrated tap.
const MAX_MONITORS: usize = 5;
/// Probes map touches onto 0..=GRID so a position reads as a fraction of the screen.
const GRID: f32 = 1000.0;

/// Set while a setup runs; `setup_finish` sets the flag.
#[derive(Default)]
pub struct Setup(Mutex<Option<Arc<AtomicBool>>>);

impl Setup {
    pub fn running(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Bound {
    pub port: String,
    pub monitor: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SetupResult {
    /// `bound` may be empty: finished or timed out before any tap.
    Done { bound: Vec<Bound> },
    Failed { message: String },
}

/// Target distance from the centre for each of `n` monitors, spread evenly from the centre out.
pub fn radii(n: usize) -> Vec<f32> {
    match n {
        0 => Vec::new(),
        1 => vec![0.0],
        _ => (0..n).map(|k| k as f32 * MAX_RADIUS / (n - 1) as f32).collect(),
    }
}

/// The monitor whose target a tap at `(u, v)` (fractions of the screen) was meant for: the nearest
/// radius among those not bound yet, if within half the spacing between targets.
pub fn pick(u: f32, v: f32, radii: &[f32], bound: &[bool]) -> Option<usize> {
    let r = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt();
    let tolerance = if radii.len() > 1 { MAX_RADIUS / (radii.len() - 1) as f32 / 2.0 } else { f32::INFINITY };
    (0..radii.len())
        .filter(|&k| !bound[k])
        .map(|k| (k, (radii[k] - r).abs()))
        .filter(|(_, d)| *d <= tolerance)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(k, _)| k)
}

/// The file's `panels` with `bindings` applied. `effective` is the loaded config's panels, index for
/// index with `file` (a file without `panels` is one default panel, i.e. `[{}]`). Each binding goes
/// to the panel with that port, else the one on that monitor, else the first panel with neither set,
/// else a new panel; that panel gets the port, the monitor and the monitor's size.
pub fn apply(file: Vec<Value>, effective: &[PanelEntry], bindings: &[(String, MonitorInfo)]) -> Vec<Value> {
    let mut panels = file;
    let mut entries: Vec<PanelEntry> = effective.to_vec();
    for (port, mon) in bindings {
        let name = mon.name.clone().unwrap_or_default();
        let has_port = |p: &PanelEntry| p.device.port.as_deref() == Some(port.as_str());
        let on_monitor = |p: &PanelEntry| p.monitor.as_ref().and_then(|m| m.name.as_deref()) == Some(name.as_str());
        let blank = |p: &PanelEntry| p.device.port.is_none() && p.monitor.is_none();
        let i = entries
            .iter()
            .position(has_port)
            .or_else(|| entries.iter().position(on_monitor))
            .or_else(|| entries.iter().position(blank))
            .unwrap_or_else(|| {
                let n = entries.len() + 1;
                entries.push(PanelEntry { name: format!("Panel {n}"), ..Default::default() });
                panels.push(json!({ "name": format!("Panel {n}") }));
                entries.len() - 1
            });
        // Another panel holding this port or monitor would fight over it: it lets go.
        for (j, e) in entries.iter_mut().enumerate() {
            if j == i {
                continue;
            }
            if e.device.port.as_deref() == Some(port.as_str()) {
                e.device.port = None;
                if let Some(d) = panels[j].get_mut("device").and_then(Value::as_object_mut) {
                    d.remove("port");
                }
            }
            if e.monitor.as_ref().and_then(|m| m.name.as_deref()) == Some(name.as_str()) {
                e.monitor = None;
                if let Some(o) = panels[j].as_object_mut() {
                    o.remove("monitor");
                }
            }
        }
        entries[i].device.port = Some(port.clone());
        entries[i].monitor = Some(crate::config::MonitorMatch { name: Some(name.clone()), position: None });
        let p = &mut panels[i];
        if !p.is_object() {
            *p = json!({});
        }
        p["device"] = merged(p.get("device"), json!({ "port": port }));
        p["monitor"] = json!({ "name": name });
        p["panel"] = merged(p.get("panel"), json!({ "width_px": mon.size[0], "height_px": mon.size[1] }));
    }
    panels
}

/// `base` (an object, or nothing) with `extra`'s keys set.
fn merged(base: Option<&Value>, extra: Value) -> Value {
    let mut out = base.filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({}));
    for (k, v) in extra.as_object().expect("object") {
        out[k] = v.clone();
    }
    out
}

/// Returns how many monitors show a target. Async: it creates windows, which deadlocks in a synchronous command on Windows (WebView2).
#[tauri::command]
pub async fn setup_start(
    app: AppHandle,
    panels: State<'_, Panels>,
    setup: State<'_, Setup>,
    pairing: State<'_, crate::pairing::Pairing>,
    current: State<'_, Mutex<AppConfig>>,
) -> Result<usize, String> {
    if pairing.running() {
        return Err("a pairing is running; finish it first".into());
    }
    let cfg = current.lock().unwrap().clone();
    let main = app.primary_monitor().map_err(|e| e.to_string())?.map(|m| [m.position().x, m.position().y]);
    let mut monitors: Vec<MonitorInfo> =
        window::monitors(&app).map_err(|e| e.to_string())?.into_iter().filter(|m| Some(m.position) != main).collect();
    monitors.sort_by_key(|m| (m.position[1], m.position[0]));
    monitors.truncate(MAX_MONITORS);
    if monitors.is_empty() {
        return Err("no monitor besides the main one: connect the touch display first".into());
    }
    let ports: Vec<String> = touch_core::usb::candidates(&DeviceConfig::default())
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|t| t.ready)
        .map(|t| t.port)
        .collect();
    if ports.is_empty() {
        return Err("no touchscreen is ready to use; see the Driver page".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = setup.0.lock().unwrap();
        if slot.is_some() {
            return Err("setup is already running".into());
        }
        *slot = Some(cancel.clone());
    }

    panels.stop();
    let count = monitors.len();
    let radii = radii(count);
    let mut windows = Vec::new();
    for (k, mon) in monitors.iter().enumerate() {
        match window::create(&app, &format!("setup-{k}"), mon, false, &format!("#setup/{}", radii[k])) {
            Ok(w) => windows.push(w),
            Err(e) => {
                windows.into_iter().for_each(|w: tauri::WebviewWindow| drop(w.destroy()));
                *setup.0.lock().unwrap() = None;
                panels.start(&app, &cfg);
                return Err(format!("opening the setup windows: {e}"));
            }
        }
    }
    log::info!("setup: {} monitor(s), {} touchscreen(s)", monitors.len(), ports.len());

    // One probe per touchscreen, mapping to a GRID×GRID square; downs funnel in tagged by port.
    let (hit_tx, hit_rx) = mpsc::channel::<(String, f32, f32)>();
    let probes: Vec<TouchDevice> = ports
        .iter()
        .map(|port| {
            let panel = PanelConfig { width_px: GRID as u32 + 1, height_px: GRID as u32 + 1, ..Default::default() };
            let device = DeviceConfig { port: Some(port.clone()), ..Default::default() };
            let touch = touch_core::Config { device, panel, gestures: cfg.gestures.clone() };
            let (tx, rx) = mpsc::channel::<DeviceMsg>();
            let (hit_tx, port) = (hit_tx.clone(), port.clone());
            std::thread::spawn(move || {
                for m in rx {
                    if let DeviceMsg::Output(Output::Pointer(p)) = m {
                        if p.kind == PointerKind::Down && hit_tx.send((port.clone(), p.x / GRID, p.y / GRID)).is_err() {
                            return;
                        }
                    }
                }
            });
            TouchDevice::spawn(touch, Vec::new(), tx)
        })
        .collect();

    std::thread::spawn(move || {
        let mut bound_monitor = vec![false; monitors.len()];
        let mut bindings: Vec<(String, MonitorInfo)> = Vec::new();
        let mut idle_until = Instant::now() + IDLE_TIMEOUT;
        while !cancel.load(Ordering::Relaxed)
            && Instant::now() < idle_until
            && bindings.len() < monitors.len().min(ports.len())
        {
            match hit_rx.recv_timeout(Duration::from_millis(100)) {
                Ok((port, u, v)) => {
                    if bindings.iter().any(|(p, _)| *p == port) {
                        continue;
                    }
                    let Some(k) = pick(u, v, &radii, &bound_monitor) else {
                        log::info!("setup: tap at ({u:.2}, {v:.2}) from {port} is on no target");
                        continue;
                    };
                    bound_monitor[k] = true;
                    let name = monitors[k].name.clone().unwrap_or_default();
                    log::info!("setup: {port} is the touchscreen of {name}");
                    let _ = app.emit_to(format!("setup-{k}").as_str(), "setup-bound", &name);
                    bindings.push((port, monitors[k].clone()));
                    idle_until = Instant::now() + IDLE_TIMEOUT;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(probes); // release every device before the panels reopen them
        // Long enough to read the last "set up" on its screen.
        std::thread::sleep(Duration::from_millis(800));
        windows.into_iter().for_each(|w| drop(w.destroy()));
        finish(&app, &cfg, &bindings);
    });
    Ok(count)
}

/// Ends a running setup, keeping what was bound so far.
#[tauri::command]
pub fn setup_finish(setup: State<'_, Setup>) {
    if let Some(c) = setup.0.lock().unwrap().as_ref() {
        c.store(true, Ordering::Relaxed);
    }
}

fn finish(app: &AppHandle, cfg: &AppConfig, bindings: &[(String, MonitorInfo)]) {
    let result = if bindings.is_empty() {
        app.state::<Panels>().start(app, cfg);
        SetupResult::Done { bound: Vec::new() }
    } else {
        let path = app.state::<ConfigPath>().0.clone();
        let file: Option<Vec<Value>> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| v.get("panels").and_then(Value::as_array).cloned().unwrap_or_else(|| vec![json!({})]));
        match file {
            None => {
                app.state::<Panels>().start(app, cfg);
                SetupResult::Failed { message: "the config file can't be read; fix it first".into() }
            }
            // The file changes, so the hot reload rebuilds the panels.
            Some(file) => match edit_config(&path, &[("panels", Some(Value::Array(apply(file, &cfg.panels, bindings))))]) {
                Ok(()) => SetupResult::Done {
                    bound: bindings
                        .iter()
                        .map(|(port, m)| Bound { port: port.clone(), monitor: m.name.clone().unwrap_or_default() })
                        .collect(),
                },
                Err(e) => {
                    app.state::<Panels>().start(app, cfg);
                    SetupResult::Failed { message: e.join("; ") }
                }
            },
        }
    };
    *app.state::<Setup>().0.lock().unwrap() = None;
    let _ = app.emit_to(crate::settings::LABEL, "setup-result", result);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(name: &str, size: [u32; 2]) -> MonitorInfo {
        MonitorInfo { name: Some(name.into()), position: [0, 0], size }
    }

    #[test]
    fn taps_pick_the_target_at_their_distance_from_the_centre() {
        let r = radii(3); // 0, 0.2, 0.4
        assert_eq!(pick(0.5, 0.5, &r, &[false; 3]), Some(0));
        // 0.2 from the centre, in any direction: rotated or mirrored digitizers land the same.
        assert_eq!(pick(0.7, 0.5, &r, &[false; 3]), Some(1));
        assert_eq!(pick(0.5, 0.3, &r, &[false; 3]), Some(1));
        assert_eq!(pick(0.1, 0.5, &r, &[false; 3]), Some(2));
        // Halfway between two targets is still the nearer one; a bound monitor is skipped.
        assert_eq!(pick(0.82, 0.5, &r, &[false, true, false]), Some(2));
        assert_eq!(pick(0.65, 0.5, &r, &[false, true, false]), None, "too far from both free targets");
        assert_eq!(pick(0.0, 0.0, &radii(1), &[false]), Some(0), "one monitor: any tap");
    }

    #[test]
    fn bindings_fill_the_blank_default_panel_then_append() {
        let effective = vec![PanelEntry::default()];
        let out = apply(
            vec![json!({})],
            &effective,
            &[("p1".into(), mon("D4", [1920, 550])), ("p2".into(), mon("D5", [1280, 800]))],
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["device"]["port"], "p1");
        assert_eq!(out[0]["monitor"]["name"], "D4");
        assert_eq!(out[0]["panel"], json!({"width_px": 1920, "height_px": 550}));
        assert_eq!(out[1]["name"], "Panel 2");
        assert_eq!(out[1]["panel"]["width_px"], 1280);
    }

    #[test]
    fn a_panel_keeps_its_settings_and_others_let_go_of_the_port() {
        let file = vec![
            json!({"name": "main", "device": {"port": "p1"}, "panel": {"rotation": "180"}}),
            json!({"name": "side", "monitor": {"name": "D4"}}),
        ];
        let mut effective = vec![PanelEntry::default(), PanelEntry::default()];
        effective[0].device.port = Some("p1".into());
        effective[1].monitor = Some(crate::config::MonitorMatch { name: Some("D4".into()), position: None });
        // p1 is tapped on D4: "main" keeps its name and rotation and moves there; "side" loses D4.
        let out = apply(file, &effective, &[("p1".into(), mon("D4", [1920, 550]))]);
        assert_eq!(out[0]["name"], "main");
        assert_eq!(out[0]["panel"]["rotation"], "180");
        assert_eq!(out[0]["monitor"]["name"], "D4");
        assert!(out[1].get("monitor").is_none());
    }
}
