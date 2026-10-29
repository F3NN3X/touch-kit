//! The Driver page: which driver each touch device has, and giving a device back to Windows.
//!
//! Installing WinUSB is guided (Zadig, by the user); uninstalling is automated: an elevated
//! PowerShell script (one UAC prompt) removes the WinUSB binding for this vid:pid and
//! the certificate libwdi/Zadig leaves behind. docs/PLAN.md §9, 5e.

use crate::config::AppConfig;
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

#[derive(Clone, Debug, Serialize)]
pub struct DriverDevice {
    pub vid: u16,
    pub pid: u16,
    pub port: String,
    pub product: Option<String>,
    /// Bound driver service on Windows (`WinUSB`, `HidUsb`, ...); `None` elsewhere.
    pub driver: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DriverView {
    /// The vid:pid of every touchscreen found, plus any a panel names explicitly (connected or not).
    ids: Vec<[u16; 2]>,
    devices: Vec<DriverDevice>,
    /// Whether this platform swaps drivers at all (Windows) or detaches the kernel driver (Linux).
    windows: bool,
}

#[tauri::command]
pub fn driver_status(current: State<'_, Mutex<AppConfig>>) -> Result<DriverView, String> {
    let configs: Vec<touch_core::DeviceConfig> = current.lock().unwrap().panels.iter().map(|p| p.device.clone()).collect();
    let mut ids: Vec<[u16; 2]> = Vec::new();
    let mut devices: Vec<DriverDevice> = Vec::new();
    for cfg in &configs {
        if let (Some(vid), Some(pid)) = (cfg.vid, cfg.pid) {
            if !ids.contains(&[vid, pid]) {
                ids.push([vid, pid]);
            }
        }
        for c in touch_core::usb::candidates(cfg).map_err(|e| e.to_string())? {
            if !devices.iter().any(|d| d.port == c.port) {
                devices.push(DriverDevice { vid: c.vid, pid: c.pid, port: c.port, product: c.product, driver: c.driver });
            }
        }
    }
    for d in &devices {
        if !ids.contains(&[d.vid, d.pid]) {
            ids.push([d.vid, d.pid]);
        }
    }
    Ok(DriverView { ids, devices, windows: cfg!(windows) })
}

/// Opens Zadig's official site in the default browser. Fixed URL: the webview can't open others.
#[tauri::command]
pub fn open_zadig(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url("https://zadig.akeo.ie", None::<&str>).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct UninstallResult {
    /// False when nothing was bound to WinUSB (the certificate is still removed).
    changed: bool,
    log: String,
}

/// Gives every vid:pid device back to Windows. Async: it waits for the UAC prompt and the script.
#[tauri::command]
pub async fn driver_uninstall(
    app: tauri::AppHandle,
    panels: State<'_, crate::panels::Panels>,
    current: State<'_, Mutex<AppConfig>>,
    vid: u16,
    pid: u16,
) -> Result<UninstallResult, String> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        // The panels hold the device open; removing it under an open handle can need a reboot.
        panels.stop();
        let result = elevated::uninstall(&dir, vid, pid);
        let cfg = current.lock().unwrap().clone();
        panels.start(&app, &cfg);
        result
    }
    #[cfg(not(windows))]
    {
        let _ = (app, panels, current, vid, pid);
        Err("drivers are only swapped on Windows".into())
    }
}

#[cfg(windows)]
mod elevated {
    use super::UninstallResult;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    const SCRIPT: &str = include_str!("../scripts/uninstall-winusb.ps1");
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    /// Exit code the launcher uses when the UAC prompt is declined (Windows' ERROR_CANCELLED).
    const CANCELLED: i32 = 1223;

    /// A single-quoted PowerShell string literal.
    fn ps_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    /// Runs the script elevated (UAC) and hands back its exit code; a declined prompt exits 1223.
    fn launcher_text(script: &Path, log: &Path, vid: u16, pid: u16) -> String {
        // Start-Process joins -ArgumentList with spaces, so paths carry their own double quotes.
        let arg = |p: &Path| ps_quote(&format!("\"{}\"", p.display()));
        format!(
            "try {{\n  $p = Start-Process -FilePath powershell.exe -Verb RunAs -Wait -PassThru -WindowStyle Hidden \
             -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',{},'-Vid','{vid:04X}','-ProductId','{pid:04X}','-Log',{})\n  \
             exit $p.ExitCode\n}} catch {{ exit {CANCELLED} }}\n",
            arg(script),
            arg(log),
        )
    }

    pub fn uninstall(dir: &Path, vid: u16, pid: u16) -> Result<UninstallResult, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        // In the app's own data dir (per user), not %TEMP%, since it runs elevated.
        let script = dir.join("uninstall-winusb.ps1");
        let launcher = dir.join("uninstall-launcher.ps1");
        let log = dir.join("uninstall-winusb.log");
        let _ = std::fs::remove_file(&log);
        std::fs::write(&script, SCRIPT).map_err(|e| e.to_string())?;
        std::fs::write(&launcher, launcher_text(&script, &log, vid, pid)).map_err(|e| e.to_string())?;
        log::info!("driver: uninstalling WinUSB for {vid:04x}:{pid:04x} (UAC prompt)");
        let status = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&launcher)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("starting PowerShell: {e}"))?;
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        log::info!("driver: uninstall exited {:?}\n{text}", status.code());
        match status.code() {
            Some(0) => Ok(UninstallResult { changed: true, log: text }),
            Some(2) => Ok(UninstallResult { changed: false, log: text }),
            Some(CANCELLED) => Err("cancelled at the Windows admin prompt".into()),
            code => Err(format!("the uninstall failed (exit {code:?}); see the log below\n{text}")),
        }
    }

    #[cfg(test)]
    mod tests {
        use std::path::Path;

        #[test]
        fn launcher_passes_quoted_paths_and_hex_ids() {
            let t = super::launcher_text(Path::new(r"C:\A B\s.ps1"), Path::new(r"C:\A B\s.log"), 0x222A, 0x1);
            assert!(t.contains(r#"'-File','"C:\A B\s.ps1"','-Vid','222A','-ProductId','0001','-Log','"C:\A B\s.log"'"#), "{t}");
            assert!(t.contains("catch { exit 1223 }"), "{t}");
        }

        #[test]
        fn quotes_survive_powershell_strings() {
            assert_eq!(super::ps_quote(r"C:\Users\o'neil\x"), r"'C:\Users\o''neil\x'");
        }
    }
}
