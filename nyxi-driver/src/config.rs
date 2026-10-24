//! The devices offered to the official app: the NYXI Striker on its cable and on its 2.4 GHz
//! receiver. Each is matched on its control interface (usage page 0xFFFF, usage 2, interface 2),
//! not the interfaces that carry the typing.

pub const VID: u16 = 0x3151;
const USAGE_PAGE: u16 = 0xFFFF;
const USAGE: u16 = 2;
const INTERFACE: i32 = 2;

pub struct Device {
    pub pid: u16,
    /// The id the official app knows the device by.
    pub id: u32,
    /// Reached through the 2.4 GHz receiver.
    pub dongle: bool,
}

pub const DEVICES: &[Device] = &[
    Device { pid: 0x4015, id: 1473, dongle: false },
    Device { pid: 0x4011, id: 2147485121, dongle: true },
];

pub fn find(vid: u16, pid: u16, usage: u16, usage_page: u16, interface: i32) -> Option<&'static Device> {
    if vid != VID || usage != USAGE || usage_page != USAGE_PAGE || interface != INTERFACE {
        return None;
    }
    DEVICES.iter().find(|d| d.pid == pid)
}
