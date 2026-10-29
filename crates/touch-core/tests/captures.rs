//! Every recording in `captures/` replays to the gestures that were performed (docs/PLAN.md §7).

use touch_core::{Dir, Gesture, GestureConfig, GestureEvent, Output, PanelConfig, PointerKind, Recording};

fn replay(name: &str) -> Vec<Output> {
    let path = format!("{}/../../captures/{name}.jsonl", env!("CARGO_MANIFEST_DIR"));
    let rec = Recording::load(path.as_ref()).unwrap_or_else(|e| panic!("{e}"));
    rec.outputs(&PanelConfig::default(), &GestureConfig::default()).unwrap()
}

fn gestures(out: &[Output]) -> Vec<&GestureEvent> {
    out.iter().filter_map(|o| if let Output::Gesture(g) = o { Some(g) } else { None }).collect()
}

/// Gestures that mark a decision, without drag/pinch progress and repeats.
fn discrete(out: &[Output]) -> Vec<Gesture> {
    gestures(out)
        .into_iter()
        .map(|g| g.gesture.clone())
        .filter(|g| {
            !matches!(
                g,
                Gesture::DragStart | Gesture::DragMove { .. } | Gesture::DragEnd | Gesture::Pinch { .. } | Gesture::LongPressRepeat { .. }
            )
        })
        .collect()
}

fn swipe(dir: Dir, fingers: u8) -> Gesture {
    Gesture::Swipe { dir, fingers }
}

#[test]
fn tap_corners() {
    let out = replay("tap");
    // The two BL taps are ~290 ms apart: outside the spec's 280 ms double-tap window.
    assert_eq!(discrete(&out), vec![Gesture::Tap; 5]);
    // TL, TR, BR, BL, BL — by panel quadrant (1920x550).
    let quadrants: Vec<(bool, bool)> = gestures(&out)
        .iter()
        .filter(|g| g.gesture == Gesture::Tap)
        .map(|g| (g.x > 960.0, g.y > 275.0))
        .collect();
    assert_eq!(quadrants, vec![(false, false), (true, false), (true, true), (false, true), (false, true)]);
}

#[test]
fn tap_stale_burst_is_drained() {
    // tap.jsonl t=1..10 holds two touches queued before the recording; they must not surface.
    let out = replay("tap");
    let first_down = out.iter().find_map(|o| match o {
        Output::Pointer(p) if p.kind == PointerKind::Down => Some(p.t_ms),
        _ => None,
    });
    assert_eq!(first_down, Some(769));
}

#[test]
fn swipes_four_directions_twice() {
    use Dir::*;
    let s = |d| swipe(d, 1);
    assert_eq!(discrete(&replay("swipes")), vec![s(Left), s(Right), s(Up), s(Down), s(Left), s(Right), s(Up), s(Down)]);
}

#[test]
fn swipe_with_dropout_is_one_stroke() {
    // The first Right swipe lifts for 16 ms at t=2477 and continues from x≈906; it must start at x≈648.
    let out = replay("swipes");
    let right = gestures(&out).into_iter().find(|g| g.gesture == swipe(Dir::Right, 1)).unwrap();
    assert!((right.x - 648.0).abs() < 2.0, "swipe started at x={}", right.x);
}

#[test]
fn two_finger_swipe_then_pinch_out_and_in() {
    let out = replay("two-finger");
    assert_eq!(discrete(&out), vec![swipe(Dir::Left, 2), Gesture::PinchStart, Gesture::PinchEnd]);
    let scales: Vec<f32> = gestures(&out)
        .iter()
        .filter_map(|g| if let Gesture::Pinch { scale } = g.gesture { Some(scale) } else { None })
        .collect();
    let peak = scales.iter().copied().fold(0.0, f32::max);
    let last = *scales.last().unwrap();
    assert!(peak > 3.0, "pinch out peaked at {peak}");
    assert!(last < 0.4 * peak, "pinch in only fell to {last} from {peak}");
}

#[test]
fn holds_are_long_presses() {
    let out = replay("hold");
    assert_eq!(discrete(&out), vec![Gesture::LongPress; 3]);
    assert!(gestures(&out).iter().any(|g| matches!(g.gesture, Gesture::LongPressRepeat { .. })));
}

#[test]
fn every_down_has_an_up() {
    for name in ["tap", "swipes", "two-finger", "hold"] {
        let out = replay(name);
        let count = |k| out.iter().filter(|o| matches!(o, Output::Pointer(p) if p.kind == k)).count();
        assert_eq!(count(PointerKind::Down), count(PointerKind::Up), "{name}");
    }
}

#[test]
fn virtual_device_plays_at_recorded_pace_and_stops() {
    use std::time::{Duration, Instant};
    use touch_core::{DeviceMsg, Status, TouchDevice};
    let path = format!("{}/../../captures/tap.jsonl", env!("CARGO_MANIFEST_DIR"));
    let rec = Recording::load(path.as_ref()).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let start = Instant::now();
    let dev = TouchDevice::replay(rec, "virtual:1".into(), PanelConfig::default(), GestureConfig::default(), tx);
    let first = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(first, DeviceMsg::Status(Status::Connected { ref port, .. }) if port == "virtual:1"));
    let tap = loop {
        match rx.recv_timeout(Duration::from_secs(3)).expect("a tap within 3 s") {
            DeviceMsg::Output(Output::Gesture(g)) if g.gesture == Gesture::Tap => break start.elapsed(),
            _ => {}
        }
    };
    // The first finger lands at t=769 ms in the recording: paced, not instant.
    assert!(tap > Duration::from_millis(700), "tap after {tap:?}");
    let stopping = Instant::now();
    drop(dev);
    assert!(stopping.elapsed() < Duration::from_millis(500), "stop took {:?}", stopping.elapsed());
}
