# Config reference

The panel app's settings window edits this file for you; everything here can also be written by hand.

`%APPDATA%\dev.touchkit.panel\config.json`. The first run writes a starter file; every key you
leave out takes its current default, so only write what you change. Saving the file applies it
within half a second, everything included: a change to the panel list, a panel's `name`, `device`
or `monitor`, `dev_mode`, or `testing` rebuilds the panel windows; the rest applies in place.

The file is checked on every read. An **error** (bad JSON, a wrong type, an unusable value) keeps
the previous config — or the defaults at startup — and shows `config not applied · …` in red. A
**warning** (an unknown key, which is usually a typo; an action for a gesture that doesn't exist)
applies the file and shows in amber. Both go to the log.

| Key | Default | Meaning |
|---|---|---|
| `dev_mode` | `false` | Real mouse reaches the panels; devtools available. |
| `prediction_ms` | `16` | How far ahead feedback draws a moving finger (0–100, 0 = off). |
| `touch_dots` | `false` | Rings under every finger, for demos and recordings. |
| `debug` | `true` | Debug strip at start; a three-finger tap toggles it. |
| `testing.enabled` | `false` | Testing mode: virtual panels in normal windows on the main monitor, fed by recorded touches in a loop instead of a USB device. Real panels keep running. Settings → Testing. |
| `testing.virtual_panels` | `1` | How many (1–4). |
| `testing.replay` | `"all"` | `all`, `tap`, `swipes`, `two-finger`, `hold` (the `captures/`, built in), `none` (idle; use the mouse), or a `touchprobe record` file path. |
| `testing.size` | `[1920, 550]` | Each virtual panel's size in CSS px (320–3840), to try the panel UI at other displays' resolutions. |
| `actions` | see below | Recognised gesture → action. Replaces the defaults when present. |
| `panels` | one default panel | List of panels; each entry takes the keys below (`panels[0].name`, ...). |
| `…name` | `"Panel 1"` | Shown in settings, the log and Identify. |
| `…monitor` | unset | `{"name": "\\\\.\\DISPLAY4"}` and/or `{"position": [x, y]}`; set by **Set up panels**. Unset: first free monitor of the panel's size. |
| `…device.vid`, `device.pid` | unset | USB ids (decimal in JSON, e.g. 8746, 1 for `222a:0001`). Unset: any touchscreen (a HID Touch Screen collection), any make. Set both to use a device by id even if it doesn't report as one; set only `vid` for any product of that vendor. |
| `…device.port` | unset | USB location (`touchprobe list` shows `port=`). Pins one of several touchscreens; set by **Set up panels** and **Pair**. If the port is gone, the panel takes the only free touchscreen it accepts. |
| `…device.iface` | first HID | Interface number. |
| `…device.device_mode` | `2` | Device Mode to send on open, if the descriptor has one (this panel doesn't). `null` skips. |
| `…device.descriptor_file` | unset | Report descriptor file instead of asking the device. |
| `…panel.width_px`, `height_px` | `1920`, `550` | Panel size touches map to. |
| `…panel.rotation` | `"0"` | `"0"`, `"90"`, `"180"`, `"270"`, clockwise. |
| `…panel.flip_x`, `flip_y` | `false` | Mirror an axis after rotation. |
| `…panel.calibration` | unset | Four `{"touch": [u, v], "panel": [x, y]}` pairs (u, v 0–1 after rotate/flip); an affine fit replaces scaling. |
| `gestures.slop_px` | `12` | Movement that still counts as a tap. |
| `gestures.long_press_ms` | `500` | Hold time for a long press (also hold-to-confirm controls). |
| `gestures.long_press_repeat_ms` | `150` | Repeat while held after a long press; 0 = off. |
| `gestures.double_tap_gap_ms`, `double_tap_slop_px` | `280`, `24` | Max gap and distance between the taps. |
| `gestures.lift_debounce_ms`, `lift_debounce_px` | `40`, `120` | A lift followed this soon and this close by a touch is one contact (hides dropouts). |
| `gestures.swipe_min_px` | `60` | A stroke longer than this is a swipe at any speed. |
| `gestures.flick_min_px`, `flick_min_speed` | `10`, `0.5` | A shorter stroke is a swipe if longer than this and faster than this (px/ms). |
| `gestures.swipe_axis_ratio` | `1.0` | Dominant axis must be this many times the other (≥ 1). |
| `gestures.multi_finger_window_ms` | `120` | Fingers landing within this window form one multi-finger gesture. |
| `gestures.pinch_min_scale` | `0.15` | Span change before a two-finger touch becomes a pinch (0–1). |
| `gestures.three_tap_max_ms` | `300` | Max duration of a three-finger tap. |

Gesture thresholds are the Touch Kit spec's (§2) and are shared: the UI kit's controls use the same
values as touch-core's recogniser.

**Actions.** Keys are gesture names — `Tap`, `DoubleTap`, `LongPress`, `LongPressRepeat`,
`DragStart`, `DragMove`, `DragEnd`, `PinchStart`, `Pinch`, `PinchEnd`, `ThreeFingerTap` — or
`Swipe <Left|Right|Up|Down> x<fingers>`. Values:

```json
"actions": {
  "ThreeFingerTap": {"action": "toggle_debug"},
  "Swipe Left x2":  {"action": "page", "to": "next"},
  "Swipe Right x2": {"action": "page", "to": "prev"},
  "LongPress":      {"action": "emit", "name": "mute"},
  "DoubleTap":      {"action": "log"}
}
```

The first three are the defaults. `emit` dispatches a `kit:<name>` DOM event in the webview;
`log` writes the gesture to the log.
