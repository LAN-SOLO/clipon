//! clipon core: clipboard history model, encrypted persistence, snippets.
//!
//! The store is a newest-first list of clipboard items plus user snippets.
//! Persistence is a single AES-256-GCM-encrypted JSON file; the key is
//! provided by the caller (the app keeps it in the OS keychain). Image
//! payloads live as separate encrypted blobs next to the store file and are
//! referenced by file name.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

pub const STORE_MAGIC: &[u8; 8] = b"CLIPON1\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Text,
    Image,
}

/// What the text content looks like — drives filter badges in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Detected {
    Plain,
    Url,
    Email,
    Color,
    Code,
    /// File path(s) copied e.g. from the system file manager.
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: Uuid,
    pub kind: ItemKind,
    /// Full text for text items; None for images.
    pub text: Option<String>,
    /// File name of the encrypted image blob (relative to the store dir).
    pub image_file: Option<String>,
    /// One-line preview ("1280 × 800 px" for images).
    pub preview: String,
    pub chars: usize,
    pub detected: Detected,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub last_copied_at: DateTime<Utc>,
    pub times_copied: u32,
    /// Content hash for dedupe.
    pub hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Uuid,
    pub name: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    All,
    Pinned,
    Text,
    Links,
    Images,
    Colors,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Store {
    /// Newest first.
    pub items: Vec<ClipItem>,
    pub snippets: Vec<Snippet>,
    /// Paste-stack: item ids in pop order (front = next).
    #[serde(default)]
    pub stack: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddOutcome {
    pub id: Uuid,
    /// True if the content was already at hand and only moved to the top.
    pub deduped: bool,
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn single_token(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(|c| c.is_whitespace())
}

pub fn detect(text: &str) -> Detected {
    let t = text.trim();
    // functional color notations may contain spaces: rgb(56, 189, 248)
    if !t.contains('\n') && t.ends_with(')') {
        let lower = t.to_lowercase();
        if ["rgb(", "rgba(", "hsl(", "hsla("]
            .iter()
            .any(|p| lower.starts_with(p))
        {
            return Detected::Color;
        }
    }
    if single_token(t) {
        let lower = t.to_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.")
        {
            return Detected::Url;
        }
        if let Some(rest) = t.strip_prefix('#') {
            let l = rest.len();
            if (l == 3 || l == 6 || l == 8) && rest.chars().all(|c| c.is_ascii_hexdigit()) {
                return Detected::Color;
            }
        }
        if let Some((local, domain)) = t.split_once('@') {
            if !local.is_empty() && domain.contains('.') && !domain.ends_with('.') {
                return Detected::Email;
            }
        }
    }
    if t.lines().count() > 1 {
        let codey = t.contains('{') && t.contains('}')
            || t.lines().filter(|l| l.trim_end().ends_with(';')).count() >= 2
            || t.contains("</")
            || t.lines().any(|l| {
                let l = l.trim_start();
                l.starts_with("fn ")
                    || l.starts_with("def ")
                    || l.starts_with("const ")
                    || l.starts_with("import ")
                    || l.starts_with("#include")
            });
        if codey {
            return Detected::Code;
        }
    }
    Detected::Plain
}

pub fn make_preview(text: &str) -> String {
    let one_line: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut p: String = one_line.chars().take(200).collect();
    if one_line.chars().count() > 200 {
        p.push('…');
    }
    p
}

impl Store {
    pub fn add_text(&mut self, text: &str) -> AddOutcome {
        self.add_text_as(text, detect(text))
    }

    /// Like `add_text`, but with the type set by the caller — used for
    /// content whose type the caller knows better (e.g. copied file paths).
    pub fn add_text_as(&mut self, text: &str, detected: Detected) -> AddOutcome {
        let hash = fnv1a(text.as_bytes());
        let now = Utc::now();
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| i.kind == ItemKind::Text && i.hash == hash)
        {
            let mut item = self.items.remove(pos);
            item.last_copied_at = now;
            item.times_copied += 1;
            let id = item.id;
            self.items.insert(0, item);
            return AddOutcome { id, deduped: true };
        }
        let item = ClipItem {
            id: Uuid::new_v4(),
            kind: ItemKind::Text,
            text: Some(text.to_string()),
            image_file: None,
            preview: make_preview(text),
            chars: text.chars().count(),
            detected,
            pinned: false,
            created_at: now,
            last_copied_at: now,
            times_copied: 1,
            hash,
        };
        let id = item.id;
        self.items.insert(0, item);
        AddOutcome { id, deduped: false }
    }

    pub fn add_image(
        &mut self,
        image_file: &str,
        hash: u64,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> AddOutcome {
        let now = Utc::now();
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| i.kind == ItemKind::Image && i.hash == hash)
        {
            let mut item = self.items.remove(pos);
            item.last_copied_at = now;
            item.times_copied += 1;
            let id = item.id;
            self.items.insert(0, item);
            return AddOutcome { id, deduped: true };
        }
        let item = ClipItem {
            id: Uuid::new_v4(),
            kind: ItemKind::Image,
            text: None,
            image_file: Some(image_file.to_string()),
            preview: match label {
                Some(name) => format!("{name} — {width} × {height} px"),
                None => format!("{width} × {height} px"),
            },
            chars: 0,
            detected: Detected::Plain,
            pinned: false,
            created_at: now,
            last_copied_at: now,
            times_copied: 1,
            hash,
        };
        let id = item.id;
        self.items.insert(0, item);
        AddOutcome { id, deduped: false }
    }

    pub fn get(&self, id: Uuid) -> Option<&ClipItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn touch(&mut self, id: Uuid) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            let mut item = self.items.remove(pos);
            item.last_copied_at = Utc::now();
            item.times_copied += 1;
            self.items.insert(0, item);
        }
    }

    pub fn set_pinned(&mut self, id: Uuid, pinned: bool) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.pinned = pinned;
        }
    }

    /// Removes the item; returns the image blob file name to delete, if any.
    pub fn delete(&mut self, id: Uuid) -> Option<String> {
        let pos = self.items.iter().position(|i| i.id == id)?;
        let item = self.items.remove(pos);
        self.stack.retain(|s| *s != id);
        item.image_file
    }

    /// Clears the history; returns image blob file names to delete.
    pub fn clear(&mut self, keep_pinned: bool) -> Vec<String> {
        let mut removed_files = Vec::new();
        self.items.retain(|i| {
            if keep_pinned && i.pinned {
                true
            } else {
                if let Some(f) = &i.image_file {
                    removed_files.push(f.clone());
                }
                false
            }
        });
        let ids: Vec<Uuid> = self.items.iter().map(|i| i.id).collect();
        self.stack.retain(|s| ids.contains(s));
        removed_files
    }

    /// Evicts the oldest unpinned items above `limit`; returns image blobs to delete.
    pub fn enforce_limit(&mut self, limit: usize) -> Vec<String> {
        let mut removed_files = Vec::new();
        while self.items.len() > limit {
            let Some(pos) = self.items.iter().rposition(|i| !i.pinned) else {
                break; // everything pinned — keep it all
            };
            let item = self.items.remove(pos);
            self.stack.retain(|s| *s != item.id);
            if let Some(f) = item.image_file {
                removed_files.push(f);
            }
        }
        removed_files
    }

    pub fn search(&self, query: &str, filter: Filter) -> Vec<&ClipItem> {
        let q = query.trim().to_lowercase();
        self.items
            .iter()
            .filter(|i| match filter {
                Filter::All => true,
                Filter::Pinned => i.pinned,
                Filter::Text => i.kind == ItemKind::Text,
                Filter::Images => i.kind == ItemKind::Image,
                Filter::Links => i.detected == Detected::Url,
                Filter::Colors => i.detected == Detected::Color,
            })
            .filter(|i| {
                q.is_empty()
                    || i.text
                        .as_deref()
                        .map(|t| t.to_lowercase().contains(&q))
                        .unwrap_or(false)
                    // images have no text — match the preview (file name, dimensions)
                    || i.preview.to_lowercase().contains(&q)
            })
            .collect()
    }

    // --- snippets ---

    pub fn save_snippet(&mut self, id: Option<Uuid>, name: &str, text: &str) -> Uuid {
        if let Some(id) = id {
            if let Some(s) = self.snippets.iter_mut().find(|s| s.id == id) {
                s.name = name.to_string();
                s.text = text.to_string();
                return id;
            }
        }
        let s = Snippet {
            id: Uuid::new_v4(),
            name: name.to_string(),
            text: text.to_string(),
            created_at: Utc::now(),
        };
        let id = s.id;
        self.snippets.push(s);
        self.snippets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        id
    }

    pub fn delete_snippet(&mut self, id: Uuid) {
        self.snippets.retain(|s| s.id != id);
    }

    // --- paste stack ---

    pub fn stack_push(&mut self, id: Uuid) {
        if self.get(id).is_some() && !self.stack.contains(&id) {
            self.stack.push(id);
        }
    }

    /// Next item to paste, without removing it.
    pub fn stack_peek(&self) -> Option<Uuid> {
        self.stack.first().copied()
    }

    pub fn stack_pop(&mut self) -> Option<Uuid> {
        if self.stack.is_empty() {
            None
        } else {
            Some(self.stack.remove(0))
        }
    }

    pub fn stack_remove(&mut self, id: Uuid) {
        self.stack.retain(|s| *s != id);
    }

    pub fn stack_clear(&mut self) {
        self.stack.clear();
    }
}

/// Fresh random 256-bit key for the history store.
pub fn generate_key() -> [u8; 32] {
    use aes_gcm::aead::rand_core::RngCore;
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

// --- encrypted persistence ---

fn cipher(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key))
}

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher(key)
        .encrypt(&nonce, plaintext)
        .map_err(|_| "encryption failed".to_string())?;
    let mut out = Vec::with_capacity(STORE_MAGIC.len() + 12 + ct.len());
    out.extend_from_slice(STORE_MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < STORE_MAGIC.len() + 12 || &data[..STORE_MAGIC.len()] != STORE_MAGIC {
        return Err("not a clipon store file".into());
    }
    let nonce_start = STORE_MAGIC.len();
    let nonce = Nonce::from_slice(&data[nonce_start..nonce_start + 12]);
    cipher(key)
        .decrypt(nonce, &data[nonce_start + 12..])
        .map_err(|_| "decryption failed (wrong key or corrupted file)".to_string())
}

pub fn save_store(store: &Store, path: &Path, key: &[u8; 32]) -> Result<(), String> {
    let json = serde_json::to_vec(store).map_err(|e| e.to_string())?;
    let blob = encrypt(key, &json)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &blob).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_store(path: &Path, key: &[u8; 32]) -> Result<Store, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let json = decrypt(key, &data)?;
    serde_json::from_slice(&json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("clipon-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn dedupe_moves_to_top_and_counts() {
        let mut s = Store::default();
        s.add_text("first");
        s.add_text("second");
        let out = s.add_text("first");
        assert!(out.deduped);
        assert_eq!(s.items.len(), 2);
        assert_eq!(s.items[0].text.as_deref(), Some("first"));
        assert_eq!(s.items[0].times_copied, 2);
    }

    #[test]
    fn limit_evicts_oldest_unpinned() {
        let mut s = Store::default();
        for i in 0..5 {
            s.add_text(&format!("item {i}"));
        }
        // "item 0" is oldest; pin it
        let oldest = s.items.last().unwrap().id;
        s.set_pinned(oldest, true);
        s.enforce_limit(3);
        assert_eq!(s.items.len(), 3);
        assert!(s.items.iter().any(|i| i.text.as_deref() == Some("item 0")));
        assert!(s.items.iter().any(|i| i.text.as_deref() == Some("item 4")));
        assert!(!s.items.iter().any(|i| i.text.as_deref() == Some("item 1")));
    }

    #[test]
    fn all_pinned_survives_limit() {
        let mut s = Store::default();
        for i in 0..3 {
            let out = s.add_text(&format!("p{i}"));
            s.set_pinned(out.id, true);
        }
        s.enforce_limit(1);
        assert_eq!(s.items.len(), 3);
    }

    #[test]
    fn clear_keeps_pinned() {
        let mut s = Store::default();
        let a = s.add_text("keep me");
        s.add_text("drop me");
        s.set_pinned(a.id, true);
        s.clear(true);
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].text.as_deref(), Some("keep me"));
    }

    #[test]
    fn detection() {
        assert_eq!(detect("https://lan-solo.de/tools"), Detected::Url);
        assert_eq!(detect("www.example.com"), Detected::Url);
        assert_eq!(detect("input@lan-solo.com"), Detected::Email);
        assert_eq!(detect("#38bdf8"), Detected::Color);
        assert_eq!(detect("#fff"), Detected::Color);
        assert_eq!(detect("rgb(56, 189, 248)"), Detected::Color);
        assert_eq!(detect("rgb(56,189,248)"), Detected::Color);
        assert_eq!(detect("rgba(56, 189, 248, 0.5)"), Detected::Color);
        assert_eq!(detect("hsl(199, 89%, 60%)"), Detected::Color);
        assert_eq!(detect("rgb( but no close paren"), Detected::Plain);
        assert_eq!(detect("fn main() {\n    println!(\"hi\");\n}"), Detected::Code);
        assert_eq!(detect("hello world"), Detected::Plain);
        assert_eq!(detect("#nothex"), Detected::Plain);
    }

    #[test]
    fn search_and_filters() {
        let mut s = Store::default();
        s.add_text("https://lan-solo.de");
        s.add_text("plain note about rust");
        s.add_image("img1.bin", 42, 800, 600, Some("Screenshot.png"));
        assert_eq!(s.search("", Filter::All).len(), 3);
        assert_eq!(s.search("", Filter::Links).len(), 1);
        assert_eq!(s.search("", Filter::Images).len(), 1);
        assert_eq!(s.search("rust", Filter::All).len(), 1);
        assert_eq!(s.search("RUST", Filter::Text).len(), 1);
        assert_eq!(s.search("nada", Filter::All).len(), 0);
        // images are searchable by their preview (file name)
        assert_eq!(s.search("screenshot", Filter::Images).len(), 1);
    }

    #[test]
    fn image_label_in_preview() {
        let mut s = Store::default();
        s.add_image("a.bin", 1, 800, 600, Some("Foto.jpg"));
        s.add_image("b.bin", 2, 800, 600, None);
        assert_eq!(s.items[1].preview, "Foto.jpg — 800 × 600 px");
        assert_eq!(s.items[0].preview, "800 × 600 px");
    }

    #[test]
    fn file_paths_as_typed_text() {
        let mut s = Store::default();
        s.add_text_as("/tmp/report.pdf", Detected::File);
        assert_eq!(s.items[0].detected, Detected::File);
        // still a text item → shows up under the Text filter and in search
        assert_eq!(s.search("report", Filter::Text).len(), 1);
    }

    #[test]
    fn stack_order_and_cleanup() {
        let mut s = Store::default();
        let a = s.add_text("a").id;
        let b = s.add_text("b").id;
        s.stack_push(a);
        s.stack_push(b);
        s.stack_push(a); // duplicate ignored
        assert_eq!(s.stack.len(), 2);
        assert_eq!(s.stack_peek(), Some(a));
        s.delete(a);
        assert_eq!(s.stack_peek(), Some(b));
        assert_eq!(s.stack_pop(), Some(b));
        assert_eq!(s.stack_pop(), None);
    }

    #[test]
    fn snippets_sorted_and_updatable() {
        let mut s = Store::default();
        let id = s.save_snippet(None, "Zebra", "zzz");
        s.save_snippet(None, "Alpha", "aaa");
        assert_eq!(s.snippets[0].name, "Alpha");
        s.save_snippet(Some(id), "Zebra 2", "zzz2");
        assert_eq!(s.snippets.iter().find(|x| x.id == id).unwrap().text, "zzz2");
        s.delete_snippet(id);
        assert_eq!(s.snippets.len(), 1);
    }

    #[test]
    fn encrypted_roundtrip_and_wrong_key() {
        let mut s = Store::default();
        s.add_text("geheim aber nur so mittel");
        let path = tmp_path("store.clipon");
        let key = [7u8; 32];
        save_store(&s, &path, &key).unwrap();
        // ciphertext must not contain the plaintext
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw
            .windows(6)
            .any(|w| w == b"geheim"));
        let loaded = load_store(&path, &key).unwrap();
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].text.as_deref(), Some("geheim aber nur so mittel"));
        let wrong = [8u8; 32];
        assert!(load_store(&path, &wrong).is_err());
    }

    #[test]
    fn preview_is_single_line_and_capped() {
        let p = make_preview("line one\nline two\n   spaced");
        assert_eq!(p, "line one line two spaced");
        let long = "x".repeat(500);
        assert_eq!(make_preview(&long).chars().count(), 201); // 200 + ellipsis
    }
}
