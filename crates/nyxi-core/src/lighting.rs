//! Backlight (LEDPARAM 0x07/0x87) and the second lighting zone (SLEDPARAM 0x08/0x88).

use crate::{checksum, Error, Keyboard, Result};
use serde::{Deserialize, Serialize};

pub const MAX_SPEED: u8 = 4;
pub const MAX_BRIGHTNESS: u8 = 4;
/// Color-mode values (low nibble of byte 4); 0..6 are the preset colors.
pub const COLOR_CUSTOM: u8 = 7;
pub const COLOR_RAINBOW: u8 = 8;
/// Backlight mode that shows one of the keyboard's three per-key color pictures (option 0..2).
pub const MODE_PICTURE: u8 = 13;
pub const PRESET_COLORS: [(&str, [u8; 3]); 7] = [
    ("Red", [0xFF, 0, 0]),
    ("Orange", [0xFF, 0x80, 0]),
    ("Yellow", [0xFF, 0xFF, 0]),
    ("Green", [0, 0xFF, 0]),
    ("Cyan", [0, 0xFF, 0xFF]),
    ("Blue", [0, 0, 0xFF]),
    ("Purple", [0xFF, 0, 0xFF]),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Zone {
    /// The per-key backlight.
    Main,
    /// The keyboard's second, independent lighting setting.
    Side,
}

impl Zone {
    fn commands(self) -> (u8, u8) {
        match self {
            Zone::Main => (0x07, 0x87),
            Zone::Side => (0x08, 0x88),
        }
    }
}

/// One zone's lighting setting. `speed` is 0 (slowest) to 4 (fastest) for both zones;
/// the main zone's inverted wire format is handled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lighting {
    pub mode: u8,
    pub speed: u8,
    pub brightness: u8,
    /// Mode-specific variant, e.g. wave direction (see `ModeInfo::options`).
    pub option: u8,
    /// 0..6 preset color, 7 custom (`rgb`), 8 rainbow.
    pub color: u8,
    pub rgb: [u8; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeInfo {
    pub id: u8,
    pub name: &'static str,
    pub speed: bool,
    /// Whether the effect uses the color setting (custom color or rainbow).
    pub color: bool,
    pub options: &'static [&'static str],
}

const fn m(id: u8, name: &'static str, speed: bool, options: &'static [&'static str]) -> ModeInfo {
    ModeInfo { id, name, speed, color: true, options }
}

const fn no_color(mut info: ModeInfo) -> ModeInfo {
    info.color = false;
    info
}

/// Backlight modes offered for the Striker. Left out: 20-22 (music/screen following, which need
/// the host to stream data). Which effects take a color follows the official app's settings for
/// this model: all but Off, Neon and the per-key picture.
pub const MAIN_MODES: &[ModeInfo] = &[
    no_color(m(0, "Off", false, &[])),
    m(1, "Solid", false, &[]),
    m(2, "Breathing", true, &[]),
    no_color(m(3, "Neon", true, &["Default", "Random"])),
    m(4, "Wave", true, &["Right", "Left", "Down", "Up"]),
    m(5, "Ripple", true, &["Full", "Single"]),
    m(6, "Raindrop", true, &[]),
    m(7, "Snake", true, &["Z path", "Return path"]),
    m(8, "Reactive", true, &["On to off", "Off to on"]),
    m(9, "Converge", true, &[]),
    m(10, "Sine wave", true, &[]),
    m(11, "Kaleidoscope", true, &["Outward", "Inward"]),
    m(12, "Line wave", true, &["Right", "Left"]),
    no_color(m(MODE_PICTURE, "Per-key colors", false, &["Picture 1", "Picture 2", "Picture 3"])),
    m(14, "Laser", true, &[]),
    m(15, "Circle wave", true, &["Anti-clockwise", "Clockwise"]),
    m(16, "Dazzle", true, &[]),
    m(17, "Rain down", true, &[]),
    m(18, "Meteor", true, &[]),
    m(19, "Reactive off", true, &[]),
];

pub const SIDE_MODES: &[ModeInfo] = &[
    no_color(m(0, "Off", false, &[])),
    m(1, "Solid", false, &[]),
    m(2, "Breathing", true, &[]),
    m(3, "Neon", true, &[]),
    m(4, "Wave", true, &["Right", "Left", "Down", "Up"]),
    m(5, "Snake", true, &["Z path", "Return path"]),
];

pub fn modes(zone: Zone) -> &'static [ModeInfo] {
    match zone {
        Zone::Main => MAIN_MODES,
        Zone::Side => SIDE_MODES,
    }
}

impl Keyboard {
    pub fn lighting(&self, zone: Zone) -> Result<Lighting> {
        let r = self.query(zone.commands().1, &[])?;
        Ok(Lighting {
            mode: r[1],
            speed: match zone {
                Zone::Main => MAX_SPEED.saturating_sub(r[2]),
                Zone::Side => r[2],
            },
            brightness: r[3],
            option: r[4] >> 4,
            color: r[4] & 0x0F,
            rgb: [r[5], r[6], r[7]],
        })
    }

    pub fn set_lighting(&self, zone: Zone, l: &Lighting) -> Result<()> {
        if l.speed > MAX_SPEED || l.brightness > MAX_BRIGHTNESS || l.option > 0x0F || l.color > COLOR_RAINBOW {
            return Err(Error::Invalid(format!("lighting value out of range: {l:?}")));
        }
        let mut p = [0u8; 64];
        p[0] = zone.commands().0;
        p[1] = l.mode;
        p[2] = match zone {
            Zone::Main => MAX_SPEED - l.speed,
            Zone::Side => l.speed,
        };
        p[3] = l.brightness;
        if zone == Zone::Main && l.mode == MODE_PICTURE {
            // As the official app sends it: no color nibble, and this fixed RGB.
            p[4] = l.option << 4;
            p[5..8].copy_from_slice(&[0x00, 0xC8, 0xC8]);
        } else {
            p[4] = (l.option << 4) | l.color;
            p[5..8].copy_from_slice(&l.rgb);
        }
        checksum(&mut p, 8);
        self.send(&p)
    }
}
