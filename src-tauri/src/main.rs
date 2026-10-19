// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod macos;
mod settings;
mod state;
mod watcher;
#[cfg(target_os = "windows")]
mod win;

use base64::Engine;
use settings::Settings;
use state::AppState;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const KEYCHAIN_SERVICE: &str = "com.lan-solo.clipon";
const KEYCHAIN_ACCOUNT: &str = "history-key";

fn key_from_bytes(bytes: &[u8]) -> Option<[u8; 32]> {
    (bytes.len() == 32).then(|| {
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        key
    })
}

fn write_key_file(path: &Path, key: &[u8; 32]) {
    let _ = std::fs::write(path, key);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

/// History key: preferably in the OS keychain, file fallback if none exists.
///
/// Only a keychain that definitely has *no* entry gets a new key. Any other
/// keychain error (locked at login, access denied, service not running) must
/// never overwrite the stored key — that would make the existing history
/// undecryptable for good.
fn history_key(data_dir: &Path) -> [u8; 32] {
    let b64 = base64::engine::general_purpose::STANDARD;
    let key_path = data_dir.join("history.key");
    let file_key = std::fs::read(&key_path).ok().and_then(|b| key_from_bytes(&b));
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        // the login keychain may still be locked right after login (autostart)
        let mut result = entry.get_password();
        for _ in 0..4 {
            match &result {
                Ok(_) | Err(keyring::Error::NoEntry) => break,
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(750));
                    result = entry.get_password();
                }
            }
        }
        match result {
            Ok(stored) => {
                if let Some(key) = b64.decode(stored).ok().and_then(|b| key_from_bytes(&b)) {
                    return key;
                }
                eprintln!("clipon: keychain entry is malformed — using the key file");
            }
            Err(keyring::Error::NoEntry) => {
                // first start — or the key so far lived in the fallback file:
                // move it into the keychain
                let key = file_key.unwrap_or_else(clipon_core::generate_key);
                if entry.set_password(&b64.encode(key)).is_ok() {
                    if file_key.is_some() {
                        let _ = std::fs::remove_file(&key_path);
                    }
                    return key;
                }
                if file_key.is_none() {
                    write_key_file(&key_path, &key);
                }
                return key;
            }
            Err(e) => eprintln!("clipon: keychain unavailable ({e}) — using the key file"),
        }
    }
    // keychain unavailable — fall back to a key file next to the store
    if let Some(key) = file_key {
        return key;
    }
    let key = clipon_core::generate_key();
    write_key_file(&key_path, &key);
    key
}

/// Loads the history. A store that exists but cannot be decrypted (other
/// key, damaged file) is moved aside instead of being overwritten by the
/// next save — it stays recoverable.
fn load_or_backup_store(store_path: &Path, key: &[u8; 32]) -> clipon_core::Store {
    if !store_path.exists() {
        return clipon_core::Store::default();
    }
    match clipon_core::load_store(store_path, key) {
        Ok(store) => store,
        Err(e) => {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
            let backup = store_path.with_extension(format!("clipon.unreadable-{stamp}"));
            eprintln!(
                "clipon: history could not be read ({e}) — kept as {}",
                backup.display()
            );
            let _ = std::fs::rename(store_path, &backup);
            clipon_core::Store::default()
        }
    }
}

/// One clipon per user: a second start (e.g. autostart + manual launch on
/// Windows) would run a second watcher writing the same store. The second
/// instance asks the first one to show its window and exits. It waits a
/// moment first — after an update the old process may still be shutting
/// down while the new one starts.
fn instance_files() -> (std::path::PathBuf, std::path::PathBuf) {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();
    let dir = std::env::temp_dir();
    (
        dir.join(format!("com.lan-solo.clipon-{user}.lock")),
        dir.join(format!("com.lan-solo.clipon-{user}.show")),
    )
}

fn acquire_single_instance() -> Option<std::fs::File> {
    let (lock_path, show_path) = instance_files();
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .ok()?; // cannot create a lock file at all — don't block the start
    if file.try_lock().is_ok() {
        return Some(file);
    }
    let _ = std::fs::write(&show_path, b"show");
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if file.try_lock().is_ok() {
            // the previous instance is gone (update restart) — we take over
            let _ = std::fs::remove_file(&show_path);
            return Some(file);
        }
    }
    std::process::exit(0);
}

/// The running instance watches for the "show" request of a second start.
fn watch_show_requests(app: AppHandle) {
    let (_, show_path) = instance_files();
    let _ = std::fs::remove_file(&show_path);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        if show_path.exists() {
            let _ = std::fs::remove_file(&show_path);
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || show_main_window(&handle));
        }
    });
}

fn tray_labels(lang: &str) -> (&'static str, &'static str, &'static str) {
    if lang == "de" {
        ("clipon öffnen", "Aufnahme pausieren", "clipon beenden")
    } else {
        ("Open clipon", "Pause capture", "Quit clipon")
    }
}

/// Re-labels the tray menu after a language change.
pub fn update_tray_labels(st: &AppState, lang: &str) {
    let (show, pause, quit) = tray_labels(lang);
    if let Some(items) = st.tray_items.lock().unwrap().as_ref() {
        let _ = items.show.set_text(show);
        let _ = items.quit.set_text(quit);
    }
    if let Some(item) = st.pause_menu.lock().unwrap().as_ref() {
        let _ = item.set_text(pause);
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        let focused = win.is_focused().unwrap_or(false);
        if visible && focused {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Registers all global shortcuts. One failing combo does not block the
/// others; all failures are reported together.
pub fn register_shortcuts(app: &AppHandle, s: &Settings) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut errors = Vec::new();
    if let Err(e) = gs.on_shortcut(s.shortcut_toggle.as_str(), |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            toggle_main_window(app);
        }
    }) {
        errors.push(format!("shortcut: {}: {e}", s.shortcut_toggle));
    }
    if let Err(e) = gs.on_shortcut(s.shortcut_stack_pop.as_str(), |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            commands::do_stack_pop(app);
        }
    }) {
        errors.push(format!("shortcut: {}: {e}", s.shortcut_stack_pop));
    }
    if let Err(e) = gs.on_shortcut(s.shortcut_picker.as_str(), |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            commands::open_picker(app);
        }
    }) {
        errors.push(format!("shortcut: {}: {e}", s.shortcut_picker));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" | "))
    }
}

fn build_tray(
    app: &tauri::App,
    s: &Settings,
) -> tauri::Result<(CheckMenuItem<tauri::Wry>, state::TrayItems)> {
    let (show_label, pause_label, quit_label) = tray_labels(&s.language);
    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", pause_label, true, s.paused, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &pause, &sep, &quit])?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "pause" => {
                let now = !app
                    .state::<AppState>()
                    .paused
                    .load(std::sync::atomic::Ordering::Relaxed);
                if let Err(e) = commands::set_paused(app.clone(), app.state::<AppState>(), now) {
                    use tauri::Emitter;
                    let _ = app.emit("save-failed", e);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok((pause, state::TrayItems { show, quit }))
}

fn main() {
    // held for the whole process lifetime; released by the OS on exit
    let _instance_lock = acquire_single_instance();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = handle
                .path()
                .app_data_dir()
                .expect("no app data dir");
            std::fs::create_dir_all(&data_dir)?;
            let key = history_key(&data_dir);
            let s = settings::load(&handle);
            let store_path = data_dir.join("history.clipon");
            let store = load_or_backup_store(&store_path, &key);
            let state = AppState {
                store: Mutex::new(store),
                settings: Mutex::new(s.clone()),
                key,
                data_dir,
                last_seen: Mutex::new(None),
                paused: AtomicBool::new(s.paused),
                pause_menu: Mutex::new(None),
                frontmost: Mutex::new(macos::frontmost_app_now()),
                picker_return: Mutex::new(None),
                picker_hwnd: Mutex::new(None),
                tray_items: Mutex::new(None),
            };
            app.manage(state);
            macos::install_frontmost_observer(handle.clone());

            let (pause_item, tray_items) = build_tray(app, &s)?;
            *app.state::<AppState>().pause_menu.lock().unwrap() = Some(pause_item);
            *app.state::<AppState>().tray_items.lock().unwrap() = Some(tray_items);
            watch_show_requests(handle.clone());

            if let Err(e) = register_shortcuts(&handle, &s) {
                eprintln!("clipon: {e}");
            }
            watcher::spawn(handle.clone());
            if macos::IS_MACOS {
                commands::ensure_picker(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            // the picker is transient: clicking elsewhere dismisses it
            tauri::WindowEvent::Focused(false) if window.label() == "picker" => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::set_paused,
            commands::list_running_apps,
            commands::list_items,
            commands::get_item_text,
            commands::get_item_image,
            commands::copy_item,
            commands::add_text_item,
            commands::pin_item,
            commands::delete_item,
            commands::clear_history,
            commands::list_snippets,
            commands::save_snippet,
            commands::delete_snippet,
            commands::copy_snippet,
            commands::stack_list,
            commands::stack_add,
            commands::stack_remove,
            commands::stack_clear,
            commands::stack_pop_copy,
            commands::transform_item,
            commands::merge_stack,
            commands::export_item,
            commands::export_data,
            commands::import_data,
            commands::show_picker,
            commands::hide_picker,
            commands::picker_paste,
            commands::accessibility_status,
            commands::request_accessibility,
            commands::check_update,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building clipon")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(app);
            }
            let _ = (app, &event);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_store_is_moved_aside_not_lost() {
        let dir = std::env::temp_dir().join(format!("clipon-main-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.clipon");
        let mut s = clipon_core::Store::default();
        s.add_text("wichtig");
        clipon_core::save_store(&s, &path, &[1u8; 32]).unwrap();

        // right key: loads
        assert_eq!(load_or_backup_store(&path, &[1u8; 32]).items.len(), 1);
        // wrong key: empty store, original file kept under another name
        let fresh = load_or_backup_store(&path, &[2u8; 32]);
        assert!(fresh.items.is_empty());
        assert!(!path.exists());
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("unreadable"))
            .collect();
        assert_eq!(backups.len(), 1);
        let restored = clipon_core::load_store(&backups[0].path(), &[1u8; 32]).unwrap();
        assert_eq!(restored.items[0].text.as_deref(), Some("wichtig"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_bytes_must_be_32_long() {
        assert!(key_from_bytes(&[0u8; 31]).is_none());
        assert!(key_from_bytes(&[0u8; 33]).is_none());
        assert_eq!(key_from_bytes(&[5u8; 32]), Some([5u8; 32]));
    }
}
