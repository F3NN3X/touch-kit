//! HID report descriptors: parsed by `hidreport`, flattened into the fields touch decoding needs
//! (tip switch, contact id, X/Y, contact count, feature reports such as Device Mode).

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Input,
    Output,
    Feature,
}

#[derive(Clone, Debug)]
pub struct CollectionRef {
    /// Unique per collection in the descriptor; used to group a finger's fields.
    pub index: usize,
    /// Full 32-bit usage (page << 16 | id).
    pub usage: u32,
    /// 0 physical, 1 application, 2 logical, ...
    pub kind: u8,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub kind: Kind,
    pub report_id: u8,
    /// Bit offset from the start of the report payload (after the report-id byte).
    pub bit_offset: u32,
    pub bit_size: u32,
    pub count: u32,
    pub usages: Vec<u32>,
    pub flags: u32,
    pub logical_min: i64,
    pub logical_max: i64,
    pub path: Vec<CollectionRef>,
}

impl Field {
    pub fn is_constant(&self) -> bool {
        self.flags & 0x01 != 0
    }
    pub fn is_variable(&self) -> bool {
        self.flags & 0x02 != 0
    }
    /// Usage of element `i`. Variable fields with fewer usages than elements repeat the last one.
    pub fn usage_at(&self, i: u32) -> Option<u32> {
        if self.usages.is_empty() {
            return None;
        }
        Some(*self.usages.get(i as usize).unwrap_or(self.usages.last().unwrap()))
    }
    pub fn element_offset(&self, i: u32) -> u32 {
        self.bit_offset + i * self.bit_size
    }
    /// Read element `i` from a report payload (report-id byte already stripped).
    pub fn read(&self, payload: &[u8], i: u32) -> i64 {
        let raw = extract_bits(payload, self.element_offset(i), self.bit_size);
        if self.logical_min < 0 && self.bit_size > 0 && self.bit_size < 64 {
            let sign = 1u64 << (self.bit_size - 1);
            if raw & sign != 0 {
                return (raw as i64) - (1i64 << self.bit_size);
            }
        }
        raw as i64
    }
    /// Write element `i` into a report payload.
    pub fn write(&self, payload: &mut [u8], i: u32, value: i64) {
        insert_bits(payload, self.element_offset(i), self.bit_size, value as u64);
    }
    /// The application (top-level) collection usage, if any.
    pub fn application(&self) -> Option<u32> {
        self.path.iter().find(|c| c.kind == 1).map(|c| c.usage)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ReportDescriptor {
    pub fields: Vec<Field>,
    pub uses_report_ids: bool,
    /// Total bits per (kind, report id).
    pub report_bits: HashMap<(Kind, u8), u32>,
}

impl ReportDescriptor {
    /// Report length in bytes including the report-id byte when ids are used.
    pub fn report_len(&self, kind: Kind, id: u8) -> usize {
        let bits = self.report_bits.get(&(kind, id)).copied().unwrap_or(0);
        (bits as usize).div_ceil(8) + usize::from(self.uses_report_ids)
    }
    pub fn max_input_len(&self) -> usize {
        self.report_bits
            .keys()
            .filter(|(k, _)| *k == Kind::Input)
            .map(|(k, id)| self.report_len(*k, *id))
            .max()
            .unwrap_or(0)
    }
    /// First field (and element index) of the given kind carrying `usage`.
    pub fn find(&self, kind: Kind, usage: u32) -> Option<(usize, u32)> {
        for (fi, f) in self.fields.iter().enumerate() {
            if f.kind != kind || f.is_constant() {
                continue;
            }
            for i in 0..f.count {
                if f.usage_at(i) == Some(usage) {
                    return Some((fi, i));
                }
            }
        }
        None
    }
    /// Split a raw report into (report id, payload).
    pub fn split<'a>(&self, report: &'a [u8]) -> (u8, &'a [u8]) {
        if self.uses_report_ids && !report.is_empty() {
            (report[0], &report[1..])
        } else {
            (0, report)
        }
    }
}

/// Parses a report descriptor with `hidreport` and flattens it into [`Field`]s.
///
/// hidreport splits a variable main item into one field per element; each becomes a `Field` with
/// `count` 1. Bit offsets here are relative to the payload, i.e. without the report-id byte.
pub fn parse(desc: &[u8]) -> Result<ReportDescriptor, String> {
    let rd = hidreport::ReportDescriptor::try_from(desc).map_err(|e| e.to_string())?;
    let mut out = ReportDescriptor::default();
    // hidreport's collection ids are opaque; number them in order of first appearance.
    let mut collections: Vec<hidreport::CollectionId> = Vec::new();

    add_reports(&mut out, &mut collections, Kind::Input, rd.input_reports());
    add_reports(&mut out, &mut collections, Kind::Output, rd.output_reports());
    add_reports(&mut out, &mut collections, Kind::Feature, rd.feature_reports());
    Ok(out)
}

fn add_reports(
    out: &mut ReportDescriptor,
    collections: &mut Vec<hidreport::CollectionId>,
    kind: Kind,
    reports: &[impl hidreport::Report],
) {
    for report in reports {
        let id = report.report_id().map(u8::from);
        let id_bits = if id.is_some() { 8 } else { 0 };
        out.uses_report_ids |= id.is_some();
        let rid = id.unwrap_or(0);
        out.report_bits.insert((kind, rid), (report.size_in_bits() - id_bits) as u32);
        for f in report.fields() {
            out.fields.push(field(kind, rid, id_bits, f, collections));
        }
    }
}

fn field(kind: Kind, report_id: u8, id_bits: usize, f: &hidreport::Field, ids: &mut Vec<hidreport::CollectionId>) -> Field {
    use hidreport::Field as F;
    let bits = f.bits();
    let path = f
        .collections()
        .iter()
        .map(|c| {
            let index = ids.iter().position(|i| i == c.id()).unwrap_or_else(|| {
                ids.push(c.id().clone());
                ids.len() - 1
            });
            CollectionRef { index, usage: c.usages().first().map_or(0, u32::from), kind: collection_kind(c.collection_type()) }
        })
        .collect();
    let (count, usages, flags, logical_min, logical_max) = match f {
        F::Variable(v) => (1, vec![u32::from(&v.usage)], 0x02, i32::from(v.logical_minimum), i32::from(v.logical_maximum)),
        F::Array(a) => {
            let usages = a.usages().iter().map(u32::from).collect();
            (usize::from(a.report_count) as u32, usages, 0x00, i32::from(a.logical_minimum), i32::from(a.logical_maximum))
        }
        F::Constant(_) => (1, Vec::new(), 0x01, 0, 0),
    };
    let bit_size = bits.len() as u32 / count.max(1);
    let mut logical_max = i64::from(logical_max);
    if logical_min >= 0 && logical_max < 0 && (1..64).contains(&bit_size) {
        // hidreport reads Logical Maximum as signed, so an unsigned max with the sign bit set
        // (e.g. 1-byte 0xFF for a 0..255 contact id) comes out negative. With a non-negative minimum
        // it is unsigned (as Linux reads it); it can't exceed what the field holds.
        logical_max &= (1i64 << bit_size) - 1;
    }
    Field {
        kind,
        report_id,
        bit_offset: (bits.start - id_bits) as u32,
        bit_size,
        count,
        usages,
        flags,
        logical_min: logical_min.into(),
        logical_max,
        path,
    }
}

/// The HID main-item collection type number (0 physical, 1 application, 2 logical, ...).
fn collection_kind(t: hidreport::CollectionType) -> u8 {
    use hidreport::CollectionType as T;
    match t {
        T::Physical => 0,
        T::Application => 1,
        T::Logical => 2,
        T::Report => 3,
        T::NamedArray => 4,
        T::UsageSwitch => 5,
        T::UsageModifier => 6,
        T::Reserved { value } | T::VendorDefined { value } => value,
    }
}

pub fn extract_bits(data: &[u8], bit_offset: u32, bits: u32) -> u64 {
    let mut v = 0u64;
    for k in 0..bits.min(64) {
        let bi = bit_offset + k;
        let byte = (bi / 8) as usize;
        if byte >= data.len() {
            break;
        }
        if (data[byte] >> (bi % 8)) & 1 == 1 {
            v |= 1 << k;
        }
    }
    v
}

pub fn insert_bits(data: &mut [u8], bit_offset: u32, bits: u32, value: u64) {
    for k in 0..bits.min(64) {
        let bi = bit_offset + k;
        let byte = (bi / 8) as usize;
        if byte >= data.len() {
            break;
        }
        let mask = 1u8 << (bi % 8);
        if (value >> k) & 1 == 1 {
            data[byte] |= mask;
        } else {
            data[byte] &= !mask;
        }
    }
}

/// The HID Usage Tables name of a full 32-bit usage (page << 16 | id), or `PPPP:UUUU` if unknown.
pub fn usage_name(u: u32) -> String {
    hut::Usage::try_from(u).map_or_else(|_| format!("{:04X}:{:04X}", u >> 16, u & 0xFFFF), |h| h.name())
}

/// Multi-line dump of every field, for `touchprobe info`.
pub fn describe(rd: &ReportDescriptor) -> String {
    let mut s = String::new();
    s.push_str(&format!("uses report ids: {}\n", rd.uses_report_ids));
    let mut ids: Vec<_> = rd.report_bits.keys().copied().collect();
    ids.sort_by_key(|(k, id)| (format!("{k:?}"), *id));
    for (k, id) in ids {
        s.push_str(&format!("  {k:?} report {id:#04x}: {} bytes\n", rd.report_len(k, id)));
    }
    s.push('\n');
    for f in &rd.fields {
        let path: Vec<String> = f.path.iter().map(|c| format!("{}#{}", usage_name(c.usage), c.index)).collect();
        let usages: Vec<String> = {
            let mut seen = Vec::new();
            for i in 0..f.count.min(16) {
                if let Some(u) = f.usage_at(i) {
                    let n = usage_name(u);
                    if seen.last() != Some(&n) {
                        seen.push(n);
                    }
                }
            }
            seen
        };
        s.push_str(&format!(
            "{:<7} id={:#04x} bit={:<4} {}x{:<3} {} {} [{}..{}] {} :: {}\n",
            format!("{:?}", f.kind),
            f.report_id,
            f.bit_offset,
            f.bit_size,
            f.count,
            if f.is_constant() { "const" } else { "data " },
            if f.is_variable() { "var" } else { "arr" },
            f.logical_min,
            f.logical_max,
            if usages.is_empty() { "-".into() } else { usages.join(",") },
            path.join(" > "),
        ));
    }
    s
}
