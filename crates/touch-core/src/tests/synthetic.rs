//! Parser/decoder tests against a synthetic ILITEK-style descriptor
//! (2 fingers per report, contact count, Device Mode feature, mouse fallback collection).

use crate::hid::{self, Kind};
use crate::touch::{self, Assembler, Decoded, Event, Layout, Tracker};
use std::time::Duration;

fn finger() -> Vec<u8> {
    vec![
        0x09, 0x22, 0xA1, 0x02, // Usage Finger, Collection Logical
        0x09, 0x42, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x01, 0x81, 0x02, // Tip Switch 1 bit
        0x75, 0x07, 0x81, 0x03, // 7 bit padding
        0x75, 0x08, 0x09, 0x51, 0x25, 0xFF, 0x81, 0x02, // Contact ID, LogMax 0xFF (quirk: reads as -1)
        0x05, 0x01, 0x26, 0xFF, 0x3F, 0x75, 0x10, 0x09, 0x30, 0x81, 0x02, // X 0..16383
        0x26, 0x7F, 0x25, 0x09, 0x31, 0x81, 0x02, // Y 0..9599
        0x05, 0x0D, 0xC0,
    ]
}

fn descriptor() -> Vec<u8> {
    let mut d = vec![0x05, 0x0D, 0x09, 0x04, 0xA1, 0x01, 0x85, 0x01];
    d.extend(finger());
    d.extend(finger());
    d.extend([0x09, 0x54, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, 0xC0]);
    // Device configuration: Device Mode + Device Identifier feature, report 3
    d.extend([
        0x05, 0x0D, 0x09, 0x0E, 0xA1, 0x01, 0x85, 0x03, 0x09, 0x23, 0xA1, 0x02, 0x09, 0x52, 0x09, 0x53, 0x15, 0x00, 0x25, 0x0A,
        0x75, 0x08, 0x95, 0x02, 0xB1, 0x02, 0xC0, 0xC0,
    ]);
    // Mouse fallback, report 2
    d.extend([
        0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x85, 0x02, 0x09, 0x01, 0xA1, 0x00, 0x05, 0x09, 0x19, 0x01, 0x29, 0x02, 0x15, 0x00,
        0x25, 0x01, 0x75, 0x01, 0x95, 0x02, 0x81, 0x02, 0x95, 0x06, 0x81, 0x03, 0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x26, 0xFF,
        0x7F, 0x75, 0x10, 0x95, 0x02, 0x81, 0x02, 0xC0, 0xC0,
    ]);
    d
}

fn finger_bytes(tip: bool, id: u8, x: u16, y: u16) -> Vec<u8> {
    let mut v = vec![tip as u8, id];
    v.extend(x.to_le_bytes());
    v.extend(y.to_le_bytes());
    v
}

fn report(f0: Vec<u8>, f1: Vec<u8>, cc: u8) -> Vec<u8> {
    let mut r = vec![0x01];
    r.extend(f0);
    r.extend(f1);
    r.push(cc);
    r
}

#[test]
fn parses_layout_and_sizes() {
    let rd = hid::parse(&descriptor()).unwrap();
    assert!(rd.uses_report_ids);
    assert_eq!(rd.report_len(Kind::Input, 1), 14);
    assert_eq!(rd.report_len(Kind::Feature, 3), 3);
    assert_eq!(rd.report_len(Kind::Input, 2), 6);

    let layout = Layout::from_descriptor(&rd);
    let tr = &layout.reports[&1];
    assert_eq!(tr.slots.len(), 2);
    assert!(tr.contact_count.is_some());
    assert!(!layout.reports.contains_key(&2), "mouse report must not be treated as touch");

    // Logical max quirk: 0xFF with min 0 is 255, not -1.
    let (fi, _) = rd.find(Kind::Input, touch::U_CONTACT_ID).unwrap();
    assert_eq!(rd.fields[fi].logical_max, 255);
    assert!(rd.find(Kind::Feature, touch::U_DEVICE_MODE).is_some());
}

#[test]
fn decodes_parallel_frame_and_events() {
    let rd = hid::parse(&descriptor()).unwrap();
    let layout = Layout::from_descriptor(&rd);
    let mut asm = Assembler::default();
    let mut tr = Tracker::new(0.001);

    let r = report(finger_bytes(true, 5, 16383, 0), finger_bytes(true, 7, 0, 9599), 2);
    let Decoded::Frame(f) = asm.feed(&rd, &layout, &r) else { panic!("expected frame") };
    assert_eq!(f.len(), 2);
    assert_eq!((f[0].id, f[0].x, f[0].y), (5, 1.0, 0.0));
    assert_eq!((f[1].id, f[1].x, f[1].y), (7, 0.0, 1.0));
    let ev = tr.update(Duration::ZERO, &f);
    assert!(matches!(ev[0], Event::Down { id: 5, .. }));
    assert!(matches!(ev[1], Event::Down { id: 7, .. }));

    // Finger 7 lifts (tip=0 reported once), finger 5 moves.
    let r = report(finger_bytes(true, 5, 8000, 100), finger_bytes(false, 7, 0, 9599), 2);
    let Decoded::Frame(f) = asm.feed(&rd, &layout, &r) else { panic!() };
    let ev = tr.update(Duration::ZERO, &f);
    assert!(ev.iter().any(|e| matches!(e, Event::Move { id: 5, .. })));
    assert!(ev.iter().any(|e| matches!(e, Event::Up { id: 7, .. })));
}

#[test]
fn assembles_hybrid_frames() {
    let rd = hid::parse(&descriptor()).unwrap();
    let layout = Layout::from_descriptor(&rd);
    let mut asm = Assembler::default();

    // 3 contacts, 2 slots per report: first report carries cc=3, second cc=0 with one valid slot.
    let a = report(finger_bytes(true, 1, 100, 100), finger_bytes(true, 2, 200, 200), 3);
    let b = report(finger_bytes(true, 3, 300, 300), finger_bytes(false, 0, 0, 0), 0);
    assert!(matches!(asm.feed(&rd, &layout, &a), Decoded::Partial));
    let Decoded::Frame(f) = asm.feed(&rd, &layout, &b) else { panic!("expected frame") };
    assert_eq!(f.iter().map(|c| c.id).collect::<Vec<_>>(), vec![1, 2, 3]);
}

#[test]
fn non_touch_report_is_flagged() {
    let rd = hid::parse(&descriptor()).unwrap();
    let layout = Layout::from_descriptor(&rd);
    let mut asm = Assembler::default();
    assert!(matches!(asm.feed(&rd, &layout, &[0x02, 0x01, 0, 0, 0, 0]), Decoded::NotTouch(2)));
}

#[test]
fn writes_feature_fields() {
    let rd = hid::parse(&descriptor()).unwrap();
    let (fi, el) = rd.find(Kind::Feature, touch::U_DEVICE_MODE).unwrap();
    let mut buf = vec![0u8; rd.report_len(Kind::Feature, 3)];
    buf[0] = 3;
    rd.fields[fi].write(&mut buf[1..], el, 2);
    assert_eq!(buf, vec![3, 2, 0]);
}
