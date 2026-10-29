//! Device reports: everything about a touchscreen another app needs to support it — USB identity,
//! the HID report layout with physical ranges and units, the features, and optionally a recording
//! of real touches (in `touchprobe record` format, so `replay` and the tests can use it).
//!
//! No serial numbers: a report is meant to be shared.

use crate::config::{DeviceConfig, GestureConfig, PanelConfig};
use crate::discover::Touchscreen;
use crate::hid::{self, Kind};
use crate::pipeline::Output;
use crate::replay::Recording;
use crate::touch::{Layout, U_DEVICE_MODE, U_TOUCH_SCREEN, U_X, U_Y};
use crate::usb::{self, Panel};
use crate::{GestureEvent, PointerKind};
use nusb::MaybeFuture;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize)]
pub struct DeviceReport {
    pub vid: String,
    pub pid: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    /// Whether the device has a USB serial number (the number itself is left out).
    pub has_serial: bool,
    pub port: String,
    pub driver: Option<String>,
    pub ready: bool,
    pub usb_version: Option<String>,
    pub device_version: Option<String>,
    pub speed: Option<String>,
    /// What opening it found; `None` when it isn't ready or couldn't be opened (see `error`).
    pub hid: Option<HidReport>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HidReport {
    pub interface: u8,
    pub endpoint: String,
    pub max_packet_size: usize,
    pub uses_report_ids: bool,
    /// The raw report descriptor, hex: everything below can be re-derived from it.
    pub descriptor_hex: String,
    /// Report ids → length in bytes (including the id byte), per kind.
    pub input_reports: BTreeMap<String, usize>,
    pub feature_reports: BTreeMap<String, usize>,
    pub touch: Vec<TouchSummary>,
    /// Contact Count Maximum as the device reports it (feature GET_REPORT).
    pub contact_count_max: Option<i64>,
    pub contact_count_max_error: Option<String>,
    pub has_device_mode_feature: bool,
    /// Input reports outside a Touch Screen collection, by application (mouse, vendor, ...).
    pub other_inputs: Vec<String>,
    pub fields: Vec<FieldInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TouchSummary {
    pub report_id: u8,
    pub length: usize,
    /// Contacts per report.
    pub slots: usize,
    pub has_contact_count: bool,
    pub has_scan_time: bool,
    pub has_confidence: bool,
    pub has_width_height: bool,
    pub x: Option<Axis>,
    pub y: Option<Axis>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Axis {
    pub logical: [i64; 2],
    pub physical: Option<[i64; 2]>,
    pub unit: Option<String>,
    pub unit_exponent: Option<i8>,
    /// Physical size of the axis in millimetres, when the descriptor says (unit cm or inch). Firmware
    /// often copies these from another model: check against the display before relying on it.
    pub size_mm: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FieldInfo {
    pub kind: String,
    pub report_id: u8,
    pub application: Option<String>,
    pub usage: String,
    pub bit_offset: usize,
    pub bit_size: usize,
    pub count: usize,
    pub logical: [i64; 2],
    pub physical: Option<[i64; 2]>,
    pub unit: Option<String>,
    pub unit_exponent: Option<i8>,
}

/// A recording of real touches and what the pipeline made of it.
#[derive(Clone, Debug, Serialize)]
pub struct TouchRecording {
    pub port: String,
    pub seconds: f64,
    pub reports: usize,
    pub reports_per_second: f64,
    /// Most fingers down at once.
    pub max_contacts: usize,
    /// Smallest and largest position reached, as fractions of the touch area: shows whether the
    /// whole surface reports and whether the axes run the expected way.
    pub x_reached: Option<[f64; 2]>,
    pub y_reached: Option<[f64; 2]>,
    /// Recognised gestures, by name.
    pub gestures: BTreeMap<String, usize>,
    /// Report ids that arrived but aren't touch reports (e.g. `0x05` while in mouse mode).
    pub non_touch_report_ids: Vec<String>,
    /// `touchprobe record` format: a descriptor line, then one `{"t_ms", "report"}` line each.
    pub capture_jsonl: String,
}

/// Reports every touchscreen `candidates` finds. Ready ones are opened (one at a time), so nothing
/// else may hold them: stop the panels first.
pub fn devices() -> Result<Vec<DeviceReport>, crate::Error> {
    let found = usb::candidates(&DeviceConfig::default())?;
    let infos: Vec<nusb::DeviceInfo> =
        nusb::list_devices().wait().map_err(|e| crate::Error::Usb(format!("listing USB devices: {e:?}")))?.collect();
    Ok(found
        .iter()
        .map(|t| {
            let info = infos.iter().find(|d| usb::location(d) == t.port);
            let mut r = identity(t, info);
            if t.ready {
                match open(&t.port).and_then(|p| hid_report(&p)) {
                    Ok(h) => r.hid = Some(h),
                    Err(e) => r.error = Some(e.to_string()),
                }
            }
            r
        })
        .collect())
}

/// Opens the touchscreen at `port`.
pub fn open(port: &str) -> Result<Panel, crate::Error> {
    usb::open(&DeviceConfig { port: Some(port.into()), ..Default::default() }, &[])
}

fn identity(t: &Touchscreen, info: Option<&nusb::DeviceInfo>) -> DeviceReport {
    let bcd = |v: u16| format!("{}.{:02x}", v >> 8, v & 0xFF);
    DeviceReport {
        vid: format!("{:04x}", t.vid),
        pid: format!("{:04x}", t.pid),
        manufacturer: info.and_then(|d| d.manufacturer_string()).map(str::to_string),
        product: t.product.clone(),
        has_serial: t.serial.is_some(),
        port: t.port.clone(),
        driver: t.driver.clone(),
        ready: t.ready,
        usb_version: info.map(|d| bcd(d.usb_version())),
        device_version: info.map(|d| bcd(d.device_version())),
        speed: info.and_then(|d| d.speed()).map(|s| format!("{s:?}")),
        hid: None,
        error: None,
    }
}

/// The HID side of an opened touchscreen.
pub fn hid_report(p: &Panel) -> Result<HidReport, crate::Error> {
    let fields = fields(&p.rd_raw).map_err(crate::Error::Descriptor)?;
    let layout = Layout::from_descriptor(&p.rd);
    let lens = |kind: Kind| -> BTreeMap<String, usize> {
        p.rd.report_bits
            .keys()
            .filter(|(k, _)| *k == kind)
            .map(|(k, id)| (format!("{id:#04x}"), p.rd.report_len(*k, *id)))
            .collect()
    };
    let mut touch: Vec<TouchSummary> = layout
        .reports
        .iter()
        .map(|(id, r)| {
            let axis = |usage: u32| {
                fields.iter().find(|f| f.kind == "Input" && f.report_id == *id && f.usage == hid::usage_name(usage)).map(|f| {
                    Axis {
                        logical: f.logical,
                        physical: f.physical,
                        unit: f.unit.clone(),
                        unit_exponent: f.unit_exponent,
                        size_mm: size_mm(f),
                    }
                })
            };
            TouchSummary {
                report_id: *id,
                length: p.rd.report_len(Kind::Input, *id),
                slots: r.slots.len(),
                has_contact_count: r.contact_count.is_some(),
                has_scan_time: r.scan_time.is_some(),
                has_confidence: r.slots.iter().any(|s| s.confidence.is_some()),
                has_width_height: r.slots.iter().any(|s| s.width.is_some() || s.height.is_some()),
                x: axis(U_X),
                y: axis(U_Y),
            }
        })
        .collect();
    touch.sort_by_key(|t| t.report_id);
    let other_inputs: Vec<String> =
        p.rd.fields
            .iter()
            .filter(|f| f.kind == Kind::Input && !f.is_constant())
            .filter_map(|f| f.application().filter(|&a| a != U_TOUCH_SCREEN).map(|a| (f.report_id, a)))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|(id, a)| format!("report {id:#04x}: {}", hid::usage_name(a)))
            .collect();
    let (contact_count_max, contact_count_max_error) = match p.contact_max() {
        Ok(v) => (v, None),
        Err(e) => (None, Some(e.to_string())),
    };
    Ok(HidReport {
        interface: p.iface,
        endpoint: format!("{:#04x}", p.ep_in),
        max_packet_size: p.mps,
        uses_report_ids: p.rd.uses_report_ids,
        descriptor_hex: hex(&p.rd_raw),
        input_reports: lens(Kind::Input),
        feature_reports: lens(Kind::Feature),
        touch,
        contact_count_max,
        contact_count_max_error,
        has_device_mode_feature: p.rd.find(Kind::Feature, U_DEVICE_MODE).is_some(),
        other_inputs,
        fields,
    })
}

/// Every field, with what our flattened `hid::Field` leaves out (physical range, unit).
pub fn fields(descriptor: &[u8]) -> Result<Vec<FieldInfo>, String> {
    let rd = hidreport::ReportDescriptor::try_from(descriptor).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    add_fields(&mut out, "Input", rd.input_reports());
    add_fields(&mut out, "Feature", rd.feature_reports());
    add_fields(&mut out, "Output", rd.output_reports());
    out.sort_by_key(|f| (f.kind.clone(), f.report_id, f.bit_offset));
    Ok(out)
}

fn add_fields(out: &mut Vec<FieldInfo>, kind: &str, reports: &[impl hidreport::Report]) {
    use hidreport::Field as F;
    for r in reports {
        let id = r.report_id().map(u8::from);
        let id_bits = if id.is_some() { 8 } else { 0 };
        for f in r.fields() {
            let (usage, count, logical, physical, unit, exponent, collections) = match f {
                F::Variable(v) => (
                    hid::usage_name(u32::from(&v.usage)),
                    1,
                    [i32::from(v.logical_minimum), i32::from(v.logical_maximum)],
                    (v.physical_minimum, v.physical_maximum),
                    v.unit,
                    v.unit_exponent,
                    v.collections.as_slice(),
                ),
                F::Array(a) => (
                    a.usages().iter().map(|u| hid::usage_name(u32::from(u))).collect::<Vec<_>>().join(", "),
                    usize::from(a.report_count),
                    [i32::from(a.logical_minimum), i32::from(a.logical_maximum)],
                    (a.physical_minimum, a.physical_maximum),
                    a.unit,
                    a.unit_exponent,
                    a.collections.as_slice(),
                ),
                F::Constant(_) => continue,
            };
            let physical = match physical {
                (Some(lo), Some(hi)) if (i32::from(lo), i32::from(hi)) != (0, 0) => {
                    Some([i32::from(lo).into(), i32::from(hi).into()])
                }
                _ => None,
            };
            let unit = unit.filter(|u| u32::from(*u) != 0);
            let bits = f.bits();
            out.push(FieldInfo {
                kind: kind.into(),
                report_id: id.unwrap_or(0),
                application: application(collections),
                usage,
                bit_offset: bits.start - id_bits,
                bit_size: bits.len() / count.max(1),
                count,
                logical: [logical[0].into(), logical[1].into()],
                physical,
                unit: unit.map(|u| u.to_string()),
                unit_exponent: unit.and(exponent).map(|e| e.exponent()),
            });
        }
    }
}

fn application(collections: &[hidreport::Collection]) -> Option<String> {
    collections
        .iter()
        .find(|c| matches!(c.collection_type(), hidreport::CollectionType::Application))
        .and_then(|c| c.usages().first())
        .map(|u| hid::usage_name(u32::from(u)))
}

/// Millimetres spanned by a field's physical range, if its unit is a length in cm or inches.
fn size_mm(f: &FieldInfo) -> Option<f64> {
    let [lo, hi] = f.physical?;
    let unit = f.unit.as_deref()?;
    let per_unit = if unit.starts_with("cm") {
        10.0
    } else if unit.starts_with("in") {
        25.4
    } else {
        return None;
    };
    Some((hi - lo) as f64 * per_unit * 10f64.powi(f.unit_exponent.unwrap_or(0).into()))
}

/// Records `panel` for `duration` (or until `stop` returns true) and summarises the touches, mapped
/// as `mapping` (the panel's size and orientation: gesture thresholds are in its pixels).
pub fn record(
    panel: &Panel,
    mapping: &PanelConfig,
    duration: Duration,
    stop: impl Fn() -> bool,
) -> Result<TouchRecording, crate::Error> {
    let mut reader = panel.reader()?;
    let start = Instant::now();
    let mut reports: Vec<(u64, Vec<u8>)> = Vec::new();
    while start.elapsed() < duration && !stop() {
        if let Some(r) = reader.next(Duration::from_millis(100))? {
            reports.push((start.elapsed().as_millis() as u64, r));
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let mut capture_jsonl = format!("{{\"descriptor\":\"{}\"}}\n", hex(&panel.rd_raw));
    for (t, r) in &reports {
        capture_jsonl.push_str(&format!("{{\"t_ms\":{t},\"report\":\"{}\"}}\n", hex(r)));
    }
    let rec = Recording { descriptor: panel.rd_raw.clone(), reports };
    Ok(summarise(&rec, mapping, &panel.port, seconds, capture_jsonl))
}

/// What the pipeline makes of a recording mapped as `mapping`; positions are given as fractions.
pub fn summarise(rec: &Recording, mapping: &PanelConfig, port: &str, seconds: f64, capture_jsonl: String) -> TouchRecording {
    let outputs = rec.outputs(mapping, &GestureConfig::default()).unwrap_or_default();
    let (w, h) = (f64::from(mapping.width_px.max(2) - 1), f64::from(mapping.height_px.max(2) - 1));
    let (mut down, mut max_contacts) = (0usize, 0usize);
    let (mut xs, mut ys): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    let mut gestures: BTreeMap<String, usize> = BTreeMap::new();
    let mut non_touch_report_ids: Vec<String> = Vec::new();
    for o in &outputs {
        match o {
            Output::Pointer(p) => {
                match p.kind {
                    PointerKind::Down => down += 1,
                    PointerKind::Up => down = down.saturating_sub(1),
                    _ => {}
                }
                max_contacts = max_contacts.max(down);
                xs.push(f64::from(p.x) / w);
                ys.push(f64::from(p.y) / h);
            }
            Output::Gesture(GestureEvent { gesture, .. }) => {
                let name = format!("{gesture:?}");
                let name = name.split([' ', '{', '(']).next().unwrap_or_default().to_string();
                *gestures.entry(name).or_default() += 1;
            }
            Output::Ignored { report_id } => non_touch_report_ids.push(format!("{report_id:#04x}")),
        }
    }
    let span = |v: &[f64]| {
        let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (!v.is_empty()).then(|| [round3(lo), round3(hi)])
    };
    TouchRecording {
        port: port.into(),
        seconds: round3(seconds),
        reports: rec.reports.len(),
        reports_per_second: if seconds > 0.0 { round3(rec.reports.len() as f64 / seconds) } else { 0.0 },
        max_contacts,
        x_reached: span(&xs),
        y_reached: span(&ys),
        gestures,
        non_touch_report_ids,
        capture_jsonl,
    }
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
