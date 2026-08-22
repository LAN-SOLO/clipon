# clipon.

Clipboard-Manager: Verlauf, Snippets und Paste-Stack — lokal, verschlüsselt, nativ.

- Merkt sich Text **und Bilder**, dedupliziert, mit Pins, Suche und Typ-Filtern
  (Links, Farben, Code, E-Mail, Dateien — automatisch erkannt).
- **Farben:** kopierte Farbwerte (`#hex`, `rgb()`, `hsl()`) mit Farbfeld,
  Umrechnung in alle Formate im Detail — plus System-Farbwähler mit Pipette.
- **Bilder & Dateien:** auch im Finder kopierte Bilddateien landen als Bild im
  Verlauf (Suche per Dateiname); andere Dateien als Pfad-Eintrag.
- **Paste-Stack:** Einträge einsammeln, der Reihe nach zurückholen
  (`Cmd+Shift+B`), z. B. für Formulare.
- **Snippets:** Textbausteine, ein Klick — kopiert.
- **Hilfe eingebaut:** First-Run-Tutorial, interaktive UI-Tour und
  durchsuchbares Handbuch (DE/EN).
- **Privat by design:** Verlauf AES-256-GCM-verschlüsselt auf der Platte,
  Schlüssel im System-Schlüsselbund. Kein Netzwerkzugriff außer Update-Check.
- Tray, globale Shortcuts, Autostart, DE/EN, signierte In-App-Updates.

## Entwicklung

```sh
pnpm install
pnpm tauri dev        # App im Dev-Modus
cargo test -p clipon-core   # Core-Tests
```

Release-Build (macOS, mit Updater-Artefakten):

```sh
TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/clipon-updater.key \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
pnpm tauri build --bundles app,dmg
```

Details: `CLIPON_PLAN.md`.
