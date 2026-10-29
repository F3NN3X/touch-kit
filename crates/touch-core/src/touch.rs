//! Turn raw digitizer reports into contact frames and down/move/up events.
//!
//! Handles both "parallel" reports (all contacts in one report) and Windows-style
//! "hybrid" reports (Contact Count in the first report, remaining contacts in follow-ups).

use crate::hid::{Kind, ReportDescriptor};
use std::collections::HashMap;
use std::time::Duration;

pub const U_TOUCH_SCREEN: u32 = 0x000D_0004;
pub const U_FINGER: u32 = 0x000D_0022;
pub const U_TIP: u32 = 0x000D_0042;
pub const U_CONFIDENCE: u32 = 0x000D_0047;
pub const U_WIDTH: u32 = 0x000D_0048;
pub const U_HEIGHT: u32 = 0x000D_0049;
pub const U_CONTACT_ID: u32 = 0x000D_0051;
pub const U_DEVICE_MODE: u32 = 0x000D_0052;
pub const U_CONTACT_COUNT: u32 = 0x000D_0054;
pub const U_CONTACT_MAX: u32 = 0x000D_0055;
pub const U_SCAN_TIME: u32 = 0x000D_0056;
pub const U_X: u32 = 0x0001_0030;
pub const U_Y: u32 = 0x0001_0031;

/// (field index, element index) into `ReportDescriptor::fields`.
type Ref = (usize, u32);

#[derive(Clone, Debug, Default)]
pub struct Slot {
    pub tip: Option<Ref>,
    pub id: Option<Ref>,
    pub x: Option<Ref>,
    pub y: Option<Ref>,
    pub confidence: Option<Ref>,
    pub width: Option<Ref>,
    pub height: Option<Ref>,
    first_bit: u32,
}

#[derive(Clone, Debug, Default)]
pub struct TouchReport {
    pub slots: Vec<Slot>,
    pub contact_count: Option<Ref>,
    pub scan_time: Option<Ref>,
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub reports: HashMap<u8, TouchReport>,
}

impl Layout {
    /// Build from the descriptor. Only Input fields inside a Touch Screen application collection count.
    pub fn from_descriptor(rd: &ReportDescriptor) -> Self {
        let mut reports: HashMap<u8, TouchReport> = HashMap::new();
        // (report id, finger collection index) -> slot
        let mut slots: HashMap<(u8, usize), Slot> = HashMap::new();

        for (fi, f) in rd.fields.iter().enumerate() {
            if f.kind != Kind::Input || f.is_constant() || !f.is_variable() {
                continue;
            }
            if f.application() != Some(U_TOUCH_SCREEN) {
                continue;
            }
            let rep = reports.entry(f.report_id).or_default();
            // Finger collection if present, otherwise the application collection (single-touch devices).
            let group =
                f.path.iter().rev().find(|c| c.usage == U_FINGER).or_else(|| f.path.first()).map(|c| c.index).unwrap_or(0);

            for i in 0..f.count {
                let Some(u) = f.usage_at(i) else { continue };
                let r = (fi, i);
                match u {
                    U_CONTACT_COUNT => rep.contact_count = Some(r),
                    U_SCAN_TIME => rep.scan_time = Some(r),
                    U_TIP | U_CONTACT_ID | U_X | U_Y | U_CONFIDENCE | U_WIDTH | U_HEIGHT => {
                        let slot = slots
                            .entry((f.report_id, group))
                            .or_insert_with(|| Slot { first_bit: f.element_offset(i), ..Default::default() });
                        let target = match u {
                            U_TIP => &mut slot.tip,
                            U_CONTACT_ID => &mut slot.id,
                            U_X => &mut slot.x,
                            U_Y => &mut slot.y,
                            U_CONFIDENCE => &mut slot.confidence,
                            U_WIDTH => &mut slot.width,
                            _ => &mut slot.height,
                        };
                        if target.is_none() {
                            *target = Some(r);
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut grouped: HashMap<u8, Vec<Slot>> = HashMap::new();
        for ((rid, _), s) in slots {
            if s.x.is_some() && s.y.is_some() {
                grouped.entry(rid).or_default().push(s);
            }
        }
        for (rid, mut v) in grouped {
            v.sort_by_key(|s| s.first_bit);
            if let Some(r) = reports.get_mut(&rid) {
                r.slots = v;
            }
        }
        reports.retain(|_, r| !r.slots.is_empty());
        Layout { reports }
    }

    pub fn summary(&self, rd: &ReportDescriptor) -> String {
        if self.reports.is_empty() {
            return "no Touch Screen input report found in the descriptor".into();
        }
        let mut out = String::new();
        let mut ids: Vec<_> = self.reports.keys().copied().collect();
        ids.sort();
        for id in ids {
            let r = &self.reports[&id];
            let xr = r.slots[0].x.map(|(f, _)| (rd.fields[f].logical_min, rd.fields[f].logical_max));
            let yr = r.slots[0].y.map(|(f, _)| (rd.fields[f].logical_min, rd.fields[f].logical_max));
            out.push_str(&format!(
                "touch report {id:#04x}: {} slots/report, contact count field: {}, X range {:?}, Y range {:?}, {} bytes\n",
                r.slots.len(),
                if r.contact_count.is_some() { "yes (hybrid capable)" } else { "no" },
                xr.unwrap_or_default(),
                yr.unwrap_or_default(),
                rd.report_len(Kind::Input, id),
            ));
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    pub id: u32,
    pub tip: bool,
    /// Normalised 0..=1 across the digitizer's logical range.
    pub x: f32,
    pub y: f32,
    pub confidence: Option<bool>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

fn read(rd: &ReportDescriptor, payload: &[u8], r: Option<Ref>) -> Option<i64> {
    r.map(|(f, i)| rd.fields[f].read(payload, i))
}

fn norm(rd: &ReportDescriptor, payload: &[u8], r: Option<Ref>) -> Option<f32> {
    r.map(|(f, i)| {
        let fld = &rd.fields[f];
        let v = fld.read(payload, i);
        let span = (fld.logical_max - fld.logical_min).max(1) as f32;
        ((v - fld.logical_min) as f32 / span).clamp(0.0, 1.0)
    })
}

/// Collects hybrid-mode report sequences into complete frames.
#[derive(Default)]
pub struct Assembler {
    expected: usize,
    pending: Vec<Contact>,
}

pub enum Decoded {
    /// A complete frame of contacts.
    Frame(Vec<Contact>),
    /// Part of a hybrid frame; waiting for more reports.
    Partial,
    /// Report id is not a touch report (e.g. mouse-mode or vendor report).
    NotTouch(u8),
}

impl Assembler {
    pub fn feed(&mut self, rd: &ReportDescriptor, layout: &Layout, report: &[u8]) -> Decoded {
        let (rid, payload) = rd.split(report);
        let Some(tr) = layout.reports.get(&rid) else {
            return Decoded::NotTouch(rid);
        };

        let decode_slot = |s: &Slot| Contact {
            id: read(rd, payload, s.id).unwrap_or(0) as u32,
            tip: read(rd, payload, s.tip).map(|v| v != 0).unwrap_or(true),
            x: norm(rd, payload, s.x).unwrap_or(0.0),
            y: norm(rd, payload, s.y).unwrap_or(0.0),
            confidence: read(rd, payload, s.confidence).map(|v| v != 0),
            width: norm(rd, payload, s.width),
            height: norm(rd, payload, s.height),
        };

        let Some(cc_ref) = tr.contact_count else {
            // Parallel mode without a count: only slots with tip down are meaningful.
            let frame = tr.slots.iter().map(decode_slot).filter(|c| c.tip).collect();
            return Decoded::Frame(frame);
        };

        let cc = read(rd, payload, Some(cc_ref)).unwrap_or(0).max(0) as usize;
        if cc > 0 {
            // First report of a new frame.
            self.expected = cc;
            self.pending.clear();
        } else if self.expected == 0 {
            // Count 0 with nothing pending: everything lifted.
            return Decoded::Frame(Vec::new());
        }
        let remaining = self.expected.saturating_sub(self.pending.len());
        for s in tr.slots.iter().take(remaining) {
            self.pending.push(decode_slot(s));
        }
        if self.pending.len() >= self.expected {
            self.expected = 0;
            Decoded::Frame(std::mem::take(&mut self.pending))
        } else {
            Decoded::Partial
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Down { id: u32, x: f32, y: f32 },
    Move { id: u32, x: f32, y: f32 },
    Up { id: u32, x: f32, y: f32, held_ms: u64 },
}

struct Live {
    x: f32,
    y: f32,
    since: Duration,
}

/// Turns frames into down/move/up. A contact that disappears from a frame is treated as lifted.
#[derive(Default)]
pub struct Tracker {
    live: HashMap<u32, Live>,
    pub move_epsilon: f32,
}

impl Tracker {
    pub fn new(move_epsilon: f32) -> Self {
        Self { live: HashMap::new(), move_epsilon }
    }

    /// `now` is monotonic time since the session started (recorded time when replaying).
    pub fn update(&mut self, now: Duration, frame: &[Contact]) -> Vec<Event> {
        let mut ev = Vec::new();
        let mut seen: Vec<u32> = Vec::new();
        for c in frame {
            if !c.tip {
                continue;
            }
            seen.push(c.id);
            match self.live.get_mut(&c.id) {
                None => {
                    self.live.insert(c.id, Live { x: c.x, y: c.y, since: now });
                    ev.push(Event::Down { id: c.id, x: c.x, y: c.y });
                }
                Some(l) => {
                    if (l.x - c.x).abs() > self.move_epsilon || (l.y - c.y).abs() > self.move_epsilon {
                        l.x = c.x;
                        l.y = c.y;
                        ev.push(Event::Move { id: c.id, x: c.x, y: c.y });
                    }
                }
            }
        }
        let gone: Vec<u32> = self.live.keys().copied().filter(|id| !seen.contains(id)).collect();
        for id in gone {
            let l = self.live.remove(&id).unwrap();
            ev.push(Event::Up { id, x: l.x, y: l.y, held_ms: now.saturating_sub(l.since).as_millis() as u64 });
        }
        ev
    }
}
