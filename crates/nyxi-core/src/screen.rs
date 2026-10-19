//! The 9x9 dot-matrix screen: uploads to its custom slot (OLEDGIFDATA 0x24), and the
//! built-in animation library from animations/*.txt.

use crate::{checksum, Error, Keyboard, Result};
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};

pub const SIZE: usize = 9;
pub const MAX_FRAMES: usize = 50;
/// Frame delay unit, approximately (observed by eye: 50 = about 1 frame per second).
pub const DELAY_UNIT_MS: u32 = 20;
const SET_OLEDGIFDATA: u8 = 0x24;

/// One 9x9 frame, `frame[y][x]`, y = 0 at the top.
pub type Frame = [[bool; SIZE]; SIZE];

/// What a screen is meant to be. Stored rather than guessed, so an animation that is down
/// to one frame stays an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Animation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animation {
    pub slug: String,
    pub name: String,
    pub about: String,
    pub kind: Kind,
    /// Frame delay in ~20 ms units.
    pub delay: u8,
    pub frames: Vec<Frame>,
}

/// Pack a frame: two bands (rows 0-7, then row 8), one byte per column, bit n = row band*8+n.
pub fn pack(frame: &Frame) -> [u8; 18] {
    let mut out = [0u8; 18];
    for band in 0..2 {
        for x in 0..SIZE {
            for n in 0..8 {
                let y = band * 8 + n;
                if y < SIZE && frame[y][x] {
                    out[band * SIZE + x] |= 1 << n;
                }
            }
        }
    }
    out
}

impl Keyboard {
    /// Upload frames to the custom slot; the screen switches to it immediately.
    /// One frame is a still image; `delay` is ignored for it.
    pub fn upload_frames(&self, frames: &[Frame], delay: u8) -> Result<()> {
        if frames.is_empty() || frames.len() > MAX_FRAMES {
            return Err(Error::Invalid(format!("an animation needs 1 to {MAX_FRAMES} frames, got {}", frames.len())));
        }
        let animated = frames.len() > 1;
        for (i, frame) in frames.iter().enumerate() {
            let mut p = [0u8; 64];
            p[0] = SET_OLEDGIFDATA;
            p[1] = if animated { (frames.len() - 1) as u8 } else { 0 };
            p[3] = i as u8;
            p[4] = if animated { delay } else { 0 };
            checksum(&mut p, 7);
            p[8..26].copy_from_slice(&pack(frame));
            self.send(&p)?;
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

/// Parse the library text format: ';' lines are metadata ("; name: ...", "; type: image" or
/// "; type: animation", "; delay: N", "; about: ..."), then 9-line frames of '#' (on) and '.' (off) separated by blank lines.
pub fn parse(slug: &str, text: &str) -> Animation {
    let mut anim = Animation { slug: slug.into(), name: slug.into(), about: String::new(), kind: Kind::Image, delay: 50, frames: vec![] };
    let mut kind = None;
    let mut rows: Vec<[bool; SIZE]> = vec![];
    for line in text.lines().chain(std::iter::once("")) {
        if let Some(meta) = line.strip_prefix(';') {
            if let Some((key, value)) = meta.split_once(':') {
                let value = value.trim();
                match key.trim() {
                    "name" => anim.name = value.into(),
                    "about" => anim.about = value.into(),
                    "type" => kind = Some(if value == "image" { Kind::Image } else { Kind::Animation }),
                    "delay" => anim.delay = value.parse().unwrap_or(50),
                    _ => {}
                }
            }
        } else if !line.trim().is_empty() {
            let mut row = [false; SIZE];
            for (x, c) in line.chars().take(SIZE).enumerate() {
                row[x] = !matches!(c, '.' | ' ');
            }
            rows.push(row);
        } else if !rows.is_empty() {
            let mut frame = [[false; SIZE]; SIZE];
            for (y, row) in rows.drain(..).take(SIZE).enumerate() {
                frame[y] = row;
            }
            anim.frames.push(frame);
        }
    }
    // Files without a type line: one frame is an image.
    anim.kind = kind.unwrap_or(if anim.frames.len() > 1 { Kind::Animation } else { Kind::Image });
    anim
}

/// Write an animation in the library text format (the inverse of `parse`).
pub fn to_text(anim: &Animation) -> String {
    let kind = match anim.kind {
        Kind::Image => "image",
        Kind::Animation => "animation",
    };
    let mut out = format!("; name: {}\n; type: {kind}\n; delay: {}\n", anim.name.replace('\n', " "), anim.delay);
    if !anim.about.is_empty() {
        out += &format!("; about: {}\n", anim.about.replace('\n', " "));
    }
    for frame in &anim.frames {
        out.push('\n');
        for row in frame {
            out.extend(row.iter().map(|&on| if on { '#' } else { '.' }));
            out.push('\n');
        }
    }
    out
}

/// A file-name-safe id from a display name: lowercase letters, digits and dashes.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(40).collect();
    if slug.is_empty() { "screen".into() } else { slug }
}

include!(concat!(env!("OUT_DIR"), "/library.rs"));

/// The animations bundled from animations/*.txt, sorted by file name.
pub fn library() -> Vec<Animation> {
    LIBRARY.iter().map(|(slug, text)| parse(slug, text)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heart_packs_like_the_verified_upload() {
        let heart = parse("heart", ".........\n.##...##.\n#########\n#########\n#########\n.#######.\n..#####..\n...###...\n....#....\n");
        assert_eq!(heart.frames.len(), 1);
        let packed = pack(&heart.frames[0]);
        assert_eq!(packed, [0x1C, 0x3E, 0x7E, 0xFC, 0xFC, 0xFC, 0x7E, 0x3E, 0x1C, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn text_round_trips() {
        for a in library() {
            let back = parse(&a.slug, &to_text(&a));
            assert_eq!((back.name, back.kind, back.delay, back.frames), (a.name, a.kind, a.delay, a.frames));
        }
    }

    #[test]
    fn kind_is_kept_for_one_frame_animations() {
        let mut a = parse("x", ".........\n");
        assert_eq!(a.kind, Kind::Image);
        a.kind = Kind::Animation;
        assert_eq!(parse("x", &to_text(&a)).kind, Kind::Animation);
    }

    #[test]
    fn slugs_are_file_safe() {
        assert_eq!(slugify("My Cat!  v2"), "my-cat-v2");
        assert_eq!(slugify("../.."), "screen");
    }

    #[test]
    fn library_loads() {
        let lib = library();
        assert!(lib.len() >= 19);
        for a in &lib {
            assert!((1..=MAX_FRAMES).contains(&a.frames.len()), "{}", a.slug);
        }
    }
}
