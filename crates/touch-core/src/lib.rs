//! touch-core: read a USB HID touch panel directly (WinUSB / libusb), decode its reports, map them
//! to panel pixels and recognise gestures. No UI dependencies.

pub mod config;
pub mod device;
pub mod gesture;
pub mod hid;
pub mod mapper;
pub mod pipeline;
pub mod replay;
pub mod touch;
pub mod usb;

pub use config::{CalPoint, Config, DeviceConfig, GestureConfig, PanelConfig, Rotation};
pub use device::{DeviceMsg, Status, TouchDevice};
pub use gesture::{Dir, Gesture, GestureEvent, GestureRecognizer, PointerEvent, PointerKind};
pub use mapper::Mapper;
pub use pipeline::{Output, Pipeline};
pub use replay::{Pacing, Recording};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no USB device {vid:04x}:{pid:04x} found (is the touch USB cable plugged in?)")]
    NotFound { vid: u16, pid: u16 },
    #[error("{0}")]
    Usb(String),
    #[error("device disconnected")]
    Disconnected,
    #[error("report descriptor: {0}")]
    Descriptor(String),
    #[error("descriptor has no touch-screen input report")]
    NoTouchReport,
    #[error("config: {0}")]
    Config(String),
    #[error("{path}: {msg}")]
    Fixture { path: String, msg: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests;
