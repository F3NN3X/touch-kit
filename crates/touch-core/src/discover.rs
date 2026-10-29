//! Finding touchscreens of any make: USB devices with a HID Touch Screen collection (Digitizer page,
//! usage 0x04), the collection every touchscreen reports for the OS's own touch support.
//!
//! - **Windows.** A touchscreen Windows still owns (HID driver) is found through SetupAPI: one of its
//!   HID collections carries the hardware id `HID_DEVICE_UP:000D_U:0004`. Once it is bound to WinUSB
//!   those collections are gone, so a WinUSB device with a HID-class interface counts; opening it
//!   reads the report descriptor and confirms it (`usb::open`).
//! - **Linux.** `/sys/bus/hid/devices/*/report_descriptor` is readable without root; a descriptor with
//!   a touch report marks its USB device. A device whose kernel driver this process has detached is
//!   missing from sysfs until it is released.
//! - **Elsewhere** every device with a HID-class interface is a candidate.

use crate::Error;
use nusb::MaybeFuture;

/// A touchscreen, and whether this process can read it.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Touchscreen {
    pub vid: u16,
    pub pid: u16,
    /// See `usb::location`.
    pub port: String,
    pub serial: Option<String>,
    pub product: Option<String>,
    /// The bound driver service (`WinUSB`, `HidUsb`, ...). Windows only; `None` elsewhere.
    pub driver: Option<String>,
    /// Readable over raw USB: bound to WinUSB on Windows; always on Linux (the kernel driver is
    /// detached on open). False means the driver setup is still to do.
    pub ready: bool,
}

/// Every connected touchscreen.
pub fn touchscreens() -> Result<Vec<Touchscreen>, Error> {
    let devices: Vec<nusb::DeviceInfo> =
        nusb::list_devices().wait().map_err(|e| Error::Usb(format!("listing USB devices: {e:?}")))?.collect();
    let probe = platform::Probe::new();
    Ok(devices.iter().filter_map(|d| Some(entry(d, probe.classify(d)?))).collect())
}

pub(crate) fn entry(d: &nusb::DeviceInfo, ready: bool) -> Touchscreen {
    Touchscreen {
        vid: d.vendor_id(),
        pid: d.product_id(),
        port: crate::usb::location(d),
        serial: d.serial_number().map(str::to_string),
        product: d.product_string().map(str::to_string),
        driver: platform::driver(d),
        ready,
    }
}

/// Whether this process can open `d` over raw USB: bound to WinUSB on Windows, always elsewhere.
pub(crate) fn readable(d: &nusb::DeviceInfo) -> bool {
    platform::driver(d).is_none_or(|s| s.eq_ignore_ascii_case("winusb"))
}

fn has_hid_interface(d: &nusb::DeviceInfo) -> bool {
    d.interfaces().any(|i| i.class() == 3)
}

#[cfg(windows)]
mod platform {
    use windows::core::PCWSTR;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Device_IDW, CM_Get_Parent, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceRegistryPropertyW, CR_SUCCESS, DIGCF_ALLCLASSES, DIGCF_PRESENT, SPDRP_HARDWAREID, SP_DEVINFO_DATA,
    };

    /// Hardware id of a HID Touch Screen collection.
    const TOUCH_SCREEN_ID: &str = "HID_DEVICE_UP:000D_U:0004";

    pub struct Probe {
        /// Instance ids (upper case) of USB devices with a Touch Screen collection Windows owns.
        hid_owned: Vec<String>,
    }

    impl Probe {
        pub fn new() -> Self {
            Self { hid_owned: hid_parents(TOUCH_SCREEN_ID) }
        }

        /// `Some(ready)` for a touchscreen, `None` for anything else.
        pub fn classify(&self, d: &nusb::DeviceInfo) -> Option<bool> {
            let id = d.instance_id().to_string_lossy().to_ascii_uppercase();
            if self.hid_owned.contains(&id) {
                return Some(false);
            }
            let winusb = d.driver().is_some_and(|s| s.eq_ignore_ascii_case("winusb"));
            (winusb && super::has_hid_interface(d)).then_some(true)
        }
    }

    pub fn driver(d: &nusb::DeviceInfo) -> Option<String> {
        d.driver().map(str::to_string)
    }

    /// The USB device above every present HID collection with hardware id `collection`.
    fn hid_parents(collection: &str) -> Vec<String> {
        let enumerator: Vec<u16> = "HID\0".encode_utf16().collect();
        let mut out = Vec::new();
        // SAFETY: SetupAPI/CfgMgr calls with buffers sized as the API requires; the info set is
        // destroyed before returning.
        unsafe {
            let Ok(set) = SetupDiGetClassDevsW(None, PCWSTR(enumerator.as_ptr()), None, DIGCF_PRESENT | DIGCF_ALLCLASSES) else {
                return out;
            };
            let mut index = 0;
            loop {
                let mut info = SP_DEVINFO_DATA { cbSize: size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
                if SetupDiEnumDeviceInfo(set, index, &mut info).is_err() {
                    break;
                }
                index += 1;
                let mut buf = [0u8; 2048];
                if SetupDiGetDeviceRegistryPropertyW(set, &info, SPDRP_HARDWAREID, None, Some(&mut buf), None).is_err() {
                    continue;
                }
                if !multi_sz(&buf).iter().any(|s| s.eq_ignore_ascii_case(collection)) {
                    continue;
                }
                // Collection -> USB device, or -> USB interface (composite, `&MI_xx`) -> USB device.
                let mut node = info.DevInst;
                for _ in 0..3 {
                    let mut parent = 0u32;
                    if CM_Get_Parent(&mut parent, node, 0) != CR_SUCCESS {
                        break;
                    }
                    let id = device_id(parent).to_ascii_uppercase();
                    if id.starts_with("USB\\") && !id.contains("&MI_") {
                        if !out.contains(&id) {
                            out.push(id);
                        }
                        break;
                    }
                    node = parent;
                }
            }
            let _ = SetupDiDestroyDeviceInfoList(set);
        }
        out
    }

    unsafe fn device_id(devinst: u32) -> String {
        let mut buf = [0u16; 512];
        if CM_Get_Device_IDW(devinst, &mut buf, 0) != CR_SUCCESS {
            return String::new();
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }

    /// A REG_MULTI_SZ value (UTF-16, NUL-separated, double-NUL-terminated) as strings.
    fn multi_sz(bytes: &[u8]) -> Vec<String> {
        let wide: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        wide.split(|&c| c == 0).take_while(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn multi_sz_splits_on_nul() {
            let text = "HID\\VID_222A&PID_0001&Col01\0HID_DEVICE_UP:000D_U:0004\0\0";
            let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            assert_eq!(super::multi_sz(&bytes), vec!["HID\\VID_222A&PID_0001&Col01", super::TOUCH_SCREEN_ID]);
        }

        /// The walk that finds Windows-owned touchscreens, checked on a mouse's collection instead.
        #[test]
        #[ignore = "needs a USB mouse Windows owns"]
        fn finds_the_usb_device_above_a_mouse_collection() {
            let parents = super::hid_parents("HID_DEVICE_UP:0001_U:0002");
            println!("{parents:#?}");
            assert!(!parents.is_empty() && parents.iter().all(|p| p.starts_with("USB\\VID_")));
            // `Probe::classify` links them to nusb's devices by instance id.
            use nusb::MaybeFuture;
            let ids: Vec<String> =
                nusb::list_devices().wait().unwrap().map(|d| d.instance_id().to_string_lossy().to_ascii_uppercase()).collect();
            assert!(parents.iter().all(|p| ids.contains(p)), "nusb ids: {ids:#?}");
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::path::PathBuf;

    pub struct Probe {
        /// Canonical sysfs paths of HID devices with a touch report.
        touch_hid: Vec<PathBuf>,
    }

    impl Probe {
        pub fn new() -> Self {
            let mut touch_hid = Vec::new();
            for entry in std::fs::read_dir("/sys/bus/hid/devices").into_iter().flatten().flatten() {
                let Ok(rd) = std::fs::read(entry.path().join("report_descriptor")) else { continue };
                let is_touch = crate::hid::parse(&rd)
                    .map(|rd| !crate::touch::Layout::from_descriptor(&rd).reports.is_empty())
                    .unwrap_or(false);
                if let (true, Ok(path)) = (is_touch, std::fs::canonicalize(entry.path())) {
                    touch_hid.push(path);
                }
            }
            Self { touch_hid }
        }

        /// A USB device is a touchscreen if a touch HID device sits below it in sysfs.
        pub fn classify(&self, d: &nusb::DeviceInfo) -> Option<bool> {
            let usb = std::fs::canonicalize(d.sysfs_path()).ok()?;
            self.touch_hid.iter().any(|h| h.starts_with(&usb)).then_some(true)
        }
    }

    pub fn driver(_: &nusb::DeviceInfo) -> Option<String> {
        None
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    pub struct Probe;

    impl Probe {
        pub fn new() -> Self {
            Self
        }

        pub fn classify(&self, d: &nusb::DeviceInfo) -> Option<bool> {
            super::has_hid_interface(d).then_some(true)
        }
    }

    pub fn driver(_: &nusb::DeviceInfo) -> Option<String> {
        None
    }
}
