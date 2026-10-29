# Changelog

All notable changes to touch-kit. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- A touchscreen that is plugged in but still on its Windows driver now reports an error naming it
  ("<product> found, but Windows still owns it: switch its driver to WinUSB") instead of
  `Disconnected`, in the app's status strip and through every binding.

### Fixed

- Running Set up panels again for a panel that was already bound to the same touchscreen and
  monitor left it stopped ("not running") until the app restarted.

## [1.2.0] - 2026-10-29

The touch engine as libraries for other apps: Rust, .NET, C and Tauri.

### Added

- **Use touch-kit in your own app** ([docs/integrating.md](docs/integrating.md)):
  - `touch-ffi`: a C ABI over touch-core, built as `touch_kit.dll`, with `touch_kit.h`.
  - `TouchKit.dll`: a .NET 8 wrapper (events on the UI thread, a never-activating window helper for
    WPF and WinForms) and a console sample.
  - `touch-kit-tauri`: the app's never-activating window and webview bridge, as a crate.
  - `@touch-kit/web`: the app's DOM touch bridge, as a package.
  - A `print_touches` example for touch-core.
  - Releases attach `touch_kit.dll`, `touch_kit.h` and `TouchKit.dll`.

### Changed

- The frontend is a pnpm workspace at the repository root (`pnpm-lock.yaml` moved there); the app
  uses `touch-kit-tauri` and `@touch-kit/web` itself.

## [1.1.1] - 2026-10-29

### Fixed

- Display reports could include local file paths (and so the Windows user name) when
  `testing.replay` or a panel's `device.descriptor_file` named a file; they now keep only the file
  name.

## [1.1.0] - 2026-10-29

Any USB touchscreen, one-touch setup, display reports, and a panel UI for any screen size.

### Added

- **Any USB touchscreen**: devices are found by their HID Touch Screen collection instead of a fixed
  vid:pid — on Windows through SetupAPI while Windows still owns them, and by their WinUSB binding
  after the driver swap; on Linux from sysfs. `device.vid`/`device.pid` are optional; unset means any
  touchscreen.
- **Set up panels** (settings → Panels): a circle on every monitor but the main one, each at a
  different distance from its centre; one tap per touch display binds its touchscreen, monitor and
  size, for any number of panels at once, and works with digitizers mounted rotated or mirrored.
- The Driver page lists every touchscreen found, with Zadig steps for each one's own USB ids.
- `touchprobe list` marks touchscreens and whether they're ready; `--vid`/`--pid` are optional.
- **Display report** (settings → Report, `touchprobe report`): one JSON file with the OS, monitors
  and scaling, each touchscreen's USB and HID details (fields with physical ranges and units, touch
  layout, contact count maximum, other reports), the app config, and optionally a recording of real
  touches with derived stats, in replayable `touchprobe record` format. No serials or user names.
  A GitHub issue form takes the file.

- The panel UI fits any display size: one row on bars, a grid on 16:9 and portrait screens, pages
  with tappable dots on small ones; the stage zooms so panels keep their designed size and touch
  targets stay at the 44 px floor where the screen allows it (screens under ~560 px tall get ~42 px).
- `testing.size` (Settings → Testing → Size): virtual panels at other displays' resolutions.

### Changed

- The "no monitor" warning points to Set up panels.
- The debug strip truncates on narrow screens instead of overlapping the touch status.

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

[Unreleased]: https://github.com/F3NN3X/touch-kit/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/F3NN3X/touch-kit/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/F3NN3X/touch-kit/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/F3NN3X/touch-kit/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/F3NN3X/touch-kit/releases/tag/v1.0.0
