//! Recogniser edge cases on synthetic pointer streams (panel px, explicit ms), at the Touch Kit
//! spec thresholds: slop 12 px, long press 500 ms, double tap 280 ms / 24 px, swipe > 60 px or a
//! > 10 px flick faster than 0.5 px/ms.

use crate::config::GestureConfig;
use crate::gesture::{Dir, Gesture, GestureRecognizer, PointerEvent, PointerKind};

fn down(id: u32, x: f32, y: f32, t: u64) -> PointerEvent {
    PointerEvent { kind: PointerKind::Down, id, x, y, t_ms: t }
}
fn mv(id: u32, x: f32, y: f32, t: u64) -> PointerEvent {
    PointerEvent { kind: PointerKind::Move, id, x, y, t_ms: t }
}
fn up(id: u32, x: f32, y: f32, t: u64) -> PointerEvent {
    PointerEvent { kind: PointerKind::Up, id, x, y, t_ms: t }
}

/// Feeds the events, ticks to `end`, and returns the discrete gestures (no DragMove/Pinch steps).
fn run(events: &[PointerEvent], end: u64) -> Vec<Gesture> {
    run_with(GestureConfig::default(), events, end)
}

fn run_with(cfg: GestureConfig, events: &[PointerEvent], end: u64) -> Vec<Gesture> {
    let mut r = GestureRecognizer::new(cfg);
    let mut out: Vec<_> = events.iter().flat_map(|e| r.feed(e)).collect();
    out.extend(r.tick(end));
    out.into_iter()
        .map(|g| g.gesture)
        .filter(|g| !matches!(g, Gesture::DragMove { .. } | Gesture::Pinch { .. }))
        .collect()
}

#[test]
fn tap_within_slop() {
    let g = run(&[down(0, 100.0, 100.0, 0), mv(0, 112.0, 100.0, 50), up(0, 112.0, 100.0, 100)], 2000);
    assert_eq!(g, vec![Gesture::Tap]);
    let g = run(&[down(0, 100.0, 100.0, 0), mv(0, 113.0, 100.0, 50), up(0, 113.0, 100.0, 1000)], 2000);
    assert_eq!(g, vec![Gesture::DragStart, Gesture::DragEnd], "13 px is past the slop");
}

#[test]
fn movement_past_slop_is_a_drag_not_a_tap() {
    let g = run(&[down(0, 100.0, 100.0, 0), mv(0, 125.0, 100.0, 50), up(0, 125.0, 100.0, 1000)], 2000);
    assert_eq!(g, vec![Gesture::DragStart, Gesture::DragEnd]);
}

#[test]
fn tap_vs_long_press_boundary() {
    assert_eq!(run(&[down(0, 0.0, 0.0, 0), up(0, 0.0, 0.0, 499)], 2000), vec![Gesture::Tap]);
    assert_eq!(run(&[down(0, 0.0, 0.0, 0), up(0, 0.0, 0.0, 500)], 2000), vec![Gesture::LongPress]);
}

#[test]
fn long_press_cancelled_by_move_before_it_fires() {
    let g = run(&[down(0, 0.0, 0.0, 0), mv(0, 30.0, 0.0, 300), up(0, 30.0, 0.0, 1500)], 3000);
    assert_eq!(g, vec![Gesture::DragStart, Gesture::DragEnd]);
}

#[test]
fn long_press_repeats_until_lift_or_move() {
    // Long press at 500, repeats every 150: 650, 800, 950.
    let g = run(&[down(0, 0.0, 0.0, 0), up(0, 0.0, 0.0, 1000)], 3000);
    let r = |n| Gesture::LongPressRepeat { n };
    assert_eq!(g, vec![Gesture::LongPress, r(1), r(2), r(3)]);

    let g = run(&[down(0, 0.0, 0.0, 0), mv(0, 30.0, 0.0, 700), up(0, 30.0, 0.0, 2000)], 3000);
    assert_eq!(g, vec![Gesture::LongPress, r(1)], "move past slop stops repeats");

    let cfg = GestureConfig { long_press_repeat_ms: 0, ..Default::default() };
    assert_eq!(run_with(cfg, &[down(0, 0.0, 0.0, 0), up(0, 0.0, 0.0, 2000)], 3000), vec![Gesture::LongPress]);
}

#[test]
fn swipe_needs_distance_speed_and_axis() {
    // First step of 30 px at t=10 starts the drag (before long-press can fire), then the rest at `ms`.
    let swipe = |dx: f32, dy: f32, ms: u64| {
        let k = 30.0 / (dx * dx + dy * dy).sqrt();
        run(
            &[
                down(0, 500.0, 300.0, 0),
                mv(0, 500.0 + dx * k, 300.0 + dy * k, 10),
                mv(0, 500.0 + dx, 300.0 + dy, ms),
                up(0, 500.0 + dx, 300.0 + dy, ms),
            ],
            ms + 1000,
        )
    };
    let s = |dir| vec![Gesture::DragStart, Gesture::Swipe { dir, fingers: 1 }, Gesture::DragEnd];
    assert_eq!(swipe(-200.0, 0.0, 300), s(Dir::Left));
    assert_eq!(swipe(200.0, 0.0, 300), s(Dir::Right));
    assert_eq!(swipe(0.0, -200.0, 300), s(Dir::Up));
    assert_eq!(swipe(0.0, 200.0, 300), s(Dir::Down));
    let no = vec![Gesture::DragStart, Gesture::DragEnd];
    assert_eq!(swipe(200.0, 0.0, 2000), s(Dir::Right), "past 60 px commits at any speed");
    assert_eq!(swipe(50.0, 0.0, 80), s(Dir::Right), "flick: 50 px at 0.63 px/ms");
    assert_eq!(swipe(50.0, 0.0, 100), no, "50 px at exactly 0.5 px/ms is not a flick");
    assert_eq!(swipe(60.0, 0.0, 1000), no, "60 px is not past 60, and too slow to flick");
    assert_eq!(swipe(150.0, 140.0, 300), s(Dir::Right), "direction is the dominant axis");
    assert_eq!(swipe(140.0, 150.0, 300), s(Dir::Down));
}

#[test]
fn second_finger_mid_swipe_becomes_two_finger_gesture() {
    // Both fingers then travel left together in 10 px steps (the device moves both per 8 ms frame).
    let mut evs = vec![down(0, 800.0, 300.0, 0), mv(0, 650.0, 300.0, 100), down(1, 650.0, 400.0, 150)];
    for k in 1..=20 {
        let x = 650.0 - 10.0 * k as f32;
        evs.push(mv(0, x, 300.0, 150 + 10 * k));
        evs.push(mv(1, x, 400.0, 150 + 10 * k));
    }
    evs.extend([up(0, 450.0, 300.0, 400), up(1, 450.0, 400.0, 400)]);
    let g = run(&evs, 2000);
    assert_eq!(g, vec![Gesture::DragStart, Gesture::DragEnd, Gesture::Swipe { dir: Dir::Left, fingers: 2 }]);
}

#[test]
fn double_tap_gap_boundary() {
    let taps = |gap: u64| run(&[down(0, 0.0, 0.0, 0), up(0, 0.0, 0.0, 50), down(0, 5.0, 5.0, 50 + gap), up(0, 5.0, 5.0, 100 + gap)], 3000);
    assert_eq!(taps(280), vec![Gesture::Tap, Gesture::Tap, Gesture::DoubleTap]);
    assert_eq!(taps(281), vec![Gesture::Tap, Gesture::Tap]);
}

#[test]
fn triple_tap_is_one_double_tap_then_a_tap() {
    let g = run(
        &[down(0, 0.0, 0.0, 0), up(0, 0.0, 0.0, 50), down(0, 0.0, 0.0, 100), up(0, 0.0, 0.0, 150), down(0, 0.0, 0.0, 200), up(0, 0.0, 0.0, 250)],
        3000,
    );
    assert_eq!(g, vec![Gesture::Tap, Gesture::Tap, Gesture::DoubleTap, Gesture::Tap]);
}

#[test]
fn lift_debounce_joins_a_brief_dropout() {
    let stroke = |redown_at: u64| {
        let mut r = GestureRecognizer::new(GestureConfig::default());
        let evs = [
            down(0, 0.0, 300.0, 0),
            mv(0, 100.0, 300.0, 100),
            up(0, 100.0, 300.0, 100),
            down(0, 150.0, 300.0, redown_at),
            mv(0, 400.0, 300.0, 300),
            up(0, 400.0, 300.0, 300),
        ];
        let mut out: Vec<_> = evs.iter().flat_map(|e| r.feed(e)).collect();
        out.extend(r.tick(2000));
        out.into_iter().filter(|g| matches!(g.gesture, Gesture::Swipe { .. })).map(|g| g.x).collect::<Vec<_>>()
    };
    assert_eq!(stroke(139), vec![0.0], "re-down within 40 ms continues the stroke from x=0");
    // At 40 ms it is two strokes: 0→100 (a swipe on its own, > 60 px) and 150→400.
    assert_eq!(stroke(140), vec![0.0, 150.0]);
}

#[test]
fn lift_debounce_rejects_a_far_redown() {
    let g = run(
        &[down(0, 0.0, 300.0, 0), mv(0, 200.0, 300.0, 100), up(0, 200.0, 300.0, 100), down(0, 1000.0, 300.0, 110), up(0, 1000.0, 300.0, 150)],
        2000,
    );
    assert_eq!(g, vec![Gesture::DragStart, Gesture::Swipe { dir: Dir::Right, fingers: 1 }, Gesture::DragEnd, Gesture::Tap]);
}

#[test]
fn pinch_out_and_in() {
    let mut r = GestureRecognizer::new(GestureConfig::default());
    let evs = [
        down(0, 500.0, 300.0, 0),
        down(1, 600.0, 300.0, 10),
        mv(0, 450.0, 300.0, 100),
        mv(1, 650.0, 300.0, 100), // span 200 → scale 2
        mv(0, 525.0, 300.0, 200),
        mv(1, 575.0, 300.0, 200), // span 50 → scale 0.5
        up(0, 525.0, 300.0, 300),
        up(1, 575.0, 300.0, 300),
    ];
    let out: Vec<_> = evs.iter().flat_map(|e| r.feed(e)).map(|g| g.gesture).collect();
    let scales: Vec<f32> = out.iter().filter_map(|g| if let Gesture::Pinch { scale } = g { Some(*scale) } else { None }).collect();
    assert_eq!(out.first(), Some(&Gesture::PinchStart));
    assert_eq!(out.last(), Some(&Gesture::PinchEnd));
    assert!(scales.iter().any(|&s| (s - 2.0).abs() < 1e-3), "{scales:?}");
    assert!((scales.last().unwrap() - 0.5).abs() < 1e-3, "{scales:?}");
}

#[test]
fn two_finger_drift_is_not_a_pinch() {
    let g = run(
        &[down(0, 500.0, 300.0, 0), down(1, 600.0, 300.0, 0), mv(0, 505.0, 300.0, 100), mv(1, 610.0, 300.0, 100), up(0, 505.0, 300.0, 200), up(1, 610.0, 300.0, 200)],
        2000,
    );
    assert!(g.is_empty(), "{g:?}");
}

#[test]
fn three_finger_tap() {
    let three = |third_at: u64, lift_at: u64| {
        run(
            &[
                down(0, 100.0, 100.0, 0),
                down(1, 200.0, 100.0, 10),
                down(2, 300.0, 100.0, third_at),
                up(0, 100.0, 100.0, lift_at),
                up(1, 200.0, 100.0, lift_at),
                up(2, 300.0, 100.0, lift_at),
            ],
            2000,
        )
    };
    assert_eq!(three(20, 200), vec![Gesture::ThreeFingerTap]);
    assert!(three(20, 301).is_empty(), "held too long");
    assert!(three(121, 250).is_empty(), "third finger outside the multi-finger window");
}

#[test]
fn finger_landing_on_a_held_press_is_swallowed() {
    let g = run(&[down(0, 0.0, 0.0, 0), down(1, 300.0, 0.0, 700), up(1, 300.0, 0.0, 750), up(0, 0.0, 0.0, 800)], 2000);
    // Long press at 500, one repeat at 650, then the second finger lands at 700.
    assert_eq!(g, vec![Gesture::LongPress, Gesture::LongPressRepeat { n: 1 }], "no more repeats, no tap");
}
