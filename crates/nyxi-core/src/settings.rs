//! Polling rate (0x04/0x84), keyboard options (0x06/0x86), debounce (0x11/0x91) and sleep
//! timers (0x12/0x92). All writes use the BIT7 checksum.

use crate::{checksum, Error, Keyboard, Result};
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};

const GET_PROFILE: u8 = 0x85;
const SET_REPORT: u8 = 0x04;
const GET_REPORT: u8 = 0x84;
const SET_KBOPTION: u8 = 0x06;
const GET_KBOPTION: u8 = 0x86;
const SET_DEBOUNCE: u8 = 0x11;
const GET_DEBOUNCE: u8 = 0x91;
const SET_SLEEPTIME: u8 = 0x12;
const GET_SLEEPTIME: u8 = 0x92;

const WIN_LOCK_BIT: u8 = 0x01;
/// Mac mode: the keyboard swaps some Fn shortcuts for Mac ones (Fn+F3, F4, F12; Fn+Win does
/// nothing). Verified by reading the Fn layer with the bit set. The official app reads this bit
/// but never sets it.
const MAC_MODE_BIT: u8 = 0x02;
pub const MAX_DEBOUNCE_MS: u8 = 10;
/// Polling rates in Hz and their wire codes.
pub const POLLING_RATES: [(u16, u8); 4] = [(1000, 1), (500, 2), (250, 4), (125, 8)];

/// Sleep timers in seconds; 0 disables one. Only used on Bluetooth and 2.4 GHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepTimers {
    pub light_bt: u16,
    pub light_24g: u16,
    pub deep_bt: u16,
    pub deep_24g: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub polling_hz: u16,
    pub debounce_ms: u8,
    pub win_lock: bool,
    pub mac_mode: bool,
    pub sleep: SleepTimers,
}

impl Keyboard {
    fn write(&self, fields: &[(usize, u8)]) -> Result<()> {
        let mut p = [0u8; 64];
        for &(i, v) in fields {
            p[i] = v;
        }
        checksum(&mut p, 7);
        self.send(&p)?;
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    fn profile(&self) -> Result<u8> {
        Ok(self.query(GET_PROFILE, &[])?[1])
    }

    pub fn settings(&self) -> Result<Settings> {
        let profile = self.profile()?;
        let code = self.query(GET_REPORT, &[profile])?[2];
        let polling_hz = POLLING_RATES.iter().find(|r| r.1 == code).map_or(0, |r| r.0);
        let s = self.query(GET_SLEEPTIME, &[])?;
        let flags = self.query(GET_KBOPTION, &[profile])?[2];
        let le = |i: usize| u16::from_le_bytes([s[i], s[i + 1]]);
        Ok(Settings {
            polling_hz,
            debounce_ms: self.query(GET_DEBOUNCE, &[])?[2],
            win_lock: flags & WIN_LOCK_BIT != 0,
            mac_mode: flags & MAC_MODE_BIT != 0,
            sleep: SleepTimers { light_bt: le(1), light_24g: le(3), deep_bt: le(5), deep_24g: le(7) },
        })
    }

    pub fn set_polling_rate(&self, hz: u16) -> Result<()> {
        let code = POLLING_RATES.iter().find(|r| r.0 == hz).ok_or_else(|| Error::Invalid(format!("unsupported polling rate {hz} Hz")))?.1;
        let profile = self.profile()?;
        self.write(&[(0, SET_REPORT), (1, profile), (2, code)])
    }

    pub fn set_debounce(&self, ms: u8) -> Result<()> {
        if ms > MAX_DEBOUNCE_MS {
            return Err(Error::Invalid(format!("debounce must be 0 to {MAX_DEBOUNCE_MS} ms")));
        }
        self.write(&[(0, SET_DEBOUNCE), (2, ms)])
    }

    /// Set or clear one keyboard option bit, keeping every other one as the keyboard has it.
    fn set_option(&self, bit: u8, on: bool) -> Result<()> {
        let profile = self.profile()?;
        let r = self.query(GET_KBOPTION, &[profile])?;
        let flags = if on { r[2] | bit } else { r[2] & !bit };
        self.write(&[(0, SET_KBOPTION), (1, profile), (2, flags), (3, r[3]), (4, r[4])])
    }

    pub fn set_win_lock(&self, locked: bool) -> Result<()> {
        self.set_option(WIN_LOCK_BIT, locked)
    }

    pub fn set_mac_mode(&self, mac: bool) -> Result<()> {
        self.set_option(MAC_MODE_BIT, mac)
    }

    pub fn set_sleep_timers(&self, t: &SleepTimers) -> Result<()> {
        let mut fields = vec![(0, SET_SLEEPTIME)];
        for (i, v) in [t.light_bt, t.light_24g, t.deep_bt, t.deep_24g].into_iter().enumerate() {
            let [lo, hi] = v.to_le_bytes();
            fields.push((8 + 2 * i, lo));
            fields.push((9 + 2 * i, hi));
        }
        self.write(&fields)
    }
}
