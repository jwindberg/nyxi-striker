//! Tauri backend: thin commands over nyxi-core. The keyboard is opened on first use and
//! dropped after any error, so unplugging and replugging recovers on the next call.
//!
//! Screen images and animations are text files (the animations/ format) in <app data>/screens;
//! see the README for where that is on each platform. The ones included with the app are a
//! starter set copied there, after which they are the user's to edit or delete like any other.

use nyxi_core::{lighting, picture, screen, Animation, Frame, Info, Key, Kind, Keyboard, Lighting, ModeInfo, Picture, SavedPicture, Settings, SleepTimers, Zone, KEYS};
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

#[tauri::command]
fn keys() -> Vec<Key> {
    KEYS.to_vec()
}

/// Reading and uploading act on the picture the backlight shows, so these only run while the
/// per-key colors mode shows `picture` (0..2); otherwise the effect was changed meanwhile.
fn showing_picture(kb: &Keyboard, picture: u8) -> nyxi_core::Result<()> {
    let l = kb.lighting(Zone::Main)?;
    if l.mode == lighting::MODE_PICTURE && l.option == picture {
        Ok(())
    } else {
        Err(nyxi_core::Error::Invalid(format!("the backlight is not showing picture {}", picture + 1)))
    }
}

#[tauri::command]
async fn get_picture(device: State<'_, Device>, picture: u8) -> Result<Picture, String> {
    with_keyboard(&device, |kb| {
        showing_picture(kb, picture)?;
        kb.picture()
    })
}

#[tauri::command]
async fn set_picture(device: State<'_, Device>, picture: u8, colors: Picture) -> Result<(), String> {
    with_keyboard(&device, |kb| {
        showing_picture(kb, picture)?;
        kb.set_picture(&colors)
    })
}

/// A folder in the app's data folder: "screens" or "pictures".
fn data_dir(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join(name);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    Ok(dir)
}

fn screens_dir(app: &AppHandle) -> Result<PathBuf, String> {
    data_dir(app, "screens")
}

/// Only slugs made by `screen::slugify` are accepted, so a file name can't leave the folder.
fn data_file(app: &AppHandle, folder: &str, slug: &str) -> Result<PathBuf, String> {
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(format!("invalid file id {slug:?}"));
    }
    Ok(data_dir(app, folder)?.join(format!("{slug}.txt")))
}

/// The .txt files in a data folder, sorted by name, as (slug, contents).
fn text_files(app: &AppHandle, folder: &str) -> Result<Vec<(String, String)>, String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(data_dir(app, folder)?)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();
    Ok(paths
        .iter()
        .filter_map(|p| Some((p.file_stem()?.to_string_lossy().into_owned(), fs::read_to_string(p).ok()?)))
        .collect())
}

/// A fresh slug from a name, not taken by any of `taken`.
fn new_slug(name: &str, taken: &[String]) -> String {
    let base = screen::slugify(name);
    (1..).map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") }).find(|s| !taken.contains(s)).unwrap()
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
    Ok(text_files(app, "screens")?
        .iter()
        .map(|(slug, text)| screen::parse(slug, text))
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
        None => new_slug(&name, &user_screens(&app)?.into_iter().map(|a| a.slug).collect::<Vec<_>>()),
    };
    let animation = Animation { slug: slug.clone(), name, about: String::new(), kind, delay, frames };
    let path = data_file(&app, "screens", &slug)?;
    fs::write(&path, screen::to_text(&animation)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(animation)
}

#[tauri::command]
fn delete_screen(app: AppHandle, slug: String) -> Result<(), String> {
    let path = data_file(&app, "screens", &slug)?;
    fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))
}

#[tauri::command]
fn screens_folder(app: AppHandle) -> Result<String, String> {
    Ok(screens_dir(&app)?.display().to_string())
}

/// Pictures saved on this computer, in <app data>/pictures, sorted by file name.
#[tauri::command]
fn saved_pictures(app: AppHandle) -> Result<Vec<SavedPicture>, String> {
    Ok(text_files(&app, "pictures")?.iter().map(|(slug, text)| picture::parse(slug, text)).collect())
}

/// Save a picture. A new one (`slug` = None) gets a fresh id from its name.
#[tauri::command]
fn save_picture(app: AppHandle, slug: Option<String>, name: String, colors: Picture) -> Result<SavedPicture, String> {
    let name = if name.trim().is_empty() { "Untitled".to_string() } else { name.trim().to_string() };
    let slug = match slug {
        Some(slug) => slug,
        None => new_slug(&name, &saved_pictures(app.clone())?.into_iter().map(|p| p.slug).collect::<Vec<_>>()),
    };
    let path = data_file(&app, "pictures", &slug)?;
    fs::write(&path, picture::to_text(&name, &colors)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(picture::parse(&slug, &fs::read_to_string(&path).map_err(|e| e.to_string())?))
}

#[tauri::command]
fn delete_picture(app: AppHandle, slug: String) -> Result<(), String> {
    let path = data_file(&app, "pictures", &slug)?;
    fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))
}

#[tauri::command]
fn pictures_folder(app: AppHandle) -> Result<String, String> {
    Ok(data_dir(&app, "pictures")?.display().to_string())
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
async fn set_mac_mode(device: State<'_, Device>, mac: bool) -> Result<(), String> {
    with_keyboard(&device, |kb| kb.set_mac_mode(mac))
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
            keys,
            get_picture,
            set_picture,
            saved_pictures,
            save_picture,
            delete_picture,
            pictures_folder,
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
            set_mac_mode,
            set_sleep_timers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
