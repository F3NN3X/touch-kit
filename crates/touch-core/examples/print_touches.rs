//! Prints touches and gestures from the first touchscreen found, or from a recording.
//!
//!     cargo run -p touch-core --example print_touches
//!     cargo run -p touch-core --example print_touches -- captures/tap.jsonl

use std::sync::mpsc;
use touch_core::{Config, DeviceMsg, Output, PanelConfig, Recording, TouchDevice};

fn main() -> Result<(), touch_core::Error> {
    for t in touch_core::discover::touchscreens()? {
        println!("{:04x}:{:04x} {} ready={}", t.vid, t.pid, t.product.as_deref().unwrap_or("?"), t.ready);
    }

    // Touches arrive in panel px: tell touch-core the panel's size (and rotation, if mounted turned).
    let config = Config { panel: PanelConfig { width_px: 1920, height_px: 550, ..Default::default() }, ..Default::default() };
    let (tx, rx) = mpsc::channel();
    // Dropping `_device` stops it; keep it alive as long as you want input.
    let _device = match std::env::args().nth(1) {
        Some(path) => TouchDevice::replay(Recording::load(path.as_ref())?, "replay".into(), config.panel, config.gestures, tx),
        None => TouchDevice::spawn(config, Vec::new(), tx),
    };

    for msg in rx {
        match msg {
            DeviceMsg::Status(s) => println!("status {s:?}"),
            DeviceMsg::Output(Output::Pointer(p)) => println!("{:?} #{} at {:.0},{:.0}", p.kind, p.id, p.x, p.y),
            DeviceMsg::Output(Output::Gesture(g)) => println!("{:?} at {:.0},{:.0}", g.gesture, g.x, g.y),
            DeviceMsg::Output(Output::Ignored { .. }) => {}
        }
    }
    Ok(())
}
