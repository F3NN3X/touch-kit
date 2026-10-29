//! USB access: find and claim the panel, fetch its report descriptor, feature reports, and an
//! interrupt-IN reader that reassembles reports longer than one packet.

use crate::config::DeviceConfig;
use crate::discover::{self, Touchscreen};
use crate::hid::{self, Kind, ReportDescriptor};
use crate::touch::{U_CONTACT_MAX, U_DEVICE_MODE};
use crate::Error;
use nusb::descriptors::TransferType;
use nusb::transfer::{ControlIn, ControlOut, ControlType, Direction, In, Interrupt, Recipient, TransferError};
use nusb::{Endpoint, Interface, MaybeFuture};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_millis(1000);

pub struct Panel {
    _device: nusb::Device,
    pub intf: Interface,
    pub iface: u8,
    pub ep_in: u8,
    pub mps: usize,
    pub rd: ReportDescriptor,
    pub rd_raw: Vec<u8>,
    /// See [`location`].
    pub port: String,
}

fn usb(ctx: &str, e: impl std::fmt::Debug) -> Error {
    Error::Usb(format!("{ctx}: {e:?}"))
}

/// Where a device is plugged in: `<bus id>-<port chain>`, e.g. `…USBROOT(0)-4.2` on Windows. Stable
/// while the device stays in the same port; the identity of a panel without a serial number.
pub fn location(d: &nusb::DeviceInfo) -> String {
    let chain: Vec<String> = d.port_chain().iter().map(u8::to_string).collect();
    format!("{}-{}", d.bus_id(), chain.join("."))
}

/// The connected devices `cfg` may use, for settings and pairing. With both ids set: every device
/// with them, touchscreen or not. Otherwise every touchscreen whose ids fit (`discover`).
pub fn candidates(cfg: &DeviceConfig) -> Result<Vec<Touchscreen>, Error> {
    if let (Some(vid), Some(pid)) = (cfg.vid, cfg.pid) {
        return Ok(list()?
            .iter()
            .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
            .map(|d| discover::entry(d, discover::readable(d)))
            .collect());
    }
    Ok(discover::touchscreens()?.into_iter().filter(|t| cfg.accepts(t.vid, t.pid)).collect())
}

fn list() -> Result<Vec<nusb::DeviceInfo>, Error> {
    Ok(nusb::list_devices().wait().map_err(|e| usb("listing USB devices", e))?.collect())
}

/// The devices `open` chooses from: `candidates` this process can read.
fn openable(cfg: &DeviceConfig) -> Result<Vec<nusb::DeviceInfo>, Error> {
    let ports: Vec<String> = candidates(cfg)?.into_iter().filter(|t| t.ready).map(|t| t.port).collect();
    Ok(list()?.into_iter().filter(|d| ports.contains(&location(d))).collect())
}

/// Which of the connected `ports` a panel uses. Its own port if present. Otherwise a port no other
/// panel claims (`taken`): the first one if the panel names no port, or the only one if it names a
/// port that is gone (the panel was moved to another port).
pub fn select(ports: &[String], want: Option<&str>, taken: &[String]) -> Option<usize> {
    if let Some(i) = want.and_then(|w| ports.iter().position(|p| p == w)) {
        return Some(i);
    }
    let free: Vec<usize> = (0..ports.len()).filter(|&i| !taken.contains(&ports[i])).collect();
    match (want, free.as_slice()) {
        (None, [first, ..]) => Some(*first),
        (Some(_), [only]) => Some(*only),
        _ => None,
    }
}

/// Opens the panel `cfg` describes; `taken` are the ports other panels use.
pub fn open(cfg: &DeviceConfig, taken: &[String]) -> Result<Panel, Error> {
    let devices = openable(cfg)?;
    let ports: Vec<String> = devices.iter().map(location).collect();
    let i = select(&ports, cfg.port.as_deref(), taken).ok_or_else(|| Error::NotFound(cfg.describe()))?;
    let info = &devices[i];

    #[cfg(target_os = "windows")]
    {
        let drv = info.driver().unwrap_or("");
        if !drv.eq_ignore_ascii_case("winusb") {
            return Err(Error::Usb(format!(
                "device driver is '{drv}', not WinUSB: Windows still owns the panel (docs/driver-setup.md, step 2)"
            )));
        }
    }

    let device = info.open().wait().map_err(|e| usb("opening USB device", e))?;
    let cfgd = device.active_configuration().map_err(|e| usb("reading active configuration", e))?;

    // Pick the interface: explicit, else first HID-class (3) interface.
    let mut chosen: Option<(u8, u8, usize, Option<usize>)> = None; // (iface, ep, mps, report desc len)
    for group in cfgd.interfaces() {
        let alt = group.first_alt_setting();
        let n = alt.interface_number();
        if let Some(want) = cfg.iface {
            if want != n {
                continue;
            }
        } else if alt.class() != 3 {
            continue;
        }
        let ep = alt.endpoints().find(|e| e.direction() == Direction::In && e.transfer_type() == TransferType::Interrupt);
        let Some(ep) = ep else { continue };
        let rdlen = alt.descriptors().find(|d| d.descriptor_type() == 0x21).and_then(|d| {
            let b = d.as_bytes();
            (b.len() >= 9 && b[6] == 0x22).then(|| u16::from_le_bytes([b[7], b[8]]) as usize)
        });
        chosen = Some((n, ep.address(), ep.max_packet_size(), rdlen));
        break;
    }
    let (iface, ep_in, mps, rdlen) =
        chosen.ok_or_else(|| Error::Usb("no HID interface with an interrupt IN endpoint found".into()))?;

    let intf = claim(&device, iface)?;

    let rd_raw = match &cfg.descriptor_file {
        Some(path) => std::fs::read(path)?,
        None => {
            let len = rdlen.unwrap_or(1024).max(64) as u16;
            intf.control_in(
                ControlIn {
                    control_type: ControlType::Standard,
                    recipient: Recipient::Interface,
                    request: 0x06, // GET_DESCRIPTOR
                    value: 0x2200, // HID report descriptor, index 0
                    index: iface as u16,
                    length: len,
                },
                TIMEOUT,
            )
            .wait()
            .map_err(|e| usb("GET_DESCRIPTOR(report) failed; try a descriptor file", e))?
        }
    };
    let rd = hid::parse(&rd_raw).map_err(Error::Descriptor)?;

    let panel = Panel { _device: device, intf, iface, ep_in, mps, rd, rd_raw, port: ports[i].clone() };
    // This panel powers up in mouse mode (report 0x05) and switches to multi-touch only once the host
    // reads Contact Count Maximum, as Windows does when it sets up a touchscreen. Without the read,
    // a freshly enumerated panel (replug, driver swap) never sends touch reports. docs/PLAN.md §8.
    match panel.contact_max() {
        Ok(Some(n)) => tracing::info!("contact count maximum: {n}"),
        Ok(None) => {}
        Err(e) => tracing::warn!("reading contact count maximum: {e}"),
    }
    Ok(panel)
}

#[cfg(target_os = "linux")]
fn claim(device: &nusb::Device, iface: u8) -> Result<Interface, Error> {
    device
        .detach_and_claim_interface(iface)
        .wait()
        .map_err(|e| usb(&format!("claiming interface {iface} (try sudo, or a udev rule for the device)"), e))
}

#[cfg(not(target_os = "linux"))]
fn claim(device: &nusb::Device, iface: u8) -> Result<Interface, Error> {
    device.claim_interface(iface).wait().map_err(|e| {
        usb(
            &format!("claiming interface {iface}: panel not bound to WinUSB (docs/driver-setup.md, step 2), or another program has it open"),
            e,
        )
    })
}

impl Panel {
    /// Set the Device Mode feature. Returns `Ok(false)` when the descriptor has no such feature.
    pub fn set_device_mode(&self, mode: i64) -> Result<bool, Error> {
        let Some((fi, el)) = self.rd.find(Kind::Feature, U_DEVICE_MODE) else { return Ok(false) };
        let id = self.rd.fields[fi].report_id;
        let mut buf = vec![0u8; self.rd.report_len(Kind::Feature, id)];
        let off = usize::from(self.rd.uses_report_ids);
        if off == 1 {
            buf[0] = id;
        }
        self.rd.fields[fi].write(&mut buf[off..], el, mode);
        self.intf
            .control_out(
                ControlOut {
                    control_type: ControlType::Class,
                    recipient: Recipient::Interface,
                    request: 0x09, // SET_REPORT
                    value: (3 << 8) | id as u16,
                    index: self.iface as u16,
                    data: &buf,
                },
                TIMEOUT,
            )
            .wait()
            .map_err(|e| usb(&format!("SET_REPORT(feature {id:#04x})"), e))?;
        Ok(true)
    }

    /// Reads the Contact Count Maximum feature, if the descriptor has one.
    pub fn contact_max(&self) -> Result<Option<i64>, Error> {
        let Some((fi, el)) = self.rd.find(Kind::Feature, U_CONTACT_MAX) else { return Ok(None) };
        let id = self.rd.fields[fi].report_id;
        let data = self.get_feature(id, self.rd.report_len(Kind::Feature, id))?;
        let (_, payload) = self.rd.split(&data);
        Ok(Some(self.rd.fields[fi].read(payload, el)))
    }

    pub fn get_feature(&self, id: u8, len: usize) -> Result<Vec<u8>, Error> {
        self.intf
            .control_in(
                ControlIn {
                    control_type: ControlType::Class,
                    recipient: Recipient::Interface,
                    request: 0x01, // GET_REPORT
                    value: (3 << 8) | id as u16,
                    index: self.iface as u16,
                    length: len as u16,
                },
                TIMEOUT,
            )
            .wait()
            .map_err(|e| usb(&format!("GET_REPORT(feature {id:#04x})"), e))
    }

    pub fn reader(&self) -> Result<Reader, Error> {
        let mut ep = self.intf.endpoint::<Interrupt, In>(self.ep_in).map_err(|e| usb("opening interrupt endpoint", e))?;
        for _ in 0..4 {
            let buf = ep.allocate(self.mps);
            ep.submit(buf);
        }
        Ok(Reader { ep, mps: self.mps, acc: Vec::new(), rd: self.rd.clone() })
    }
}

pub struct Reader {
    ep: Endpoint<Interrupt, In>,
    mps: usize,
    acc: Vec<u8>,
    rd: ReportDescriptor,
}

impl Reader {
    /// Next complete report, or `Ok(None)` if nothing completed within `timeout`.
    pub fn next(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, Error> {
        let Some(c) = self.ep.wait_next_complete(timeout) else { return Ok(None) };
        let mut report = None;
        match c.status {
            Ok(()) => {
                let pkt = &c.buffer[..c.actual_len];
                self.acc.extend_from_slice(pkt);
                let expected = if self.rd.uses_report_ids {
                    self.acc.first().map_or(0, |&id| self.rd.report_len(Kind::Input, id))
                } else {
                    self.rd.max_input_len()
                };
                // Short packet or enough bytes => report complete.
                if !self.acc.is_empty() && (pkt.len() < self.mps || expected == 0 || self.acc.len() >= expected) {
                    report = Some(std::mem::take(&mut self.acc));
                }
            }
            Err(TransferError::Disconnected) => return Err(Error::Disconnected),
            Err(TransferError::Cancelled) => {}
            Err(e) => tracing::warn!("transfer error: {e:?}"),
        }
        let mut buf = c.buffer;
        buf.clear();
        self.ep.submit(buf);
        Ok(report)
    }
}
