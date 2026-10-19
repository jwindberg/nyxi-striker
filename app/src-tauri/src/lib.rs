//! Tauri backend: thin commands over nyxi-core. The keyboard is opened on first use and
//! dropped after any error, so unplugging and replugging recovers on the next call.
//!
//! Screen images and animations are text files (the animations/ format) in <app data>/screens;
//! see the README for where that is on each platform. The ones included with the app are a
//! starter set copied there, after which they are the user's to edit or delete like any other.

use nyxi_core::{lighting, screen, Animation, Frame, Info, Kind, Keyboard, Lighting, ModeInfo, Settings, SleepTimers, Zone};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};

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

fn screens_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("screens");
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Only slugs made by `screen::slugify` are accepted, so a file name can't leave the folder.
fn screen_file(app: &AppHandle, slug: &str) -> Result<PathBuf, String> {
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(format!("invalid screen id {slug:?}"));
    }
    Ok(screens_dir(app)?.join(format!("{slug}.txt")))
}

/// Lists the included screens already copied into the folder, one id per line, so one the
/// user deleted isn't copied back; only screens new in a later version of the app are added.
const INCLUDED_RECORD: &str = ".included";

fn add_included(app: &AppHandle) -> Result<(), String> {
    let dir = screens_dir(app)?;
    let record = dir.join(INCLUDED_RECORD);
    let done = fs::read_to_string(&record).unwrap_or_default();
    let done: Vec<&str> = done.lines().collect();
    let mut added = String::new();
    for a in screen::library().into_iter().filter(|a| !done.contains(&a.slug.as_str())) {
        let path = dir.join(format!("{}.txt", a.slug));
        if !path.exists() {
            fs::write(&path, screen::to_text(&a)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        }
        added += &format!("{}\n", a.slug);
    }
    if !added.is_empty() {
        let all = done.iter().map(|s| format!("{s}\n")).collect::<String>() + &added;
        fs::write(&record, all).map_err(|e| format!("cannot write {}: {e}", record.display()))?;
    }
    Ok(())
}

fn user_screens(app: &AppHandle) -> Result<Vec<Animation>, String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(screens_dir(app)?)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();
    Ok(paths
        .iter()
        .filter_map(|p| Some(screen::parse(&p.file_stem()?.to_string_lossy(), &fs::read_to_string(p).ok()?)))
        .filter(|a| !a.frames.is_empty())
        .collect())
}

#[tauri::command]
fn library(app: AppHandle) -> Result<Vec<Animation>, String> {
    add_included(&app)?;
    user_screens(&app)
}

/// Put every included screen back as it came with the app, replacing any changed copy.
#[tauri::command]
fn restore_included(app: AppHandle) -> Result<(), String> {
    let dir = screens_dir(&app)?;
    for a in screen::library() {
        let path = dir.join(format!("{}.txt", a.slug));
        fs::write(&path, screen::to_text(&a)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Save a screen. A new one (`slug` = None) gets a fresh id from its name.
#[tauri::command]
fn save_screen(app: AppHandle, slug: Option<String>, name: String, kind: Kind, delay: u8, frames: Vec<Frame>) -> Result<Animation, String> {
    if frames.is_empty() || frames.len() > screen::MAX_FRAMES || (kind == Kind::Image && frames.len() > 1) {
        return Err(format!("an animation needs 1 to {} frames", screen::MAX_FRAMES));
    }
    let name = if name.trim().is_empty() { "Untitled".to_string() } else { name.trim().to_string() };
    let slug = match slug {
        Some(slug) => slug,
        None => {
            let base = screen::slugify(&name);
            let taken: Vec<String> = user_screens(&app)?.into_iter().map(|a| a.slug).collect();
            (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|s| !taken.contains(s)).unwrap()
        }
    };
    let animation = Animation { slug: slug.clone(), name, about: String::new(), kind, delay, frames };
    let path = screen_file(&app, &slug)?;
    fs::write(&path, screen::to_text(&animation)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(animation)
}

#[tauri::command]
fn delete_screen(app: AppHandle, slug: String) -> Result<(), String> {
    let path = screen_file(&app, &slug)?;
    fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))
}

#[tauri::command]
fn screens_folder(app: AppHandle) -> Result<String, String> {
    Ok(screens_dir(&app)?.display().to_string())
}

#[tauri::command]
async fn play(device: State<'_, Device>, frames: Vec<Frame>, delay: u8) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.upload_frames(&frames, delay))
}

#[tauri::command]
async fn get_settings(device: State<'_, Device>) -> Result<Settings, String> {
    with_keyboard(&device, |kb| kb.settings())
}

#[tauri::command]
async fn set_polling_rate(device: State<'_, Device>, hz: u16) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.set_polling_rate(hz))
}

#[tauri::command]
async fn set_debounce(device: State<'_, Device>, ms: u8) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.set_debounce(ms))
}

#[tauri::command]
async fn set_win_lock(device: State<'_, Device>, locked: bool) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.set_win_lock(locked))
}

#[tauri::command]
async fn set_sleep_timers(device: State<'_, Device>, timers: SleepTimers) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.set_sleep_timers(&timers))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Device::default())
        .invoke_handler(tauri::generate_handler![
            info,
            modes,
            get_lighting,
            set_lighting,
            library,
            restore_included,
            save_screen,
            delete_screen,
            screens_folder,
            play,
            get_settings,
            set_polling_rate,
            set_debounce,
            set_win_lock,
            set_sleep_timers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
