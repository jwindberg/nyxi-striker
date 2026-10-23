//! Per-key color pictures, shown by backlight mode 13 (`lighting::MODE_PICTURE`). The keyboard
//! keeps three; the backlight's option picks which one is shown, and both reading (0x8C) and
//! uploading (0x0C) act on the picture being shown. A picture is one RGB color per key slot
//! (see `keys::KEYS`), 128 slots.

use crate::{checksum, Keyboard, Result};
use std::{thread, time::Duration};

pub const SLOTS: usize = 128;
pub type Picture = Vec<[u8; 3]>;

const SET_USERPIC: u8 = 0x0C;
const GET_USERPIC: u8 = 0x8C;
/// Upload chunks carry 56 bytes after an 8-byte header; 7 of them cover the 384 bytes.
const CHUNK: usize = 56;
const BYTES: usize = SLOTS * 3;

impl Keyboard {
    /// Read the picture being shown. Reply chunks are 64 bytes of data with no command byte.
    pub fn picture(&self) -> Result<Picture> {
        let mut data = Vec::with_capacity(BYTES);
        for chunk in 0..(BYTES / 64) as u8 {
            data.extend_from_slice(&self.query_raw(GET_USERPIC, &[0, chunk])?);
        }
        Ok(data.chunks(3).map(|c| [c[0], c[1], c[2]]).collect())
    }

    /// Replace the picture being shown, as the official app does: 7 chunks, then a pause
    /// before the keyboard takes further commands.
    pub fn set_picture(&self, picture: &[[u8; 3]]) -> Result<()> {
        let mut data = [0u8; CHUNK * 7];
        for (i, rgb) in picture.iter().take(SLOTS).enumerate() {
            data[i * 3..i * 3 + 3].copy_from_slice(rgb);
        }
        let len = (BYTES as u16).to_le_bytes();
        for (chunk, body) in data.chunks(CHUNK).enumerate() {
            let mut p = [0u8; 64];
            p[..5].copy_from_slice(&[SET_USERPIC, 0, len[0], len[1], chunk as u8]);
            checksum(&mut p, 7);
            p[8..].copy_from_slice(body);
            self.send(&p)?;
            thread::sleep(Duration::from_millis(20));
        }
        thread::sleep(Duration::from_millis(500));
        Ok(())
    }
}
