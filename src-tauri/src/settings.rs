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
    /// Global shortcut: open the quick picker at the cursor.
    #[serde(default = "default_shortcut_picker")]
    pub shortcut_picker: String,
    /// In-app shortcut overrides (action id → combo). The frontend owns the
    /// action list and defaults; this only stores what the user changed.
    pub keymap: std::collections::BTreeMap<String, String>,
    /// Capture paused.
    pub paused: bool,
    /// Bundle ids whose copies are never recorded. Field-level default so
    /// settings files from older versions pick up the password-manager list.
    #[serde(default = "default_ignored_apps")]
    pub ignored_apps: Vec<String>,
    /// Appearance: "dark" | "light". Field-level default so older settings
    /// files keep the dark look.
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "dark".into()
}

fn default_shortcut_picker() -> String {
    "CmdOrCtrl+Shift+Space".into()
}

fn default_ignored_apps() -> Vec<String> {
    // macOS: bundle ids · Windows: executable file names (the ignore list
    // matches whichever the platform reports, case-insensitively)
    let list: &[&str] = if cfg!(target_os = "windows") {
        &["1Password.exe", "Bitwarden.exe", "KeePass.exe", "KeePassXC.exe", "Enpass.exe"]
    } else {
        &[
            "com.1password.1password",
            "com.agilebits.onepassword7",
            "com.bitwarden.desktop",
            "org.keepassxc.keepassxc",
            "com.apple.keychainaccess",
            "com.apple.Passwords",
        ]
    };
    list
    .iter()
    .map(|s| s.to_string())
    .collect()
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
            shortcut_picker: default_shortcut_picker(),
            keymap: Default::default(),
            paused: false,
            ignored_apps: default_ignored_apps(),
            theme: default_theme(),
        }
    }
}

fn sys_locale_is_german() -> bool {
    crate::macos::system_language_is_german()
}

pub const HISTORY_LIMIT_MAX: u32 = 100_000;

impl Settings {
    /// Settings come from the webview (or a hand-edited file): clamp numbers
    /// and fall back to defaults for unknown enum-like strings.
    pub fn sanitized(mut self) -> Self {
        if self.language != "de" && self.language != "en" {
            self.language = Settings::default().language;
        }
        if self.theme != "dark" && self.theme != "light" {
            self.theme = default_theme();
        }
        self.history_limit = self.history_limit.clamp(1, HISTORY_LIMIT_MAX);
        self.ignored_apps = self
            .ignored_apps
            .into_iter()
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect();
        self
    }
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
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default()
        .sanitized()
}

pub fn store(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("save-failed: {e}"))?;
    let path = settings_path(app);
    // write + rename: a crash mid-write must not leave a truncated file
    // (which would silently reset every setting on the next start); the
    // rename retries while Windows briefly locks the target (scanner, sync)
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)
        .and_then(|_| clipon_core::replace_file(&tmp, &path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            eprintln!("clipon: could not save settings to {}: {e}", path.display());
            format!("save-failed: {e}")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_and_defaults() {
        let s = Settings {
            language: "fr".into(),
            theme: "neon".into(),
            history_limit: 0,
            ignored_apps: vec!["  KeePass.exe ".into(), "   ".into()],
            ..Settings::default()
        }
        .sanitized();
        assert!(s.language == "de" || s.language == "en");
        assert_eq!(s.theme, "dark");
        assert_eq!(s.history_limit, 1);
        assert_eq!(s.ignored_apps, vec!["KeePass.exe".to_string()]);
        let big = Settings { history_limit: u32::MAX, ..Settings::default() }.sanitized();
        assert_eq!(big.history_limit, HISTORY_LIMIT_MAX);
    }

    #[test]
    fn old_settings_file_gets_new_defaults() {
        let s: Settings = serde_json::from_str(r#"{"language":"de","historyLimit":42}"#).unwrap();
        assert_eq!(s.history_limit, 42);
        assert_eq!(s.theme, "dark");
        assert_eq!(s.shortcut_picker, "CmdOrCtrl+Shift+Space");
        assert!(!s.ignored_apps.is_empty());
    }
}
