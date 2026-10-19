//! Tauri backend: thin commands over nyxi-core. The keyboard is opened on first use and
//! dropped after any error, so unplugging and replugging recovers on the next call.

use nyxi_core::{lighting, screen, Animation, Frame, Info, Keyboard, Lighting, ModeInfo, Zone};
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
struct Device(Mutex<Option<Keyboard>>);

fn with_keyboard<T>(device: &Device, f: impl FnOnce(&Keyboard) -> nyxi_core::Result<T>) -> Result<T, String> {
    let mut slot = device.0.lock().unwrap();
    if slot.is_none() {
        *slot = Some(Keyboard::open().map_err(|e| e.to_string())?);
    }
    let result = f(slot.as_ref().unwrap());
    if result.is_err() {
        *slot = None;
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn info(device: State<'_, Device>) -> Result<Info, String> {
    with_keyboard(&device, |kb| kb.info())
}

#[tauri::command]
fn modes(zone: Zone) -> Vec<ModeInfo> {
    lighting::modes(zone).to_vec()
}

#[tauri::command]
async fn get_lighting(device: State<'_, Device>, zone: Zone) -> Result<Lighting, String> {
    with_keyboard(&device, |kb| kb.lighting(zone))
}

#[tauri::command]
async fn set_lighting(device: State<'_, Device>, zone: Zone, lighting: Lighting) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.set_lighting(zone, &lighting))
}

#[tauri::command]
fn animations() -> Vec<Animation> {
    screen::library()
}

#[tauri::command]
async fn play(device: State<'_, Device>, frames: Vec<Frame>, delay: u8) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.upload_frames(&frames, delay))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Device::default())
        .invoke_handler(tauri::generate_handler![info, modes, get_lighting, set_lighting, animations, play])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
