//! Feed a `touchprobe record` file through the same pipeline as the live device.
//!
//! Format: line 1 `{"descriptor":"<hex>"}`, then `{"t_ms":N,"report":"<hex>"}` per report.

use crate::config::{GestureConfig, PanelConfig};
use crate::hid;
use crate::pipeline::{Output, Pipeline};
use crate::Error;
use serde::Deserialize;
use std::path::Path;
use std::time::{Duration, Instant};

/// After the last report, keep firing deadlines (long-press repeat, lift debounce) for this long.
const TAIL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pacing {
    /// Sleep so outputs appear with the recording's original timing.
    Realtime,
    /// As fast as possible (tests).
    Instant,
}

pub struct Recording {
    pub descriptor: Vec<u8>,
    /// (t_ms since recording start, raw report)
    pub reports: Vec<(u64, Vec<u8>)>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Line {
    Descriptor { descriptor: String },
    Report { t_ms: u64, report: String },
}

impl Recording {
    pub fn load(path: &Path) -> Result<Self, Error> {
        Self::parse(&std::fs::read_to_string(path)?, &path.display().to_string())
    }

    pub fn parse(text: &str, label: &str) -> Result<Self, Error> {
        let bad = |n: usize, msg: String| Error::Fixture { path: label.into(), msg: format!("line {n}: {msg}") };
        let mut descriptor = None;
        let mut reports = Vec::new();
        for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            match serde_json::from_str::<Line>(line).map_err(|e| bad(i + 1, e.to_string()))? {
                Line::Descriptor { descriptor: d } => descriptor = Some(unhex(&d).map_err(|e| bad(i + 1, e))?),
                Line::Report { t_ms, report } => reports.push((t_ms, unhex(&report).map_err(|e| bad(i + 1, e))?)),
            }
        }
        let descriptor = descriptor.ok_or_else(|| bad(1, "no descriptor line".into()))?;
        Ok(Self { descriptor, reports })
    }

    /// Runs the recording through a fresh pipeline, handing every output to `sink`.
    pub fn run(
        &self,
        panel: &PanelConfig,
        gestures: &GestureConfig,
        pacing: Pacing,
        mut sink: impl FnMut(Output),
    ) -> Result<(), Error> {
        let rd = hid::parse(&self.descriptor).map_err(Error::Descriptor)?;
        let mut pipe = Pipeline::new(rd, panel, gestures)?;
        let start = Instant::now();
        let wait_until = |t: Duration| {
            if pacing == Pacing::Realtime {
                if let Some(d) = t.checked_sub(start.elapsed()) {
                    std::thread::sleep(d);
                }
            }
        };
        let fire_deadlines = |pipe: &mut Pipeline, until: Duration, sink: &mut dyn FnMut(Output)| {
            while let Some(d) = pipe.next_deadline().filter(|d| *d <= until) {
                wait_until(d);
                pipe.tick(d).into_iter().for_each(&mut *sink);
            }
        };
        let mut last = Duration::ZERO;
        for (t_ms, report) in &self.reports {
            let t = Duration::from_millis(*t_ms);
            fire_deadlines(&mut pipe, t, &mut sink);
            wait_until(t);
            pipe.feed(t, report).into_iter().for_each(&mut sink);
            last = t;
        }
        fire_deadlines(&mut pipe, last + TAIL, &mut sink);
        Ok(())
    }

    /// All outputs, without pacing.
    pub fn outputs(&self, panel: &PanelConfig, gestures: &GestureConfig) -> Result<Vec<Output>, Error> {
        let mut out = Vec::new();
        self.run(panel, gestures, Pacing::Instant, |o| out.push(o))?;
        Ok(out)
    }
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("bad hex at {i}: {e}"))).collect()
}
