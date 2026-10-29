# Driver setup

Each step ends with a **gate** — don't go on until it passes. The panel app's **Settings → Driver**
page walks through step 2 too, with this panel's exact USB ID, and turns green when it is done.

---

## Step 1 — Find the panel (read-only)

```powershell
cd touch-kit\touchprobe
cargo build --release
$tp = "..\target\release\touchprobe.exe"   # workspace target dir
& $tp list
```

Check:
- A line marked `touchscreen, needs the WinUSB driver`: your display's touch device, with its USB
  ids (`vvvv:pppp`, e.g. `222a:0001`) and `port=`. Settings → Driver in the app shows the same.
- With several touch displays, note which ids and port belong to which.
- The panel's resolution: Windows display settings, or later the monitor list in the panel app's
  Settings → Panels.

**Gate 1:** you know the touchscreen's USB ids.

## Step 2 — Swap the driver to WinUSB (manual, deliberate)

Use Zadig (https://zadig.akeo.ie — the official site only).

1. Run Zadig. **Options → List All Devices**. Untick **Ignore Hubs or Composite Parents**.
2. In the dropdown pick the entry whose **USB ID matches your touchscreen's ids from step 1**
   (e.g. `222A 0001`; the Driver page shows them big).
   Double-check the USB ID field — choosing your keyboard or mouse here is the classic mistake.
   If Zadig lists `(Interface N)` entries for it, the device is composite: pick the interface that
   owns the touch collections.
3. Target driver (right side, after the arrow): **WinUSB**. Click **Replace Driver**.
4. Windows touch on that monitor stops working. That's the point.

**Gate 2:** `& $tp list` marks it `touchscreen, ready` (Settings → Driver turns green). Repeat
steps 1–2 for every touch display.

> Undo at any time: Settings → Driver → **Uninstall…** in the panel app (one admin prompt). Without
> the app, from an elevated PowerShell in the repo root:
> `powershell -ExecutionPolicy Bypass -File apps\panel\src-tauri\scripts\uninstall-winusb.ps1 -Vid <vvvv> -ProductId <pppp> -Log uninstall.log`
> removes the WinUSB binding and Zadig's certificate for this device only.
> Zadig binds by VID/PID, so the swap survives reboots. If Windows Update ever rebinds the HID
> driver, just rerun Zadig.

## Step 3 — Probe

```powershell
& $tp info --save ..\captures # decodes the HID report descriptor, saves .bin + .txt
& $tp raw --count 20          # touch the panel: hex reports should scroll
& $tp touch --px 1920x480     # use YOUR panel resolution: DOWN / MOVE / UP per finger
```

What to look for in `info`:
- `touch report 0x..: N slots/report, contact count field: yes` → multi-touch layout found.
- X/Y logical ranges (e.g. 0..16383) — the probe normalises these to 0..1.
- `device mode feature: report 0x..` → if touches arrive as *mouse* reports
  (`touch` prints `note: report 0x.. belongs to 'GD Mouse'`), run `& $tp set-mode 2`
  or `& $tp touch --set-mode`. Windows used to send that for you; now you own it.

Corner check: touch each corner. Top-left should be ~`x=0 y=0`, bottom-right ~`x=W-1 y=H-1`.
If axes are swapped or mirrored, note it — the app gets a rotation/flip setting.

**Gate 3:** `touch` shows clean DOWN/MOVE/UP for 1, 2 and 3 fingers, and the Windows cursor never moves.

## Step 4 — Record fixtures

```powershell
mkdir ..\captures -Force
& $tp record --out ..\captures\tap.jsonl --seconds 10          # single taps in each corner
& $tp record --out ..\captures\swipes.jsonl --seconds 15       # left/right/up/down swipes
& $tp record --out ..\captures\two-finger.jsonl --seconds 15   # 2-finger swipe + pinch
& $tp record --out ..\captures\hold.jsonl --seconds 10         # long presses
```

Each file starts with the report descriptor, then one timestamped report per line.
These become the test fixtures for the gesture recogniser, and testing mode's built-in replays.

**Gate 4:** `captures/` has `report-descriptor.txt` + the four recordings.

---

## Linux / Fedora

Untested so far (no Linux machine yet); the code builds for Linux and keeps Windows-only parts
behind `cfg(windows)`.

No driver swap: `touchprobe` detaches the kernel `usbhid` driver while it runs (needs root or a
udev rule granting your user access to the touchscreen's `vvvv:pppp`). The kernel driver reattaches
on replug. Touchscreens are found from `/sys/bus/hid/devices/*/report_descriptor`.

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `claiming interface 0 ... not bound to WinUSB` | Step 2 not done, or wrong device swapped in Zadig |
| `GET_DESCRIPTOR(report) failed` | Rare WinUSB quirk. Rollback, dump the descriptor with USBTreeView, save as `.bin`, pass `--descriptor file.bin` |
| `raw` shows nothing when touching | Device may be in mouse mode or idle → `set-mode 2`; also try `--iface 1` |
| Connected, but touches do nothing | Panel stuck in mouse mode (sends only report 0x05). The app reads Contact Count Maximum on every open, which switches it; unplug/replug or restart the app. `touchprobe get-feature 2` does the same by hand |
| Only one finger ever reported | Device Mode is 0/1 → `set-mode 2` |
| Coordinates jump by full screen | Axes swapped / logical max wrong — share `report-descriptor.txt` |
