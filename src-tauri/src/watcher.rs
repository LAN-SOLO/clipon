//! Clipboard watcher: polls the system clipboard and feeds new content into
//! the store. Runs on its own thread for the whole app lifetime.

use crate::state::AppState;
use arboard::ImageData;
use clipon_core::fnv1a;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const POLL_MS: u64 = 400;

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
        store.add_image(&file_name, hash, img.width as u32, img.height as u32);
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
