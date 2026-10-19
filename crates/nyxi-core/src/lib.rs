//! NYXI CA01 "Striker" keyboard control. The protocol is documented in PROTOCOL.md.
//!
//! Every command is a 64-byte HID feature report, report ID 0, on the keyboard's vendor
//! interface (USB interface 2, usage page 0xFFFF, usage 2). A GET is a SET of the GET
//! command followed by a feature-report read.

pub mod lighting;
pub mod screen;
pub mod settings;

use hidapi::{HidApi, HidDevice};
use std::{thread, time::Duration};

pub use lighting::{Lighting, ModeInfo, Zone};
pub use screen::{Animation, Frame};
pub use settings::{Settings, SleepTimers};

pub const VID: u16 = 0x3151;
pub const PID_WIRED: u16 = 0x4015;
pub const PID_DONGLE: u16 = 0x4011;

const GET_REV: u8 = 0x80;
const GET_INFOR: u8 = 0x8F;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("keyboard not found (is it plugged in, and on Linux is the udev rule installed?)")]
    NotFound,
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),
    #[error("unexpected reply to 0x{cmd:02x}: {reply:02x?}")]
    BadReply { cmd: u8, reply: Vec<u8> },
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Checksum "BIT7"/"BIT8": byte `pos` = 0xFF - (sum of the bytes before it).
pub fn checksum(buf: &mut [u8; 64], pos: usize) {
    let sum = buf[..pos].iter().fold(0u8, |a, &b| a.wrapping_add(b));
    buf[pos] = 0xFF - sum;
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Info {
    /// Firmware version, e.g. 0x0105 = 1.05.
    pub firmware: u16,
    /// NYXI device id, 1473 = CA01 wired.
    pub device_id: u16,
    pub wireless: bool,
}

pub struct Keyboard {
    dev: HidDevice,
    pid: u16,
}

impl Keyboard {
    /// Open the first connected CA01 (wired or 2.4 GHz dongle).
    pub fn open() -> Result<Self> {
        let api = HidApi::new()?;
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VID
                    && [PID_WIRED, PID_DONGLE].contains(&d.product_id())
                    && ((d.usage_page() == 0xFFFF && d.usage() == 2) || d.interface_number() == 2)
            })
            .ok_or(Error::NotFound)?;
        Ok(Self { dev: info.open_device(&api)?, pid: info.product_id() })
    }

    pub fn send(&self, packet: &[u8; 64]) -> Result<()> {
        let mut buf = [0u8; 65]; // leading report ID 0
        buf[1..].copy_from_slice(packet);
        self.dev.send_feature_report(&buf)?;
        Ok(())
    }

    /// Send a GET command and return the 64-byte reply, checking it echoes the command.
    pub fn query(&self, cmd: u8, args: &[u8]) -> Result<[u8; 64]> {
        let mut p = [0u8; 64];
        p[0] = cmd;
        p[1..1 + args.len()].copy_from_slice(args);
        checksum(&mut p, 7);
        self.send(&p)?;
        thread::sleep(Duration::from_millis(20));
        let mut buf = [0u8; 65];
        let n = self.dev.get_feature_report(&mut buf)?;
        let mut reply = [0u8; 64];
        reply[..n.saturating_sub(1)].copy_from_slice(&buf[1..n]);
        if reply[0] != cmd {
            return Err(Error::BadReply { cmd, reply: reply[..16].to_vec() });
        }
        Ok(reply)
    }

    pub fn info(&self) -> Result<Info> {
        let rev = self.query(GET_REV, &[])?;
        let id = self.query(GET_INFOR, &[])?;
        Ok(Info {
            firmware: u16::from_le_bytes([rev[1], rev[2]]),
            device_id: u16::from_le_bytes([id[1], id[2]]),
            wireless: self.pid == PID_DONGLE,
        })
    }
}
