//! Clipboard watcher: polls the system clipboard and feeds new content into
//! the store. Runs on its own thread for the whole app lifetime.

use crate::state::AppState;
use arboard::ImageData;
use clipon_core::fnv1a;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const POLL_MS: u64 = 400;
/// Image file types we decode when files are copied (e.g. from Finder/Explorer).
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"];
const MAX_IMAGE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FILES_PER_COPY: usize = 20;

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        loop {
            std::thread::sleep(Duration::from_millis(POLL_MS));
            let state = app.state::<AppState>();
            if state.paused.load(Ordering::Relaxed) {
                continue;
            }
            // files first: a Finder/Explorer copy also carries the file name as
            // plain text, which would otherwise shadow the actual content
            if let Ok(files) = clipboard.get().file_list() {
                if !files.is_empty() {
                    handle_files(&app, &state, &files);
                    continue;
                }
            }
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    handle_text(&app, &state, &text);
                    continue;
                }
            }
            let capture_images = state.settings.lock().unwrap().capture_images;
            if capture_images {
                if let Ok(img) = clipboard.get_image() {
                    handle_image(&app, &state, &img);
                }
            }
        }
    });
}

fn handle_text(app: &AppHandle, state: &AppState, text: &str) {
    let hash = fnv1a(text.as_bytes());
    {
        let mut last = state.last_seen.lock().unwrap();
        if *last == Some(hash) {
            return;
        }
        *last = Some(hash);
    }
    let (limit, max_kb) = {
        let s = state.settings.lock().unwrap();
        (s.history_limit as usize, s.max_text_kb as usize)
    };
    if max_kb > 0 && text.len() > max_kb * 1024 {
        return;
    }
    let evicted = {
        let mut store = state.store.lock().unwrap();
        store.add_text(text);
        store.enforce_limit(limit.max(1))
    };
    state.delete_blobs(&evicted);
    state.persist();
    let _ = app.emit("history-changed", ());
}

/// Copied files: image files become image items (decoded like a bitmap copy),
/// everything else is kept as one path-list text item.
fn handle_files(app: &AppHandle, state: &AppState, files: &[PathBuf]) {
    let joined = files
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    let hash = fnv1a(joined.as_bytes());
    {
        let mut last = state.last_seen.lock().unwrap();
        if *last == Some(hash) {
            return;
        }
        *last = Some(hash);
    }
    let (limit, capture_images) = {
        let s = state.settings.lock().unwrap();
        (s.history_limit as usize, s.capture_images)
    };
    let mut changed = false;
    let mut others: Vec<String> = Vec::new();
    for path in files.iter().take(MAX_FILES_PER_COPY) {
        let is_image = path
            .extension()
            .map(|e| IMAGE_EXTS.contains(&e.to_string_lossy().to_lowercase().as_str()))
            .unwrap_or(false);
        if is_image && capture_images && add_image_file(state, path, limit) {
            changed = true;
        } else {
            others.push(path.to_string_lossy().into_owned());
        }
    }
    if !others.is_empty() {
        let evicted = {
            let mut store = state.store.lock().unwrap();
            store.add_text_as(&others.join("\n"), clipon_core::Detected::File);
            store.enforce_limit(limit.max(1))
        };
        state.delete_blobs(&evicted);
        changed = true;
    }
    if changed {
        state.persist();
        let _ = app.emit("history-changed", ());
    }
}

/// Reads and decodes one image file into an encrypted blob + history item.
fn add_image_file(state: &AppState, path: &Path, limit: usize) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if meta.len() > MAX_IMAGE_FILE_BYTES {
        return false;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let hash = fnv1a(&bytes);
    let already_known = {
        let store = state.store.lock().unwrap();
        store
            .items
            .iter()
            .any(|i| i.kind == clipon_core::ItemKind::Image && i.hash == hash)
    };
    let mut file_name = String::new();
    let (mut width, mut height) = (0u32, 0u32);
    if !already_known {
        let Ok(img) = image::load_from_memory(&bytes) else {
            return false;
        };
        (width, height) = (img.width(), img.height());
        let mut png = Vec::new();
        if img
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .is_err()
        {
            return false;
        }
        let Ok(blob) = clipon_core::encrypt(&state.key, &png) else {
            return false;
        };
        file_name = format!("{}.bin", uuid::Uuid::new_v4());
        if std::fs::write(state.blobs_dir().join(&file_name), blob).is_err() {
            return false;
        }
    }
    let label = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let evicted = {
        let mut store = state.store.lock().unwrap();
        store.add_image(&file_name, hash, width, height, label.as_deref());
        store.enforce_limit(limit.max(1))
    };
    state.delete_blobs(&evicted);
    true
}

/// Cheap content hash: dimensions + sampled pixel bytes, so we don't chew
/// through a full retina screenshot on every poll tick.
pub fn image_hash(img: &ImageData) -> u64 {
    let bytes = img.bytes.as_ref();
    let stride = (bytes.len() / 65536).max(1);
    let sampled: Vec<u8> = bytes.iter().step_by(stride).copied().collect();
    let mut seed = format!("{}x{}x{}", img.width, img.height, bytes.len()).into_bytes();
    seed.extend_from_slice(&sampled);
    fnv1a(&seed)
}

fn handle_image(app: &AppHandle, state: &AppState, img: &ImageData) {
    let hash = image_hash(img);
    {
        let mut last = state.last_seen.lock().unwrap();
        if *last == Some(hash) {
            return;
        }
        *last = Some(hash);
    }
    let already_known = {
        let store = state.store.lock().unwrap();
        store
            .items
            .iter()
            .any(|i| i.kind == clipon_core::ItemKind::Image && i.hash == hash)
    };
    let mut file_name = String::new();
    if !already_known {
        let Some(png) = encode_png(img) else { return };
        let Ok(blob) = clipon_core::encrypt(&state.key, &png) else {
            return;
        };
        file_name = format!("{}.bin", uuid::Uuid::new_v4());
        if std::fs::write(state.blobs_dir().join(&file_name), blob).is_err() {
            return;
        }
    }
    let limit = state.settings.lock().unwrap().history_limit as usize;
    let evicted = {
        let mut store = state.store.lock().unwrap();
        store.add_image(&file_name, hash, img.width as u32, img.height as u32, None);
        store.enforce_limit(limit.max(1))
    };
    state.delete_blobs(&evicted);
    state.persist();
    let _ = app.emit("history-changed", ());
}

pub fn encode_png(img: &ImageData) -> Option<Vec<u8>> {
    let rgba = image::RgbaImage::from_raw(
        img.width as u32,
        img.height as u32,
        img.bytes.as_ref().to_vec(),
    )?;
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}
