//! `TouchDevice`: one background thread that finds the panel, reads it through the pipeline and
//! reopens it after an unplug. While disconnected it waits for a nusb hotplug event for the panel,
//! with a 1 s retry as fallback (an arrival can be reported before the driver is ready). docs/PLAN.md §4.
//!
//! A virtual device (`TouchDevice::replay`) plays a recording in a loop instead, for testing
//! without hardware; it looks connected and takes the same reconfigures.

use crate::config::{Config, GestureConfig, PanelConfig};
use crate::hid;
use crate::pipeline::{Output, Pipeline};
use crate::replay::Recording;
use crate::usb::{self, Panel};
use crate::Error;
use futures_core::Stream;
use nusb::hotplug::{HotplugEvent, HotplugWatch};
use serde::Serialize;
use std::pin::Pin;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::task::{Context, Poll, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const RECONNECT_POLL: Duration = Duration::from_secs(1);
/// How often the disconnected wait checks for hotplug events and control messages.
const WAIT_SLICE: Duration = Duration::from_millis(100);
/// Longest blocking read; bounds how long a reconfigure or stop waits.
const READ_SLICE: Duration = Duration::from_millis(250);
/// Pause between two plays of a virtual device's recording.
const REPLAY_GAP: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status")]
pub enum Status {
    /// `port`: where the device is plugged in (see `usb::location`).
    Connected { interface: u8, report_len: usize, port: String },
    Disconnected,
    Error { message: String },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DeviceMsg {
    Status(Status),
    Output(Output),
}

enum Ctrl {
    Reconfigure(PanelConfig, GestureConfig),
    Stop,
}

enum Exit {
    Stop,
    Disconnected,
    Error(String),
}

pub struct TouchDevice {
    ctrl: Sender<Ctrl>,
    thread: Option<JoinHandle<()>>,
}

impl TouchDevice {
    /// Starts the device thread. Messages flow until the `TouchDevice` is dropped or `tx`'s
    /// receiver goes away. `taken`: ports other panels use, never opened by this one.
    pub fn spawn(cfg: Config, taken: Vec<String>, tx: Sender<DeviceMsg>) -> Self {
        Self::start(move |ctrl| run(cfg, taken, tx, ctrl))
    }

    /// Starts a virtual device that plays `rec` in a loop at its recorded pace, reporting itself as
    /// connected at `port`.
    pub fn replay(rec: Recording, port: String, panel: PanelConfig, gestures: GestureConfig, tx: Sender<DeviceMsg>) -> Self {
        Self::start(move |ctrl| run_replay(&rec, port, &panel, &gestures, &tx, &ctrl))
    }

    fn start(body: impl FnOnce(Receiver<Ctrl>) + Send + 'static) -> Self {
        let (ctrl, ctrl_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("touch-device".into())
            .spawn(move || body(ctrl_rx))
            .expect("spawning touch-device thread");
        Self { ctrl, thread: Some(thread) }
    }

    /// Applies new panel geometry and gesture thresholds without reopening the device.
    pub fn reconfigure(&self, panel: PanelConfig, gestures: GestureConfig) {
        let _ = self.ctrl.send(Ctrl::Reconfigure(panel, gestures));
    }
}

impl Drop for TouchDevice {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(mut cfg: Config, taken: Vec<String>, tx: Sender<DeviceMsg>, ctrl: Receiver<Ctrl>) {
    // Created before the first open so an arrival between the two isn't missed.
    let mut watch = match nusb::watch_devices() {
        Ok(w) => Some(w),
        Err(e) => {
            tracing::warn!("hotplug unavailable, polling only: {e}");
            None
        }
    };
    // Last status reported while not connected, so a 1 s poll doesn't repeat it.
    let mut last: Option<Status> = None;
    let report = |s: Status, last: &mut Option<Status>| {
        if last.as_ref() != Some(&s) {
            *last = Some(s.clone());
            return tx.send(DeviceMsg::Status(s)).is_ok();
        }
        true
    };
    loop {
        let alive = match usb::open(&cfg.device, &taken) {
            Ok(panel) => match session(&panel, &mut cfg, &tx, &ctrl) {
                Exit::Stop => return,
                Exit::Disconnected => {
                    last = None;
                    report(Status::Disconnected, &mut last)
                }
                Exit::Error(message) => report(Status::Error { message }, &mut last),
            },
            Err(Error::NotFound { .. }) => report(Status::Disconnected, &mut last),
            Err(e) => report(Status::Error { message: e.to_string() }, &mut last),
        };
        if !alive {
            return;
        }
        let until = Instant::now() + RECONNECT_POLL;
        while Instant::now() < until {
            if watch.as_mut().is_some_and(|w| arrived(w, cfg.device.vid, cfg.device.pid)) {
                break;
            }
            match ctrl.recv_timeout(WAIT_SLICE) {
                Ok(Ctrl::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(Ctrl::Reconfigure(p, g)) => {
                    cfg.panel = p;
                    cfg.gestures = g;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

/// Drains queued hotplug events; true if the panel was among the arrivals.
fn arrived(watch: &mut HotplugWatch, vid: u16, pid: u16) -> bool {
    let mut cx = Context::from_waker(Waker::noop());
    let mut seen = false;
    while let Poll::Ready(Some(ev)) = Pin::new(&mut *watch).poll_next(&mut cx) {
        if let HotplugEvent::Connected(d) = ev {
            seen |= d.vendor_id() == vid && d.product_id() == pid;
        }
    }
    seen
}

fn session(panel: &Panel, cfg: &mut Config, tx: &Sender<DeviceMsg>, ctrl: &Receiver<Ctrl>) -> Exit {
    if let Some(mode) = cfg.device.device_mode {
        match panel.set_device_mode(mode) {
            Ok(true) => tracing::info!("device mode set to {mode}"),
            Ok(false) => tracing::debug!("descriptor has no Device Mode feature; skipped"),
            Err(e) => tracing::warn!("setting device mode: {e}"),
        }
    }
    let mut pipe = match Pipeline::new(panel.rd.clone(), &cfg.panel, &cfg.gestures) {
        Ok(p) => p,
        Err(e) => return Exit::Error(e.to_string()),
    };
    let mut reader = match panel.reader() {
        Ok(r) => r,
        Err(e) => return Exit::Error(e.to_string()),
    };
    let report_len = pipe.descriptor().max_input_len();
    let connected = Status::Connected { interface: panel.iface, report_len, port: panel.port.clone() };
    if tx.send(DeviceMsg::Status(connected)).is_err() {
        return Exit::Stop;
    }

    let start = Instant::now();
    loop {
        loop {
            match ctrl.try_recv() {
                Ok(Ctrl::Stop) | Err(TryRecvError::Disconnected) => return Exit::Stop,
                Ok(Ctrl::Reconfigure(p, g)) => match pipe.reconfigure(&p, &g) {
                    Ok(()) => {
                        cfg.panel = p;
                        cfg.gestures = g;
                    }
                    Err(e) => {
                        let _ = tx.send(DeviceMsg::Status(Status::Error { message: e.to_string() }));
                    }
                },
                Err(TryRecvError::Empty) => break,
            }
        }
        let now = start.elapsed();
        let timeout = pipe.next_deadline().map_or(READ_SLICE, |d| d.saturating_sub(now).min(READ_SLICE));
        let (outputs, exit) = match reader.next(timeout) {
            Ok(Some(r)) => (pipe.feed(start.elapsed(), &r), None),
            Ok(None) => (pipe.tick(start.elapsed()), None),
            Err(Error::Disconnected) => (pipe.release_all(start.elapsed()), Some(Exit::Disconnected)),
            Err(e) => (pipe.release_all(start.elapsed()), Some(Exit::Error(e.to_string()))),
        };
        for o in outputs {
            if tx.send(DeviceMsg::Output(o)).is_err() {
                return Exit::Stop;
            }
        }
        if let Some(exit) = exit {
            return exit;
        }
    }
}

fn run_replay(rec: &Recording, port: String, panel: &PanelConfig, gestures: &GestureConfig, tx: &Sender<DeviceMsg>, ctrl: &Receiver<Ctrl>) {
    let pipe = hid::parse(&rec.descriptor)
        .map_err(Error::Descriptor)
        .and_then(|rd| Pipeline::new(rd, panel, gestures));
    let mut pipe = match pipe {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(DeviceMsg::Status(Status::Error { message: e.to_string() }));
            // Stay until stopped, as a real device thread would.
            while !matches!(ctrl.recv(), Ok(Ctrl::Stop) | Err(_)) {}
            return;
        }
    };
    let report_len = pipe.descriptor().max_input_len();
    if tx.send(DeviceMsg::Status(Status::Connected { interface: 0, report_len, port })).is_err() {
        return;
    }
    let start = Instant::now();
    let length = rec.reports.last().map_or(Duration::ZERO, |(t, _)| Duration::from_millis(*t));
    // Pipeline time of the current play's t = 0; it keeps counting up so deadlines stay ordered.
    let mut base = Duration::ZERO;
    loop {
        for (t_ms, report) in &rec.reports {
            let at = base + Duration::from_millis(*t_ms);
            if !replay_wait(&mut pipe, start, at, tx, ctrl) || !send_all(tx, pipe.feed(at, report)) {
                return;
            }
        }
        base += length + REPLAY_GAP;
        if !replay_wait(&mut pipe, start, base, tx, ctrl) {
            return;
        }
    }
}

/// Sleeps until pipeline time `until`, firing deadlines and taking control messages on the way.
/// False when the device should stop.
fn replay_wait(pipe: &mut Pipeline, start: Instant, until: Duration, tx: &Sender<DeviceMsg>, ctrl: &Receiver<Ctrl>) -> bool {
    loop {
        let now = start.elapsed().min(until);
        while let Some(d) = pipe.next_deadline().filter(|d| *d <= now) {
            if !send_all(tx, pipe.tick(d)) {
                return false;
            }
        }
        if now >= until {
            return true;
        }
        let wake = pipe.next_deadline().map_or(until, |d| d.min(until));
        match ctrl.recv_timeout(wake.saturating_sub(start.elapsed())) {
            Ok(Ctrl::Stop) | Err(RecvTimeoutError::Disconnected) => return false,
            Ok(Ctrl::Reconfigure(p, g)) => {
                if let Err(e) = pipe.reconfigure(&p, &g) {
                    let _ = tx.send(DeviceMsg::Status(Status::Error { message: e.to_string() }));
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn send_all(tx: &Sender<DeviceMsg>, outputs: Vec<Output>) -> bool {
    outputs.into_iter().all(|o| tx.send(DeviceMsg::Output(o)).is_ok())
}
