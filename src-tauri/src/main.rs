// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod settings;
mod state;
mod watcher;

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

/// History key: preferably in the OS keychain, file fallback if none exists.
fn history_key(data_dir: &Path) -> [u8; 32] {
    let b64 = base64::engine::general_purpose::STANDARD;
    if let Ok(entry) = keyring::Entry::new("com.lan-solo.clipon", "history-key") {
        match entry.get_password() {
            Ok(stored) => {
                if let Ok(bytes) = b64.decode(stored) {
                    if bytes.len() == 32 {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&bytes);
                        return key;
                    }
                }
            }
            Err(_) => {
                let key = clipon_core::generate_key();
                if entry.set_password(&b64.encode(key)).is_ok() {
                    return key;
                }
            }
        }
    }
    // keychain unavailable — fall back to a key file next to the store
    let key_path = data_dir.join("history.key");
    if let Ok(bytes) = std::fs::read(&key_path) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return key;
        }
    }
    let key = clipon_core::generate_key();
    let _ = std::fs::write(&key_path, key);
    key
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

pub fn register_shortcuts(app: &AppHandle, s: &Settings) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.on_shortcut(s.shortcut_toggle.as_str(), |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            toggle_main_window(app);
        }
    })
    .map_err(|e| format!("Shortcut „{}“: {e}", s.shortcut_toggle))?;
    gs.on_shortcut(s.shortcut_stack_pop.as_str(), |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            commands::do_stack_pop(app);
        }
    })
    .map_err(|e| format!("Shortcut „{}“: {e}", s.shortcut_stack_pop))?;
    Ok(())
}

fn build_tray(app: &tauri::App, s: &Settings) -> tauri::Result<CheckMenuItem<tauri::Wry>> {
    let de = s.language == "de";
    let show = MenuItem::with_id(
        app,
        "show",
        if de { "clipon öffnen" } else { "Open clipon" },
        true,
        None::<&str>,
    )?;
    let pause = CheckMenuItem::with_id(
        app,
        "pause",
        if de { "Aufnahme pausieren" } else { "Pause capture" },
        true,
        s.paused,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        if de { "clipon beenden" } else { "Quit clipon" },
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &pause, &sep, &quit])?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "pause" => {
                let st = app.state::<AppState>();
                let now = !st.paused.load(std::sync::atomic::Ordering::Relaxed);
                drop(st);
                let app2 = app.clone();
                let st = app2.state::<AppState>();
                commands::set_paused(app.clone(), st, now);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(pause)
}

fn main() {
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
            let store = if store_path.exists() {
                clipon_core::load_store(&store_path, &key).unwrap_or_default()
            } else {
                clipon_core::Store::default()
            };
            let state = AppState {
                store: Mutex::new(store),
                settings: Mutex::new(s.clone()),
                key,
                data_dir,
                last_seen: Mutex::new(None),
                paused: AtomicBool::new(s.paused),
                pause_menu: Mutex::new(None),
            };
            app.manage(state);

            let pause_item = build_tray(app, &s)?;
            *app.state::<AppState>().pause_menu.lock().unwrap() = Some(pause_item);

            if let Err(e) = register_shortcuts(&handle, &s) {
                eprintln!("clipon: {e}");
            }
            watcher::spawn(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::set_paused,
            commands::list_items,
            commands::get_item_text,
            commands::get_item_image,
            commands::copy_item,
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
