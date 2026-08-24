//! Portable backups: history + snippets as JSON, optionally encrypted with a
//! passphrase (PBKDF2-HMAC-SHA256 → AES-256-GCM). Unlike the store, an export
//! never depends on the device keychain, so it can be restored anywhere.

use crate::{ClipItem, Detected, ItemKind, Snippet, SourceApp, Store};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Layout: magic + salt (16) + nonce (12) + ciphertext.
pub const EXPORT_MAGIC: &[u8; 8] = b"CLIPONX1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KDF_ROUNDS: u32 = 600_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    #[serde(default)]
    pub items: Vec<ExportItem>,
    #[serde(default)]
    pub snippets: Vec<Snippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportItem {
    pub kind: ItemKind,
    pub text: Option<String>,
    /// PNG bytes, base64 — decoded and encrypted by the app on import.
    pub image_png_b64: Option<String>,
    pub preview: String,
    pub chars: usize,
    pub detected: Detected,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub last_copied_at: DateTime<Utc>,
    pub times_copied: u32,
    pub hash: u64,
    #[serde(default)]
    pub source_app: Option<SourceApp>,
}

impl ExportItem {
    pub fn from_item(item: &ClipItem, image_png_b64: Option<String>) -> Self {
        Self {
            kind: item.kind,
            text: item.text.clone(),
            image_png_b64,
            preview: item.preview.clone(),
            chars: item.chars,
            detected: item.detected,
            pinned: item.pinned,
            created_at: item.created_at,
            last_copied_at: item.last_copied_at,
            times_copied: item.times_copied,
            hash: item.hash,
            source_app: item.source_app.clone(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStats {
    pub items_added: usize,
    pub items_skipped: usize,
    pub snippets_added: usize,
    pub snippets_skipped: usize,
}

pub fn is_encrypted(data: &[u8]) -> bool {
    data.starts_with(EXPORT_MAGIC)
}

pub fn derive_export_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, KDF_ROUNDS, &mut key);
    key
}

pub fn encrypt_export(passphrase: &str, json: &[u8]) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_export_key(passphrase, &salt);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key))
        .encrypt(&nonce, json)
        .map_err(|_| "encryption failed".to_string())?;
    let mut out = Vec::with_capacity(EXPORT_MAGIC.len() + SALT_LEN + NONCE_LEN + ct.len());
    out.extend_from_slice(EXPORT_MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt_export(passphrase: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let header = EXPORT_MAGIC.len() + SALT_LEN + NONCE_LEN;
    if data.len() < header || !is_encrypted(data) {
        return Err("not an encrypted clipon export".into());
    }
    let salt = &data[EXPORT_MAGIC.len()..EXPORT_MAGIC.len() + SALT_LEN];
    let nonce = Nonce::from_slice(&data[EXPORT_MAGIC.len() + SALT_LEN..header]);
    let key = derive_export_key(passphrase, salt);
    Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key))
        .decrypt(nonce, &data[header..])
        .map_err(|_| "wrong passphrase or corrupted file".to_string())
}

impl Store {
    pub fn to_bundle(
        &self,
        include_history: bool,
        include_snippets: bool,
        mut load_image_b64: impl FnMut(&str) -> Option<String>,
    ) -> ExportBundle {
        let items = if include_history {
            self.items
                .iter()
                .filter_map(|i| {
                    let b64 = match (&i.kind, &i.image_file) {
                        (ItemKind::Image, Some(f)) => Some(load_image_b64(f)?),
                        _ => None,
                    };
                    Some(ExportItem::from_item(i, b64))
                })
                .collect()
        } else {
            Vec::new()
        };
        ExportBundle {
            version: 1,
            exported_at: Utc::now(),
            items,
            snippets: if include_snippets {
                self.snippets.clone()
            } else {
                Vec::new()
            },
        }
    }

    /// Merges a bundle into the store. Items are deduped by (kind, hash),
    /// snippets by (name, text). `save_image` receives the base64 PNG and
    /// returns the stored blob file name; None skips the item.
    pub fn merge_bundle(
        &mut self,
        bundle: ExportBundle,
        mut save_image: impl FnMut(&str) -> Option<String>,
    ) -> ImportStats {
        let mut stats = ImportStats::default();
        for e in bundle.items {
            if self
                .items
                .iter()
                .any(|i| i.kind == e.kind && i.hash == e.hash)
            {
                stats.items_skipped += 1;
                continue;
            }
            let image_file = match e.kind {
                ItemKind::Image => match e.image_png_b64.as_deref().and_then(&mut save_image) {
                    Some(f) => Some(f),
                    None => {
                        stats.items_skipped += 1;
                        continue;
                    }
                },
                ItemKind::Text => None,
            };
            self.items.push(ClipItem {
                id: Uuid::new_v4(),
                kind: e.kind,
                text: e.text,
                image_file,
                preview: e.preview,
                chars: e.chars,
                detected: e.detected,
                pinned: e.pinned,
                created_at: e.created_at,
                last_copied_at: e.last_copied_at,
                times_copied: e.times_copied,
                hash: e.hash,
                source_app: e.source_app,
            });
            stats.items_added += 1;
        }
        self.items
            .sort_by(|a, b| b.last_copied_at.cmp(&a.last_copied_at));

        for s in bundle.snippets {
            if self
                .snippets
                .iter()
                .any(|x| x.name == s.name && x.text == s.text)
            {
                stats.snippets_skipped += 1;
                continue;
            }
            self.snippets.push(Snippet {
                id: Uuid::new_v4(),
                ..s
            });
            stats.snippets_added += 1;
        }
        self.snippets
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Store {
        let mut s = Store::default();
        s.add_text("older");
        s.add_text("newer");
        s.add_image("img.bin", 77, 10, 10, None, None);
        s.save_snippet(None, "Sig", "-- me");
        s
    }

    #[test]
    fn plain_roundtrip_preserves_order_and_flags() {
        let mut src = sample();
        let pinned = src.items[1].id;
        src.set_pinned(pinned, true);
        let bundle = src.to_bundle(true, true, |_| Some("cGln".into()));
        let json = serde_json::to_vec(&bundle).unwrap();

        let mut dst = Store::default();
        let parsed: ExportBundle = serde_json::from_slice(&json).unwrap();
        let stats = dst.merge_bundle(parsed, |b64| {
            assert_eq!(b64, "cGln");
            Some("new.bin".into())
        });
        assert_eq!(stats, ImportStats { items_added: 3, items_skipped: 0, snippets_added: 1, snippets_skipped: 0 });
        assert_eq!(dst.items[0].kind, ItemKind::Image);
        assert_eq!(dst.items[0].image_file.as_deref(), Some("new.bin"));
        assert_eq!(dst.items[1].text.as_deref(), Some("newer"));
        assert!(dst.items[1].pinned);
        assert_eq!(dst.items[2].text.as_deref(), Some("older"));

        // importing again changes nothing
        let again: ExportBundle = serde_json::from_slice(&json).unwrap();
        let stats = dst.merge_bundle(again, |_| Some("x.bin".into()));
        assert_eq!(stats.items_added + stats.snippets_added, 0);
        assert_eq!(stats.items_skipped, 3);
        assert_eq!(stats.snippets_skipped, 1);
    }

    #[test]
    fn image_without_blob_is_skipped() {
        let src = sample();
        let bundle = src.to_bundle(true, false, |_| None);
        assert_eq!(bundle.items.len(), 2);
        assert!(bundle.snippets.is_empty());
        let mut dst = Store::default();
        let stats = dst.merge_bundle(bundle, |_| None);
        assert_eq!(stats.items_added, 2);
    }

    #[test]
    fn encrypted_roundtrip_and_wrong_passphrase() {
        let payload = b"{\"version\":1,\"exported_at\":\"2026-01-01T00:00:00Z\",\"items\":[],\"snippets\":[]} secret";
        let blob = encrypt_export("pass", payload).unwrap();
        assert!(is_encrypted(&blob));
        assert!(!blob.windows(6).any(|w| w == b"secret"));
        assert_eq!(decrypt_export("pass", &blob).unwrap(), payload);
        assert!(decrypt_export("nope", &blob).is_err());
        assert!(decrypt_export("pass", b"garbage").is_err());
        assert!(!is_encrypted(b"{\"version\":1}"));
    }
}
