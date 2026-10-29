//! The running panels: per config entry a touch device thread, a bridge and a no-activate window.
//!
//! A change to the panel list (count, name, device, monitor) rebuilds all of them; geometry and
//! gesture threshold changes are applied to the running devices in place.

use crate::bridge::{Bridge, UiMsg};
use crate::config::{AppConfig, Issue, PanelEntry};
use crate::testing;
use crate::window::{self, MonitorInfo};
use serde::Serialize;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, WebviewWindow};
use touch_core::{Status, TouchDevice};

struct Running {
    /// Position in the config's `panels`; virtual panels follow after the last one.
    index: usize,
    name: String,
    /// A virtual panel's mapping; it has no config entry to take one from (see `testing.rs`).
    virtual_panel: Option<touch_core::PanelConfig>,
    monitor: MonitorInfo,
    label: String,
    bridge: Bridge,
    device: TouchDevice,
    window: WebviewWindow,
}

#[derive(Default)]
struct Inner {
    running: Vec<Running>,
    /// Bumped on every rebuild so new windows never reuse a label that is still closing.
    generation: u32,
    /// Problems from the last start (no monitor, ...); still true until the next rebuild.
    issues: Vec<Issue>,
    /// Channels from windows that loaded while their panel was still starting, by window label.
    early: Vec<(String, Channel<UiMsg>)>,
}

#[derive(Default)]
pub struct Panels(Mutex<Inner>);

/// A configured panel as settings shows it.
#[derive(Clone, Debug, Serialize)]
pub struct PanelStatus {
    pub index: usize,
    pub name: String,
    /// A testing mode panel (see `testing.rs`).
    pub is_virtual: bool,
    /// The monitor it opened on; `None` if it isn't running (see the issues).
    pub monitor: Option<String>,
    /// The device's last status; `None` before the first one arrives or if it isn't running.
    pub touch: Option<Status>,
}

/// Fields whose change needs the panel's device and window rebuilt.
fn identity(p: &PanelEntry) -> (&str, &touch_core::DeviceConfig, &Option<crate::config::MonitorMatch>) {
    (&p.name, &p.device, &p.monitor)
}

impl Panels {
    /// Starts every panel in `cfg`. Problems (no monitor, no windows) come back as issues.
    pub fn start(&self, app: &AppHandle, cfg: &AppConfig) -> Vec<Issue> {
        let generation = {
            let mut inner = self.0.lock().unwrap();
            inner.generation += 1;
            inner.generation
        };
        // Not under the lock: building a window waits for the main thread, and the main thread may
        // be waiting for the lock (`panels_status` is a sync command) — a deadlock.
        let (running, issues) = start_all(app, cfg, generation);
        let mut inner = self.0.lock().unwrap();
        // A window can load and subscribe before `start_all` returns; hand those channels over now.
        for (label, channel) in std::mem::take(&mut inner.early) {
            if let Some(r) = running.iter().find(|r| r.label == label) {
                r.bridge.subscribe(channel);
            }
        }
        inner.running = running;
        for i in &issues {
            log::warn!("{}", i.message);
        }
        inner.issues = issues.clone();
        issues
    }

    /// Applies a reloaded config: rebuilds on a panel list change, else reconfigures in place.
    pub fn apply(&self, app: &AppHandle, old: &AppConfig, new: &AppConfig) -> Vec<Issue> {
        let rebuild = old.panels.len() != new.panels.len()
            || old.panels.iter().zip(&new.panels).any(|(a, b)| identity(a) != identity(b))
            || old.dev_mode != new.dev_mode
            || old.testing != new.testing;
        if rebuild {
            self.stop();
            return self.start(app, new);
        }
        let inner = self.0.lock().unwrap();
        for r in &inner.running {
            match &r.virtual_panel {
                Some(p) if old.gestures != new.gestures => r.device.reconfigure(p.clone(), new.gestures.clone()),
                Some(_) => {}
                None => {
                    let (a, b) = (&old.panels[r.index], &new.panels[r.index]);
                    if a.panel != b.panel || old.gestures != new.gestures {
                        r.device.reconfigure(b.panel.clone(), new.gestures.clone());
                    }
                }
            }
        }
        inner.issues.clone()
    }

    /// Stops every device thread and closes every panel window.
    pub fn stop(&self) {
        let running = std::mem::take(&mut self.0.lock().unwrap().running);
        for r in running {
            let _ = r.window.destroy();
            drop(r.device); // joins the device thread; its bridge thread ends with it
        }
    }

    /// Every running panel's monitor and touch status, by config index.
    pub fn status(&self) -> Vec<PanelStatus> {
        let inner = self.0.lock().unwrap();
        inner
            .running
            .iter()
            .map(|r| PanelStatus {
                index: r.index,
                name: r.name.clone(),
                is_virtual: r.virtual_panel.is_some(),
                monitor: r.monitor.name.clone(),
                touch: r.bridge.status(),
            })
            .collect()
    }

    /// The monitor running panel `index` opened on.
    pub fn monitor_of(&self, index: usize) -> Option<MonitorInfo> {
        self.0.lock().unwrap().running.iter().find(|r| r.index == index).map(|r| r.monitor.clone())
    }

    /// Gives running panel `index` a different mapping (calibration uses this), without a rebuild.
    pub fn reconfigure_one(&self, index: usize, panel: touch_core::PanelConfig, gestures: touch_core::GestureConfig) -> bool {
        match self.0.lock().unwrap().running.iter().find(|r| r.index == index) {
            Some(r) => {
                r.device.reconfigure(panel, gestures);
                true
            }
            None => false,
        }
    }

    /// Sends an event to running panel `index`'s window only.
    pub fn emit_to_panel<S: Serialize + Clone>(&self, app: &AppHandle, index: usize, event: &str, payload: S) -> bool {
        match self.0.lock().unwrap().running.iter().find(|r| r.index == index) {
            Some(r) => app.emit_to(r.label.as_str(), event, payload).is_ok(),
            None => false,
        }
    }

    /// Asks panel `index`'s window to show `name` for a moment.
    pub fn identify(&self, app: &AppHandle, index: usize, name: &str) {
        self.emit_to_panel(app, index, "identify", name);
    }

    /// Connects a panel window's channel to that panel's device.
    pub fn subscribe(&self, label: &str, channel: Channel<UiMsg>) {
        let mut inner = self.0.lock().unwrap();
        match inner.running.iter().find(|r| r.label == label) {
            Some(r) => r.bridge.subscribe(channel),
            None => inner.early.push((label.to_string(), channel)),
        }
    }
}

fn start_all(app: &AppHandle, cfg: &AppConfig, generation: u32) -> (Vec<Running>, Vec<Issue>) {
    let mut issues = Vec::new();
    let monitors = match window::monitors(app) {
        Ok(m) => m,
        Err(e) => return (Vec::new(), vec![Issue::error(format!("listing monitors: {e}"))]),
    };
    for m in &monitors {
        log::info!("monitor {:?}: {:?} at {:?}", m.name, m.size, m.position);
    }
    let mut taken_monitors: Vec<[i32; 2]> = Vec::new();
    let mut running = Vec::new();
    for (i, entry) in cfg.panels.iter().enumerate() {
        let at = format!("panels[{i}] \"{}\"", entry.name);
        let Some(mon) = window::pick_monitor(&monitors, entry, &taken_monitors).cloned() else {
            issues.push(Issue::warning(format!(
                "{at}: no free monitor matches ({}x{}); set its \"monitor\"",
                entry.panel.width_px, entry.panel.height_px
            )));
            continue;
        };
        taken_monitors.push(mon.position);
        match start_one(app, cfg, i, &mon, generation) {
            Ok(r) => running.push(r),
            Err(e) => issues.push(Issue::error(format!("{at}: {e}"))),
        }
    }
    if cfg.testing.enabled {
        match start_virtual(app, cfg, generation) {
            Ok(v) => running.extend(v),
            Err(e) => issues.push(Issue::error(format!("testing: {e}"))),
        }
    }
    (running, issues)
}

/// Testing mode's panels, on the main monitor, each playing `testing.replay` in a loop.
fn start_virtual(app: &AppHandle, cfg: &AppConfig, generation: u32) -> anyhow::Result<Vec<Running>> {
    let rec = testing::recording(&cfg.testing.replay).map_err(anyhow::Error::msg)?;
    let main = app.primary_monitor()?.ok_or_else(|| anyhow::anyhow!("no main monitor"))?;
    let info = MonitorInfo {
        name: main.name().cloned(),
        position: [main.position().x, main.position().y],
        size: [main.size().width, main.size().height],
    };
    let base = touch_core::PanelConfig::default();
    let spots = testing::slots(&info, main.scale_factor(), [base.width_px, base.height_px], cfg.testing.virtual_panels.into());
    let mut running = Vec::new();
    for (k, spot) in spots.into_iter().enumerate() {
        let index = cfg.panels.len() + k;
        let name = format!("Virtual {}", k + 1);
        // Mapped to the window's physical size: the webview divides by its scale back to CSS px.
        let panel = touch_core::PanelConfig { width_px: spot.size[0], height_px: spot.size[1], ..base.clone() };
        let bridge = Bridge::new(&name);
        let (tx, rx) = std::sync::mpsc::channel();
        bridge.spawn(rx);
        let device = TouchDevice::replay(
            touch_core::Recording { descriptor: rec.descriptor.clone(), reports: rec.reports.clone() },
            format!("virtual:{}", k + 1),
            panel.clone(),
            cfg.gestures.clone(),
            tx,
        );
        let label = format!("panel-{generation}-{index}");
        let window = window::create_virtual(app, &label, &format!("Touch Kit · {name} (testing)"), &spot)?;
        log::info!("{name}: playing {} at {:?}", cfg.testing.replay, spot.position);
        running.push(Running { index, name, virtual_panel: Some(panel), monitor: spot, label, bridge, device, window });
    }
    Ok(running)
}

fn start_one(app: &AppHandle, cfg: &AppConfig, i: usize, mon: &MonitorInfo, generation: u32) -> anyhow::Result<Running> {
    let entry = &cfg.panels[i];
    log::info!("{}: monitor {:?}", entry.name, mon.name);
    // Ports other panels name: this panel's device thread must never open those.
    let taken: Vec<String> =
        cfg.panels.iter().enumerate().filter(|(j, _)| *j != i).filter_map(|(_, p)| p.device.port.clone()).collect();
    let bridge = Bridge::new(&entry.name);
    let (tx, rx) = std::sync::mpsc::channel();
    bridge.spawn(rx);
    let device = TouchDevice::spawn(entry.touch(&cfg.gestures), taken, tx);
    let label = format!("panel-{generation}-{i}");
    let window = window::create(app, &label, mon, cfg.dev_mode, "")?;
    Ok(Running { index: i, name: entry.name.clone(), virtual_panel: None, monitor: mon.clone(), label, bridge, device, window })
}
