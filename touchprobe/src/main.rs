//! touchprobe - talk to a USB HID touch panel directly, without the OS touch stack.
//!
//! On Windows the panel must be bound to WinUSB first (see docs/driver-setup.md, step 2).
//! On Linux the kernel HID driver is detached automatically while the probe runs.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use nusb::MaybeFuture;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use touch_core::usb::{self, Panel};
use touch_core::{hid, touch, Config, DeviceMsg, Gesture, Output, Pacing, PointerKind, Recording, Status, TouchDevice};

#[derive(Parser)]
#[command(name = "touchprobe", version, about)]
struct Cli {
    /// USB vendor id (hex) [default: any touchscreen]
    #[arg(long, value_parser = parse_hex16, global = true)]
    vid: Option<u16>,
    /// USB product id (hex) [default: any touchscreen]
    #[arg(long, value_parser = parse_hex16, global = true)]
    pid: Option<u16>,
    /// Interface number (default: first HID interface)
    #[arg(long, global = true)]
    iface: Option<u8>,
    /// Use this report descriptor file instead of asking the device
    #[arg(long, global = true)]
    descriptor: Option<PathBuf>,
    /// JSON config ({"device":{..},"panel":{..},"gestures":{..}}); flags above override it
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List USB devices (marks touchscreens)
    List,
    /// Write a JSON report of every touchscreen: USB and HID details, optionally a touch recording
    Report {
        /// Output file
        #[arg(long, default_value = "touch-report.json")]
        out: PathBuf,
        /// Also record this many seconds of touches on every ready touchscreen (0 = no recording)
        #[arg(long, default_value_t = 0)]
        record: u64,
    },
    /// Show interfaces/endpoints, fetch and decode the HID report descriptor
    Info {
        /// Folder to save report-descriptor.bin / .txt into
        #[arg(long, default_value = ".")]
        save: PathBuf,
    },
    /// Print raw interrupt reports as hex
    Raw {
        /// Stop after N reports (0 = forever)
        #[arg(long, default_value_t = 0)]
        count: u64,
    },
    /// Record raw reports to a JSONL file (fixtures for gesture tests)
    Record {
        /// Output file
        #[arg(long, default_value = "session.jsonl")]
        out: PathBuf,
        /// Stop after this many seconds
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
    /// Decode reports into contacts and down/move/up events
    Touch {
        /// Panel resolution for pixel output, e.g. 1920x480
        #[arg(long, value_parser = parse_res)]
        px: Option<(u32, u32)>,
        /// Send Device Mode = 2 (multi-touch) before reading
        #[arg(long)]
        set_mode: bool,
        /// Also print every full frame
        #[arg(long)]
        frames: bool,
    },
    /// Live gesture recogniser output (reconnects after unplug)
    Gestures {
        /// Also print MOVE, DragMove and every Pinch step
        #[arg(long)]
        verbose: bool,
    },
    /// Run a recording through the gesture pipeline
    Replay {
        file: PathBuf,
        /// Don't sleep: print everything immediately
        #[arg(long)]
        instant: bool,
        /// Also print MOVE, DragMove and every Pinch step
        #[arg(long)]
        verbose: bool,
    },
    /// Set the digitizer Device Mode feature (0 mouse, 1 single-input, 2 multi-input)
    SetMode { mode: i64 },
    /// GET_REPORT for a feature report id and print it
    GetFeature {
        #[arg(value_parser = parse_hex8)]
        report_id: u8,
    },
}

fn parse_hex16(s: &str) -> Result<u16, String> {
    u16::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16).map_err(|e| e.to_string())
}
fn parse_hex8(s: &str) -> Result<u8, String> {
    u8::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16).map_err(|e| e.to_string())
}
fn parse_res(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s.split_once(['x', 'X']).ok_or("expected WIDTHxHEIGHT")?;
    Ok((w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = load_config(&cli)?;
    match &cli.cmd {
        Cmd::List => list(&cfg),
        Cmd::Report { out, record } => report(&cfg, out, *record),
        Cmd::Info { save } => info(&cfg, save),
        Cmd::Raw { count } => raw(&cfg, *count),
        Cmd::Record { out, seconds } => record(&cfg, out, *seconds),
        Cmd::Touch { px, set_mode, frames } => run_touch(&cfg, *px, *set_mode, *frames),
        Cmd::Gestures { verbose } => gestures(cfg, *verbose),
        Cmd::Replay { file, instant, verbose } => replay(&cfg, file, *instant, *verbose),
        Cmd::SetMode { mode } => {
            let p = open(&cfg)?;
            set_device_mode(&p, *mode)
        }
        Cmd::GetFeature { report_id } => {
            let p = open(&cfg)?;
            let len = p.rd.report_len(hid::Kind::Feature, *report_id).max(2);
            let data = p.get_feature(*report_id, len)?;
            println!("feature {report_id:#04x} ({} bytes): {}", data.len(), hex(&data));
            Ok(())
        }
    }
}

fn load_config(cli: &Cli) -> Result<Config> {
    let mut cfg: Config = match &cli.config {
        Some(path) => {
            serde_json::from_str(&std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?)
                .with_context(|| format!("parsing {}", path.display()))?
        }
        None => Config::default(),
    };
    if cli.vid.is_some() {
        cfg.device.vid = cli.vid;
    }
    if cli.pid.is_some() {
        cfg.device.pid = cli.pid;
    }
    if cli.iface.is_some() {
        cfg.device.iface = cli.iface;
    }
    if cli.descriptor.is_some() {
        cfg.device.descriptor_file = cli.descriptor.clone();
    }
    let errors = cfg.validate();
    if !errors.is_empty() {
        bail!(
            "invalid config:
  {}",
            errors.join(
                "
  "
            )
        );
    }
    Ok(cfg)
}

fn open(cfg: &Config) -> Result<Panel> {
    Ok(usb::open(&cfg.device, &[])?)
}

fn list(cfg: &Config) -> Result<()> {
    let touch = usb::candidates(&cfg.device)?;
    for d in nusb::list_devices().wait()? {
        let port = usb::location(&d);
        let mark = match touch.iter().find(|t| t.port == port) {
            Some(t) if t.ready => "  <== touchscreen, ready",
            Some(_) => "  <== touchscreen, needs the WinUSB driver (docs/driver-setup.md)",
            None => "",
        };
        #[cfg(target_os = "windows")]
        let drv = format!(" driver={}", d.driver().unwrap_or("?"));
        #[cfg(not(target_os = "windows"))]
        let drv = String::new();
        println!(
            "{:04x}:{:04x}  {} / {}  serial={} port={}{}{}",
            d.vendor_id(),
            d.product_id(),
            d.manufacturer_string().unwrap_or("-"),
            d.product_string().unwrap_or("-"),
            d.serial_number().unwrap_or("-"),
            port,
            drv,
            mark
        );
    }
    Ok(())
}

/// The device part of the app's display report (no monitors or app config).
fn report(cfg: &Config, out: &PathBuf, seconds: u64) -> Result<()> {
    let devices = touch_core::report::devices()?;
    println!("{} touchscreen(s)", devices.len());
    let mut recordings = Vec::new();
    for d in devices.iter().filter(|d| d.ready && seconds > 0) {
        println!("recording {} for {seconds} s: tap, swipe, pinch, hold, then many fingers...", d.port);
        let p = touch_core::report::open(&d.port)?;
        recordings.push(touch_core::report::record(&p, &cfg.panel, Duration::from_secs(seconds), || false)?);
    }
    let json = serde_json::json!({ "touchscreens": devices, "recordings": recordings });
    std::fs::write(out, serde_json::to_string_pretty(&json)?).with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

fn info(cfg: &Config, save: &PathBuf) -> Result<()> {
    let p = open(cfg)?;
    println!("interface {}  interrupt IN ep {:#04x}  max packet {} bytes", p.iface, p.ep_in, p.mps);
    println!("report descriptor: {} bytes\n", p.rd_raw.len());
    let text = hid::describe(&p.rd);
    print!("{text}");
    let layout = touch::Layout::from_descriptor(&p.rd);
    println!("\n{}", layout.summary(&p.rd));

    match p.contact_max() {
        Ok(Some(n)) => println!("contact count maximum: {n}"),
        Ok(None) => {}
        Err(e) => println!("contact count maximum: unreadable ({e})"),
    }
    match p.rd.find(hid::Kind::Feature, touch::U_DEVICE_MODE) {
        Some((fi, _)) => println!(
            "device mode feature: report {:#04x} (use `set-mode 2` if touches show up as mouse reports)",
            p.rd.fields[fi].report_id
        ),
        None => println!("device mode feature: not present"),
    }

    std::fs::create_dir_all(save)?;
    std::fs::write(save.join("report-descriptor.bin"), &p.rd_raw)?;
    std::fs::write(save.join("report-descriptor.txt"), format!("{}\n\n{}", hex(&p.rd_raw), text))?;
    println!("\nsaved {} and report-descriptor.txt", save.join("report-descriptor.bin").display());
    Ok(())
}

/// Calls `on_report` for each report until it returns false, or until `stop_at` passes.
fn read_reports(p: &Panel, stop_at: Option<Instant>, mut on_report: impl FnMut(&[u8]) -> bool) -> Result<()> {
    let mut reader = p.reader()?;
    loop {
        if stop_at.is_some_and(|t| Instant::now() >= t) {
            return Ok(());
        }
        if let Some(r) = reader.next(Duration::from_millis(100))? {
            if !on_report(&r) {
                return Ok(());
            }
        }
    }
}

fn raw(cfg: &Config, count: u64) -> Result<()> {
    let p = open(cfg)?;
    println!("reading ep {:#04x} - touch the panel (Ctrl+C to stop)", p.ep_in);
    let start = Instant::now();
    let mut n = 0u64;
    read_reports(&p, None, |r| {
        n += 1;
        println!("{:>8.3}s  {:>3}B  {}", start.elapsed().as_secs_f64(), r.len(), hex(r));
        count == 0 || n < count
    })
}

fn record(cfg: &Config, out: &PathBuf, seconds: u64) -> Result<()> {
    let p = open(cfg)?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(out)?);
    // First line: the report descriptor, so a recording is self-contained.
    writeln!(f, "{{\"descriptor\":\"{}\"}}", hex(&p.rd_raw).replace(' ', ""))?;
    println!("recording for {seconds}s to {} - perform the gesture now", out.display());
    let start = Instant::now();
    let limit = Duration::from_secs(seconds);
    let mut n = 0u64;
    read_reports(&p, Some(start + limit), |r| {
        n += 1;
        let _ = writeln!(f, "{{\"t_ms\":{},\"report\":\"{}\"}}", start.elapsed().as_millis(), hex(r).replace(' ', ""));
        start.elapsed() < limit
    })?;
    f.flush()?;
    println!("wrote {n} reports");
    Ok(())
}

fn run_touch(cfg: &Config, px: Option<(u32, u32)>, set_mode: bool, frames: bool) -> Result<()> {
    let p = open(cfg)?;
    let layout = touch::Layout::from_descriptor(&p.rd);
    print!("{}", layout.summary(&p.rd));
    if layout.reports.is_empty() {
        bail!("descriptor has no touch-screen input report; run `info` and share report-descriptor.txt");
    }
    if set_mode {
        set_device_mode(&p, 2)?;
    }
    println!("touch the panel (Ctrl+C to stop)\n");

    let fmt = |x: f32, y: f32| match px {
        Some((w, h)) => format!("x={:>5} y={:>5}", (x * (w - 1) as f32).round() as i32, (y * (h - 1) as f32).round() as i32),
        None => format!("x={x:.4} y={y:.4}"),
    };
    let mut asm = touch::Assembler::default();
    let mut tracker = touch::Tracker::new(0.002);
    let mut warned: Vec<u8> = Vec::new();
    let start = Instant::now();
    let stdout = std::io::stdout();

    read_reports(&p, None, |r| {
        let t = start.elapsed().as_secs_f64();
        let mut out = stdout.lock();
        match asm.feed(&p.rd, &layout, r) {
            touch::Decoded::Frame(frame) => {
                if frames {
                    let parts: Vec<String> =
                        frame.iter().map(|c| format!("[id={} tip={} {}]", c.id, c.tip as u8, fmt(c.x, c.y))).collect();
                    let _ = writeln!(out, "{t:>8.3}s frame {}", parts.join(" "));
                }
                for e in tracker.update(start.elapsed(), &frame) {
                    let line = match e {
                        touch::Event::Down { id, x, y } => format!("DOWN id={id} {}", fmt(x, y)),
                        touch::Event::Move { id, x, y } => format!("MOVE id={id} {}", fmt(x, y)),
                        touch::Event::Up { id, x, y, held_ms } => format!("UP   id={id} {} held={held_ms}ms", fmt(x, y)),
                    };
                    let _ = writeln!(out, "{t:>8.3}s {line}");
                }
            }
            touch::Decoded::Partial => {}
            touch::Decoded::NotTouch(id) => {
                if !warned.contains(&id) {
                    warned.push(id);
                    let _ = writeln!(out, "note: {}. Raw: {}", not_touch_note(&p.rd, id), hex(r));
                }
            }
        }
        true
    })
}

fn not_touch_note(rd: &hid::ReportDescriptor, id: u8) -> String {
    let app = rd
        .fields
        .iter()
        .find(|f| f.kind == hid::Kind::Input && f.report_id == id)
        .and_then(|f| f.application())
        .map(hid::usage_name)
        .unwrap_or_else(|| "unknown".into());
    format!(
        "report {id:#04x} belongs to '{app}', not the touch screen. \
         If touches only arrive here, run `touchprobe set-mode 2` (if the descriptor has Device Mode)"
    )
}

fn gestures(cfg: Config, verbose: bool) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _device = TouchDevice::spawn(cfg, Vec::new(), tx);
    println!("waiting for the panel - touch it (Ctrl+C to stop)");
    for msg in rx {
        match msg {
            DeviceMsg::Status(s) => println!("{}", status_line(&s)),
            DeviceMsg::Output(o) => print_output(&o, verbose),
        }
    }
    Ok(())
}

fn replay(cfg: &Config, file: &std::path::Path, instant: bool, verbose: bool) -> Result<()> {
    let rec = Recording::load(file)?;
    let pacing = if instant { Pacing::Instant } else { Pacing::Realtime };
    println!("replaying {} reports from {} ({:?})", rec.reports.len(), file.display(), pacing);
    rec.run(&cfg.panel, &cfg.gestures, pacing, |o| print_output(&o, verbose))?;
    Ok(())
}

fn status_line(s: &Status) -> String {
    match s {
        Status::Connected { interface, report_len, port } => {
            format!("-- connected at {port} (interface {interface}, {report_len}-byte reports)")
        }
        Status::Disconnected => "-- disconnected".into(),
        Status::Error { message } => format!("-- error: {message}"),
    }
}

fn print_output(o: &Output, verbose: bool) {
    let line = match o {
        Output::Pointer(p) => {
            if p.kind == PointerKind::Move && !verbose {
                return;
            }
            let kind = match p.kind {
                PointerKind::Down => "DOWN",
                PointerKind::Move => "MOVE",
                PointerKind::Up => "UP",
            };
            format!("{:>7}ms   {kind:<4} id={} ({:>4.0},{:>4.0})", p.t_ms, p.id, p.x, p.y)
        }
        Output::Gesture(g) => {
            let name = match &g.gesture {
                Gesture::DragMove { .. } | Gesture::Pinch { .. } | Gesture::LongPressRepeat { .. } if !verbose => return,
                Gesture::Swipe { dir, fingers } => format!("Swipe {dir:?} x{fingers}"),
                Gesture::Pinch { scale } => format!("Pinch {scale:.2}"),
                Gesture::DragMove { dx, dy } => format!("DragMove {dx:+.0},{dy:+.0}"),
                Gesture::LongPressRepeat { n } => format!("LongPressRepeat {n}"),
                other => format!("{other:?}"),
            };
            format!("{:>7}ms * {name:<18} at ({:>4.0},{:>4.0})", g.t_ms, g.x, g.y)
        }
        Output::Ignored { report_id } => format!("note: ignoring non-touch report {report_id:#04x}"),
    };
    println!("{line}");
}

fn set_device_mode(p: &Panel, mode: i64) -> Result<()> {
    if p.set_device_mode(mode)? {
        println!("device mode set to {mode}");
        Ok(())
    } else {
        Err(anyhow!("descriptor has no Device Mode feature (0x0D:0x52)"))
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}
