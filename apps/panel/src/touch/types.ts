// Wire format of touch-core's DeviceMsg (locked by crates/touch-core/src/tests/wire.rs).
// Coordinates are panel pixels (physical).

export type Status =
  | { status: "Connected"; interface: number; report_len: number; port: string }
  | { status: "Disconnected" }
  | { status: "Error"; message: string };

export type PointerMsg = {
  type: "Pointer";
  kind: "Down" | "Move" | "Up";
  id: number;
  x: number;
  y: number;
  t_ms: number;
};

export type Dir = "Left" | "Right" | "Up" | "Down";

export type Gesture =
  | { type: "Tap" }
  | { type: "DoubleTap" }
  | { type: "LongPress" }
  | { type: "LongPressRepeat"; n: number }
  | { type: "DragStart" }
  | { type: "DragMove"; dx: number; dy: number }
  | { type: "DragEnd" }
  | { type: "Swipe"; dir: Dir; fingers: number }
  | { type: "PinchStart" }
  | { type: "Pinch"; scale: number }
  | { type: "PinchEnd" }
  | { type: "ThreeFingerTap" };

export type GestureMsg = { type: "Gesture"; gesture: Gesture; x: number; y: number; t_ms: number };

export type DeviceMsg = Status | PointerMsg | GestureMsg | { type: "Ignored"; report_id: number };

export const isStatus = (m: DeviceMsg): m is Status => "status" in m;
