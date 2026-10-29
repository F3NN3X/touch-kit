//! Configuration. Every struct deserialises with defaults for missing fields. Gesture defaults are the
//! Touch Kit spec; the panel size default is the 1920×550 bar the project started on (docs/PLAN.md §1),
//! and setup in the app replaces it with the panel's monitor size.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    /// USB vendor id. `None` (with `pid`): any touchscreen (see `discover`).
    pub vid: Option<u16>,
    /// USB product id. `None`: any product of `vid`, or any touchscreen.
    pub pid: Option<u16>,
    /// Interface number; `None` picks the first HID interface.
    pub iface: Option<u8>,
    /// Device Mode to send on open, if the descriptor has the feature. `None` skips it.
    pub device_mode: Option<i64>,
    /// Use this report descriptor instead of asking the device.
    pub descriptor_file: Option<PathBuf>,
    /// USB location (`<bus id>-<port chain>`, see `usb::location`) of the device to use, for telling
    /// identical panels apart (many have no serial number). `None` takes the first free match.
    pub port: Option<String>,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self { vid: None, pid: None, iface: None, device_mode: Some(2), descriptor_file: None, port: None }
    }
}

impl DeviceConfig {
    /// Whether a device with these ids fits the config's (optional) ids.
    pub fn accepts(&self, vid: u16, pid: u16) -> bool {
        self.vid.is_none_or(|v| v == vid) && self.pid.is_none_or(|p| p == pid)
    }

    /// `222a:0001`, `222a:*` or `any touchscreen`, for messages.
    pub fn describe(&self) -> String {
        match (self.vid, self.pid) {
            (None, None) => "any touchscreen".into(),
            (v, p) => {
                format!("{}:{}", v.map_or("*".into(), |v| format!("{v:04x}")), p.map_or("*".into(), |p| format!("{p:04x}")))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rotation {
    #[default]
    #[serde(rename = "0")]
    R0,
    #[serde(rename = "90")]
    R90,
    #[serde(rename = "180")]
    R180,
    #[serde(rename = "270")]
    R270,
}

/// One calibration pair: where the digitizer reported a touch (normalised 0..1, after rotation and
/// flip) and where on the panel (px) that touch actually was.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalPoint {
    pub touch: [f32; 2],
    pub panel: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    pub width_px: u32,
    pub height_px: u32,
    /// Clockwise rotation applied to touch coordinates.
    pub rotation: Rotation,
    pub flip_x: bool,
    pub flip_y: bool,
    pub calibration: Option<[CalPoint; 4]>,
}

impl Default for PanelConfig {
    fn default() -> Self {
        // 1920x550 is what Windows reported for DISPLAY4; probably the panel, unconfirmed (docs/PLAN.md §1).
        Self { width_px: 1920, height_px: 550, rotation: Rotation::R0, flip_x: false, flip_y: false, calibration: None }
    }
}

/// Gesture thresholds, in panel px and ms. Defaults are the Touch Kit spec (§2) — the contract
/// for both this recogniser and the UI kit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GestureConfig {
    pub slop_px: f32,
    pub long_press_ms: u64,
    /// 0 disables repeat.
    pub long_press_repeat_ms: u64,
    pub double_tap_gap_ms: u64,
    pub double_tap_slop_px: f32,
    pub lift_debounce_ms: u64,
    pub lift_debounce_px: f32,
    /// A stroke longer than this on its dominant axis is a swipe at any speed.
    pub swipe_min_px: f32,
    /// A shorter stroke still is a swipe if it is longer than this and flicked faster than
    /// `flick_min_speed` (average px/ms over the stroke).
    pub flick_min_px: f32,
    pub flick_min_speed: f32,
    /// Dominant axis must be at least this many times the other (1.0 = just the dominant axis).
    pub swipe_axis_ratio: f32,
    pub multi_finger_window_ms: u64,
    pub pinch_min_scale: f32,
    pub three_tap_max_ms: u64,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            slop_px: 12.0,
            long_press_ms: 500,
            long_press_repeat_ms: 150,
            double_tap_gap_ms: 280,
            double_tap_slop_px: 24.0,
            lift_debounce_ms: 40,
            lift_debounce_px: 120.0,
            swipe_min_px: 60.0,
            flick_min_px: 10.0,
            flick_min_speed: 0.5,
            swipe_axis_ratio: 1.0,
            multi_finger_window_ms: 120,
            pinch_min_scale: 0.15,
            three_tap_max_ms: 300,
        }
    }
}

/// Everything touch-core needs, as one JSON document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub device: DeviceConfig,
    pub panel: PanelConfig,
    pub gestures: GestureConfig,
}

impl Config {
    /// Values serde accepts but the pipeline can't use, prefixed `panel: ` / `gestures: `. Empty means valid.
    pub fn validate(&self) -> Vec<String> {
        let panel = self.panel.validate().into_iter().map(|e| format!("panel: {e}"));
        let gestures = self.gestures.validate().into_iter().map(|e| format!("gestures: {e}"));
        panel.chain(gestures).collect()
    }
}

impl PanelConfig {
    /// Problems that would stop the mapper from being built. Empty means valid.
    pub fn validate(&self) -> Vec<String> {
        match crate::Mapper::new(self) {
            Ok(_) => Vec::new(),
            Err(e) => vec![e.to_string().trim_start_matches("config: ").to_string()],
        }
    }
}

impl GestureConfig {
    /// Thresholds the recogniser can't work with. Empty means valid.
    pub fn validate(&self) -> Vec<String> {
        let g = self;
        let checks = [
            (g.slop_px > 0.0, "slop_px must be > 0"),
            (g.long_press_ms > 0, "long_press_ms must be > 0"),
            (g.double_tap_slop_px >= 0.0, "double_tap_slop_px must be >= 0"),
            (g.lift_debounce_px >= 0.0, "lift_debounce_px must be >= 0"),
            (g.flick_min_px >= 0.0, "flick_min_px must be >= 0"),
            (g.swipe_min_px > g.flick_min_px, "swipe_min_px must be > flick_min_px"),
            (g.flick_min_speed > 0.0, "flick_min_speed must be > 0"),
            (g.swipe_axis_ratio >= 1.0, "swipe_axis_ratio must be >= 1"),
            (g.multi_finger_window_ms > 0, "multi_finger_window_ms must be > 0"),
            (g.pinch_min_scale > 0.0 && g.pinch_min_scale < 1.0, "pinch_min_scale must be between 0 and 1"),
        ];
        checks.iter().filter(|(ok, _)| !ok).map(|(_, msg)| msg.to_string()).collect()
    }
}
