# PLAN — touch panel prototype (Phase 0)

Status: **draft for GATE 0**. Section 1 decoded from the real descriptor and the four captures
(recorded 2026-10-29 after the WinUSB swap).

---

## 1. Device facts

### From the device inventory (taken before the swap)

| Fact | Value |
|---|---|
| USB node | `USB\VID_222A&PID_0001`, behind hub `214B:7250`, **not composite** |
| Collections | Col01 Touch Screen · Col02 vendor `FF00:0001` · Col03 Mouse |
| Panel screen (probably) | `\\.\DISPLAY4`, 1920×550 at (3833, 2178) — only non-standard size; no join key to `IPS0001`, and 550 may be DPI-scaled. Tauri's monitor API settles it in Phase 2. |

### From the descriptor (`captures/report-descriptor.txt`, 850 bytes, interface 0, EP 0x81, 64-byte packets)

| Report | Kind | Size | Content |
|---|---|---|---|
| `0x04` | Input | 64 B | **Touch: 10 finger slots** × 5 B, Scan Time u32, Contact Count u8, 8 B pad |
| `0x05` | Input | 6 B | Mouse fallback (5 buttons, X/Y 0..16384) — **never seen in any capture** |
| `0x03` | In/Out | 64 B | Vendor `FF00:0001` channel — ignore |
| `0x02` | Feature | 2 B | Contact Count Maximum = **10** (read from device) |
| `0x06` | Feature | 257 B | `FF00:00C5` certification blob (THQA) — ignore |

Slot layout (5 bytes): byte 0 = `id` (bits 0–5) | `tip` (bit 6) | const (bit 7); X u16 LE; Y u16 LE.
X and Y both **0..16384** (inclusive; note 16384, not 16383).

- **No Device Mode feature.** Nothing to set; the panel sends touch report `0x04` by default (all
  2 025 captured reports are `0x04`). `TouchDevice` skips the mode step when the usage is absent.
- **Parallel mode, never hybrid.** 10 slots = Contact Count Max, so every frame fits one report and
  the Assembler's hybrid path is never taken on this device (kept, and covered by the synthetic test).
- One report = one USB packet; multi-packet reassembly is not exercised.
- **Cadence:** Scan Time advances 80 units (100 µs) per report = **8 ms / 125 Hz**; host-side gaps
  alternate 6/10 ms. **No reports at all while untouched** → idle reader just times out.
- **Lift:** one final report with that slot's `tip = 0` (contact count still includes it), then silence.
  The existing `Tracker` already turns this into `Up`.

### Hand decode — one report per capture

`04` | slot0 `40 ef3d 6d3a` | slots 1–9 zero | scan `40690100` | cc `01` | pad — from `tap.jsonl` t=1:
`0x40` → id 0, tip 1; X `0x3DEF` = 15855 → 0.968 → **1858 px**; Y `0x3A6D` = 14957 → 0.913 → **502 px**;
scan 0x16940; cc 1.

| Capture | t ms | Slot bytes | Decoded (px at 1920×550) | cc |
|---|---|---|---|---|
| tap | 1 | `40 ef3d 6d3a` | id0 tip1 (1858, 502) | 1 |
| swipes | 710 | `40 4123 251f` | id0 tip1 (1057, 267) | 1 |
| two-finger | 707 | `40 5426 0f11` · `41 4429 d222` | id0 (1149, 146) · id1 (1237, 299) | 2 |
| hold | 997 | `40 e61d c01d` | id0 tip1 (896, 255) | 1 |

### What the captures contain (per stroke, decoded with a throwaway script)

Confirmed by you on 2026-10-29: 5 taps (TL, TR, BR, BL, BL), swipe sequence performed twice,
two-finger = 2-finger swipe left + pinch out + pinch in. These are the Phase 1 test expectations.

| Capture | Strokes | Notes |
|---|---|---|
| tap | TL (9,49) 77 ms · TR (1880,45) 260 ms · BR (1876,539) 435 ms · BL (0,534) 205 ms · BL (3,519) 55 ms | **5 taps**, not 4. Taps last up to **435 ms**. |
| tap, t=1–10 | (1858,502) then (26,545), same id, 1 ms apart, no lift between | **Stale reports** queued before the recording started — see risk 1 |
| swipes | L 537 px/729 ms · R 203+465 px · U 335 px · D 388 px · L · R · U · D | **Sequence performed twice.** The first R has a **16 ms lift** mid-swipe (tip 0 at 2477, re-down at 2493, 55 px further on). Durations 337–729 ms, ≥ 0.5 px/ms. |
| two-finger | 2-finger swipe left ~560 px / 1336 ms (span steady ~170 px) · pinch: 2nd finger lands **55 ms** after 1st, span 80 → 341 (out) → 73 (in) | During pinch-in, finger 1 drops out and re-appears twice (6–12 ms blips) as the fingers converge below ~75 px. |
| hold | 3 holds: 778 / 1081 / 1213 ms, max drift 7 px | clean |

---

## 2. Workspace layout

```
touch-kit/
  Cargo.toml                    [workspace] members = crates/touch-core, touchprobe, apps/panel/src-tauri
  crates/touch-core/
    src/lib.rs                  re-exports
    src/hid.rs                  adapter over hidreport (fields per element, usage names via hut)
    src/touch.rs                Assembler + Tracker; Tracker takes a timestamp instead of Instant::now()
    src/usb.rs                  find/select/claim, GET_DESCRIPTOR, feature reports, interrupt reader
    src/device.rs               TouchDevice: device thread with hotplug; replay (virtual) devices
    src/mapper.rs               normalised → panel px, rotation/flip/affine calibration
    src/gesture.rs              GestureRecognizer
    src/pipeline.rs             Assembler → Tracker → Mapper → Recognizer
    src/replay.rs               .jsonl → pipeline
    src/config.rs               serde config structs + defaults + validation
    tests/captures.rs           replay every fixture → expected gesture sequence
    src/tests/                  synthetic, real-descriptor, mapper, recogniser, config, port selection
  touchprobe/                   thin CLI over touch-core; all existing subcommands + `gestures`, `replay`
  apps/panel/
    package.json, vite.config.ts, src/ (React: touch/, kit/, screens/, components/, settings/)
    src-tauri/src/              lib (tray, startup), panels, window, bridge, config, reload, settings,
                                pairing, calibrate, driver, testing; win.rs is the only cfg(windows) module
    src-tauri/scripts/          uninstall-winusb.ps1 (run elevated by the Driver page)
  captures/                     fixtures (committed); also testing mode's built-in replays
  docs/                         this plan, config.md, driver-setup.md
```

`touchprobe` keeps `clap` (already a dependency; not in the original dependency list but pre-existing).

---

## 3. touch-core public types

```rust
// config.rs — all serde, all with defaults = today's values
pub struct DeviceConfig { vid: u16 /*0x222A*/, pid: u16 /*0x0001*/, iface: Option<u8>,
                          device_mode: Option<i64> /*Some(2)*/, descriptor_file: Option<PathBuf> }
pub struct PanelConfig  { width_px: u32, height_px: u32, rotation: Rotation, flip_x: bool, flip_y: bool,
                          calibration: Option<[CalPoint; 4]> }
pub struct GestureConfig { /* thresholds, §5 */ }

// mapper.rs
pub enum Rotation { R0, R90, R180, R270 }
pub struct Mapper { .. }                    // built from PanelConfig
impl Mapper { pub fn map(&self, x: f32, y: f32) -> (f32, f32) }   // → panel px
// order: rotate → flip → scale to px → optional affine (least-squares fit of 4 point pairs)

// pipeline.rs
pub struct Pipeline { /* Assembler, Tracker, Mapper, GestureRecognizer */ }
impl Pipeline {
    pub fn new(rd: ReportDescriptor, panel: &PanelConfig, g: &GestureConfig) -> Self;
    pub fn feed(&mut self, t: Duration, report: &[u8]) -> Vec<Output>;
    pub fn tick(&mut self, t: Duration) -> Vec<Output>;        // time-driven: long-press, repeat
    pub fn next_deadline(&self) -> Option<Duration>;
    pub fn reconfigure(&mut self, panel: &PanelConfig, g: &GestureConfig);
}
pub enum Output { Pointer(PointerEvent), Gesture(GestureEvent) }
pub struct PointerEvent { kind: PointerKind /*Down|Move|Up*/, id: u32, x: f32, y: f32, t_ms: u64 }

// gesture.rs
pub enum Gesture {
    Tap, DoubleTap, LongPress, LongPressRepeat { n: u32 },
    DragStart, DragMove { dx: f32, dy: f32 }, DragEnd,
    Swipe { dir: Dir, fingers: u8 },        // fingers = 1 or 2
    PinchStart, Pinch { scale: f32 }, PinchEnd,
    ThreeFingerTap,
}
pub struct GestureEvent { gesture: Gesture, x: f32, y: f32 /* start point or centroid */, t_ms: u64 }

// device.rs
pub enum Status { Connected { interface: u8, report_len: usize }, Disconnected, Error(String) }
pub enum DeviceMsg { Status(Status), Output(Output) }
pub struct TouchDevice { .. }               // owns threads; Drop = stop + join
impl TouchDevice {
    pub fn spawn(dev: DeviceConfig, panel: PanelConfig, g: GestureConfig, tx: mpsc::Sender<DeviceMsg>) -> Self;
    pub fn reconfigure(&self, panel: PanelConfig, g: GestureConfig);   // hot reload, via control channel
}

// replay.rs
pub enum Pacing { Realtime, Instant }
pub fn replay(path: &Path, panel: &PanelConfig, g: &GestureConfig, pacing: Pacing) -> Result<Vec<Output>>;
```

Errors: `thiserror` enum in touch-core (`Usb`, `Descriptor`, `NoTouchReport`, `Io`, `Fixture`);
`anyhow` only in the binaries.

Change to existing code: `Tracker::update(&mut self, t: Duration, frame)` — it currently calls
`Instant::now()`, which makes replay and timing tests impossible. Rest of `hid.rs`/`touch.rs` moves as is.

---

## 4. Threading model

```
device thread ── while disconnected: wait for a nusb hotplug arrival of the panel (checked every 100 ms),
   │  retrying open every 1 s regardless (arrival can precede driver readiness)
   │  found: open → set Device Mode if the descriptor has it (this panel: no) → Status::Connected
   │  connected: reader.next(timeout = min(next_deadline, 250 ms))
   │    report → Pipeline::feed(t) ; timeout → Pipeline::tick(t) ; poll control channel (reconfigure/stop)
   │  TransferError::Disconnected → Pipeline::release_all (Ups for live contacts) → Status::Disconnected
   └─► mpsc::Sender<DeviceMsg>   (std mpsc)
```

- One thread, one pipeline: recognition is microseconds, a second hop adds latency for nothing.
- Idle: 4 wakeups/s while connected (the panel sends nothing while untouched), 10/s while
  disconnected → well under 1 % CPU.
- Hotplug: `nusb::watch_devices` (needs `futures-core` as a direct dependency — agreed at Gate 1),
  polled with a no-op waker in the 100 ms wait slices; no executor.
- **Drain on open** (in `Pipeline`, so replay gets it too): reports in the first 50 ms of a session are
  discarded, and draining continues past 50 ms while reports keep arriving < 2 ms after the last
  discarded one (a stale queue drains at USB speed; the device's 8 ms scan makes real reports ≥ 5 ms
  apart). This drops the stale burst at the start of `tap.jsonl` (risk 1). `hold.jsonl`'s one genuine
  1 ms gap mid-hold (t=6769) is untouched because nothing before it was being drained.
- Open failure (e.g. driver not WinUSB) → `Status::Error(msg)`, retried on the 1 s poll; the same
  error is reported once, not every second.
- `t` is monotonic `Duration` since the device session started; replay supplies the recorded `t_ms`.

---

## 5. Gesture state machine

Thresholds (`GestureConfig`, panel px / ms). Since Phase 3 the defaults are the **Touch Kit spec §2**
(handoff from the user, "the contract"); the UI kit reads the same values from the config, so Rust and
UI agree. Everything still passes on the captures (§7), with one changed result noted there.

| name | default | meaning | evidence / source |
|---|---|---|---|
| `slop_px` | 12 | movement under this is still a tap/press | spec; captured taps/holds drift ≤ 9 px |
| `long_press_ms` | 500 | hold without exceeding slop | spec; captured taps ≤ 435 ms, holds ≥ 778 ms |
| `long_press_repeat_ms` | 150 (0 = off) | repeat interval after long-press | |
| `double_tap_gap_ms` / `double_tap_slop_px` | 280 / 24 | second tap window | spec |
| `lift_debounce_ms` / `lift_debounce_px` | 40 / 120 | a re-Down this soon and this close to an Up continues the stroke | 16 ms / 55 px dropout in `swipes` |
| `swipe_min_px` | 60 | longer than this on the dominant axis = swipe at any speed | spec |
| `flick_min_px` / `flick_min_speed` | 10 / 0.5 px/ms | shorter strokes are a swipe if flicked this fast | spec |
| `swipe_axis_ratio` | 1.0 | dominant / other axis (1.0 = just the dominant axis) | spec: "direction = dominant axis" |
| `multi_finger_window_ms` | 120 | 2nd finger within this of the 1st → multi-finger from the start | pinch 2nd finger at +55 ms |
| `pinch_min_scale` | 0.15 | `abs(span/start_span − 1)` to enter pinch | 2-finger swipe span drifts ≤ 6 %; pinch 80→341→73 |
| `three_tap_max_ms` | 300 | max duration of a 3-finger tap | |

There is no maximum tap or swipe duration: taps ran to 435 ms (a tap is "up before long-press fired,
within slop") and swipes to 1.3 s. A lift debounce covers the firmware's short dropouts.

States and transitions:

```
Idle
  Down                         → One{start, t0}
One
  Move > slop                  → Drag            emit DragStart (+DragMove)
  tick ≥ t0+long_press_ms      → Held            emit LongPress, arm repeat
  Up                           → Idle            emit Tap; if previous Tap within gap+slop → also DoubleTap
  2nd Down                     → Multi           (long-press timer cancelled)
Drag
  Move                         →                 emit DragMove
  Up                           → Lifting{deadline = t + lift_debounce_ms}
  2nd Down                     → Multi           emit DragEnd (no swipe) — 2-finger gesture takes over
Lifting
  Down within debounce_px      → Drag            same stroke continues (new contact id is re-mapped)
  tick ≥ deadline / other Down → Idle            emit Swipe{1} if travel/speed/ratio pass, then DragEnd
Held
  tick ≥ next repeat           →                 emit LongPressRepeat{n}
  Move > slop                  → Consumed        repeats stop
  Up                           → Idle
  2nd Down                     → Consumed
Multi{n, start centroid, start span}
  n reaches 3 within window    → Multi(3)
  n==2, |scale−1| ≥ pinch_min  → Pinch           emit PinchStart, Pinch{scale}
  any Up (n==2)                → Consumed or Idle; emit Swipe{2} if centroid travel passes swipe test
  all Up (n==3), ≤ three_tap_max_ms, no travel → Idle  emit ThreeFingerTap
Pinch
  Move                         →                 emit Pinch{scale}   (scale = span/start_span)
  any Up                       → Consumed/Idle   emit PinchEnd
Consumed
  all Up                       → Idle            (swallows the remainder of a mixed gesture)
```

Decisions baked in:
- **Tap fires immediately; DoubleTap fires in addition** on the second tap. Avoids delaying every tap
  by 300 ms; a binding that wants only one of the two binds only one.
- **Long-press is cancelled by moving** past slop before it fires (→ Drag). After it has fired,
  moving just stops repeats.
- **2nd finger mid-swipe** cancels the 1-finger swipe (DragEnd, no Swipe) and becomes a 2-finger gesture.
- **Lift debounce lives in the recogniser only.** Raw `PointerEvent`s pass through undelayed (so a
  button release is never 40 ms late); only the swipe/drag decision waits.
- **Pinch-in blips** (a finger dropping out for 6–12 ms as fingers converge) end the pinch at the
  first Up; `Consumed` swallows the blips until all fingers are up.
- Raw `PointerEvent`s always flow alongside gestures; buttons and the slider use those, not gestures.
  A slow drag past 60 px also qualifies as a swipe, so the UI kit decides per control from raw pointer
  events; Rust gestures only drive the global action map (§6).

---

## 6. Tauri app

### Window — never activatable, never takes the mouse

The window is built in Rust (`apps/panel/src-tauri/src/window.rs`), not in `tauri.conf.json`, so it
is placed before it is ever shown: `visible(false), focused(false), focusable(false)` (tao turns
`focusable: false` into `WS_EX_NOACTIVATE`), `decorations/resizable/shadow(false), skip_taskbar(true)`.

1. Pick the monitor: config `monitor` (`name` and/or physical `position`) → else the first monitor whose
   physical size equals `panel.width_px × height_px` → else the app errors out. It never falls back to
   another screen.
2. `set_position`/`set_size` in physical px to cover it exactly.
3. `win.rs` (cfg(windows)): add `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, drop `WS_EX_APPWINDOW`, then
   `ShowWindow(SW_SHOWNOACTIVATE)`. tao's own `show()` uses `SW_SHOW` once the window exists, and
   that activates.
4. `set_ignore_cursor_events(true)` unless `dev_mode` is on in config (agreed at Gate 0; dev mode
   also opens devtools). In normal mode the real mouse passes through the panel window entirely, so
   no mouse message ever reaches WebView2.
5. Never call `set_focus` / `SetForegroundWindow`. No keyboard handling in the webview.

### Event path

`TouchDevice` → `mpsc` → bridge thread → `tauri::ipc::Channel<UiMsg>` (frontend calls a
`subscribe(channel)` command on load). **Every message is forwarded unthrottled** (decided after Gate 2):
the panel reports at 125 Hz, and a ≤120 Hz cap in Rust measured 3–4 ms extra delay and dropped half the
moves on real swipe timing. Each message carries `sent_ms` (wall clock) for latency measurement.

### Frontend

- `touch/bridge.ts`: CSS px = panel px / `devicePixelRatio`. Each contact becomes **synthetic
  `PointerEvent`s** (`pointerType: "touch"`, `pointerId` 1000 + id) dispatched on the element under the
  finger at down and kept for move/up (implicit capture); a lift within the slop also dispatches
  `click`. The UI kit is written against Pointer Events (Touch Kit spec §1) and works identically with
  the real mouse in dev mode. Nothing leaves the webview. Moves are applied once per animation frame,
  latest wins; device loss sends `pointercancel`.
- `touch/predictor.ts`: per-contact least-squares velocity; visuals that follow a finger draw at frame
  time + `prediction_ms` (default 16) — hides panel/compositor/display latency we can't remove.
- `touch/latency.ts`: p50/p95 of IPC, arrival→frame and frame interval, shown in the status strip and
  logged every 2 s. Measured on the device: our code adds ~3 ms p50 / ~8 ms p95; the panel is 60 Hz.
- UI: the Touch Kit design (handoff `handoff_touch_kit/`), 5 panels on the 1920 × 550 stage, plus a
  status strip with connection state and the numbers above; touch dots per spec §8. Pressed look
  (bg + ring + scale .94) shows in every state but disabled, active and alert included; carousel dots
  have 44 px hit boxes; fills and bars animate as transforms or clips, not widths.
- Config (`<app config dir>/config.json`), **hot-reloaded** by polling mtime every 500 ms
  (`src-tauri/src/reload.rs`): gesture thresholds (to touch-core and the UI), panel mapping,
  `prediction_ms`, `touch_dots`, `debug`, and `actions` — Rust gesture → `toggle_debug` | `page`
  (`next`/`prev`) | `emit` (`kit:<name>` DOM event + log) | `log`. Keys: `Tap`, `LongPress`,
  `Swipe Left x2`, `ThreeFingerTap`, ... `dev_mode`, `monitor`, `device` need a restart.
- Config checks (Phase 4): serde, then `touch_core::Config::validate` (mapper builds, threshold
  ranges, `swipe_min_px > flick_min_px`) and `prediction_ms` 0–100 are **errors** — the file is not
  applied; the previous config (or the defaults at startup) stays. Unknown keys (serde ignores them, so
  they're usually typos) and action keys that name no gesture are **warnings** — applied. Both reach the
  webview as `UiConfig.issues` and the status strip shows the first one; `touchprobe --config` refuses
  an invalid file. Full key list in `docs/config.md`.
- Error surfaces (Phase 4): touch state and config issues in the status strip; webview errors (uncaught,
  unhandled rejection, failed IPC, a panel's render error) go to the log via `webview_error` and to the
  strip; an error boundary per panel keeps one broken panel from blanking the stage. Log file:
  `<app log dir>/Touch Panel.log`, 1 MB × 5; touch-core's `tracing` reaches it through tracing's `log`
  feature (no tracing subscriber is installed).

---

## 7. Tests (no hardware)

- Existing synthetic descriptor tests move with the code.
- `tests/captures.rs`: each fixture through `replay(.., Pacing::Instant)` → assert gesture sequence
  (from what was performed, cross-checked with the decode in §1):
  - `tap` → Tap TL, TR, BR, BL, BL. The two BL taps are ≈ 290 ms apart (first Up t=7489 → second Down
    t≈7780): a DoubleTap under the original 300 ms window, not under the spec's 280 ms.
  - `swipes` → Swipe{1} L, R, U, D, L, R, U, D — the R with the 16 ms dropout must come out as **one**
    swipe (this is the lift-debounce test on real data).
  - `two-finger` → Swipe{2} L, then PinchStart, Pinch… with scale peaking > 3 (span 80→341) then falling below 0.4 × peak (→73), PinchEnd;
    no Tap/Swipe from the pinch-in blips.
  - `hold` → 3× LongPress, with repeats, no Tap.
- Hybrid assembly: real descriptor is parallel-only, so this stays on the synthetic descriptor.
- Real-descriptor decode: the four hand-decoded reports in §1 decode to exactly those values.
- Mapper: each rotation × flip on the four corners; affine fit recovers a known transform.
- Recogniser (synthetic event streams with explicit times): tap vs drag at slop±1, tap vs long-press
  at long_press_ms±1, long-press cancelled by move, repeat cadence, 2nd finger mid-swipe, double-tap
  gap, lift debounce at debounce_ms±1.

---

## 8. Risks / unknowns

1. **Stale reports on open.** `tap.jsonl` starts with 10 reports 1 ms apart (t=1–10) holding two
   touches from *before* the recording, with the same contact id and no lift between them — the
   Tracker would read that as an 1832 px Move. Handled by drain-on-open (§4). For the fixture: replay
   applies the same drain rule, so the file stays untouched.
2. **Short lifts mid-stroke** (16 ms in `swipes`) and **finger merge on pinch-in** — handled in the
   recogniser (§5); thresholds may need tuning on more data.
3. ~~Panel resolution~~ — settled in Phase 2: Tauri reports `\\.\DISPLAY4` as 1920×550 physical at
   scale 1.0.
10. **IPC custom protocol falls back to postMessage** in dev runs (`IPC custom protocol failed … Failed
   to fetch`, then a stale-callback warning). Messages do arrive (status chip turns green). Seen only
   after a dev-server restart under a running app; a clean start has none (checked again in Phase 4),
   so the measured latency is the custom protocol's.
4. **WebView2 focus at creation** — it may grab focus when the controller is created even with the
   window hidden. Checklist item in Phase 2; fallback is creating the window off-screen-hidden and
   restoring foreground is *not* allowed (no `SetForegroundWindow`), so if this bites I'll come back
   with options rather than work around it silently.
5. **`set_ignore_cursor_events` side effect**: you can't use the mouse on the panel at all (incl.
   devtools clicks). Intended; `dev_mode` turns it off.
6. **Hotplug on Windows** relies on nusb's arrival events for a WinUSB-bound device; untested until the
   Gate 2 unplug check. The 1 s retry covers it if events don't arrive.
7. **Latency < 20 ms**: USB poll (bInterval, likely 1–8 ms) + IPC (~1–3 ms) fit; display vsync adds up
   to 16.7 ms on top, outside our control. I'll measure Rust-receipt → JS-dispatch (wall clock both
   sides) and report that number, not touch-to-photon.
8. **Mouse mode at power-up — found and handled in Phase 5.** After an uninstall + Zadig reinstall
   the panel sent only report 0x05 (6-byte absolute mouse: buttons, X, Y) and the app saw no touch.
   It switched to multi-touch reports 0x04 right after one GET_REPORT of feature 0x02 (Contact Count
   Maximum, reads `02 0a`), which Windows does when it sets up a touchscreen. At the first Zadig swap
   Windows had already done that read and the panel kept the mode; a re-enumeration (driver swap,
   probably also unplug/replug) resets it. `usb::open` now reads Contact Count Maximum on every open.
   The panel still has no Device Mode feature.
9. **Contact id reuse**: ids restart at 0 per touch on this device (always 0/1 in the captures);
   Tracker keys by id and relies on the tip=0 lift report, which every capture has.
11. **Linux** is coming but untested by either of us (no Linux machine). The rules stay: no
   Windows-only code outside `cfg(windows)`; on Linux the "driver" page becomes a udev rule + kernel
   driver detach (nusb `detach_and_claim_interface`), no install/uninstall.

---

## 9. Phase 5 — panels, settings, driver, testing (built)

Scope: the prototype's code is ported into the real app later, so no sensor widgets or widget/theme
system here. Built for N panels, tested with the one we have.

### 5a. hidreport + hut (built)

`hidreport` 0.6 (MIT; pulls `hut` 0.5 and `thiserror` 2) replaces the descriptor parsing in
`hid.rs`. `hid.rs` shrinks to an adapter: hidreport's input/feature reports and fields → the existing
touch layout (slots, tip/id/X/Y, contact count, scan time, Device Mode / Contact Count Max features).
`touch.rs` (per-finger grouping, multi-report frames, tracking) is unchanged. Proof: the
real-descriptor, synthetic and capture tests pass **unchanged**.
Done: a one-off test compared every field element against the old parser on the real and synthetic
descriptors (then deleted with the old parser). One difference, fixed in the adapter: hidreport reads
Logical Maximum as signed, so a 1-byte `0xFF` with minimum 0 came out as -1; with a non-negative
minimum it is now read unsigned within the field's width (Linux does the same). Usage names in
`touchprobe info` now come from `hut`.

### 5b. Multi-panel (built)

- Identity: the panel has **no USB serial** (`touchprobe list`: `serial=-`). A device is `vid:pid` +
  `usb::location` = `bus_id` + port chain (here `…USBROOT(0)-4.2`), stable while it stays in the
  same port. `usb::select`: a panel's own port if present; else, if it names no port, the first port
  no other panel names; if its port is gone, the only such free port (moved panel), else it waits.
  Caveat: two panels that **both** name no port race for the same device; pairing (5d) sets ports.
- Config: `panels: [{ name, device: {vid, pid, port, …}, monitor, panel: {rotation, flip,
  calibration, …} }]`; `gestures`, prediction, actions, debug, dev_mode stay global. No legacy
  fallback: the old top-level `device`/`panel`/`monitor` load as "unknown key" warnings.
  Validation names the panel (`panels[1] "B": …`), rejects a port two panels share, warns on duplicate
  names and on an empty list; unknown keys are checked inside array elements too.
- Runtime (`src-tauri/src/panels.rs`): per panel a `TouchDevice` (told the other panels' ports), a
  `Bridge` and a window labelled `panel-<generation>-<index>`; `subscribe` finds the panel by the
  calling webview's label. Monitors already used by a panel are skipped. A change to the panel list,
  a panel's name/device/monitor, or `dev_mode` rebuilds all panels; mapping and thresholds go to the
  running devices. **Nothing needs a restart any more.** Runtime problems (no free monitor, ...) join
  the config issues in the status strip.
- Tray icon (Tauri `tray-icon`): Reload config, Quit (Settings joins in 5c). The app stays alive
  with no panel window; only Quit exits.
- Verified live: rename → rebuild, second panel without a monitor → warning only, threshold change →
  same window handle; the foreground window never changed and the windows kept
  NOACTIVATE|TOOLWINDOW|TRANSPARENT. The Tauri capability now matches `panel-*`.

### 5c. Settings window (built)

A normal, focusable window on the main monitor (`settings.rs`, label `settings`, route `#settings`),
opened by a left click on the tray icon, the tray's Settings…, `--settings` on the command line, and
on first run (when the starter config is written). Same React bundle, mouse-first Tailwind styling,
no component library. Pages: **Panels** (status, Identify, Pair, Calibrate, name, monitor, USB port
pin/unpin, rotation, mirror, size, add/remove), **General** (prediction, touch dots, debug, dev mode),
**Gestures** (every threshold, spec default shown). Each row has a reset that deletes the key.
- Writes: `config_set(key, value | null)` → `edit_config`: dotted path, one key, `null` deletes;
  the result is run through `config::check` first and **refused with the errors if it wouldn't
  apply** (shown in a banner; nothing written). Write-then-rename so the reload never reads half a
  file. Keys come out sorted (serde_json without `preserve_order`).
- Reads: `config_get` = the file as written + the config in effect + the defaults (for "is set" and
  reset); re-read on every `ui-config` event. `panels_status` is polled each second.
- Panel list edits: with no `panels` key the file means one default panel, so the first panel edit
  writes `[{}]` first (same config), then the key.
- Identify: the panel's name full-screen on its own window for 2.5 s.

### 5d. Pairing + calibration (built)

- **Pair** (`pairing.rs`): needs the panel running (it has a monitor). Stops all panels, opens a
  no-activate "Touch this screen" window (`pair`, route `#pair/<name>`) on that monitor, and a device
  thread per connected WinUSB device with the panel's vid:pid. The first finger down wins: its port
  goes to `panels.<i>.device.port` and is removed from any other panel that had it. Cancel from
  settings or 30 s timeout. A file change restarts the panels through the hot reload; no change →
  restarted directly. Result comes back as a `pair-result` event.
- **Calibrate** (`calibrate.rs` + `components/Calibrate.tsx`): runs on the panel itself. The device
  is reconfigured without its calibration (`reconfigure_one`, no rebuild), so the webview converts a
  finger's px back to the raw touch (`u = x / (w − 1)`). Four targets at 10 % insets; then
  `touch_core::mapper::calibration_error` (RMS px of the affine fit) → good ≤ 6 px, usable ≤ 20 px,
  else "probably a missed target"; Save / Again / Cancel on the panel. Save writes
  `panels.<i>.panel.calibration`; Cancel restores the configured mapping. Clearing = the row's reset.

### 5e. Driver: guided Zadig install, automated uninstall (built)

Decided against bundling libwdi: no prebuilt `wdi-simple.exe` exists, building it needs MSBuild + the
WDK 8 redistributable, and we'd ship an unsigned exe that runs as admin. Zadig is signed, maintained
by libwdi's author, and fetched by the user from the official site.
- **Check** (live, via nusb hotplug): every `222A:0001` device with its driver — *WinUSB · ready*,
  *Windows HID · needs setup*, *not found*. Turns green the moment Zadig finishes, no restart. Warns if
  the device shows up composite or on another interface than the config's.
- **Install = guided Zadig**: link to zadig.akeo.ie, then the steps with this panel's exact values
  (Options → List All Devices → **ILITEK-TP**, USB ID **222A 0001** shown big to compare against →
  target **WinUSB** → Replace Driver), and the troubleshooting table from `docs/driver-setup.md`.
- **Uninstall = automated** (built by me, **run only by you**): one button, one UAC prompt, runs the
  logic of the original rollback script (only an `oem*.inf` that names this vid:pid is deleted; remove
  node; rescan) **plus** removing `CN=USB\VID_222A&PID_0001 (libwdi autogenerated)` from LocalMachine
  `Root` and `TrustedPublisher` (Zadig adds one per install and never removes it). Confirm dialog
  first; the app re-reads the driver state afterwards.
- For the real app later: an attestation-signed WinUSB INF installed with `pnputil` (needs an EV cert
  and a Partner Center account) removes the self-signed cert entirely; otherwise this flow carries over.

Built as planned (`driver.rs`, `settings/DriverPage.tsx`, `src-tauri/scripts/uninstall-winusb.ps1`):
- `driver_status` lists every device with a configured panel's vid:pid and its driver; the page
  re-reads it every 2 s. Not on WinUSB → the Zadig steps with the exact USB ID; "Open zadig.akeo.ie"
  goes through a fixed-URL command (official `tauri-plugin-opener`, Rust side only, so the webview
  can't open other URLs).
- `driver_uninstall` (async): stops the panels (they hold the device), writes the script and a
  launcher to the app's local data dir (per user, not %TEMP%, since it runs elevated), runs
  `Start-Process -Verb RunAs -Wait` (one UAC prompt), reads the transcript back, restarts the panels.
  Exit 0 done, 2 nothing bound, 1223 prompt declined, else failed with the log.
  Unlike the original rollback script (since removed), it has no prompts, checks pnputil's
  exit codes, and removes the libwdi certificate for exactly this vid:pid.
- Verified by me: both scripts parse; the launcher's quoting has a unit test. **Not run by me** (it
  changes a driver): Gate 5.3 is yours.
- `--settings=<page>` / `#settings/<page>` open settings on a page.

### 5f. Spec polish and testing mode (built)

- Touch Kit spec deviations from Phase 3 closed: the pressed look (bg + ring + scale .94) shows in
  every state but disabled, active and alert included; carousel dots have 44 px hit boxes; hold
  fills, slider fill and zoom bar animate as transforms or clips, not widths. The settings
  calibration hint clears when calibration is saved or cancelled (`calibrate-end` event).
- **Testing mode** (`testing` in config, Settings → Testing): 1–4 virtual panels as normal windows
  on the main monitor (panel size in CSS px, stacked), each a `TouchDevice::replay` playing a capture
  in a loop through the real pipeline (`all`/`tap`/`swipes`/`two-finger`/`hold`, built in; `none`; or
  a file). Real panels keep running. Virtual panels have no config entry, so no pairing, calibration
  or identify.
- Found on the way:
  - `Panels::start` built windows while holding its lock; the settings window's sync
    `panels_status` poll (main thread) then waited on the lock while the build waited on the main
    thread. Windows are now built without the lock, and a window that subscribes before its panel is
    registered has its channel held (`early`) and attached once the start finishes.
  - Tauri v2's global JS `listen` also hears events sent with `emit_to` to *other* windows, so
    Identify and Calibrate reached every panel. Panel-targeted events are listened to with
    `getCurrentWebviewWindow().listen`; any new one must be too.

### Gates

- **Gate 5.1** after 5a + 5b (passed): tests unchanged and green; one panel works as before from a
  `panels` config and from an old config; tray quits.
- **Gate 5.2** after 5c + 5d (passed): settings edits apply live; pairing + calibration on the real
  panel.
- **Gate 5.3** after 5e (passed): you uninstall from the app → touch goes to Windows → follow the
  in-app Zadig steps → the check turns green and touch comes back to the app. Surfaced risk #8
  (mouse mode at power-up).
- **5f** (passed): virtual panels play and rebuild with settings open; main panel touch and Identify
  per panel confirmed by you with testing mode on.
