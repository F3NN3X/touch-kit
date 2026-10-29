use crate::usb::select;

fn ports(p: &[&str]) -> Vec<String> {
    p.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_panel_keeps_its_port_and_never_takes_another_panels() {
    let connected = ports(&["B-1", "B-2"]);
    assert_eq!(select(&connected, Some("B-2"), &ports(&["B-1"])), Some(1));
    assert_eq!(select(&connected, None, &ports(&["B-1"])), Some(1), "unnamed port: first free one");
    assert_eq!(select(&connected, None, &ports(&["B-1", "B-2"])), None);
}

#[test]
fn a_moved_panel_follows_only_an_unambiguous_free_device() {
    // B-9 is gone: the panel was replugged elsewhere.
    assert_eq!(select(&ports(&["B-1", "B-3"]), Some("B-9"), &ports(&["B-1"])), Some(1));
    assert_eq!(select(&ports(&["B-1", "B-3"]), Some("B-9"), &[]), None, "two free devices: wait for pairing");
    assert_eq!(select(&[], Some("B-9"), &[]), None);
}
