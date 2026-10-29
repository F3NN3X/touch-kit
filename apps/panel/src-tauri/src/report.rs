//! The display report (Settings → Report): one JSON file with everything needed to support a user's
//! touch displays elsewhere — OS, monitors, this app's config, every touchscreen's USB and HID
//! details (`touch_core::report`), and optionally a recording of real touches on each.
//!
//! Saved to the Downloads folder and shown in the file manager. No serial numbers, user or machine
//! names: it is meant to be attached to a public issue.

use crate::config::AppConfig;
use crate::panels::Panels;
use crate::window::{self, MonitorInfo};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};
use touch_core::report::{DeviceReport, TouchRecording};
use touch_core::PanelConfig;

/// Bumped when a field changes meaning or goes away.
const REPORT_VERSION: u32 = 1;
const ISSUE_URL: &str = "https://github.com/F3NN3X/touch-kit/issues/new?template=display-report.yml";

#[derive(Serialize)]
struct Report {
    report_version: u32,
    app_version: &'static str,
    created_utc: String,
    os: Os,
    monitors: Vec<Monitor>,
    config: AppConfig,
    touchscreens: Vec<DeviceReport>,
    /// Empty when the report was made without a recording.
    recordings: Vec<TouchRecording>,
    /// Problems while recording (a device that wouldn't open, ...).
    recording_errors: Vec<String>,
}

#[derive(Serialize)]
struct Os {
    family: &'static str,
    arch: &'static str,
    version: String,
}

#[derive(Serialize)]
struct Monitor {
    name: Option<String>,
    position: [i32; 2],
    size: [u32; 2],
    scale_factor: f64,
    primary: bool,
}

/// Creates the report, recording for `record_seconds` (0 = no recording); returns the file's path.
/// Async: the recording prompts are windows, which deadlock in a synchronous command on Windows.
#[tauri::command]
pub async fn report_create(
    app: AppHandle,
    panels: State<'_, Panels>,
    current: State<'_, Mutex<AppConfig>>,
    record_seconds: u32,
) -> Result<String, String> {
    if app.state::<crate::setup::Setup>().running() || app.state::<crate::pairing::Pairing>().running() {
        return Err("setup or pairing is running; finish it first".into());
    }
    let cfg = current.lock().unwrap().clone();
    let monitors = monitors(&app)?;

    // The panels hold their devices open; the report opens each one itself.
    panels.stop();
    let gathered = gather(&app, &cfg, record_seconds.min(60));
    panels.start(&app, &cfg);
    let (touchscreens, recordings, recording_errors) = gathered?;

    let report = Report {
        report_version: REPORT_VERSION,
        app_version: env!("CARGO_PKG_VERSION"),
        created_utc: utc(now()),
        os: os(),
        monitors,
        config: cfg,
        touchscreens,
        recordings,
        recording_errors,
    };
    let dir = app.path().download_dir().or_else(|_| app.path().document_dir()).map_err(|e| e.to_string())?;
    let path: PathBuf = dir.join(format!("touch-kit-report-{}.json", utc(now()).replace([':', 'T'], "-").trim_end_matches('Z')));
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("writing {}: {e}", path.display()))?;
    log::info!("report: wrote {}", path.display());
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().reveal_item_in_dir(&path);
    Ok(path.display().to_string())
}

/// Opens the GitHub form for submitting a report. Fixed URL: the webview can't open others.
#[tauri::command]
pub fn open_report_issue(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(ISSUE_URL, None::<&str>).map_err(|e| e.to_string())
}

type Gathered = (Vec<DeviceReport>, Vec<TouchRecording>, Vec<String>);

fn gather(app: &AppHandle, cfg: &AppConfig, record_seconds: u32) -> Result<Gathered, String> {
    let devices = touch_core::report::devices().map_err(|e| e.to_string())?;
    if record_seconds == 0 {
        return Ok((devices, Vec::new(), Vec::new()));
    }
    // A prompt on every panel's monitor, or on every other monitor before any setup.
    let all = window::monitors(app).map_err(|e| e.to_string())?;
    let main = app.primary_monitor().ok().flatten().map(|m| [m.position().x, m.position().y]);
    let mut spots: Vec<MonitorInfo> = cfg.panels.iter().filter_map(|p| window::pick_monitor(&all, p, &[]).cloned()).collect();
    if spots.is_empty() {
        spots = all.into_iter().filter(|m| Some(m.position) != main).collect();
    }
    spots.dedup_by_key(|m| m.position);
    let windows: Vec<tauri::WebviewWindow> = spots
        .iter()
        .enumerate()
        .filter_map(|(k, m)| window::create(app, &format!("record-{k}"), m, false, &format!("#record/{record_seconds}")).ok())
        .collect();

    // Every ready touchscreen at once, each mapped like the panel that uses it (gesture thresholds
    // are in panel pixels); one not used by a panel maps at the default size.
    let duration = Duration::from_secs(record_seconds.into());
    let threads: Vec<_> = devices
        .iter()
        .filter(|d| d.ready)
        .map(|d| {
            let port = d.port.clone();
            let mapping = cfg
                .panels
                .iter()
                .find(|p| p.device.port.as_deref() == Some(port.as_str()))
                .map_or_else(PanelConfig::default, |p| PanelConfig { calibration: None, ..p.panel.clone() });
            std::thread::spawn(move || {
                touch_core::report::open(&port)
                    .and_then(|panel| touch_core::report::record(&panel, &mapping, duration, || false))
                    .map_err(|e| format!("{port}: {e}"))
            })
        })
        .collect();
    let (mut recordings, mut errors) = (Vec::new(), Vec::new());
    for t in threads {
        match t.join() {
            Ok(Ok(r)) => recordings.push(r),
            Ok(Err(e)) => errors.push(e),
            Err(_) => errors.push("a recording thread panicked".into()),
        }
    }
    windows.into_iter().for_each(|w| drop(w.destroy()));
    Ok((devices, recordings, errors))
}

fn monitors(app: &AppHandle) -> Result<Vec<Monitor>, String> {
    let main = app.primary_monitor().ok().flatten().map(|m| *m.position());
    Ok(app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|m| Monitor {
            name: m.name().cloned(),
            position: [m.position().x, m.position().y],
            size: [m.size().width, m.size().height],
            scale_factor: m.scale_factor(),
            primary: Some(*m.position()) == main,
        })
        .collect())
}

fn os() -> Os {
    Os { family: std::env::consts::OS, arch: std::env::consts::ARCH, version: os_version().unwrap_or_default() }
}

#[cfg(windows)]
fn os_version() -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // "Microsoft Windows [Version 10.0.26300.1234]"
    let out = std::process::Command::new("cmd").args(["/c", "ver"]).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(not(windows))]
fn os_version() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    text.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=")).map(|v| v.trim_matches('"').to_string())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `YYYY-MM-DDTHH:MM:SSZ` for Unix seconds (civil-from-days, H. Hinnant).
fn utc(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc_formats_unix_seconds() {
        assert_eq!(super::utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::utc(1_793_289_000), "2026-10-29T15:50:00Z");
        assert_eq!(super::utc(951_782_400), "2000-02-29T00:00:00Z");
    }
}
