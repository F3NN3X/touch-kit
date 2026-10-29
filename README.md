# touch-kit — take touch displays away from Windows

Use a USB touch display — a monitor bar, a panel under the main screen, a small touch screen —
as its own input surface: touches drive the app on that display and never move the cursor, click
Windows, or steal focus from what you're doing on the main screen. The behaviour iCUE approximates
on the Xeneon Edge, done properly, for any USB touchscreen.

How: bind the display's USB touch device to **WinUSB** instead of Windows' HID driver. Windows then
no longer sees a touchscreen at all; the panel app (and `touchprobe`) reads the raw HID reports over
USB and decides what a touch means.

**In your own app.** The engine is a library with Rust, .NET, C and Tauri bindings; see
[docs/integrating.md](docs/integrating.md).

**Which displays.** Any USB touchscreen that reports a standard HID Touch Screen collection (Digitizer
page, usage 0x04) — what every display does for Windows' own touch support. The app finds them by
that, not by make: it reads each device's report descriptor (field layout, contact count, ranges)
and, if needed, switches it to multi-touch mode the way Windows does. Developed and tested on an
ILITEK `222A:0001` panel (1920×550); other displays are expected to work and reports are welcome.
Pens, touchpads and Bluetooth/I²C touchscreens (most laptops' built-in ones) are out of scope.

```
touch-kit/
  crates/touch-core/        library: USB, HID decode, contact tracking, mapping, gestures, replay
  crates/touch-ffi/         C ABI over touch-core (touch_kit.dll + include/touch_kit.h)
  crates/touch-kit-tauri/   Tauri glue: never-activating window, Channel bridge to the webview
  packages/touch-kit-web/   webview half: touch-core messages as DOM pointer events + gestures
  bindings/dotnet/          C# wrapper (TouchKit.dll) and a console sample
  apps/panel/               Tauri v2 app: the panels, settings, setup, and a React UI kit
    src-tauri/              Rust side: panels, config + hot reload, settings, setup, pairing,
                            calibration, driver, testing mode, display report
    src/                    webview: kit/, screens/, settings/ (touch input via @touch-kit/web)
  touchprobe/               Rust CLI over touch-core: list / info / raw / record / touch /
                            gestures / replay / report / set-mode / get-feature
  captures/                 recorded reports + report descriptor; the test fixtures
  docs/                     PLAN.md (device facts, design, thresholds, risks), config.md,
                            driver-setup.md, integrating.md
  CHANGELOG.md
```

## Install

1. Download `Touch.Panel_<version>_x64-setup.exe` (and `touchprobe.exe` if you want the CLI) from
   [Releases](https://github.com/F3NN3X/touch-kit/releases). The installer is not code-signed, so
   Windows SmartScreen asks once: **More info → Run anyway**.
2. Do the [driver setup](docs/driver-setup.md) — the app's Settings → Driver page guides the Zadig
   step too.
3. Start **Touch Panel**; settings open on first run. Press **Set up panels…** and tap the circle
   on each touch display: that binds its touchscreen to its monitor and size. The app lives in the
   tray.

Building your own app on touch-kit? Each release also has `touch_kit.dll`, `touch_kit.h` and
`TouchKit.dll`; see [docs/integrating.md](docs/integrating.md).

## Share a display report

Helping support more displays: in the app, **Settings → Report → Create report…** (keep the touch
recording on and touch the screen as the prompt asks). It saves `touch-kit-report-<date>.json` to
Downloads, then **Open issue form** takes you to the
[Display report](https://github.com/F3NN3X/touch-kit/issues/new?template=display-report.yml) form;
drag the file in.

The report holds the Windows version, monitors and scaling, every touchscreen's USB ids, HID report
layout (fields, ranges, physical size and units, contact count maximum, other reports such as a
mouse mode), the app's settings, and a recording of real touches with what the gesture recogniser
made of it — the recording replays through the same code (`touchprobe replay`), so a display can be
worked on without having it. No serial numbers, user or computer names. Without the app:
`touchprobe report --record 15`.

## Build, run, test

Needs Rust (stable), Node 20+, pnpm, and on Windows the WebView2 runtime (ships with Windows 11).
The .NET wrapper also needs the .NET 8 SDK.

```powershell
cargo test --workspace                           # no hardware needed: replays captures/
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check                          # rustfmt.toml: width 130

pnpm install                                     # the whole workspace, from the repo root
pnpm -r typecheck; pnpm -r test                  # tsc, Vitest (app and @touch-kit/web)
cd apps\panel
pnpm tauri dev                                   # run the panel app

cargo run -p touchprobe -- gestures              # live gestures in the terminal
cargo run -p touchprobe -- replay ..\..\captures\swipes.jsonl --instant
cd ..\..

cargo build --release -p touch-ffi               # touch_kit.dll, for the .NET wrapper
dotnet run --project bindings\dotnet\TouchKit.Sample -- captures\tap.jsonl
```

Close the panel app before `touchprobe` talks to the device: only one process can claim the interface.

`pnpm tauri build` (in `apps\panel`) builds the Windows installer into
`target\release\bundle\nsis\`.

**CI** (`.github/workflows/ci.yml`) runs the format check, clippy, the Rust and frontend tests, and
builds of the app and libraries on every push to `main` and every pull request.

### Releasing

Bump the version in `apps/panel/src-tauri/tauri.conf.json`, the two `package.json` files
(`apps/panel`, `packages/touch-kit-web`), `bindings/dotnet/TouchKit/TouchKit.csproj` and the five
`Cargo.toml` files (`.github/scripts/check-version.ps1` lists them), and the `v<version>` install
lines in `docs/integrating.md`; run `cargo update -w`; move the changelog's Unreleased entries under
the new version; commit, then push an annotated tag:
`git tag -a v1.2.3 -m "touch-kit 1.2.3" && git push origin v1.2.3`. The **Release** workflow checks
that every version matches the tag, tests, builds the installer, `touchprobe.exe` and the libraries
(`touch_kit.dll`, `touch_kit.h`, `TouchKit.dll`), and publishes a GitHub Release with that version's
changelog section as notes.

## The panel app

Each configured panel gets a window, borderless on its monitor (matched by size, 1920×550, unless
its `monitor` says otherwise), which never takes focus and lets the real mouse pass through. Touch
input comes only from the raw USB reports, so Windows never sees it. A tray icon (Touch Kit) opens
**settings** on a left click; its menu has Settings…, Reload config and Quit. The app keeps running
with no panel window open.

**Settings** opens on the main monitor (also with `--settings`, and on first run). Every change is
validated before it is written — a change that would break the config is refused with the reason —
and applies live. **Set up panels…** shows a circle on every monitor but the main one, each at a
different distance from its centre; tapping it on a touch display tells the app which touchscreen
belongs to which monitor (the distance survives a digitizer mounted rotated or mirrored), and the
panel takes that monitor and its size; running it again later is safe. Per panel: **Pair…** shows
"Touch this screen" on the panel's monitor and pins whichever touch device you touch (by USB port);
**Calibrate…** shows four targets on the panel and saves the fit after you check its error;
**Identify** shows the panel's name on it.

**Driver** (settings page) lists every touchscreen found, of any make, with its driver, live. A
device Windows still owns gets step-by-step Zadig instructions with the exact USB ID to pick; it
turns green by itself when Zadig is done, and its panel connects without a restart.
**Uninstall…** gives a device back to Windows after a confirm and one admin prompt, and also removes
the certificate Zadig added for it. The same script also runs from a shell; see the driver setup.

**Status strip** (bottom edge): touch connection state, always (including "<touchscreen> found, but
Windows still owns it" while the driver step is still to do); the first config problem or
webview error, when there is one; latency and thresholds when debug is on (three-finger tap).
A panel that fails to render shows its error in its own place; the others keep working.

**Testing** (settings page) turns on testing mode: one to four virtual panels open as normal windows
on the main monitor, each playing touches recorded on the real panel (`captures/`) in a loop through
the same pipeline as a USB device — or nothing, for clicking around with the mouse. The real panels
keep running. Use it to try several panels, rebuilds and gestures without more hardware.

**Any screen size.** The demo stage arranges its five panels for the display: one row on a bar like
1920 × 550 or 2560 × 720, a grid on 16:9 and portrait screens, pages with dots on small ones. It
zooms so the panels keep their designed size; try sizes with Testing → Size.

**Dev mode** (General page) lets the real mouse reach the panel windows and opens devtools.

**Report** (settings page) exports the display report above.

**Logs**: `%LOCALAPPDATA%\dev.touchkit.panel\logs\Touch Panel.log` (1 MB per file, last 5 kept).
Config problems, touch connects/disconnects, webview errors, gesture actions and a latency summary
every 2 s go there; touch-core's `tracing` events arrive through tracing's `log` bridge.

### Config

`%APPDATA%\dev.touchkit.panel\config.json`, edited by the settings window or by hand; saving applies
it within half a second. Every key, its default and the gesture actions: [docs/config.md](docs/config.md).

## Docs

- [docs/driver-setup.md](docs/driver-setup.md) — inventory, Zadig swap to WinUSB, probing, recording
  captures, Linux, troubleshooting.
- [docs/config.md](docs/config.md) — config file reference.
- [docs/integrating.md](docs/integrating.md) — the same touch input in your own Rust, .NET, C or
  Tauri app.
- [docs/PLAN.md](docs/PLAN.md) — device facts, architecture, gesture state machine, tests, risks,
  and the phase log.
- [CHANGELOG.md](CHANGELOG.md)

## License

[MIT](LICENSE)
