use crate::settings::Settings;
use clipon_core::Store;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::menu::CheckMenuItem;

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
}

impl AppState {
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
        let _ = clipon_core::save_store(&store, &self.store_path(), &self.key);
    }

    pub fn delete_blobs(&self, files: &[String]) {
        let dir = self.blobs_dir();
        for f in files {
            let _ = std::fs::remove_file(dir.join(f));
        }
    }
}
