//! The ILITEK 222A:0001 descriptor from `captures/`, and the hand-decoded reports in docs/PLAN.md §1.

use crate::hid::{self, Kind, ReportDescriptor};
use crate::touch::{self, Assembler, Decoded, Layout};

fn rd() -> ReportDescriptor {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../captures/report-descriptor.bin");
    hid::parse(&std::fs::read(path).expect("captures/report-descriptor.bin")).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn layout_is_ten_slot_parallel_report_4() {
    let rd = rd();
    let layout = Layout::from_descriptor(&rd);
    assert_eq!(layout.reports.keys().copied().collect::<Vec<_>>(), vec![4]);
    let tr = &layout.reports[&4];
    assert_eq!(tr.slots.len(), 10);
    assert!(tr.contact_count.is_some());
    assert_eq!(rd.report_len(Kind::Input, 4), 64);
    let (fx, _) = tr.slots[0].x.unwrap();
    assert_eq!((rd.fields[fx].logical_min, rd.fields[fx].logical_max), (0, 16384));
    assert!(rd.find(Kind::Feature, touch::U_DEVICE_MODE).is_none(), "this panel has no Device Mode");
    assert!(!layout.reports.contains_key(&5), "mouse report 0x05 is not touch");
}

/// Decodes a report given its leading bytes; contacts as (id, x px, y px) at 1920x550, rounded.
/// (docs/PLAN.md §1 lists the same values truncated.)
fn decode(report_prefix: &str, cc: u8) -> Vec<(u32, i32, i32)> {
    let rd = rd();
    let layout = Layout::from_descriptor(&rd);
    let mut r = unhex(report_prefix);
    r.resize(64, 0);
    r[55] = cc; // payload byte 54 = Contact Count
    let Decoded::Frame(f) = Assembler::default().feed(&rd, &layout, &r) else { panic!("expected a frame") };
    f.iter().map(|c| (c.id, (c.x * 1920.0).round() as i32, (c.y * 550.0).round() as i32)).collect()
}

#[test]
fn hand_decoded_reports_match() {
    assert_eq!(decode("0440ef3d6d3a", 1), vec![(0, 1858, 502)]); // tap t=1
    assert_eq!(decode("04404123251f", 1), vec![(0, 1058, 268)]); // swipes t=710
    assert_eq!(decode("044054260f11414429d222", 2), vec![(0, 1150, 147), (1, 1238, 299)]); // two-finger t=707
    assert_eq!(decode("0440e61dc01d", 1), vec![(0, 897, 256)]); // hold t=997
}

#[test]
fn lift_report_has_tip_clear() {
    // tap.jsonl t=851: the single report that ends the top-left tap.
    let rd = rd();
    let layout = Layout::from_descriptor(&rd);
    let mut r = unhex("0400");
    r.resize(64, 0);
    r[55] = 1;
    let Decoded::Frame(f) = Assembler::default().feed(&rd, &layout, &r) else { panic!() };
    assert_eq!(f.len(), 1);
    assert!(!f[0].tip);
}
