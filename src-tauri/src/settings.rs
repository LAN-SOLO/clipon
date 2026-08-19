//! Non-secret app settings, stored as JSON in the OS config directory.
//! The history itself lives encrypted in the data directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// "de" | "en"
    pub language: String,
    /// Max number of history items (pinned items never count against eviction).
    pub history_limit: u32,
    /// Also capture images from the clipboard.
    pub capture_images: bool,
    /// Skip text larger than this (KB); 0 = no limit.
    pub max_text_kb: u32,
    /// "Clear history" keeps pinned items.
    pub keep_pinned_on_clear: bool,
    /// Launch clipon at login.
    pub autostart: bool,
    /// Global shortcut: show/hide the clipon window.
    pub shortcut_toggle: String,
    /// Global shortcut: copy the next paste-stack item.
    pub shortcut_stack_pop: String,
    /// Capture paused.
    pub paused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: if sys_locale_is_german() { "de" } else { "en" }.into(),
            history_limit: 500,
            capture_images: true,
            max_text_kb: 512,
            keep_pinned_on_clear: true,
            autostart: false,
            shortcut_toggle: "CmdOrCtrl+Shift+V".into(),
            shortcut_stack_pop: "CmdOrCtrl+Shift+B".into(),
            paused: false,
        }
    }
}

fn sys_locale_is_german() -> bool {
    std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .map(|l| l.to_lowercase().starts_with("de"))
        .unwrap_or(false)
}

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

pub fn load(app: &tauri::AppHandle) -> Settings {
    std::fs::read_to_string(settings_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn store(app: &tauri::AppHandle, settings: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_path(app), json);
    }
}
