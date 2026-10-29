use crate::config::DeviceConfig;
use crate::discover::Touchscreen;
use crate::usb::{needs_driver, select};

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

fn screen(port: &str, product: Option<&str>, ready: bool) -> Touchscreen {
    Touchscreen {
        vid: 0x1234,
        pid: 0x5678,
        port: port.into(),
        serial: None,
        product: product.map(Into::into),
        driver: None,
        ready,
    }
}

#[test]
fn a_touchscreen_without_its_driver_set_up_says_so() {
    let any = DeviceConfig::default();
    let err = needs_driver(&[screen("B-1", Some("Some Panel"), false)], &any, &[]).unwrap().to_string();
    assert!(err.starts_with("Some Panel found") && err.contains("WinUSB"), "{err}");
    let unnamed = needs_driver(&[screen("B-1", None, false)], &any, &[]).unwrap().to_string();
    assert!(unnamed.starts_with("1234:5678 found"), "{unnamed}");
    assert!(needs_driver(&[screen("B-1", None, true)], &any, &[]).is_none(), "a ready one isn't waiting for setup");
    assert!(needs_driver(&[screen("B-1", None, false)], &any, &ports(&["B-1"])).is_none(), "another panel's");
}
