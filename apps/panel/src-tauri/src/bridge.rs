//! Forwards `TouchDevice` messages to the webview over a Tauri `Channel`, unthrottled: the panel
//! reports at 125 Hz and the webview applies only the latest move per animation frame (agreed
//! after Gate 2; a 120 Hz cap here added 3–4 ms and dropped moves). Each message carries the
//! wall-clock send time so the webview can measure IPC latency.

use serde::Serialize;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;
use touch_core::{DeviceMsg, Status};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UiMsg {
    #[serde(flatten)]
    pub msg: DeviceMsg,
    /// Wall-clock ms (Unix epoch) when the bridge handed the message to the channel.
    pub sent_ms: f64,
}

fn wall_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

#[derive(Default)]
struct Inner {
    /// The panel's name, for the log.
    name: String,
    channel: Option<Channel<UiMsg>>,
    last_status: Option<Status>,
}

/// One per panel: its device's messages go to its window only.
#[derive(Clone, Default)]
pub struct Bridge(Arc<Mutex<Inner>>);

impl Bridge {
    pub fn new(name: &str) -> Self {
        Self(Arc::new(Mutex::new(Inner { name: name.into(), ..Default::default() })))
    }

    /// The device's last status, for settings.
    pub fn status(&self) -> Option<Status> {
        self.0.lock().unwrap().last_status.clone()
    }

    /// A (re)loaded webview subscribes; it gets the current device status straight away.
    pub fn subscribe(&self, channel: Channel<UiMsg>) {
        let mut inner = self.0.lock().unwrap();
        if let Some(s) = &inner.last_status {
            let _ = channel.send(UiMsg { msg: DeviceMsg::Status(s.clone()), sent_ms: wall_ms() });
        }
        inner.channel = Some(channel);
    }

    fn send(&self, msg: DeviceMsg) {
        let mut inner = self.0.lock().unwrap();
        if let DeviceMsg::Status(s) = &msg {
            log::info!("{}: touch status: {s:?}", inner.name);
            inner.last_status = Some(s.clone());
        }
        if let Some(ch) = &inner.channel {
            if ch.send(UiMsg { msg, sent_ms: wall_ms() }).is_err() {
                inner.channel = None;
            }
        }
    }

    pub fn spawn(&self, rx: Receiver<DeviceMsg>) {
        let bridge = self.clone();
        std::thread::Builder::new()
            .name("touch-bridge".into())
            .spawn(move || rx.into_iter().for_each(|m| bridge.send(m)))
            .expect("spawning touch-bridge thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use touch_core::{Output, PointerEvent, PointerKind};

    #[test]
    fn ui_msg_flattens_the_device_message() {
        let p = PointerEvent { kind: PointerKind::Move, id: 0, x: 1.0, y: 2.0, t_ms: 3 };
        let v = serde_json::to_value(UiMsg { msg: DeviceMsg::Output(Output::Pointer(p)), sent_ms: 4.5 }).unwrap();
        assert_eq!(v, json!({"type": "Pointer", "kind": "Move", "id": 0, "x": 1.0, "y": 2.0, "t_ms": 3, "sent_ms": 4.5}));
        let v = serde_json::to_value(UiMsg { msg: DeviceMsg::Status(Status::Disconnected), sent_ms: 1.0 }).unwrap();
        assert_eq!(v, json!({"status": "Disconnected", "sent_ms": 1.0}));
    }
}
