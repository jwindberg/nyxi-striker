//! Minimal command-line access to the keyboard, for testing nyxi-core without the app.
//!
//!   nyxi info                 firmware version and device id
//!   nyxi light [main|side]    read a lighting zone
//!   nyxi screen list          list the animation library
//!   nyxi screen <name>        play a library animation on the screen

use nyxi_core::{lighting, screen, Keyboard, Zone};
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        exit(1);
    }
}

fn run(args: &[&str]) -> nyxi_core::Result<()> {
    match args {
        ["info"] => {
            let info = Keyboard::open()?.info()?;
            println!(
                "firmware {}.{:02}, device id {}{}",
                info.firmware >> 8,
                info.firmware & 0xFF,
                info.device_id,
                if info.wireless { " (2.4 GHz)" } else { " (wired)" }
            );
        }
        ["light"] | ["light", "main"] | ["light", "side"] => {
            let zone = if args.get(1) == Some(&"side") { Zone::Side } else { Zone::Main };
            let l = Keyboard::open()?.lighting(zone)?;
            let name = lighting::modes(zone).iter().find(|m| m.id == l.mode).map_or("?", |m| m.name);
            println!("{zone:?}: {name} ({l:?})");
        }
        ["screen", "list"] => {
            for a in screen::library() {
                println!("{:12} {:2} frames  {}", a.slug, a.frames.len(), a.about);
            }
        }
        ["screen", slug] => {
            let anim = screen::library().into_iter().find(|a| a.slug == *slug);
            let anim = anim.ok_or_else(|| nyxi_core::Error::Invalid(format!("no animation named {slug}")))?;
            Keyboard::open()?.upload_frames(&anim.frames, anim.delay)?;
            println!("playing {}", anim.name);
        }
        _ => {
            eprintln!("usage: nyxi info | light [main|side] | screen list | screen <name>");
            exit(2);
        }
    }
    Ok(())
}
