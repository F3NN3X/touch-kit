//! The Tauri side of a touch panel, as the Touch Panel app uses it: a window on the panel's monitor
//! that never takes focus ([`window`]), and a [`Bridge`] that forwards a `touch_core::TouchDevice`'s
//! messages to that window's webview over a `Channel`. The webview half is `@touch-kit/web`
//! (packages/touch-kit-web), which turns the messages into DOM pointer events and gestures.
//!
//! Wiring, in the app's `setup`:
//!
//! ```no_run
//! # fn setup(app: &tauri::App) -> tauri::Result<()> {
//! use tauri::{Manager, WebviewUrl};
//! use touch_kit_tauri::{window, Bridge};
//!
//! let bridge = Bridge::new("panel");
//! let (tx, rx) = std::sync::mpsc::channel();
//! bridge.spawn(rx);
//! let device = touch_core::TouchDevice::spawn(touch_core::Config::default(), Vec::new(), tx);
//! app.manage(bridge);
//! app.manage(device); // reads until dropped
//!
//! let win = window::builder(app, "panel", WebviewUrl::default()).build()?;
//! window::show(&win)?;
//! # Ok(())
//! # }
//! ```
//!
//! and the command the webview calls once to subscribe (`@touch-kit/web` calls `subscribe`):
//!
//! ```no_run
//! use tauri::{ipc::Channel, State};
//! use touch_kit_tauri::{Bridge, UiMsg};
//!
//! #[tauri::command]
//! fn subscribe(bridge: State<'_, Bridge>, channel: Channel<UiMsg>) {
//!     bridge.subscribe(channel);
//! }
//! ```
//!
//! docs/integrating.md has the whole walkthrough.

mod bridge;
pub mod window;

pub use bridge::{Bridge, UiMsg};
