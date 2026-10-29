//! Raw report → contacts → pointer events (panel px) → gestures, with a single clock.

use crate::config::{GestureConfig, PanelConfig};
use crate::gesture::{GestureEvent, GestureRecognizer, PointerEvent, PointerKind};
use crate::hid::ReportDescriptor;
use crate::mapper::Mapper;
use crate::touch::{Assembler, Decoded, Event, Layout, Tracker};
use crate::Error;
use serde::Serialize;
use std::time::Duration;

/// Reports in the first `DRAIN_WINDOW` of a session are stale queue contents, as are reports
/// chained less than `DRAIN_GAP` after one being drained. See docs/PLAN.md §4.
const DRAIN_WINDOW: Duration = Duration::from_millis(50);
const DRAIN_GAP: Duration = Duration::from_millis(2);

/// Serialised with a `type` tag (`PointerEvent` already has a `kind` field).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Output {
    Pointer(PointerEvent),
    Gesture(GestureEvent),
    /// A report id that is not a touch report (e.g. mouse mode). Reported once per id per session.
    Ignored { report_id: u8 },
}

pub struct Pipeline {
    rd: ReportDescriptor,
    layout: Layout,
    asm: Assembler,
    tracker: Tracker,
    mapper: Mapper,
    rec: GestureRecognizer,
    draining: bool,
    drain_last: Option<Duration>,
    ignored: Vec<u8>,
}

impl Pipeline {
    pub fn new(rd: ReportDescriptor, panel: &PanelConfig, gestures: &GestureConfig) -> Result<Self, Error> {
        let layout = Layout::from_descriptor(&rd);
        if layout.reports.is_empty() {
            return Err(Error::NoTouchReport);
        }
        Ok(Self {
            rd,
            layout,
            asm: Assembler::default(),
            tracker: Tracker::new(0.0),
            mapper: Mapper::new(panel)?,
            rec: GestureRecognizer::new(gestures.clone()),
            draining: true,
            drain_last: None,
            ignored: Vec::new(),
        })
    }

    pub fn descriptor(&self) -> &ReportDescriptor {
        &self.rd
    }

    pub fn reconfigure(&mut self, panel: &PanelConfig, gestures: &GestureConfig) -> Result<(), Error> {
        self.mapper = Mapper::new(panel)?;
        self.rec.set_config(gestures.clone());
        Ok(())
    }

    /// `t` is time since the session started.
    pub fn feed(&mut self, t: Duration, report: &[u8]) -> Vec<Output> {
        if self.drain(t) {
            return Vec::new();
        }
        let mut out = self.tick(t);
        match self.asm.feed(&self.rd, &self.layout, report) {
            Decoded::Frame(frame) => {
                let events = self.tracker.update(t, &frame);
                self.push_events(t, events, &mut out);
            }
            Decoded::Partial => {}
            Decoded::NotTouch(id) => {
                if !self.ignored.contains(&id) {
                    self.ignored.push(id);
                    out.push(Output::Ignored { report_id: id });
                }
            }
        }
        out
    }

    pub fn tick(&mut self, t: Duration) -> Vec<Output> {
        self.rec.tick(ms(t)).into_iter().map(Output::Gesture).collect()
    }

    pub fn next_deadline(&self) -> Option<Duration> {
        self.rec.next_deadline().map(Duration::from_millis)
    }

    /// Lift every live contact, e.g. when the device disconnects mid-touch.
    pub fn release_all(&mut self, t: Duration) -> Vec<Output> {
        let mut out = self.tick(t);
        let events = self.tracker.update(t, &[]);
        self.push_events(t, events, &mut out);
        out
    }

    fn drain(&mut self, t: Duration) -> bool {
        if !self.draining {
            return false;
        }
        let chained = self.drain_last.is_some_and(|l| t.saturating_sub(l) < DRAIN_GAP);
        if t < DRAIN_WINDOW || chained {
            self.drain_last = Some(t);
            true
        } else {
            self.draining = false;
            false
        }
    }

    fn push_events(&mut self, t: Duration, events: Vec<Event>, out: &mut Vec<Output>) {
        for e in events {
            let (kind, id, x, y) = match e {
                Event::Down { id, x, y } => (PointerKind::Down, id, x, y),
                Event::Move { id, x, y } => (PointerKind::Move, id, x, y),
                Event::Up { id, x, y, .. } => (PointerKind::Up, id, x, y),
            };
            let (x, y) = self.mapper.map(x, y);
            let pe = PointerEvent { kind, id, x, y, t_ms: ms(t) };
            let gestures = self.rec.feed(&pe);
            out.push(Output::Pointer(pe));
            out.extend(gestures.into_iter().map(Output::Gesture));
        }
    }
}

fn ms(t: Duration) -> u64 {
    t.as_millis() as u64
}
