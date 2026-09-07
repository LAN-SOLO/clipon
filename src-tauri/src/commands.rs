use crate::settings::{self, Settings};
use crate::state::AppState;
use base64::Engine;
use clipon_core::{Detected, Filter, ItemKind};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

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
    let old = st.settings.lock().unwrap().clone();
    settings::store(&app, &new);
    *st.settings.lock().unwrap() = new.clone();
    st.paused.store(new.paused, Ordering::Relaxed);
    sync_pause_menu(&st, new.paused);

    if old.autostart != new.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let res = if new.autostart {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        res.map_err(|e| e.to_string())?;
    }
    if old.shortcut_toggle != new.shortcut_toggle
        || old.shortcut_stack_pop != new.shortcut_stack_pop
        || old.shortcut_picker != new.shortcut_picker
    {
        crate::register_shortcuts(&app, &new)?;
    }
    // shrink history if the limit went down
    let evicted = {
        let mut store = st.store.lock().unwrap();
        store.enforce_limit(new.history_limit.max(1) as usize)
    };
    if !evicted.is_empty() {
        st.delete_blobs(&evicted);
        st.persist();
        let _ = app.emit("history-changed", ());
    }
    // every window (main + quick picker) mirrors language/theme from settings
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_paused(app: AppHandle, st: State<'_, AppState>, paused: bool) {
    st.paused.store(paused, Ordering::Relaxed);
    let mut s = st.settings.lock().unwrap();
    s.paused = paused;
    settings::store(&app, &s);
    drop(s);
    sync_pause_menu(&st, paused);
    let _ = app.emit("paused-changed", paused);
}

pub fn sync_pause_menu(st: &AppState, paused: bool) {
    if let Some(item) = st.pause_menu.lock().unwrap().as_ref() {
        let _ = item.set_checked(paused);
    }
}

// --- history ---

#[tauri::command]
pub fn list_items(st: State<'_, AppState>, query: String, filter: Filter) -> Vec<ItemDto> {
    let store = st.store.lock().unwrap();
    store.search(&query, filter).into_iter().map(to_dto).collect()
}

#[tauri::command]
pub fn get_item_text(st: State<'_, AppState>, id: Uuid) -> Result<String, String> {
    let store = st.store.lock().unwrap();
    let item = store.get(id).ok_or("unbekanntes Element")?;
    item.text.clone().ok_or("kein Text".into())
}

/// Returns the image as a base64 PNG data URL, downscaled to `max_dim` if given.
#[tauri::command]
pub fn get_item_image(st: State<'_, AppState>, id: Uuid, max_dim: Option<u32>) -> Result<String, String> {
    let file = {
        let store = st.store.lock().unwrap();
        let item = store.get(id).ok_or("unbekanntes Element")?;
        item.image_file.clone().ok_or("kein Bild")?
    };
    let blob = std::fs::read(st.blobs_dir().join(&file)).map_err(|e| e.to_string())?;
    let png = clipon_core::decrypt(&st.key, &blob)?;
    let png = match max_dim {
        Some(max) => {
            let img = image::load_from_memory(&png).map_err(|e| e.to_string())?;
            if img.width().max(img.height()) > max {
                let thumb = img.thumbnail(max, max);
                let mut out = Vec::new();
                thumb
                    .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
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
}

fn write_clipboard_text(st: &AppState, text: &str) -> Result<(), String> {
    *st.last_seen.lock().unwrap() = Some(clipon_core::fnv1a(text.as_bytes()));
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text.to_string()).map_err(|e| e.to_string())
}

fn copy_to_clipboard(st: &AppState, id: Uuid) -> Result<(), String> {
    let (kind, text, image_file) = {
        let store = st.store.lock().unwrap();
        let item = store.get(id).ok_or("unbekanntes Element")?;
        (item.kind, item.text.clone(), item.image_file.clone())
    };
    match kind {
        ItemKind::Text => write_clipboard_text(st, &text.ok_or("kein Text")?),
        ItemKind::Image => {
            let file = image_file.ok_or("kein Bild")?;
            let blob = std::fs::read(st.blobs_dir().join(&file)).map_err(|e| e.to_string())?;
            let png = clipon_core::decrypt(&st.key, &blob)?;
            let img = image::load_from_memory(&png).map_err(|e| e.to_string())?.to_rgba8();
            let (w, h) = img.dimensions();
            let data = arboard::ImageData {
                width: w as usize,
                height: h as usize,
                bytes: std::borrow::Cow::Owned(img.into_raw()),
            };
            *st.last_seen.lock().unwrap() = Some(crate::watcher::image_hash(&data));
            let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            cb.set_image(data).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub fn copy_item(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    copy_to_clipboard(&st, id)?;
    st.store.lock().unwrap().touch(id);
    st.persist();
    let _ = app.emit("history-changed", ());
    Ok(())
}

/// Shared sink for derived text (color formats, transforms, merges): new
/// history entry + clipboard.
fn add_text_and_copy(app: &AppHandle, st: &AppState, text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("leerer Text".into());
    }
    write_clipboard_text(st, text)?;
    let limit = st.settings.lock().unwrap().history_limit as usize;
    let evicted = {
        let mut store = st.store.lock().unwrap();
        store.add_text(text);
        store.enforce_limit(limit.max(1))
    };
    st.delete_blobs(&evicted);
    st.persist();
    let _ = app.emit("history-changed", ());
    Ok(())
}

/// Adds text to the history and puts it on the clipboard — used by the color
/// picker and the format-conversion buttons in the detail pane.
#[tauri::command]
pub fn add_text_item(app: AppHandle, st: State<'_, AppState>, text: String) -> Result<(), String> {
    add_text_and_copy(&app, &st, &text)
}

#[tauri::command]
pub fn transform_item(
    app: AppHandle,
    st: State<'_, AppState>,
    id: Uuid,
    op: clipon_core::transform::Transform,
) -> Result<(), String> {
    let text = {
        let store = st.store.lock().unwrap();
        let item = store.get(id).ok_or("unbekanntes Element")?;
        item.text.clone().ok_or("kein Text")?
    };
    let out = clipon_core::transform::apply(op, &text)?;
    add_text_and_copy(&app, &st, &out)
}

/// Joins the text of all stack items into one new entry (stack stays intact).
#[tauri::command]
pub fn merge_stack(app: AppHandle, st: State<'_, AppState>) -> Result<(), String> {
    let text = st
        .store
        .lock()
        .unwrap()
        .stack_merged_text("\n")
        .ok_or("kein Text im Stack")?;
    add_text_and_copy(&app, &st, &text)
}

/// Writes one item to disk: `.txt` for text, `.png` for images.
#[tauri::command]
pub fn export_item(st: State<'_, AppState>, id: Uuid, path: String) -> Result<(), String> {
    let (text, image_file) = {
        let store = st.store.lock().unwrap();
        let item = store.get(id).ok_or("unbekanntes Element")?;
        (item.text.clone(), item.image_file.clone())
    };
    let bytes = match (text, image_file) {
        (Some(t), _) => t.into_bytes(),
        (None, Some(f)) => {
            let blob = std::fs::read(st.blobs_dir().join(&f)).map_err(|e| e.to_string())?;
            clipon_core::decrypt(&st.key, &blob)?
        }
        _ => return Err("leeres Element".into()),
    };
    std::fs::write(&path, bytes).map_err(|e| e.to_string())
}

fn write_atomic(path: &std::path::Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Backup of history and/or snippets; encrypted with a passphrase if given.
/// Async because the key derivation takes a moment — all locks are released
/// before it runs.
#[tauri::command]
pub async fn export_data(
    st: State<'_, AppState>,
    path: String,
    include_history: bool,
    include_snippets: bool,
    passphrase: Option<String>,
) -> Result<(), String> {
    let b64 = base64::engine::general_purpose::STANDARD;
    let bundle = {
        let store = st.store.lock().unwrap();
        store.to_bundle(include_history, include_snippets, |file| {
            let blob = std::fs::read(st.blobs_dir().join(file)).ok()?;
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
    write_atomic(std::path::Path::new(&path), &data)
}

#[tauri::command]
pub async fn import_data(
    app: AppHandle,
    st: State<'_, AppState>,
    path: String,
    passphrase: Option<String>,
) -> Result<clipon_core::export::ImportStats, String> {
    use clipon_core::export;
    let b64 = base64::engine::general_purpose::STANDARD;
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let json = if export::is_encrypted(&data) {
        let p = passphrase
            .filter(|p| !p.is_empty())
            .ok_or("passphrase-required")?;
        export::decrypt_export(&p, &data)?
    } else {
        data
    };
    let bundle: export::ExportBundle =
        serde_json::from_slice(&json).map_err(|e| format!("keine clipon-Exportdatei: {e}"))?;
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
    st.persist();
    let _ = app.emit("history-changed", ());
    let _ = app.emit("snippets-changed", ());
    Ok(stats)
}

#[tauri::command]
pub fn pin_item(app: AppHandle, st: State<'_, AppState>, id: Uuid, pinned: bool) {
    st.store.lock().unwrap().set_pinned(id, pinned);
    st.persist();
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
pub fn delete_item(app: AppHandle, st: State<'_, AppState>, id: Uuid) {
    let removed = st.store.lock().unwrap().delete(id);
    if let Some(f) = removed {
        st.delete_blobs(&[f]);
    }
    st.persist();
    let _ = app.emit("history-changed", ());
}

#[tauri::command]
pub fn clear_history(app: AppHandle, st: State<'_, AppState>) {
    let keep_pinned = st.settings.lock().unwrap().keep_pinned_on_clear;
    let files = st.store.lock().unwrap().clear(keep_pinned);
    st.delete_blobs(&files);
    st.persist();
    let _ = app.emit("history-changed", ());
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
pub fn save_snippet(
    st: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    text: String,
) -> Result<Uuid, String> {
    if name.trim().is_empty() {
        return Err("Name fehlt".into());
    }
    let id = st.store.lock().unwrap().save_snippet(id, name.trim(), &text);
    st.persist();
    Ok(id)
}

#[tauri::command]
pub fn delete_snippet(st: State<'_, AppState>, id: Uuid) {
    st.store.lock().unwrap().delete_snippet(id);
    st.persist();
}

#[tauri::command]
pub fn copy_snippet(st: State<'_, AppState>, id: Uuid) -> Result<(), String> {
    let text = {
        let store = st.store.lock().unwrap();
        store
            .snippets
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.text.clone())
            .ok_or("unbekanntes Snippet")?
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
pub fn stack_add(app: AppHandle, st: State<'_, AppState>, id: Uuid) {
    st.store.lock().unwrap().stack_push(id);
    st.persist();
    let _ = app.emit("stack-changed", ());
}

#[tauri::command]
pub fn stack_remove(app: AppHandle, st: State<'_, AppState>, id: Uuid) {
    st.store.lock().unwrap().stack_remove(id);
    st.persist();
    let _ = app.emit("stack-changed", ());
}

#[tauri::command]
pub fn stack_clear(app: AppHandle, st: State<'_, AppState>) {
    st.store.lock().unwrap().stack_clear();
    st.persist();
    let _ = app.emit("stack-changed", ());
}

/// Copies the next stack item to the clipboard and removes it from the stack.
#[tauri::command]
pub fn stack_pop_copy(app: AppHandle, st: State<'_, AppState>) -> Result<Option<ItemDto>, String> {
    let id = {
        let mut store = st.store.lock().unwrap();
        store.stack_pop()
    };
    let Some(id) = id else { return Ok(None) };
    copy_to_clipboard(&st, id)?;
    let dto = {
        let mut store = st.store.lock().unwrap();
        store.touch(id);
        store.get(id).map(to_dto)
    };
    st.persist();
    let _ = app.emit("stack-changed", ());
    let _ = app.emit("history-changed", ());
    Ok(dto)
}

/// Shortcut path for the global hotkey: pop + copy without the command plumbing.
pub fn do_stack_pop(app: &AppHandle) {
    let st = app.state::<AppState>();
    let id = st.store.lock().unwrap().stack_pop();
    if let Some(id) = id {
        if copy_to_clipboard(&st, id).is_ok() {
            st.store.lock().unwrap().touch(id);
            st.persist();
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
pub fn open_picker(app: &AppHandle) {
    let Some(win) = app.get_webview_window("picker") else { return };
    let st = app.state::<AppState>();
    *st.picker_return.lock().unwrap() = st.frontmost.lock().unwrap().clone();
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
/// Accessibility permission — pastes via ⌘V. Returns whether it pasted;
/// `false` means the content is on the clipboard and the user presses ⌘V.
#[tauri::command]
pub async fn picker_paste(app: AppHandle, st: State<'_, AppState>, id: Uuid) -> Result<bool, String> {
    copy_to_clipboard(&st, id)?;
    st.store.lock().unwrap().touch(id);
    st.persist();
    let _ = app.emit("history-changed", ());
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
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfoDto {
            version: update.version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Update-Prüfung fehlgeschlagen: {e}")),
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("Update-Prüfung fehlgeschlagen: {e}"))?
        .ok_or("Kein Update verfügbar")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Update fehlgeschlagen: {e}"))?;
    app.restart();
}
