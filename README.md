# clipon.

Clipboard-Manager: Verlauf, Snippets und Paste-Stack — lokal, verschlüsselt, nativ.

- Merkt sich Text **und Bilder**, dedupliziert, mit Pins, Suche und Typ-Filtern
  (Links, Farben, Code, E-Mail, Dateien — automatisch erkannt). Die Verlaufsgröße
  zählt nur unangepinnte Einträge; Pins kommen dazu und werden nie aufgeräumt.
  Vorschaubilder laden erst beim Hineinscrollen (max. 3 gleichzeitig).
- **Farben:** kopierte Farbwerte (`#hex`, `rgb()`, `hsl()`) mit Farbfeld,
  Umrechnung in alle Formate im Detail — plus System-Farbwähler mit Pipette.
- **Bilder & Dateien:** auch im Finder kopierte Bilddateien landen als Bild im
  Verlauf (Suche per Dateiname); andere Dateien als Pfad-Eintrag.
- **Paste-Stack:** Einträge einsammeln, der Reihe nach zurückholen
  (`Cmd/Strg+Shift+B`), z. B. für Formulare. Ein Eintrag verlässt den Stack
  erst nach erfolgreichem Kopieren; nicht mehr kopierbare (Bilddatei fehlt)
  werden mit Meldung übersprungen.
- **Snippets:** Textbausteine, ein Klick — kopiert.
- **Hilfe eingebaut:** First-Run-Tutorial, interaktive UI-Tour und
  durchsuchbares Handbuch (DE/EN).
- **Privat by design:** Verlauf AES-256-GCM-verschlüsselt auf der Platte,
  Schlüssel im System-Schlüsselbund. Kein Netzwerkzugriff außer Update-Check.
- **Quick-Picker** (`Cmd/Strg+Shift+Space`): Suchfenster am Mauszeiger, fügt
  direkt in die vorherige App ein (macOS mit Bedienungshilfen-Freigabe,
  Windows ohne Freigabe).
- **Passwort-Manager werden übersprungen:** macOS `org.nspasteboard.ConcealedType`
  & Co., Windows `ExcludeClipboardContentFromMonitorProcessing` /
  `CanIncludeInClipboardHistory = 0`; dazu eine Ignorier-Liste pro App
  (Bundle-ID bzw. Programmdatei).
- Werkzeuge (Trim, Groß/klein, JSON, URL-Kodierung), Snippet-Platzhalter,
  Backup-Export/-Import (optional mit Passphrase verschlüsselt).
- Tray, globale Shortcuts, Autostart, DE/EN, Hell/Dunkel, signierte In-App-Updates.
- Läuft nur einmal pro Benutzer; ein zweiter Start holt das Fenster nach vorn.
- Fehler erscheinen immer in der UI-Sprache (das Backend liefert nur Codes,
  `src/errors.ts` übersetzt); Fehler aus dem Hintergrund (Watcher, Tray,
  globale Kürzel) bei verstecktem Fenster werden beim nächsten Öffnen erneut
  angezeigt (kein Notification-Plugin, daher keine Systemmitteilung); Kürzel stehen da, wie sie auf der Tastatur
  heißen (⌘ auf dem Mac, Strg/Ctrl unter Windows).

Version 1.0.0 — Audit-Stand (Runde 1–3), bekannte Grenzen und manuelle
Testliste: `AUDIT-1.0.md`.

## Entwicklung

```sh
pnpm install
pnpm tauri dev        # App im Dev-Modus
cargo test --workspace      # Core- und App-Tests
pnpm build            # Frontend (tsc + vite)
```

Hinweis: Dev-Build und installierte App teilen Datenordner und Einzelinstanz-
Sperre — vor `pnpm tauri dev` die installierte clipon beenden.

Release-Build (macOS, mit Updater-Artefakten):

```sh
TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/clipon-updater.key \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
pnpm tauri build --bundles app,dmg
```

Details: `CLIPON_PLAN.md`.
