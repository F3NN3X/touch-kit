//! The JSON shape the panel frontend parses (apps/panel/src/touch/types.ts).

use crate::device::{DeviceMsg, Status};
use crate::gesture::{Dir, Gesture, GestureEvent, PointerEvent, PointerKind};
use crate::pipeline::Output;
use serde_json::json;

fn wire(m: DeviceMsg) -> serde_json::Value {
    serde_json::to_value(m).unwrap()
}

#[test]
fn status_messages() {
    assert_eq!(
        wire(DeviceMsg::Status(Status::Connected { interface: 0, report_len: 64, port: "B-4.2".into() })),
        json!({"status": "Connected", "interface": 0, "report_len": 64, "port": "B-4.2"})
    );
    assert_eq!(wire(DeviceMsg::Status(Status::Disconnected)), json!({"status": "Disconnected"}));
    assert_eq!(
        wire(DeviceMsg::Status(Status::Error { message: "x".into() })),
        json!({"status": "Error", "message": "x"})
    );
}

#[test]
fn output_messages() {
    let p = PointerEvent { kind: PointerKind::Down, id: 1, x: 2.0, y: 3.0, t_ms: 4 };
    assert_eq!(
        wire(DeviceMsg::Output(Output::Pointer(p))),
        json!({"type": "Pointer", "kind": "Down", "id": 1, "x": 2.0, "y": 3.0, "t_ms": 4})
    );
    let g = GestureEvent { gesture: Gesture::Swipe { dir: Dir::Left, fingers: 2 }, x: 1.0, y: 2.0, t_ms: 3 };
    assert_eq!(
        wire(DeviceMsg::Output(Output::Gesture(g))),
        json!({"type": "Gesture", "gesture": {"type": "Swipe", "dir": "Left", "fingers": 2}, "x": 1.0, "y": 2.0, "t_ms": 3})
    );
    assert_eq!(wire(DeviceMsg::Output(Output::Ignored { report_id: 5 })), json!({"type": "Ignored", "report_id": 5}));
}
