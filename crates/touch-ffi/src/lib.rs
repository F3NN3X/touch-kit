//! A C ABI over touch-core, built as `touch_kit.dll` (Windows) / `libtouch_kit.so` (Linux), for apps
//! that aren't written in Rust. The C# wrapper in `bindings/dotnet` sits on it; `include/touch_kit.h`
//! declares it for C and C++. docs/integrating.md walks through using it.
//!
//! Everything crosses as UTF-8 JSON, so the interface stays small and doesn't change when a message
//! gains a field:
//! - configs in: touch-core's `Config` (`{"device": .., "panel": .., "gestures": ..}`, every key
//!   optional), plus `"taken"` (ports other panels use) and `"replay"` (a `touchprobe record` file to
//!   play in a loop instead of a USB device, for testing without hardware);
//! - messages out: touch-core's `DeviceMsg`: `{"status": "Connected", ..}`,
//!   `{"type": "Pointer", "kind": "Down", "id", "x", "y", "t_ms"}`, `{"type": "Gesture", "gesture":
//!   {"type": "Swipe", "dir": "Left", "fingers": 1}, "x", "y", "t_ms"}`, `{"type": "Ignored", ..}`.
//!
//! Strings returned by `tk_*` functions are owned by the caller and freed with [`tk_free`]. Errors
//! leave null/false and set a message for [`tk_last_error`] on the calling thread.

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use touch_core::{Config, GestureConfig, PanelConfig, Recording, TouchDevice};

/// Receives each message as a NUL-terminated JSON string, valid only during the call, on a thread of
/// the library's (not the caller's UI thread). `user` is what was passed to [`tk_start`].
pub type TkCallback = extern "C" fn(user: *mut c_void, json: *const c_char);

/// A running device: its thread and the thread that calls the callback.
pub struct TkDevice {
    device: Option<TouchDevice>,
    forward: Option<JoinHandle<()>>,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Start {
    #[serde(flatten)]
    config: Config,
    taken: Vec<String>,
    replay: Option<String>,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn fail(message: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(message.into()));
}

fn owned(s: String) -> *mut c_char {
    // Interior NULs can't occur in serde_json output; replace them anyway rather than fail.
    CString::new(s.replace('\0', " ")).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// # Safety
/// `p` must be null or a NUL-terminated string.
unsafe fn read(p: *const c_char) -> Result<String, String> {
    if p.is_null() {
        return Ok(String::new());
    }
    CStr::from_ptr(p).to_str().map(str::to_string).map_err(|e| format!("not UTF-8: {e}"))
}

/// The library version, e.g. `"1.2.0"`. Static: don't free it.
#[no_mangle]
pub extern "C" fn tk_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// The last error on this thread, or null. Free it with [`tk_free`].
#[no_mangle]
pub extern "C" fn tk_last_error() -> *mut c_char {
    LAST_ERROR.with(|e| e.borrow_mut().take()).map_or(std::ptr::null_mut(), owned)
}

/// Frees a string a `tk_*` function returned. Null is fine.
///
/// # Safety
/// `s` must come from this library and not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn tk_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Every connected touchscreen, as a JSON array of `{vid, pid, port, serial, product, driver,
/// ready}`; `ready` false means its driver setup is still to do. Null on error.
#[no_mangle]
pub extern "C" fn tk_touchscreens() -> *mut c_char {
    match touch_core::discover::touchscreens().map(|t| serde_json::to_string(&t)) {
        Ok(Ok(json)) => owned(json),
        Ok(Err(e)) => {
            fail(e.to_string());
            std::ptr::null_mut()
        }
        Err(e) => {
            fail(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// A device report for every touchscreen (`{"touchscreens": [..], "recordings": [..]}`), recording
/// `record_seconds` of touches on each ready one first (0 = none). Opens the devices itself: stop
/// any running device first. Blocks for the recording. Null on error.
#[no_mangle]
pub extern "C" fn tk_report(record_seconds: u32) -> *mut c_char {
    let run = || -> Result<String, String> {
        let devices = touch_core::report::devices().map_err(|e| e.to_string())?;
        let mut recordings = Vec::new();
        for d in devices.iter().filter(|d| d.ready && record_seconds > 0) {
            let panel = touch_core::report::open(&d.port).map_err(|e| e.to_string())?;
            let duration = Duration::from_secs(record_seconds.into());
            recordings.push(
                touch_core::report::record(&panel, &PanelConfig::default(), duration, || false).map_err(|e| e.to_string())?,
            );
        }
        serde_json::to_string(&serde_json::json!({ "touchscreens": devices, "recordings": recordings }))
            .map_err(|e| e.to_string())
    };
    match run() {
        Ok(json) => owned(json),
        Err(e) => {
            fail(e);
            std::ptr::null_mut()
        }
    }
}

/// Starts reading a touchscreen (see the module docs for `config_json`; null or `{}` = any
/// touchscreen, 1920×550 panel, spec gestures). Every message goes to `callback` until
/// [`tk_stop`]. Returns null on a bad config.
///
/// # Safety
/// `config_json` must be null or a NUL-terminated string; `callback` must stay callable, and `user`
/// valid for it, until `tk_stop` returns.
#[no_mangle]
pub unsafe extern "C" fn tk_start(config_json: *const c_char, callback: TkCallback, user: *mut c_void) -> *mut TkDevice {
    let start = match read(config_json).and_then(|t| {
        if t.trim().is_empty() {
            Ok(Start::default())
        } else {
            serde_json::from_str::<Start>(&t).map_err(|e| format!("config: {e}"))
        }
    }) {
        Ok(s) => s,
        Err(e) => {
            fail(e);
            return std::ptr::null_mut();
        }
    };
    let problems = start.config.validate();
    if !problems.is_empty() {
        fail(problems.join("; "));
        return std::ptr::null_mut();
    }
    let (tx, rx) = mpsc::channel();
    let device = match &start.replay {
        Some(path) => match Recording::load(path.as_ref()) {
            Ok(rec) => TouchDevice::replay(rec, format!("replay:{path}"), start.config.panel, start.config.gestures, tx),
            Err(e) => {
                fail(e.to_string());
                return std::ptr::null_mut();
            }
        },
        None => TouchDevice::spawn(start.config, start.taken, tx),
    };
    struct User(*mut c_void);
    // SAFETY: the caller guarantees `user` stays valid for the callback until `tk_stop`, which joins
    // this thread; what the pointer means is the caller's business.
    unsafe impl Send for User {}
    let user = User(user);
    let forward = std::thread::Builder::new()
        .name("touch-kit-callback".into())
        .spawn(move || {
            let user = user;
            for msg in rx {
                if let Ok(json) = serde_json::to_string(&msg).map(|j| CString::new(j).unwrap_or_default()) {
                    callback(user.0, json.as_ptr());
                }
            }
        })
        .expect("spawning the callback thread");
    Box::into_raw(Box::new(TkDevice { device: Some(device), forward: Some(forward) }))
}

/// Applies new panel geometry (`{"width_px": .., "rotation": "90", ...}`, a touch-core
/// `PanelConfig`) and gesture thresholds (`GestureConfig`) without reopening the device. Null JSON
/// keeps the defaults. False on a bad config.
///
/// # Safety
/// `device` must come from [`tk_start`] and not be stopped; the strings null or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn tk_reconfigure(device: *mut TkDevice, panel_json: *const c_char, gestures_json: *const c_char) -> bool {
    let Some(d) = device.as_ref().and_then(|d| d.device.as_ref()) else {
        fail("no device");
        return false;
    };
    let parse = |p: *const c_char| -> Result<serde_json::Value, String> {
        let t = read(p)?;
        if t.trim().is_empty() {
            Ok(serde_json::json!({}))
        } else {
            serde_json::from_str(&t).map_err(|e| e.to_string())
        }
    };
    let configs = parse(panel_json).and_then(|p| {
        let g = parse(gestures_json)?;
        let panel: PanelConfig = serde_json::from_value(p).map_err(|e| format!("panel: {e}"))?;
        let gestures: GestureConfig = serde_json::from_value(g).map_err(|e| format!("gestures: {e}"))?;
        let problems: Vec<String> = panel.validate().into_iter().chain(gestures.validate()).collect();
        if problems.is_empty() {
            Ok((panel, gestures))
        } else {
            Err(problems.join("; "))
        }
    });
    match configs {
        Ok((panel, gestures)) => {
            d.reconfigure(panel, gestures);
            true
        }
        Err(e) => {
            fail(e);
            false
        }
    }
}

/// Stops the device and frees it. No callback runs after this returns. Null is fine.
///
/// # Safety
/// `device` must come from [`tk_start`] and not be stopped already.
#[no_mangle]
pub unsafe extern "C" fn tk_stop(device: *mut TkDevice) {
    if device.is_null() {
        return;
    }
    let mut d = Box::from_raw(device);
    drop(d.device.take()); // joins the device thread, which drops the channel's sender
    if let Some(t) = d.forward.take() {
        let _ = t.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static GOT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    extern "C" fn collect(user: *mut c_void, json: *const c_char) {
        assert_eq!(user as usize, 42, "user pointer passes through");
        GOT.lock().unwrap().push(unsafe { CStr::from_ptr(json) }.to_str().unwrap().to_string());
    }

    #[test]
    fn a_replayed_device_reports_status_and_touches_then_stops() {
        let path = format!("{}/../../captures/tap.jsonl", env!("CARGO_MANIFEST_DIR")).replace('\\', "/");
        let config = CString::new(format!(r#"{{"replay": "{path}", "panel": {{"width_px": 1920, "height_px": 550}}}}"#)).unwrap();
        let dev = unsafe { tk_start(config.as_ptr(), collect, 42 as *mut c_void) };
        assert!(!dev.is_null(), "{:?}", unsafe { CString::from_raw(tk_last_error()) });
        std::thread::sleep(Duration::from_millis(1500));
        unsafe { tk_stop(dev) };
        let got = GOT.lock().unwrap().clone();
        assert!(got[0].contains(r#""status":"Connected""#), "{got:?}");
        assert!(got.iter().any(|m| m.contains(r#""type":"Pointer""#) && m.contains(r#""kind":"Down""#)));
        assert!(got.iter().any(|m| m.contains(r#""gesture":{"type":"Tap"}"#)), "{got:?}");
        let after = got.len();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(GOT.lock().unwrap().len(), after, "no callbacks after tk_stop");
    }

    #[test]
    fn bad_configs_fail_with_a_reason() {
        let bad = CString::new(r#"{"gestures": {"slop_px": -1}}"#).unwrap();
        assert!(unsafe { tk_start(bad.as_ptr(), collect, std::ptr::null_mut()) }.is_null());
        let msg = unsafe { CString::from_raw(tk_last_error()) };
        assert!(msg.to_str().unwrap().contains("slop_px"), "{msg:?}");
        assert!(tk_last_error().is_null(), "taken once");
        assert_eq!(unsafe { CStr::from_ptr(tk_version()) }.to_str().unwrap(), env!("CARGO_PKG_VERSION"));
    }
}
