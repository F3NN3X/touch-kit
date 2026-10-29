//! App config, in `<app config dir>/config.json`: the panels (each a touch device, a monitor and its
//! mapping), gesture thresholds shared by all panels, and app-only settings.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// One entry per touch panel. Each gets its own touch device thread and window.
    pub panels: Vec<PanelEntry>,
    /// Shared by every panel's recogniser and the UI kit.
    pub gestures: touch_core::GestureConfig,
    /// Lets the real mouse reach the panel windows and opens devtools. Off: mouse passes through.
    pub dev_mode: bool,
    /// How far ahead visual feedback draws a moving contact, ms (0 = off). Hides panel, compositor
    /// and display latency; too high overshoots when the finger stops.
    pub prediction_ms: f64,
    /// White rings under every finger above everything (Touch Kit spec §8). For demos/recordings.
    pub touch_dots: bool,
    /// Debug strip (status, latency, contacts) at start; a ThreeFingerTap toggles it by default.
    pub debug: bool,
    /// Recognised gesture → action. Keys: `Tap`, `DoubleTap`, `LongPress`, `Swipe Left x1`
    /// (direction, finger count), `PinchStart`, `ThreeFingerTap`, ...
    pub actions: BTreeMap<String, Action>,
    /// Virtual panels for testing without hardware.
    pub testing: Testing,
}

/// Testing mode: extra panels in normal windows on the main monitor, each fed by a recording played
/// in a loop instead of a USB device. The real panels keep running alongside.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Testing {
    pub enabled: bool,
    /// How many virtual panels, 1–4.
    pub virtual_panels: u8,
    /// What they play: a built-in capture (`all`, `tap`, `swipes`, `two-finger`, `hold`), `none`
    /// (idle; drive it with the mouse) or a `touchprobe record` file.
    pub replay: String,
}

impl Default for Testing {
    fn default() -> Self {
        Self { enabled: false, virtual_panels: 1, replay: "all".into() }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            panels: vec![PanelEntry::default()],
            gestures: Default::default(),
            dev_mode: false,
            prediction_ms: 16.0,
            touch_dots: false,
            debug: true,
            actions: BTreeMap::from([
                ("ThreeFingerTap".into(), Action::ToggleDebug),
                ("Swipe Left x2".into(), Action::Page { to: PageDir::Next }),
                ("Swipe Right x2".into(), Action::Page { to: PageDir::Prev }),
            ]),
            testing: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Show or hide the debug strip.
    ToggleDebug,
    /// Move the scenes pager.
    Page { to: PageDir },
    /// Dispatch a named DOM event (`kit:<name>`) and log it.
    Emit { name: String },
    /// Only log the gesture.
    Log,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageDir {
    Next,
    Prev,
}

/// What the webview needs from the config. Sent on load and on every hot reload.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UiConfig {
    pub prediction_ms: f64,
    pub dev_mode: bool,
    pub touch_dots: bool,
    pub debug: bool,
    /// The same thresholds the Rust recogniser uses, so the UI kit and touch-core agree.
    pub gestures: touch_core::GestureConfig,
    pub actions: BTreeMap<String, Action>,
    /// Problems found the last time the file was read. With an error, the values above are the
    /// previous config (or the defaults at startup), not the file's.
    pub issues: Vec<Issue>,
}

impl UiConfig {
    pub fn new(c: &AppConfig, issues: Vec<Issue>) -> Self {
        Self {
            prediction_ms: c.prediction_ms,
            dev_mode: c.dev_mode,
            touch_dots: c.touch_dots,
            debug: c.debug,
            gestures: c.gestures.clone(),
            actions: c.actions.clone(),
            issues,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Issue {
    pub level: Level,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// The file was not applied.
    Error,
    /// Applied, but something in it is probably a mistake (e.g. a misspelt key, which serde ignores).
    Warning,
}

impl Issue {
    pub fn error(message: impl Into<String>) -> Self {
        Self { level: Level::Error, message: message.into() }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self { level: Level::Warning, message: message.into() }
    }
}

/// A read of the config file: the config if it can be applied, and everything wrong with it.
#[derive(Debug)]
pub struct Loaded {
    pub config: Option<AppConfig>,
    pub issues: Vec<Issue>,
    /// The starter file was written just now (first run).
    pub created: bool,
}

/// A touch panel: which USB device, which monitor, and how touches map onto it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelEntry {
    pub name: String,
    /// `device.port` pins one of several identical devices (set by pairing).
    pub device: touch_core::DeviceConfig,
    /// Which monitor. Unset: the first free monitor whose physical size equals `panel`'s.
    pub monitor: Option<MonitorMatch>,
    pub panel: touch_core::PanelConfig,
}

impl Default for PanelEntry {
    fn default() -> Self {
        Self { name: "Panel 1".into(), device: Default::default(), monitor: None, panel: Default::default() }
    }
}

impl PanelEntry {
    /// touch-core's config for this panel.
    pub fn touch(&self, gestures: &touch_core::GestureConfig) -> touch_core::Config {
        touch_core::Config { device: self.device.clone(), panel: self.panel.clone(), gestures: gestures.clone() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonitorMatch {
    pub name: Option<String>,
    /// Physical position of the monitor's top-left corner.
    pub position: Option<[i32; 2]>,
}

/// Written on first run. Deliberately not every default: a written default would pin today's value
/// and hide later changes to it. Missing keys take the current defaults.
const STARTER: &str = r#"{
  "dev_mode": false,
  "touch_dots": false
}
"#;

/// Loads the config, writing a starter file first if none exists yet.
pub fn load_or_init(path: &Path) -> Loaded {
    if !path.exists() {
        let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(path, STARTER));
        match written {
            Ok(()) => log::info!("wrote starter config to {}", path.display()),
            Err(e) => return failed(format!("writing starter config: {e}")),
        }
        return Loaded { created: true, ..load(path) };
    }
    load(path)
}

pub fn load(path: &Path) -> Loaded {
    match std::fs::read_to_string(path) {
        Ok(text) => check(&text),
        Err(e) => failed(format!("reading: {e}")),
    }
}

fn failed(message: String) -> Loaded {
    Loaded { config: None, issues: vec![Issue::error(message)], created: false }
}

/// Parses and validates a config file's text.
pub fn check(text: &str) -> Loaded {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => return failed(e.to_string()),
    };
    let cfg: AppConfig = match serde_json::from_value(value.clone()) {
        Ok(c) => c,
        Err(e) => return failed(e.to_string()),
    };

    let mut issues: Vec<Issue> = cfg.gestures.validate().into_iter().map(|e| Issue::error(format!("gestures: {e}"))).collect();
    for (i, p) in cfg.panels.iter().enumerate() {
        let at = format!("panels[{i}] \"{}\"", p.name);
        issues.extend(p.panel.validate().into_iter().map(|e| Issue::error(format!("{at}: panel: {e}"))));
        let same_port = p.device.port.is_some() && cfg.panels[..i].iter().any(|q| q.device.port == p.device.port);
        if same_port {
            issues.push(Issue::error(format!("{at}: device.port is already used by another panel")));
        }
        if cfg.panels[..i].iter().any(|q| q.name == p.name) {
            issues.push(Issue::warning(format!("{at}: another panel has the same name")));
        }
    }
    if cfg.panels.is_empty() {
        issues.push(Issue::warning("no panels configured"));
    }
    if !(0.0..=100.0).contains(&cfg.prediction_ms) {
        issues.push(Issue::error("prediction_ms must be between 0 and 100"));
    }
    if !(1..=4).contains(&cfg.testing.virtual_panels) {
        issues.push(Issue::error("testing.virtual_panels must be between 1 and 4"));
    }
    let replay = &cfg.testing.replay;
    if !crate::testing::BUILT_IN.contains(&replay.as_str()) && replay != "none" && !Path::new(replay).is_file() {
        issues.push(Issue::error(format!("testing.replay: \"{replay}\" is no built-in capture and no file")));
    }
    let defaults = serde_json::to_value(AppConfig::default()).expect("config serialises");
    unknown_keys(&value, &defaults, "", &mut issues);
    for key in cfg.actions.keys().filter(|k| !is_gesture_key(k)) {
        issues.push(Issue::warning(format!("actions: no gesture is called \"{key}\"")));
    }

    let ok = issues.iter().all(|i| i.level == Level::Warning);
    Loaded { config: ok.then_some(cfg), issues, created: false }
}

/// Keys the file has but the config doesn't: serde ignores them, so they are usually typos.
/// Array elements are checked against the default's first element (e.g. `panels[0]`).
fn unknown_keys(file: &serde_json::Value, known: &serde_json::Value, prefix: &str, issues: &mut Vec<Issue>) {
    if let (Some(items), Some(model)) = (file.as_array(), known.as_array().and_then(|a| a.first())) {
        let base = prefix.trim_end_matches('.');
        for (i, item) in items.iter().enumerate() {
            unknown_keys(item, model, &format!("{base}[{i}]."), issues);
        }
        return;
    }
    let (Some(file), Some(known)) = (file.as_object(), known.as_object()) else { return };
    for (k, v) in file {
        let path = format!("{prefix}{k}");
        match known.get(k) {
            None => issues.push(Issue::warning(format!("unknown key \"{path}\" (ignored)"))),
            Some(_) if k == "actions" => {}
            Some(d) => unknown_keys(v, d, &format!("{path}."), issues),
        }
    }
}

/// Matches the webview's `gestureKey`: the gesture's name, or `Swipe <Dir> x<fingers>`.
fn is_gesture_key(key: &str) -> bool {
    const NAMES: [&str; 11] = [
        "Tap", "DoubleTap", "LongPress", "LongPressRepeat", "DragStart", "DragMove", "DragEnd",
        "PinchStart", "Pinch", "PinchEnd", "ThreeFingerTap",
    ];
    if NAMES.contains(&key) {
        return true;
    }
    let mut parts = key.split(' ');
    matches!(
        (parts.next(), parts.next(), parts.next().and_then(|n| n.strip_prefix('x')), parts.next()),
        (Some("Swipe"), Some("Left" | "Right" | "Up" | "Down"), Some(n), None) if n.parse::<u8>().is_ok_and(|n| n > 0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panels_and_partial_files_fill_defaults() {
        let cfg: AppConfig = serde_json::from_str(
            r#"{"dev_mode": true, "gestures": {"slop_px": 20}, "panels": [{"panel": {"rotation": "180"}}, {"name": "Right"}]}"#,
        )
        .unwrap();
        assert!(cfg.dev_mode);
        assert_eq!(cfg.gestures.slop_px, 20.0);
        assert_eq!(cfg.panels.len(), 2);
        assert_eq!(cfg.panels[0].name, "Panel 1");
        assert_eq!(cfg.panels[0].panel.rotation, touch_core::Rotation::R180);
        assert_eq!(cfg.panels[0].panel.width_px, 1920);
        assert_eq!(cfg.panels[1].device.vid, 0x222A);
        assert_eq!(cfg.panels[1].monitor, None);
        assert_eq!(cfg.panels[1].touch(&cfg.gestures).gestures.slop_px, 20.0, "gestures are shared");
        assert_eq!(AppConfig::default().panels, vec![PanelEntry::default()], "no panels key: one default panel");
        assert_eq!(cfg.prediction_ms, 16.0);
        assert_eq!(cfg.actions.get("ThreeFingerTap"), Some(&Action::ToggleDebug));
    }

    #[test]
    fn panel_problems_name_the_panel() {
        let dup = check(
            r#"{"panels": [{"name": "A", "device": {"port": "B-1"}}, {"name": "B", "device": {"port": "B-1"},
                "panel": {"width_px": 0}, "monitr": {}}]}"#,
        );
        let msgs: Vec<_> = dup.issues.iter().map(|i| (i.level, i.message.as_str())).collect();
        assert_eq!(
            msgs,
            [
                (Level::Error, r#"panels[1] "B": panel: panel size must be non-zero"#),
                (Level::Error, r#"panels[1] "B": device.port is already used by another panel"#),
                (Level::Warning, r#"unknown key "panels[1].monitr" (ignored)"#),
            ]
        );
        assert_eq!(dup.config, None);
        // The pre-panels layout still loads, and says where its keys went.
        let old = check(r#"{"panel": {"rotation": "180"}}"#);
        assert!(old.config.is_some());
        assert_eq!(old.issues[0].message, r#"unknown key "panel" (ignored)"#);
    }

    #[test]
    fn actions_parse_from_json() {
        let cfg: AppConfig = serde_json::from_str(
            r#"{"actions": {"LongPress": {"action": "emit", "name": "mute"}, "Tap": {"action": "log"},
                "Swipe Up x1": {"action": "page", "to": "prev"}}}"#,
        )
        .unwrap();
        assert_eq!(cfg.actions.len(), 3, "a file's actions replace the defaults");
        assert_eq!(cfg.actions["LongPress"], Action::Emit { name: "mute".into() });
        assert_eq!(cfg.actions["Swipe Up x1"], Action::Page { to: PageDir::Prev });
        assert!(serde_json::from_str::<AppConfig>(r#"{"actions": {"Tap": {"action": "explode"}}}"#).is_err());
    }

    #[test]
    fn writes_defaults_when_missing_then_reads_them_back() {
        let dir = std::env::temp_dir().join(format!("panel-config-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let _ = std::fs::remove_dir_all(&dir);
        for _ in 0..2 {
            let loaded = load_or_init(&path);
            assert_eq!(loaded.issues, vec![]);
            assert_eq!(loaded.config, Some(AppConfig::default()));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_values_reject_the_file_and_typos_only_warn() {
        let bad = check(r#"{"prediction_ms": -1, "gestures": {"slop_px": 0}}"#);
        assert_eq!(bad.config, None);
        assert_eq!(bad.issues.len(), 2, "{:?}", bad.issues);
        assert!(bad.issues.iter().all(|i| i.level == Level::Error));

        let typo = check(
            r#"{"touch_dot": true, "gestures": {"slop": 5}, "panels": [{"monitor": {"name": "x"}}],
                "actions": {"Swipe Left x2": {"action": "log"}, "Swipe left x2": {"action": "log"}}}"#,
        );
        let msgs: Vec<_> = typo.issues.iter().map(|i| (i.level, i.message.as_str())).collect();
        assert_eq!(
            msgs,
            [
                (Level::Warning, r#"unknown key "gestures.slop" (ignored)"#),
                (Level::Warning, r#"unknown key "touch_dot" (ignored)"#),
                (Level::Warning, r#"actions: no gesture is called "Swipe left x2""#),
            ]
        );
        assert!(typo.config.is_some(), "warnings still apply the file");

        assert_eq!(check("{").config, None);
    }
}
