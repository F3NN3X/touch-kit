# Using touch-kit in your own app

The Touch Panel app is one user of a few reusable pieces. Your app can use the same ones to get the
same results: a touchscreen whose input never goes to Windows, so touching it never moves the mouse
or takes focus, plus multi-touch pointer events and gestures in your UI.

| Your app | Use | Get it from |
|---|---|---|
| Rust (any UI, or none) | `touch-core` crate | git dependency |
| .NET / C# (WPF, WinForms, console) | `TouchKit.dll` + `touch_kit.dll` | release assets, or build from source |
| C / C++ / anything with a C FFI | `touch_kit.dll` + `touch_kit.h` | release assets |
| Tauri 2 | `touch-kit-tauri` crate + `@touch-kit/web` package | git dependencies |

All of them wrap the same engine, so the behaviour, config and gesture thresholds are identical
everywhere.

## Concepts

**The driver.** touch-kit reads the touchscreen over raw USB, so Windows must not own it. Each user
switches the device's driver to WinUSB once, with Zadig ([driver-setup.md](driver-setup.md)). Your
app can't do this for them and shouldn't try. List the touchscreens instead (`touchscreens()` /
`Touchscreens.List()`): `ready: false` means the device was found but still has its Windows driver,
so show the user the steps. A running device reports the same case as an `Error` status naming
the touchscreen, rather than `Disconnected`. On Linux, a udev rule does the same job.

**Panel pixels.** Every coordinate is in the panel's *physical* pixels, (0, 0) top-left, after
rotation and flips. Tell touch-core the panel's size (`panel.width_px` / `height_px`, the monitor's
resolution) and, if the digitizer is mounted turned, its `rotation` / `flip_x` / `flip_y`. To place
a touch in your window, convert to your UI's units:

- WPF: divide by the monitor's DPI scale (`VisualTreeHelper.GetDpi(window).DpiScaleX`), if the
  window covers the whole panel monitor.
- Web / Tauri: divide by `devicePixelRatio`. `@touch-kit/web` already does this.

**A window that never activates.** A panel UI must not steal focus from the game or editor the user
is typing in. Give the window `WS_EX_NOACTIVATE`, keep it out of the taskbar, and show it without
activating it. `TouchKit.NoActivate.Apply` and `touch_kit_tauri::window` do exactly this. Real mouse
clicks can still land on the window unless you also make it click-through. The Touch Panel app does
that outside dev mode, since all its input comes from touch-core.

**Devices and ports.** `device.vid` / `device.pid` unset means "any touchscreen". With several
identical panels, pick one by `device.port`, its USB location from the touchscreen list, and pass
the other panels' ports as `taken`. A device reconnects on its own after an unplug; you'll see
`Disconnected`, then `Connected` again.

**Messages.** Each device sends:

- a `Status`: `Connected { interface, report_len, port }`, `Disconnected` or `Error { message }`;
- `Pointer` events: `Down` / `Move` / `Up`, with a contact `id`, `x`, `y` and `t_ms`;
- `Gesture` events: `Tap`, `DoubleTap`, `LongPress`, `LongPressRepeat {n}`, `DragStart`,
  `DragMove {dx, dy}`, `DragEnd`, `Swipe {dir, fingers}`, `PinchStart`, `Pinch {scale}`,
  `PinchEnd` and `ThreeFingerTap`. Each comes with a point: where it started, or the centre for
  multi-finger gestures.

Use pointer events for anything that follows a finger (sliders, drags, drawing). Use gestures for
anything that is a decision (tap a button, swipe to page). Their thresholds are in `gestures`
([config.md](config.md)).

**Threads.** Messages arrive on a background thread. Every binding below says how to get them onto
your UI thread.

**Testing without hardware.** Every entry point can play a recording instead of a USB device: the
files in `captures/`, or your own from `touchprobe record`. It reports `Connected` and loops the
recording at its real pace, so the CI and other developers don't need a panel.

## Rust

```toml
[dependencies]
touch-core = { git = "https://github.com/F3NN3X/touch-kit", tag = "v1.2.0" }
```

```rust
use std::sync::mpsc;
use touch_core::{Config, DeviceMsg, Output, PanelConfig, TouchDevice};

let config = Config {
    panel: PanelConfig { width_px: 1920, height_px: 550, ..Default::default() },
    ..Default::default() // any touchscreen, spec gesture thresholds
};
let (tx, rx) = mpsc::channel();
let device = TouchDevice::spawn(config, Vec::new(), tx); // reads until dropped

for msg in rx {
    match msg {
        DeviceMsg::Status(s) => println!("{s:?}"),
        DeviceMsg::Output(Output::Pointer(p)) => println!("{:?} #{} {},{}", p.kind, p.id, p.x, p.y),
        DeviceMsg::Output(Output::Gesture(g)) => println!("{:?}", g.gesture),
        DeviceMsg::Output(Output::Ignored { .. }) => {}
    }
}
```

- `device.reconfigure(panel, gestures)` applies a new mapping or new thresholds without reopening
  the device.
- `touch_core::discover::touchscreens()` lists devices.
- `TouchDevice::replay(Recording::load(path)?, …)` plays a recording.
- `touch_core::report` builds the same display report as the app.
- `Config` deserialises from the same JSON as the app's panel entries (every field optional), so you
  can keep it in your own config file.

A runnable version: `cargo run -p touch-core --example print_touches [-- captures/tap.jsonl]`. API
docs: `cargo doc -p touch-core --open`.

## .NET / C#

Add the files from a release to your project, or build them: `cargo build --release -p touch-ffi`
and `dotnet build bindings/dotnet/TouchKit -c Release`.

- `TouchKit.dll`: reference it, or reference `bindings/dotnet/TouchKit/TouchKit.csproj` directly.
- `touch_kit.dll`: copy it next to your exe:
  `<None Include="touch_kit.dll" CopyToOutputDirectory="PreserveNewest" />`. It is built for x64,
  so build your app for x64 or AnyCPU (on 64-bit Windows).

```csharp
using TouchKit;

var device = new TouchDevice(new TouchConfig
{
    Panel = new PanelConfig { WidthPx = 1920, HeightPx = 550 },
    // Device = new DeviceConfig { Port = "..." },  from Touchscreens.List()
    // Replay = "captures/tap.jsonl",               no hardware needed
});
device.Status += s => statusText.Text = s.State.ToString();
device.Pointer += p => { /* p.Kind, p.Id, p.X, p.Y in panel px */ };
device.Gesture += g =>
{
    if (g.Type == "Swipe" && g.Dir == "Left") NextPage();
};
// ...
device.Dispose(); // on window close
```

- **Threads:** events are raised on the `SynchronizationContext` that was current when the
  `TouchDevice` was created. Create it on the UI thread (in the window's constructor or `Loaded`) and
  handlers can touch controls directly. In a console app there is no context, so handlers run on a
  library thread and must not call `Dispose`.
- **Config:** a property left null keeps touch-core's default. `Reconfigure(panel, gestures)`
  changes the mapping live. A bad value throws `TouchKitException` with the reason.
- **Never-focused WPF window:**

  ```csharp
  // In the panel window; also set ShowActivated="False" and WindowStyle="None" in XAML.
  protected override void OnSourceInitialized(EventArgs e)
  {
      base.OnSourceInitialized(e);
      NoActivate.Apply(new WindowInteropHelper(this).Handle);
  }
  ```

  Position the window to cover the panel monitor. Touch coordinates ÷ the window's DPI scale are its
  device-independent units.
- **Hit-testing:** turn a pointer into a WPF hit with
  `InputHitTest(new Point(p.X / dpi.DpiScaleX, p.Y / dpi.DpiScaleY))`. For a whole control set, keep
  per-contact state keyed by `p.Id` from `Down` to `Up`.
- `Touchscreens.List()` lists devices. `Touchscreens.CreateReport(seconds)` builds the display
  report JSON; dispose running devices first, because they hold the USB device.

Sample: `bindings/dotnet/TouchKit.Sample` (`dotnet run --project bindings/dotnet/TouchKit.Sample --
captures/tap.jsonl`).

## C / C++

`touch_kit.h` declares eight functions. Strings in and out are UTF-8 JSON (the shapes above, as in
[config.md](config.md)), and every returned string is freed with `tk_free`.

```c
static void on_message(void *user, const char *json) { /* parse json; runs on a library thread */ }

TkDevice *dev = tk_start("{\"panel\": {\"width_px\": 1920, \"height_px\": 550}}", on_message, NULL);
if (!dev) { char *e = tk_last_error(); /* ... */ tk_free(e); }
/* ... */
tk_stop(dev); /* no callback runs after this returns */
```

Don't call `tk_stop` from inside the callback: it waits for the callback thread.

## Tauri 2

There are two halves. The Rust crate opens the device and forwards its messages over a `Channel` to
the panel window. The web package turns them into DOM events, so your UI code is ordinary Pointer
Events code.

```toml
# src-tauri/Cargo.toml
touch-core = { git = "https://github.com/F3NN3X/touch-kit", tag = "v1.2.0" }
touch-kit-tauri = { git = "https://github.com/F3NN3X/touch-kit", tag = "v1.2.0" }
```

```rust
use tauri::{ipc::Channel, Manager, State, WebviewUrl};
use touch_kit_tauri::{window, Bridge, UiMsg};

#[tauri::command]
fn subscribe(bridge: State<'_, Bridge>, channel: Channel<UiMsg>) {
    bridge.subscribe(channel); // also sends the current status straight away
}

tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![subscribe])
    .setup(|app| {
        let bridge = Bridge::new("panel");
        let (tx, rx) = std::sync::mpsc::channel();
        bridge.spawn(rx);
        let config = touch_core::Config::default(); // set panel size etc.
        app.manage(touch_core::TouchDevice::spawn(config, Vec::new(), tx));
        app.manage(bridge);

        let win = window::builder(app, "panel", WebviewUrl::default()).build()?;
        // win.set_position(..)? / win.set_size(..)? to cover the panel monitor
        win.set_ignore_cursor_events(true)?; // optional: the real mouse passes through
        window::show(&win)?;
        Ok(())
    })
```

List the window in a capability (as `apps/panel/src-tauri/capabilities/default.json` does) so it can call the
command.

```sh
pnpm add "github:F3NN3X/touch-kit#v1.2.0&path:packages/touch-kit-web"
```

```ts
import { onDeviceMessage, configureBridge } from "@touch-kit/web";

// configureBridge({ command: "subscribe", onError: (what, e) => console.error(what, e) });
onDeviceMessage((m) => {
  /* raw messages (status, pointer, gesture), if you want them */
});

button.addEventListener("click", play); // a touch tap is a click
slider.addEventListener("pointermove", (e) => { /* e.pointerType === "touch" */ });
el.addEventListener("tp-gesture", (e) => {
  if (e.detail.gesture.type === "Swipe") page(e.detail.gesture.dir);
});
```

The package ships TypeScript source; Vite and other bundlers compile it as part of your app. Once
something calls `onDeviceMessage`, which subscribes:

- each contact becomes `pointerdown` / `pointermove` / `pointerup` on the element under it, with
  `pointerType: "touch"` and `pointerId` = 1000 + contact id;
- a lift within the tap slop also fires `click`;
- gestures arrive as bubbling `tp-gesture` events;
- moves are applied once per animation frame;
- `drawPositions(frameTime)` gives predicted positions, so things that follow a finger don't lag
  behind it.

`setPointerCapture` only works with real pointers. The bridge already keeps each contact's target
from down to up, so wrap the call in try/catch. The Touch Panel app (`apps/panel`) is the complete
example: multi-panel, settings, setup and calibration.
