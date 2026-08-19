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
    }
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
    if old.shortcut_toggle != new.shortcut_toggle || old.shortcut_stack_pop != new.shortcut_stack_pop
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
