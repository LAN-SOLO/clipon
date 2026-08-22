# clipon. — Entwicklungsplan

Clipboard-Manager: Verlauf, Snippets und Paste-Stack — lokal, verschlüsselt, nativ (Tauri 2 + React).
Website: https://lan-solo.de/de/tools/clipon

## Architektur

- `core/` (`clipon-core`) — reine Rust-Logik, ohne Tauri:
  Datenmodell (`ClipItem`, `Snippet`), Store mit Dedupe/Pins/Suche/Limit,
  Paste-Stack, AES-256-GCM-Persistenz. Vollständig unit-getestet.
- `src-tauri/` — App-Schicht: Clipboard-Watcher (Polling, 400 ms),
  Keychain-Schlüssel (`keyring`), Tray, globale Shortcuts, Commands, Updater.
- `src/` — React-UI: Verlauf mit Filtern/Suche/Detail, Stack, Snippets,
  Einstellungen. i18n DE/EN, dunkles LAN-SOLO-Theme.

## Datenablage

- Verlauf: `~/Library/Application Support/com.lan-solo.clipon/history.clipon`
  — JSON, AES-256-GCM-verschlüsselt (Magic `CLIPON1\n` + Nonce + Ciphertext).
- Bilder: einzeln verschlüsselte Blobs unter `blobs/<uuid>.bin`.
- Schlüssel: macOS-Schlüsselbund (Service `com.lan-solo.clipon`), Fallback `history.key`.
- Einstellungen (unkritisch): `settings.json` im Config-Verzeichnis.

## Phasen

- [x] **Phase 0 — Gerüst:** Tauri-2-Workspace nach keypile-Vorbild, Icons aus Website-SVG,
  Updater-Schlüsselpaar (`~/.tauri/clipon-updater.key`).
- [x] **Phase 1 — Core:** Store, Dedupe (FNV-Hash), Erkennung (URL/E-Mail/Farbe/Code),
  Pins, Limit-Eviction (Pins überleben), Suche/Filter, Snippets, Stack,
  verschlüsselte Persistenz. 10 Unit-Tests.
- [x] **Phase 2 — Watcher & Commands:** Polling-Watcher (Text + Bilder, eigener
  Kopien-Unterdrückung), alle CRUD-Commands, Paste-Stack-Pop per Command + Hotkey.
- [x] **Phase 3 — UI:** Sidebar-Filter, Liste mit Badges/Thumbnails/Farb-Swatches,
  Detail-Pane mit Statistiken, Stack- und Snippet-Ansicht, Einstellungen, Toasts.
- [x] **Phase 4 — System-Integration:** Tray (Öffnen/Pause/Beenden), globale Shortcuts
  (Fenster-Toggle, Stack-Pop), Autostart, Fenster-Schließen = Verstecken,
  Dock-Klick öffnet wieder (macOS Reopen).
- [x] **Phase 5 — Updater:** signierte In-App-Updates (tauri-plugin-updater),
  Endpoint `github.com/LAN-SOLO/clipon/releases/latest/download/latest.json`.
- [x] **Phase 6 — v0.3.0:** Logo in Sidebar & Hilfe, Farb-Studio (Erkennung auch
  für `rgb()/hsl()` mit Leerzeichen, System-Farbwähler mit Pipette, Hex/RGB/HSL-
  Umrechnung im Detail), Bilddateien aus dem Finder als Bild-Einträge (andere
  Dateien als Pfad mit `file`-Badge, Suche findet Bilder per Dateiname),
  Filter-Leerzustände mit Erklärung, interaktive UI-Tour, Hilfe-Knopf in der Sidebar.
- [ ] **Phase 7 — später:** Regeln pro Quell-App, Overlay am Cursor, direktes
  Einfügen (Bedienungshilfen), clipon pinned (12 €/Jahr).

## Shortcuts (Standard)

- `Cmd/Ctrl+Shift+V` — Fenster ein-/ausblenden
- `Cmd/Ctrl+Shift+B` — Paste-Stack: nächsten Eintrag kopieren

## Build

- `pnpm install` · `pnpm tauri dev`
- Release: `TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/clipon-updater.key \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm tauri build --bundles app,dmg`
