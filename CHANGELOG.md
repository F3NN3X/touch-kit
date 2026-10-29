# Changelog

All notable changes to touch-kit. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-10-29

First public release: the panel app as a Windows installer, plus `touchprobe`.

### Added

- **Testing mode** (Settings → Testing, `testing` in config): 1–4 virtual panels as normal windows on
  the main monitor, each playing a recorded capture in a loop through the real pipeline
  (`TouchDevice::replay`), or idle for mouse use. Real panels keep running.
- **Driver page**: each touch device's driver, live; guided Zadig install with the exact USB ID;
  one-click uninstall back to Windows (one admin prompt) that also removes the libwdi certificate.
- **Settings window** (tray, `--settings[=<page>]`, first run): Panels, General, Gestures, Driver and
  Testing pages. Every change is validated before it is written and applies live.
- **Pairing**: "Touch this screen" on a panel's monitor pins whichever touch device is touched, by
  USB port.
- **On-panel calibration**: four targets, fit error shown before saving.
- **Identify**: shows a panel's name on its screen.
- **Multiple panels**: one device thread, bridge and window per `panels` entry; identical devices
  told apart by USB port.
- **Tray icon** with Settings…, Reload config and Quit; the app keeps running without panel windows.
- **Config validation** with errors (file not applied) and warnings (unknown keys, unknown gesture
  actions), shown in the status strip and settings.
- **Error surfaces**: webview errors and failed IPC go to the log and the status strip; an error
  boundary per panel.
- **Rotating log file** (`Touch Panel.log`, 1 MB × 5).
- **Touch Kit UI**: five panels on the 1920 × 550 stage, written against Pointer Events; touch dots;
  debug strip with latency (p50/p95).
- **Hot-reloaded config** with gesture actions (`toggle_debug`, `page`, `emit`, `log`).
- **Tauri panel app**: a window on the panel's monitor that never takes focus and lets the mouse pass
  through; touches reach the webview as synthetic PointerEvents, with motion prediction.
- **touch-core** crate: USB access, HID decode, contact tracking, mapping (rotation, flip, affine
  calibration), gesture recognition, replay and hotplug reconnect.
- **touchprobe** subcommands `gestures` and `replay`.
- Docs: `docs/config.md`, `docs/driver-setup.md`; the design plan moved to `docs/PLAN.md`.

### Changed

- HID report descriptors are parsed with `hidreport` (usage names via `hut`) instead of a
  hand-written parser.
- Driver setup uses `touchprobe list` and the Driver page; the app's uninstall script also runs
  from an elevated shell, replacing the separate inventory and rollback scripts.
- Gesture thresholds default to the Touch Kit spec and are shared by touch-core and the UI kit.
- UI matches the Touch Kit spec more closely: the pressed look shows in every state but disabled,
  carousel dots have 44 px hit targets, and fills and bars animate without layout.

### Fixed

- A freshly enumerated panel (replug, driver swap) stayed in mouse mode and sent no touches: the
  Contact Count Maximum feature is now read on every open, as Windows does.
- Panel rebuilds could deadlock while the settings window was open.
- With testing mode on, the main panel could lose its touch feed when its window loaded before the
  panels finished starting.
- Identify and calibration reached every panel window instead of only the one asked.
- The settings calibration hint stayed on "tap the targets" after calibrating.

[Unreleased]: https://github.com/F3NN3X/touch-kit/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/F3NN3X/touch-kit/releases/tag/v1.0.0
