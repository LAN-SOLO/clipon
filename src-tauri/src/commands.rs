use crate::settings::{self, Settings};
use crate::state::AppState;
use base64::Engine;
use clipon_core::{Detected, Filter, ItemKind};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

// Errors cross the IPC boundary as stable codes ("code" or "code: detail");
// the frontend maps them to the UI language (src/errors.ts). Never send
// translated prose from here.
const E_UNKNOWN_ITEM: &str = "unknown-item";
const E_NO_TEXT: &str = "no-text";
const E_NO_IMAGE: &str = "no-image";

fn io_err(e: impl std::fmt::Display) -> String {
    format!("io: {e}")
}
fn clip_err(e: impl std::fmt::Display) -> String {
    format!("clipboard: {e}")
}
fn img_err(e: impl std::fmt::Display) -> String {
    format!("image: {e}")
}

/// Saves the store outside a command (global shortcut, picker): a failure
/// is reported to the main window as a `save-failed` event.
pub fn report_persist(app: &AppHandle, st: &AppState) {
    if let Err(e) = st.persist() {
        let _ = app.emit("save-failed", e);
    }
}

/// Runs heavy work (crypto, image decoding, file IO) on tokio's blocking
/// pool instead of the IPC/main thread, so the window never freezes.
async fn blocking<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| format!("internal: {e}"))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: Uuid,
    pub kind: ItemKind,
    pub preview: String,
    pub chars: usize,
    pub detected: Detected,
    pub pinned: bool,
    pub times_copied: u32,
    pub created_at: String,
    pub last_copied_at: String,
    pub source_app_id: Option<String>,
    pub source_app_name: Option<String>,
}

fn to_dto(i: &clipon_core::ClipItem) -> ItemDto {
    ItemDto {
        id: i.id,
        kind: i.kind,
        preview: i.preview.clone(),
        chars: i.chars,
        detected: i.detected,
        pinned: i.pinned,
        times_copied: i.times_copied,
        created_at: i.created_at.to_rfc3339(),
        last_copied_at: i.last_copied_at.to_rfc3339(),
        source_app_id: i.source_app.as_ref().and_then(|s| s.bundle_id.clone()),
        source_app_name: i.source_app.as_ref().and_then(|s| s.name.clone()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningAppDto {
    pub bundle_id: String,
    pub name: String,
}

/// Async so the wait for the main thread cannot deadlock the caller.
#[tauri::command]
pub async fn list_running_apps(app: AppHandle) -> Vec<RunningAppDto> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(crate::macos::running_apps());
    });
    rx.recv_timeout(std::time::Duration::from_secs(2))
        .unwrap_or_default()
        .into_iter()
        .map(|(bundle_id, name)| RunningAppDto { bundle_id, name })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetDto {
    pub id: Uuid,
    pub name: String,
    pub text: String,
}

// --- settings ---

#[tauri::command]
pub fn get_settings(st: State<'_, AppState>) -> Settings {
    st.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, st: State<'_, AppState>, new: Settings) -> Result<(), String> {
    let new = new.sanitized();
    let old = st.settings.lock().unwrap().clone();
    let mut errors: Vec<String> = Vec::new();
    if let Err(e) = settings::store(&app, &new) {
        errors.push(e);
    }
    *st.settings.lock().unwrap() = new.clone();
    st.paused.store(new.paused, Ordering::Relaxed);
    sync_pause_menu(&st, new.paused);

    // A failing step (autostart, shortcut registration) must not skip the
    // remaining ones — the settings are already stored at this point.
    if old.autostart != new.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let res = if new.autostart {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        if let Err(e) = res {
            errors.push(format!("autostart: {e}"));
        }
    }
    if old.shortcut_toggle != new.shortcut_toggle
        || old.shortcut_stack_pop != new.shortcut_stack_pop
        || old.shortcut_picker != new.shortcut_picker
    {
        if let Err(e) = crate::register_shortcuts(&app, &new) {
            errors.push(e);
        }
    }
    // shrink history if the limit went down — text items have no blob, so
    // compare the item count instead of the evicted blob list
    let (shrunk, evicted) = {
        let mut store = st.store.lock().unwrap();
        let before = store.items.len();
        let evicted = store.enforce_limit(new.history_limit.max(1) as usize);
        (store.items.len() != before, evicted)
    };
    if shrunk {
        st.delete_blobs(&evicted);
        if let Err(e) = st.persist() {
            errors.push(e);
        }
        let _ = app.emit("history-changed", ());
        let _ = app.emit("stack-changed", ());
    }
    if old.language != new.language {
        crate::update_tray_labels(&st, &new.language);
    }
    // every window (main + quick picker) mirrors language/theme from settings
    let _ = app.emit("settings-changed", ());
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" | "))
    }
}

#[tauri::command]
pub fn set_paused(app: AppHandle, st: State<'_, AppState>, paused: bool) -> Result<(), String> {
    st.paused.store(paused, Ordering::Relaxed);
    let mut s = st.settings.lock().unwrap();
    s.paused = paused;
    let saved = settings::store(&app, &s);
    drop(s);
    sync_pause_menu(&st, paused);
    let _ = app.emit("paused-changed", paused);
    // the pause itself is in effect; only remembering it across restarts failed
    saved
}

pub fn sync_pause_menu(st: &AppState, paused: bool) {
    if let Some(item) = st.pause_menu.lock().unwrap().as_ref() {
        let _ = item.set_checked(paused);
    }
}

// --- history ---

// Commands without `async` run on the main thread in Tauri 2 — everything
// that touches the store (persist = encrypt + write), the clipboard or image
// data is async so a big history never stalls the window.

#[tauri::command]
pub async fn list_items(st: State<'_, AppState>, query: String, filter: Filter) -> Result<Vec<ItemDto>, String> {
    let store = st.store.lock().unwrap();
    Ok(store.search(&query, filter).into_iter().map(to_dto).collect())
}

#[tauri::command]
pub async fn get_item_text(st: State<'_, AppState>, id: Uuid) -> Result<String, String> {
    let store = st.store.lock().unwrap();
    let item = store.get(id).ok_or(E_UNKNOWN_ITEM)?;
    item.text.clone().ok_or_else(|| E_NO_TEXT.into())
}

/// Returns the image as a base64 PNG data URL, downscaled to `max_dim` if given.
#[tauri::command]
pub async fn get_item_image(app: AppHandle, id: Uuid, max_dim: Option<u32>) -> Result<String, String> {
    blocking(&app, move |app| {
        let st = app.state::<AppState>();
        let file = {
            let store = st.store.lock().unwrap();
            let item = store.get(id).ok_or(E_UNKNOWN_ITEM)?;
            item.image_file.clone().ok_or(E_NO_IMAGE)?
        };
        let blob = clipon_core::read_file(&st.blob_path(&file)?).map_err(io_err)?;
        let png = clipon_core::decrypt(&st.key, &blob)?;
        let png = match max_dim {
            Some(max) => {
                let img = image::load_from_memory(&png).map_err(img_err)?;
                if img.width().max(img.height()) > max {
                    let thumb = img.thumbnail(max, max);
                    let mut out = Vec::new();
                    thumb
                        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                        .map_err(img_err)?;
                    out
                } else {
                    png
                }
            }
            None => png,
        };
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        ))
    })
    .await
}

fn write_clipboard_text(st: &AppState, text: &str) -> Result<(), String> {
    *st.last_seen.lock().unwrap() = Some(clipon_core::fnv1a(text.as_bytes()));
    let mut cb = arboard::Clipboard::new().map_err(clip_err)?;
    cb.set_text(text.to_string()).map_err(clip_err)
}

fn copy_to_clipboard(st: &AppState, id: Uuid) -> Result<(), String> {
    let (kind, text, image_file) = {
        let store = st.store.lock().unwrap();
        let item = store.get(id).ok_or(E_UNKNOWN_ITEM)?;
        (item.kind, item.text.clone(), item.image_file.clone())
    };
    match kind {
        ItemKind::Text => write_clipboard_text(st, &text.ok_or(E_NO_TEXT)?),
        ItemKind::Image => {
            let file = image_file.ok_or(E_NO_IMAGE)?;
            let blob = clipon_core::read_file(&st.blob_path(&file)?).map_err(io_err)?;
            let png = clipon_core::decrypt(&st.key, &blob)?;
            let img = image::load_from_memory(&png).map_err(img_err)?.to_rgba8();
            let (w, h) = img.dimensions();
            let data = arboard::ImageData {
                width: w as usize,
                height: h as usize,
                bytes: std::borrow::Cow::Owned(img.into_raw()),
            };
            *st.last_seen.lock().unwrap() = Some(crate::watcher::image_hash(&data));
            let mut cb = arboard::Clipboard::new().map_err(clip_err)?;
            cb.set_image(data).map_err(clip_err)
        }
    }
}

#[tauri::command]
pub async fn copy_item(app: AppHandle, id: Uuid) -> Result<(), String> {
    blocking(&app, move |app| {
        let st = app.state::<AppState>();
        copy_to_clipboard(&st, id)?;
        st.store.lock().unwrap().touch(id);
        let saved = st.persist();
        let _ = app.emit("history-changed", ());
        saved
    })
    .await
}

/// Shared sink for derived text (color formats, transforms, merges): new
/// history entry + clipboard.
fn add_text_and_copy(app: &AppHandle, st: &AppState, text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("empty-text".into());
    }
    write_clipboard_text(st, text)?;
    let limit = st.settings.lock().unwrap().history_limit as usize;
    let evicted = {
        let mut store = st.store.lock().unwrap();
        store.add_text(text);
        store.enforce_limit(limit.max(1))
    };
    st.delete_blobs(&evicted);
    let saved = st.persist();
    let _ = app.emit("history-changed", ());
    // limit eviction may have taken stack entries along (text items leave
    // no blob behind, so `evicted` alone cannot tell)
    let _ = app.emit("stack-changed", ());
    saved
}

/// Adds text to the history and puts it on the clipboard — used by the color
/// picker and the format-conversion buttons in the detail pane.
#[tauri::command]
pub async fn add_text_item(app: AppHandle, st: State<'_, AppState>, text: String) -> Result<(), String> {
    add_text_and_copy(&app, &st, &text)
}

#[tauri::command]
pub async fn transform_item(
    app: AppHandle,
    st: State<'_, AppState>,
    id: Uuid,
    op: clipon_core::transform::Transform,
) -> Result<(), String> {
    let text = {
        let store = st.store.lock().unwrap();
        let item = store.get(id).ok_or(E_UNKNOWN_ITEM)?;
        item.text.clone().ok_or(E_NO_TEXT)?
    };
    let out = clipon_core::transform::apply(op, &text)?;
    add_text_and_copy(&app, &st, &out)
}

/// Joins the text of all stack items into one new entry (stack stays intact).
#[tauri::command]
pub async fn merge_stack(app: AppHandle, st: State<'_, AppState>) -> Result<(), String> {
    let text = st
        .store
        .lock()
        .unwrap()
        .stack_merged_text("\n")
        .ok_or("stack-no-text")?;
    add_text_and_copy(&app, &st, &text)
}

/// Paths come from the save dialog, but the IPC boundary must not become a
/// "write any file" primitive: the extension is forced to the expected type.
fn with_allowed_extension(path: &str, allowed: &[&str]) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(path);
    let ok = p
        .extension()
        .map(|e| allowed.iter().any(|a| e.to_string_lossy().eq_ignore_ascii_case(a)))
        .unwrap_or(false);
    if ok {
        p
    } else {
        let mut s = p.into_os_string();
        s.push(".");
        s.push(allowed[0]);
        s.into()
    }
}

/// Writes one item to disk: `.txt` for text, `.png` for images.
#[tauri::command]
pub async fn export_item(app: AppHandle, id: Uuid, path: String) -> Result<(), String> {
    blocking(&app, move |app| {
        let st = app.state::<AppState>();
        let (text, image_file) = {
            let store = st.store.lock().unwrap();
            let item = store.get(id).ok_or(E_UNKNOWN_ITEM)?;
            (item.text.clone(), item.image_file.clone())
        };
        let path = with_allowed_extension(&path, if text.is_some() { &["txt"] } else { &["png"] });
        let bytes = match (text, image_file) {
            (Some(t), _) => t.into_bytes(),
            (None, Some(f)) => {
                let blob = clipon_core::read_file(&st.blob_path(&f)?).map_err(io_err)?;
                clipon_core::decrypt(&st.key, &blob)?
            }
            _ => return Err("empty-item".into()),
        };
        write_atomic(&path, &bytes)
    })
    .await
}

fn write_atomic(path: &std::path::Path, data: &[u8]) -> Result<(), String> {
    // sibling temp file with its own suffix — never clobbers an existing
    // "<name>.tmp" of the user
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".clipon-part");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, data).map_err(io_err)?;
    // Windows: the target may be open briefly (scanner, OneDrive) — retry
    clipon_core::replace_file(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io_err(e)
    })
}

/// Upper bound for backup files read on import (they are loaded into memory).
const MAX_IMPORT_BYTES: u64 = 1024 * 1024 * 1024;

/// Backup of history and/or snippets; encrypted with a passphrase if given.
/// The key derivation (PBKDF2, 600k rounds), image decryption and the file
/// write run on the blocking pool.
#[tauri::command]
pub async fn export_data(
    app: AppHandle,
    path: String,
    include_history: bool,
    include_snippets: bool,
    passphrase: Option<String>,
) -> Result<(), String> {
    blocking(&app, move |app| {
        let st = app.state::<AppState>();
        let b64 = base64::engine::general_purpose::STANDARD;
        let bundle = {
            let store = st.store.lock().unwrap();
            store.to_bundle(include_history, include_snippets, |file| {
                let blob = clipon_core::read_file(&st.blob_path(file).ok()?).ok()?;
                let png = clipon_core::decrypt(&st.key, &blob).ok()?;
                Some(b64.encode(png))
            })
        };
        let passphrase = passphrase.filter(|p| !p.is_empty());
        let data = match passphrase {
            Some(p) => {
                let json = serde_json::to_vec(&bundle).map_err(|e| e.to_string())?;
                clipon_core::export::encrypt_export(&p, &json)?
            }
            None => serde_json::to_vec_pretty(&bundle).map_err(|e| e.to_string())?,
        };
        write_atomic(&with_allowed_extension(&path, &["json", "clipon-backup"]), &data)
    })
    .await
}

/// Parses an import file: optional passphrase decryption, then JSON (a UTF-8
/// BOM from Windows editors is accepted).
fn parse_import(
    data: Vec<u8>,
    passphrase: Option<String>,
) -> Result<clipon_core::export::ExportBundle, String> {
    use clipon_core::export;
    let json = if export::is_encrypted(&data) {
        let p = passphrase
            .filter(|p| !p.is_empty())
            .ok_or("passphrase-required")?;
        export::decrypt_export(&p, &data)?
    } else {
        data
    };
    serde_json::from_slice(clipon_core::strip_bom(&json)).map_err(|e| format!("not-clipon-export: {e}"))
}

#[tauri::command]
pub async fn import_data(
    app: AppHandle,
    path: String,
    passphrase: Option<String>,
) -> Result<clipon_core::export::ImportStats, String> {
    blocking(&app, move |app| {
        let st = app.state::<AppState>();
        let b64 = base64::engine::general_purpose::STANDARD;
        let size = std::fs::metadata(&path).map_err(io_err)?.len();
        if size > MAX_IMPORT_BYTES {
            return Err("import-too-large".into());
        }
        let data = clipon_core::read_file(std::path::Path::new(&path)).map_err(io_err)?;
        let bundle = parse_import(data, passphrase)?;
        let limit = st.settings.lock().unwrap().history_limit as usize;
        let (stats, evicted) = {
            let mut store = st.store.lock().unwrap();
            let stats = store.merge_bundle(bundle, |png_b64| {
                let png = b64.decode(png_b64).ok()?;
                image::load_from_memory(&png).ok()?;
                let blob = clipon_core::encrypt(&st.key, &png).ok()?;
                let name = format!("{}.bin", Uuid::new_v4());
                std::fs::write(st.blobs_dir().join(&name), blob).ok()?;
                Some(name)
            });
            (stats, store.enforce_limit(limit.max(1)))
        };
        st.delete_blobs(&evicted);
        let saved = st.persist();
        let _ = app.emit("history-changed", ());
        let _ = app.emit("stack-changed", ());
        let _ = app.emit("snippets-changed", ());
        saved.map(|_| stats)
    })
    .await
}

#[tauri::command]
pub async fn pin_item(app: AppHandle, st: State<'_, AppState>, id: Uuid, pinned: bool) -> Result<(), String> {
    st.store.lock().unwrap().set_pinned(id, pinned);
    let saved = st.persist();
    let _ = app.emit("history-changed", ());
    saved
}

#[tauri::command]
pub async fn delete_item(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let removed = st.store.lock().unwrap().delete(id);
    if let Some(f) = removed {
        st.delete_blobs(&[f]);
    }
    let saved = st.persist();
    let _ = app.emit("history-changed", ());
    // the item may have been on the paste stack, too
    let _ = app.emit("stack-changed", ());
    saved
}

#[tauri::command]
pub async fn clear_history(app: AppHandle, st: State<'_, AppState>) -> Result<(), String> {
    let keep_pinned = st.settings.lock().unwrap().keep_pinned_on_clear;
    let files = st.store.lock().unwrap().clear(keep_pinned);
    st.delete_blobs(&files);
    let saved = st.persist();
    let _ = app.emit("history-changed", ());
    let _ = app.emit("stack-changed", ());
    saved
}

// --- snippets ---

#[tauri::command]
pub fn list_snippets(st: State<'_, AppState>) -> Vec<SnippetDto> {
    st.store
        .lock()
        .unwrap()
        .snippets
        .iter()
        .map(|s| SnippetDto {
            id: s.id,
            name: s.name.clone(),
            text: s.text.clone(),
        })
        .collect()
}

#[tauri::command]
pub async fn save_snippet(
    st: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    text: String,
) -> Result<Uuid, String> {
    if name.trim().is_empty() {
        return Err("snippet-name-missing".into());
    }
    let id = st.store.lock().unwrap().save_snippet(id, name.trim(), &text);
    let saved = st.persist();
    saved.map(|_| id)
}

#[tauri::command]
pub async fn delete_snippet(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    st.store.lock().unwrap().delete_snippet(id);
    let saved = st.persist();
    let _ = app.emit("snippets-changed", ());
    saved
}

#[tauri::command]
pub async fn copy_snippet(st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let text = {
        let store = st.store.lock().unwrap();
        store
            .snippets
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.text.clone())
            .ok_or("unknown-snippet")?
    };
    // placeholders see the clipboard as it is *before* the snippet lands there
    let text = if text.contains('{') {
        let current = arboard::Clipboard::new()
            .ok()
            .and_then(|mut c| c.get_text().ok());
        clipon_core::placeholder::expand(&text, chrono::Local::now(), current.as_deref())
    } else {
        text
    };
    write_clipboard_text(&st, &text)
}

// --- paste stack ---

#[tauri::command]
pub fn stack_list(st: State<'_, AppState>) -> Vec<ItemDto> {
    let store = st.store.lock().unwrap();
    store
        .stack
        .iter()
        .filter_map(|id| store.get(*id))
        .map(to_dto)
        .collect()
}

#[tauri::command]
pub async fn stack_add(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    st.store.lock().unwrap().stack_push(id);
    let saved = st.persist();
    let _ = app.emit("stack-changed", ());
    saved
}

#[tauri::command]
pub async fn stack_remove(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    st.store.lock().unwrap().stack_remove(id);
    let saved = st.persist();
    let _ = app.emit("stack-changed", ());
    saved
}

#[tauri::command]
pub async fn stack_clear(app: AppHandle, st: State<'_, AppState>) -> Result<(), String> {
    st.store.lock().unwrap().stack_clear();
    let saved = st.persist();
    let _ = app.emit("stack-changed", ());
    saved
}

/// Copies the next stack item to the clipboard and removes it from the stack.
#[tauri::command]
pub async fn stack_pop_copy(app: AppHandle, st: State<'_, AppState>) -> Result<Option<ItemDto>, String> {
    // copy first, pop after: a failing clipboard must not lose the entry
    let Some(id) = st.store.lock().unwrap().stack_peek() else { return Ok(None) };
    copy_to_clipboard(&st, id)?;
    st.store.lock().unwrap().stack_remove(id);
    let dto = {
        let mut store = st.store.lock().unwrap();
        store.touch(id);
        store.get(id).map(to_dto)
    };
    let saved = st.persist();
    let _ = app.emit("stack-changed", ());
    let _ = app.emit("history-changed", ());
    saved.map(|_| dto)
}

/// Shortcut path for the global hotkey: pop + copy without the command plumbing.
pub fn do_stack_pop(app: &AppHandle) {
    let st = app.state::<AppState>();
    let id = st.store.lock().unwrap().stack_peek();
    if let Some(id) = id {
        if copy_to_clipboard(&st, id).is_ok() {
            let mut store = st.store.lock().unwrap();
            store.stack_remove(id);
            store.touch(id);
            drop(store);
            report_persist(app, &st);
            let _ = app.emit("stack-changed", ());
            let _ = app.emit("history-changed", ());
        }
    }
}

// --- quick picker ---

const PICKER_W: f64 = 560.0;
const PICKER_H: f64 = 420.0;

/// Opens the picker near the cursor. Remembers the frontmost app first, so
/// `picker_paste` knows where to return to once the picker has taken focus.
/// The quick picker lives in its own always-on-top window. It is created
/// here in code rather than in tauri.conf.json: on macOS eagerly at startup
/// (so the first ⌘⇧Space is instant), on Windows/Linux only on first use —
/// a second, hidden WebView2 created during startup left the main window
/// black there.
pub fn ensure_picker(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(win) = app.get_webview_window("picker") {
        return Some(win);
    }
    tauri::WebviewWindowBuilder::new(app, "picker", tauri::WebviewUrl::App("index.html".into()))
        .title("clipon")
        .inner_size(PICKER_W, PICKER_H)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .background_color(tauri::utils::config::Color(0x0b, 0x12, 0x20, 0xff))
        .build()
        .ok()
}

pub fn open_picker(app: &AppHandle) {
    // remember the paste target first — the global shortcut fires while the
    // target app is still in front
    let st = app.state::<AppState>();
    *st.picker_return.lock().unwrap() = st.frontmost.lock().unwrap().clone();
    #[cfg(target_os = "windows")]
    {
        *st.picker_hwnd.lock().unwrap() = crate::win::foreground_window();
    }
    if let Some(win) = app.get_webview_window("picker") {
        present_picker(app, &win);
        return;
    }
    // First use on Windows/Linux: this runs in the global-shortcut handler or
    // a sync command, i.e. on the event-loop thread, where building a WebView2
    // window deadlocks (tauri docs, "Known issues"). Build it on a worker
    // thread instead; one build at a time.
    static BUILDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if BUILDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        if let Some(win) = ensure_picker(&handle) {
            present_picker(&handle, &win);
        }
        BUILDING.store(false, Ordering::SeqCst);
    });
}

/// Places the picker at the mouse pointer (clamped to the work area of that
/// monitor), shows and focuses it.
fn present_picker(app: &AppHandle, win: &tauri::WebviewWindow) {
    if let Ok(cursor) = app.cursor_position() {
        let monitor = app.monitor_from_point(cursor.x, cursor.y).ok().flatten();
        if let Some(m) = monitor {
            let scale = m.scale_factor();
            let (w, h) = (PICKER_W * scale, PICKER_H * scale);
            let area = m.work_area();
            let (ax, ay) = (area.position.x as f64, area.position.y as f64);
            let (aw, ah) = (area.size.width as f64, area.size.height as f64);
            let x = (cursor.x - w / 2.0).clamp(ax, (ax + aw - w).max(ax));
            let y = (cursor.y + 12.0).clamp(ay, (ay + ah - h).max(ay));
            let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        } else {
            let _ = win.center();
        }
    } else {
        let _ = win.center();
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = app.emit_to("picker", "picker-open", ());
}

#[tauri::command]
pub fn show_picker(app: AppHandle) {
    open_picker(&app);
}

#[tauri::command]
pub fn hide_picker(app: AppHandle) {
    if let Some(win) = app.get_webview_window("picker") {
        let _ = win.hide();
    }
}

/// Copies the item, hides the picker, brings the previous app back and — with
/// Accessibility permission (macOS) or always (Windows) — pastes via ⌘V/Ctrl+V. Returns whether it pasted;
/// `false` means the content is on the clipboard and the user presses ⌘V.
#[tauri::command]
pub async fn picker_paste(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<bool, String> {
    copy_to_clipboard(&st, id)?;
    st.store.lock().unwrap().touch(id);
    report_persist(&app, &st);
    let _ = app.emit("history-changed", ());

    // Windows: hand the focus back *before* hiding — only the foreground
    // process may call SetForegroundWindow — then type Ctrl+V.
    #[cfg(target_os = "windows")]
    {
        let Some(hwnd) = *st.picker_hwnd.lock().unwrap() else {
            hide_picker(app.clone());
            return Ok(false);
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = app.run_on_main_thread(move || {
            let _ = tx.send(crate::win::request_foreground(hwnd));
        });
        let requested = rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap_or(false);
        hide_picker(app.clone());
        if !requested || !crate::win::wait_foreground(hwnd) {
            return Ok(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(60));
        return Ok(crate::win::send_ctrl_v());
    }

    #[allow(unreachable_code)]
    hide_picker(app.clone());

    let target = st.picker_return.lock().unwrap().clone();
    let Some((_, pid)) = target else { return Ok(false) };
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(crate::macos::activate_pid(pid));
    });
    if !rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap_or(false) {
        return Ok(false);
    }
    // wait until the target really is frontmost again (observer-driven cache)
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(25));
        let front = st.frontmost.lock().unwrap().as_ref().map(|(_, p)| *p);
        if front == Some(pid) {
            break;
        }
    }
    if !crate::macos::ax_trusted(false) {
        return Ok(false);
    }
    std::thread::sleep(std::time::Duration::from_millis(60));
    Ok(crate::macos::send_cmd_v())
}

#[tauri::command]
pub fn accessibility_status() -> bool {
    crate::macos::ax_trusted(false)
}

/// Triggers the system prompt if the permission is missing.
#[tauri::command]
pub fn request_accessibility() -> bool {
    crate::macos::ax_trusted(true)
}

// --- updates ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfoDto>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| format!("update-check-failed: {e}"))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfoDto {
            version: update.version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("update-check-failed: {e}")),
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| format!("update-check-failed: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("update-check-failed: {e}"))?
        .ok_or("no-update")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("update-failed: {e}"))?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::{parse_import, with_allowed_extension};
    use std::path::PathBuf;

    fn sample_json() -> Vec<u8> {
        let mut store = clipon_core::Store::default();
        store.add_text("hallo");
        let bundle = store.to_bundle(true, true, |_| None);
        serde_json::to_vec(&bundle).unwrap()
    }

    /// Every error code the backend sends must have a DE and an EN text in
    /// src/i18n.ts (`errors` map), or the UI would show the raw code.
    #[test]
    fn every_error_code_is_translated() {
        const CODES: &[&str] = &[
            "unknown-item", "unknown-snippet", "no-text", "no-image", "empty-text", "empty-item",
            "stack-no-text", "snippet-name-missing", "import-too-large", "not-clipon-export",
            "passphrase-required", "wrong-passphrase", "encrypt-failed", "decrypt-failed",
            "not-clipon-file", "invalid-blob", "invalid-json", "url-malformed", "url-not-utf8",
            "update-check-failed", "update-failed", "no-update", "io", "clipboard", "image",
            "internal", "save-failed", "autostart",
        ];
        let i18n = include_str!("../../src/i18n.ts");
        let split = i18n.find("const en").expect("en dict");
        let (de, en) = i18n.split_at(split);
        for code in CODES {
            let key = if code.contains('-') { format!("'{code}':") } else { format!("{code}:") };
            assert!(de.contains(&key), "DE text missing for error code {code}");
            assert!(en.contains(&key), "EN text missing for error code {code}");
        }
        // and the sources really use these spellings
        let sources = [
            include_str!("commands.rs"),
            include_str!("state.rs"),
            include_str!("../../core/src/lib.rs"),
            include_str!("../../core/src/export.rs"),
            include_str!("../../core/src/transform.rs"),
        ]
        .concat();
        for code in CODES.iter().filter(|c| c.contains('-')) {
            assert!(sources.contains(&format!("\"{code}")), "code {code} not used by the backend");
        }
    }

    #[test]
    fn import_accepts_utf8_bom_and_reports_codes() {
        // Windows Notepad saves JSON as UTF-8 with BOM
        let mut with_bom = b"\xEF\xBB\xBF".to_vec();
        with_bom.extend(sample_json());
        assert_eq!(parse_import(with_bom, None).unwrap().items.len(), 1);
        assert_eq!(parse_import(sample_json(), None).unwrap().items.len(), 1);

        // errors are stable codes for the frontend's i18n mapping
        let e = parse_import(b"no json".to_vec(), None).unwrap_err();
        assert!(e.starts_with("not-clipon-export"), "{e}");
        let enc = clipon_core::export::encrypt_export("geheim", &sample_json()).unwrap();
        assert_eq!(parse_import(enc.clone(), None).unwrap_err(), "passphrase-required");
        assert_eq!(parse_import(enc.clone(), Some(String::new())).unwrap_err(), "passphrase-required");
        assert_eq!(parse_import(enc.clone(), Some("falsch".into())).unwrap_err(), "wrong-passphrase");
        assert_eq!(parse_import(enc, Some("geheim".into())).unwrap().items.len(), 1);
    }

    #[test]
    fn export_paths_keep_or_get_the_expected_extension() {
        assert_eq!(with_allowed_extension("/a/b.txt", &["txt"]), PathBuf::from("/a/b.txt"));
        assert_eq!(with_allowed_extension("/a/b.TXT", &["txt"]), PathBuf::from("/a/b.TXT"));
        assert_eq!(with_allowed_extension("/a/b", &["txt"]), PathBuf::from("/a/b.txt"));
        assert_eq!(with_allowed_extension("/a/.zshrc", &["txt"]), PathBuf::from("/a/.zshrc.txt"));
        assert_eq!(
            with_allowed_extension(r"C:\Users\a\backup.clipon-backup", &["json", "clipon-backup"]),
            PathBuf::from(r"C:\Users\a\backup.clipon-backup")
        );
        assert_eq!(
            with_allowed_extension(r"C:\Users\a\evil.exe", &["png"]),
            PathBuf::from(r"C:\Users\a\evil.exe.png")
        );
    }
}
