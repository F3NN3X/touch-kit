//! Gesture recogniser: a state machine over pointer down/move/up in panel px with timestamps.
//!
//! Time-driven transitions (long-press, repeat, lift debounce) happen in `tick`; callers drive it
//! with `next_deadline`. `feed` ticks up to the event's time first, so replays need no extra calls.
//! States and thresholds are described in docs/PLAN.md §5.

use crate::config::GestureConfig;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PointerKind {
    Down,
    Move,
    Up,
}

/// One raw contact event, already mapped to panel px.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub t_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Gesture {
    Tap,
    DoubleTap,
    LongPress,
    LongPressRepeat { n: u32 },
    DragStart,
    /// Offset from the drag's start point.
    DragMove { dx: f32, dy: f32 },
    DragEnd,
    Swipe { dir: Dir, fingers: u8 },
    PinchStart,
    /// Current span / span when the second finger landed.
    Pinch { scale: f32 },
    PinchEnd,
    ThreeFingerTap,
}

/// A recognised gesture at a point (start point, or the centroid for multi-finger gestures).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GestureEvent {
    pub gesture: Gesture,
    pub x: f32,
    pub y: f32,
    pub t_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct P {
    x: f32,
    y: f32,
}

impl P {
    fn dist(self, o: P) -> f32 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Debug)]
enum State {
    Idle,
    One { start: P, t0: u64, last: P },
    Drag { start: P, t0: u64, last: P },
    /// Finger lifted mid-drag; a nearby re-Down before the deadline continues the stroke.
    Lifting { start: P, t0: u64, last: P, t_up: u64 },
    Held { start: P, next_repeat: u64, n: u32 },
    Multi(Multi),
    Pinch { start_span: f32, last_c: P },
    /// Swallows the rest of a gesture that has already been decided or abandoned.
    Consumed,
}

#[derive(Clone, Debug)]
struct Multi {
    t0: u64,
    first_down: u64,
    start_c: P,
    start_span: f32,
    last_c: P,
    max_n: usize,
    three_ok: bool,
    downs: BTreeMap<u32, P>,
}

pub struct GestureRecognizer {
    cfg: GestureConfig,
    state: State,
    contacts: BTreeMap<u32, P>,
    last_tap: Option<(u64, P)>,
}

impl GestureRecognizer {
    pub fn new(cfg: GestureConfig) -> Self {
        Self { cfg, state: State::Idle, contacts: BTreeMap::new(), last_tap: None }
    }

    pub fn set_config(&mut self, cfg: GestureConfig) {
        self.cfg = cfg;
    }

    pub fn next_deadline(&self) -> Option<u64> {
        match &self.state {
            State::One { t0, .. } => Some(t0 + self.cfg.long_press_ms),
            State::Held { next_repeat, .. } if self.cfg.long_press_repeat_ms > 0 => Some(*next_repeat),
            State::Lifting { t_up, .. } => Some(t_up + self.cfg.lift_debounce_ms),
            _ => None,
        }
    }

    pub fn tick(&mut self, t: u64) -> Vec<GestureEvent> {
        let mut out = Vec::new();
        self.tick_into(t, &mut out);
        out
    }

    pub fn feed(&mut self, ev: &PointerEvent) -> Vec<GestureEvent> {
        let mut out = Vec::new();
        self.tick_into(ev.t_ms, &mut out);
        let p = P { x: ev.x, y: ev.y };
        match ev.kind {
            PointerKind::Down | PointerKind::Move => {
                self.contacts.insert(ev.id, p);
            }
            PointerKind::Up => {
                self.contacts.remove(&ev.id);
            }
        }
        let state = std::mem::replace(&mut self.state, State::Idle);
        self.state = match ev.kind {
            PointerKind::Down => self.on_down(state, p, ev.t_ms, &mut out),
            PointerKind::Move => self.on_move(state, p, ev.t_ms, &mut out),
            PointerKind::Up => self.on_up(state, p, ev.t_ms, &mut out),
        };
        out
    }

    fn tick_into(&mut self, t: u64, out: &mut Vec<GestureEvent>) {
        loop {
            let Some(deadline) = self.next_deadline() else { return };
            if deadline > t {
                return;
            }
            let state = std::mem::replace(&mut self.state, State::Idle);
            self.state = match state {
                State::One { start, last, .. } => {
                    emit(out, Gesture::LongPress, last, deadline);
                    State::Held { start, next_repeat: deadline + self.cfg.long_press_repeat_ms, n: 0 }
                }
                State::Held { start, next_repeat, n } => {
                    emit(out, Gesture::LongPressRepeat { n: n + 1 }, start, next_repeat);
                    State::Held { start, next_repeat: next_repeat + self.cfg.long_press_repeat_ms, n: n + 1 }
                }
                State::Lifting { start, t0, last, t_up } => {
                    self.finish_drag(start, t0, last, t_up, out);
                    State::Idle
                }
                other => other,
            };
        }
    }

    fn on_down(&mut self, state: State, p: P, t: u64, out: &mut Vec<GestureEvent>) -> State {
        match state {
            State::Idle => State::One { start: p, t0: t, last: p },
            State::One { t0, .. } => self.enter_multi(t, t0),
            State::Drag { t0, last, .. } => {
                emit(out, Gesture::DragEnd, last, t);
                self.enter_multi(t, t0)
            }
            State::Lifting { start, t0, last, t_up } => {
                if p.dist(last) <= self.cfg.lift_debounce_px {
                    State::Drag { start, t0, last: p }
                } else {
                    self.finish_drag(start, t0, last, t_up, out);
                    State::One { start: p, t0: t, last: p }
                }
            }
            State::Multi(mut m) => {
                let n = self.contacts.len();
                if n > m.max_n {
                    m.max_n = n;
                }
                if n > 3 || t.saturating_sub(m.first_down) > self.cfg.multi_finger_window_ms {
                    m.three_ok = false;
                }
                m.downs = self.contacts.clone();
                State::Multi(m)
            }
            State::Pinch { last_c, .. } => {
                emit(out, Gesture::PinchEnd, last_c, t);
                State::Consumed
            }
            State::Held { .. } | State::Consumed => State::Consumed,
        }
    }

    fn on_move(&mut self, state: State, p: P, t: u64, out: &mut Vec<GestureEvent>) -> State {
        match state {
            State::One { start, t0, .. } if p.dist(start) > self.cfg.slop_px => {
                emit(out, Gesture::DragStart, start, t);
                emit(out, Gesture::DragMove { dx: p.x - start.x, dy: p.y - start.y }, p, t);
                State::Drag { start, t0, last: p }
            }
            State::One { start, t0, .. } => State::One { start, t0, last: p },
            State::Drag { start, t0, .. } => {
                emit(out, Gesture::DragMove { dx: p.x - start.x, dy: p.y - start.y }, p, t);
                State::Drag { start, t0, last: p }
            }
            State::Held { start, .. } if p.dist(start) > self.cfg.slop_px => State::Consumed,
            State::Multi(mut m) => {
                if m.max_n == 2 {
                    let Some((c, span)) = self.centroid_span() else { return State::Multi(m) };
                    m.last_c = c;
                    let scale = span / m.start_span;
                    if (scale - 1.0).abs() >= self.cfg.pinch_min_scale {
                        emit(out, Gesture::PinchStart, c, t);
                        emit(out, Gesture::Pinch { scale }, c, t);
                        return State::Pinch { start_span: m.start_span, last_c: c };
                    }
                } else if self.contacts.iter().any(|(id, q)| m.downs.get(id).is_some_and(|d| d.dist(*q) > self.cfg.slop_px)) {
                    m.three_ok = false;
                }
                State::Multi(m)
            }
            State::Pinch { start_span, last_c } => match self.centroid_span() {
                Some((c, span)) => {
                    emit(out, Gesture::Pinch { scale: span / start_span }, c, t);
                    State::Pinch { start_span, last_c: c }
                }
                None => State::Pinch { start_span, last_c },
            },
            other => other,
        }
    }

    fn on_up(&mut self, state: State, p: P, t: u64, out: &mut Vec<GestureEvent>) -> State {
        let empty = self.contacts.is_empty();
        let rest = if empty { State::Idle } else { State::Consumed };
        match state {
            State::One { start, t0, .. } => {
                emit(out, Gesture::Tap, start, t);
                match self.last_tap {
                    Some((t_up, q))
                        if t0.saturating_sub(t_up) <= self.cfg.double_tap_gap_ms
                            && q.dist(start) <= self.cfg.double_tap_slop_px =>
                    {
                        emit(out, Gesture::DoubleTap, start, t);
                        self.last_tap = None;
                    }
                    _ => self.last_tap = Some((t, start)),
                }
                rest
            }
            State::Drag { start, t0, .. } => State::Lifting { start, t0, last: p, t_up: t },
            State::Multi(m) if m.max_n == 2 => {
                if let Some(dir) = self.swipe_dir(m.start_c, m.last_c, t.saturating_sub(m.t0)) {
                    emit(out, Gesture::Swipe { dir, fingers: 2 }, m.start_c, t);
                }
                rest
            }
            State::Multi(m) => {
                if !empty {
                    return State::Multi(m);
                }
                if m.max_n == 3 && m.three_ok && t.saturating_sub(m.first_down) <= self.cfg.three_tap_max_ms {
                    emit(out, Gesture::ThreeFingerTap, m.start_c, t);
                }
                State::Idle
            }
            State::Pinch { last_c, .. } => {
                emit(out, Gesture::PinchEnd, last_c, t);
                rest
            }
            State::Held { .. } | State::Consumed | State::Idle | State::Lifting { .. } => rest,
        }
    }

    fn enter_multi(&self, t: u64, first_down: u64) -> State {
        let (c, span) = self.centroid_span().unwrap_or((P { x: 0.0, y: 0.0 }, 1.0));
        State::Multi(Multi {
            t0: t,
            first_down,
            start_c: c,
            start_span: span.max(1.0),
            last_c: c,
            max_n: self.contacts.len(),
            three_ok: t.saturating_sub(first_down) <= self.cfg.multi_finger_window_ms,
            downs: self.contacts.clone(),
        })
    }

    /// Centroid and distance of the first two contacts.
    fn centroid_span(&self) -> Option<(P, f32)> {
        let mut it = self.contacts.values();
        let (a, b) = (*it.next()?, *it.next()?);
        Some((P { x: (a.x + b.x) / 2.0, y: (a.y + b.y) / 2.0 }, a.dist(b)))
    }

    fn finish_drag(&mut self, start: P, t0: u64, last: P, t_up: u64, out: &mut Vec<GestureEvent>) {
        if let Some(dir) = self.swipe_dir(start, last, t_up.saturating_sub(t0)) {
            emit(out, Gesture::Swipe { dir, fingers: 1 }, start, t_up);
        }
        emit(out, Gesture::DragEnd, last, t_up);
    }

    fn swipe_dir(&self, a: P, b: P, dur_ms: u64) -> Option<Dir> {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let horizontal = dx.abs() >= dy.abs();
        let (dom, other) = if horizontal { (dx.abs(), dy.abs()) } else { (dy.abs(), dx.abs()) };
        let speed = dom / dur_ms.max(1) as f32;
        let commits = dom > self.cfg.swipe_min_px || (dom > self.cfg.flick_min_px && speed > self.cfg.flick_min_speed);
        if !commits || other * self.cfg.swipe_axis_ratio > dom {
            return None;
        }
        Some(match (horizontal, dx < 0.0, dy < 0.0) {
            (true, true, _) => Dir::Left,
            (true, false, _) => Dir::Right,
            (false, _, true) => Dir::Up,
            (false, _, false) => Dir::Down,
        })
    }
}

fn emit(out: &mut Vec<GestureEvent>, gesture: Gesture, p: P, t_ms: u64) {
    out.push(GestureEvent { gesture, x: p.x, y: p.y, t_ms });
}
