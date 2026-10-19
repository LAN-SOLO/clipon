use crate::settings::Settings;
use clipon_core::{SourceApp, Store};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, MenuItem};

/// Tray menu entries whose labels follow the language setting.
pub struct TrayItems {
    pub show: MenuItem<tauri::Wry>,
    pub quit: MenuItem<tauri::Wry>,
}

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<Settings>,
    pub key: [u8; 32],
    pub data_dir: PathBuf,
    /// Hash of the clipboard content we saw (or wrote) last — used by the
    /// watcher to detect changes and to ignore our own copies.
    pub last_seen: Mutex<Option<u64>>,
    pub paused: AtomicBool,
    /// Tray "pause" checkbox, so UI-side toggles stay in sync.
    pub pause_menu: Mutex<Option<CheckMenuItem<tauri::Wry>>>,
    /// Frontmost app (+ pid), maintained by the macOS activation observer.
    pub frontmost: Mutex<Option<(SourceApp, i32)>>,
    /// App that was frontmost when the quick picker opened — paste target.
    pub picker_return: Mutex<Option<(SourceApp, i32)>>,
    /// Windows: foreground window (HWND) when the quick picker opened.
    pub picker_hwnd: Mutex<Option<isize>>,
    pub tray_items: Mutex<Option<TrayItems>>,
}

impl AppState {
    pub fn frontmost_source(&self) -> Option<SourceApp> {
        self.frontmost.lock().unwrap().as_ref().map(|(s, _)| s.clone())
    }

    pub fn store_path(&self) -> PathBuf {
        self.data_dir.join("history.clipon")
    }

    pub fn blobs_dir(&self) -> PathBuf {
        let dir = self.data_dir.join("blobs");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub fn persist(&self) {
        let store = self.store.lock().unwrap();
        if let Err(e) = clipon_core::save_store(&store, &self.store_path(), &self.key) {
            eprintln!("clipon: could not save history: {e}");
        }
    }

    /// Blob names come from the store; only plain file names inside the
    /// blobs directory are ever touched.
    pub fn delete_blobs(&self, files: &[String]) {
        let dir = self.blobs_dir();
        for f in files.iter().filter(|f| is_blob_name(f)) {
            let _ = std::fs::remove_file(dir.join(f));
        }
    }

    pub fn blob_path(&self, file: &str) -> Result<PathBuf, String> {
        if is_blob_name(file) {
            Ok(self.blobs_dir().join(file))
        } else {
            Err("invalid blob name".into())
        }
    }
}

/// Blob files are named `<uuid>.bin` by clipon itself.
pub fn is_blob_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && !name.starts_with('.')
        && !name.contains("..")
}

#[cfg(test)]
mod tests {
    use super::is_blob_name;

    #[test]
    fn blob_names_cannot_escape_the_blob_dir() {
        assert!(is_blob_name("9f2b2c1e-6a0f-4e0e-9b9e-1c2d3e4f5a6b.bin"));
        assert!(!is_blob_name("../history.clipon"));
        assert!(!is_blob_name("..\\history.key"));
        assert!(!is_blob_name("C:\\Windows\\x.bin"));
        assert!(!is_blob_name("/etc/passwd"));
        assert!(!is_blob_name(""));
        assert!(!is_blob_name(".."));
    }
}
