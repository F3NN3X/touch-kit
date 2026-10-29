//! Pairing: which USB touch device belongs to which screen, decided by touching the screen.
//!
//! The panels stop (pairing needs every candidate device), a "touch this screen" window opens on
//! the panel's monitor, and every connected WinUSB device with the panel's vid:pid is opened. The
//! first one to report a finger down is written as the panel's `device.port`; the panels then start
//! again (through the hot reload when the file changed, directly when it didn't).

use crate::config::AppConfig;
use crate::panels::Panels;
use crate::settings::{edit_config, ConfigPath};
use crate::window;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use touch_core::{DeviceMsg, Output, PointerKind, TouchDevice};

const TIMEOUT: Duration = Duration::from_secs(30);
pub const LABEL: &str = "pair";

/// Set while a pairing runs; `pair_cancel` sets the flag.
#[derive(Default)]
pub struct Pairing(Mutex<Option<Arc<AtomicBool>>>);

#[derive(Clone, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum PairResult {
    Paired { index: usize, port: String },
    Cancelled { index: usize },
    TimedOut { index: usize },
    Failed { index: usize, message: String },
}

/// Async on purpose: it creates a window, which deadlocks in a synchronous command on Windows
/// (WebView2; see `WebviewWindowBuilder::build`). Async commands run off the main thread.
#[tauri::command]
pub async fn pair_start(
    app: AppHandle,
    pairing: State<'_, Pairing>,
    panels: State<'_, Panels>,
    current: State<'_, Mutex<AppConfig>>,
    index: usize,
) -> Result<(), String> {
    let cfg = current.lock().unwrap().clone();
    let entry = cfg.panels.get(index).ok_or("no such panel")?.clone();
    let mon = panels.monitor_of(index).ok_or("this panel has no monitor yet; choose one first")?;
    let ports: Vec<String> = touch_core::usb::candidates(entry.device.vid, entry.device.pid)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|c| c.driver.as_deref().is_none_or(|d| d.eq_ignore_ascii_case("winusb")))
        .map(|c| c.port)
        .collect();
    if ports.is_empty() {
        return Err(format!(
            "no {:04x}:{:04x} device with the WinUSB driver is connected (see Driver)",
            entry.device.vid, entry.device.pid
        ));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = pairing.0.lock().unwrap();
        if slot.is_some() {
            return Err("a pairing is already running".into());
        }
        *slot = Some(cancel.clone());
    }

    panels.stop();
    let route = format!("#pair/{}", urlencoding(&entry.name));
    let win = match window::create(&app, LABEL, &mon, false, &route) {
        Ok(w) => w,
        Err(e) => {
            *pairing.0.lock().unwrap() = None;
            panels.start(&app, &cfg);
            return Err(format!("opening the pairing window: {e}"));
        }
    };
    log::info!("pairing {}: listening on {} device(s)", entry.name, ports.len());

    // One device thread per candidate; their touches funnel into one channel, tagged by port.
    let (hit_tx, hit_rx) = mpsc::channel::<String>();
    let probes: Vec<TouchDevice> = ports
        .iter()
        .map(|port| {
            let mut touch = entry.touch(&cfg.gestures);
            touch.device.port = Some(port.clone());
            let (tx, rx) = mpsc::channel::<DeviceMsg>();
            let (hit_tx, port) = (hit_tx.clone(), port.clone());
            std::thread::spawn(move || {
                for m in rx {
                    if matches!(m, DeviceMsg::Output(Output::Pointer(ref p)) if p.kind == PointerKind::Down) {
                        let _ = hit_tx.send(port.clone());
                        return;
                    }
                }
            });
            TouchDevice::spawn(touch, Vec::new(), tx)
        })
        .collect();

    std::thread::spawn(move || {
        let deadline = Instant::now() + TIMEOUT;
        let result = loop {
            if cancel.load(Ordering::Relaxed) {
                break PairResult::Cancelled { index };
            }
            if Instant::now() > deadline {
                break PairResult::TimedOut { index };
            }
            match hit_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(port) => break PairResult::Paired { index, port },
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break PairResult::Failed { index, message: "devices closed".into() },
            }
        };
        drop(probes); // release every candidate before the panels reopen them
        let _ = win.destroy();
        finish(&app, &cfg, result);
    });
    Ok(())
}

#[tauri::command]
pub fn pair_cancel(pairing: State<'_, Pairing>) {
    if let Some(c) = pairing.0.lock().unwrap().as_ref() {
        c.store(true, Ordering::Relaxed);
    }
}

fn finish(app: &AppHandle, cfg: &AppConfig, result: PairResult) {
    let mut result = result;
    let mut written = false;
    if let PairResult::Paired { index, port } = result.clone() {
        let (index, port) = (&index, &port);
        // The port moves to this panel; a panel that had it pinned loses it (it was the wrong one).
        let mut edits: Vec<(String, Option<Value>)> = vec![(format!("panels.{index}.device.port"), Some(Value::from(port.clone())))];
        for (j, p) in cfg.panels.iter().enumerate() {
            if j != *index && p.device.port.as_deref() == Some(port) {
                edits.push((format!("panels.{j}.device.port"), None));
            }
        }
        let unchanged = cfg.panels[*index].device.port.as_deref() == Some(port.as_str()) && edits.len() == 1;
        if !unchanged {
            let path = app.state::<ConfigPath>().0.clone();
            let edits: Vec<(&str, Option<Value>)> = edits.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            match edit_config(&path, &edits) {
                Ok(()) => written = true,
                Err(e) => result = PairResult::Failed { index: *index, message: e.join("; ") },
            }
        }
        log::info!("pairing: panel {index} is the device at {port}");
    }
    // A changed file rebuilds the panels through the hot reload; otherwise restart them here.
    if !written {
        app.state::<Panels>().start(app, cfg);
    }
    *app.state::<Pairing>().0.lock().unwrap() = None;
    let _ = app.emit("pair-result", result);
}

/// Enough percent-encoding for a name in a URL fragment.
fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_survive_the_url_fragment() {
        assert_eq!(super::urlencoding("Edge #2 / ünï"), "Edge%20%232%20%2F%20%C3%BCn%C3%AF");
    }
}
