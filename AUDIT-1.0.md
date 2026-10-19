# clipon. 1.0.0 — Audit

Stand: 2026-10-19 · Basis `32da487` · geprüft auf macOS (arm64), Windows per
Cross-Typecheck und Code-Review.

Legende Teststatus:

- **auto** — durch automatisierte Tests abgedeckt (`cargo test --workspace`)
- **ui** — in der echten Web-Oberfläche mit gemocktem Backend durchgeklickt
  (headless Chrome, 30 Szenarien, siehe unten)
- **statisch** — nur Code-Review / Typecheck; braucht einen Menschen am Gerät
- **manuell offen** — steht in der Testliste unten

## 1. Feature-Inventar

| Bereich | Feature | Status |
|---|---|---|
| Verlauf | Text- und Bildaufnahme per Watcher (400 ms), Dedupe (Eintrag rückt nach oben, Zähler) | auto (Core), Watcher statisch |
| Verlauf | Typ-Erkennung url / @ / col / { } / file / txt | auto + ui |
| Verlauf | Klick = Details, Doppelklick = kopieren, Zeilen-Aktionen (Kopieren, Pin, Stack, Löschen) | ui |
| Verlauf | Verlaufslimit (Pins zählen nicht), „Verlauf leeren“ mit „Pins behalten“ | auto + ui |
| Verlauf | Quell-App (macOS Vordergrund-App, Windows Zwischenablage-Besitzer) | macOS statisch, Windows statisch |
| Filter & Suche | Alles/Pins/Text/Links/Farben/Bilder, Volltextsuche inkl. Bild-Dateiname, Leer-Zustände pro Filter | auto + ui |
| Farben | Erkennung hex/rgb()/hsl(), Farbfeld, Umrechnung Hex/RGB/HSL kopieren, Farbwähler „Merken & kopieren“ | auto (Erkennung) + ui |
| Bilder & Dateien | Bitmap-Kopien, Bilddateien aus Finder/Explorer (≤ 64 MB, max. 20 pro Kopie), andere Dateien als Pfad | statisch (braucht echte Zwischenablage) |
| Werkzeuge | Trim, lower, UPPER, Title, Zeilenumbrüche, JSON formatieren, URL en-/decode | auto + ui (Fehler-Toast) |
| Paste-Stack | Hinzufügen, Reihenfolge, Nächsten kopieren (Button, In-App-Kürzel, global Cmd/Ctrl+Shift+B), Entfernen, Leeren, Zusammenführen | auto + ui; globales Kürzel manuell offen |
| Snippets | Anlegen, Bearbeiten, Löschen, Klick kopiert, Platzhalter {date} {date:FMT} {time} {clipboard} | auto (Platzhalter) + ui |
| Quick-Picker | Fenster am Mauszeiger, Tippen filtert, ↑↓, ↩/Klick, 1–9, Esc/Fokusverlust schließt | ui (Fenster-Logik), Direkt-Einfügen manuell offen |
| Quick-Picker | Direkt-Einfügen macOS (AX-Freigabe + CGEvent ⌘V) | statisch, manuell offen |
| Quick-Picker | Direkt-Einfügen Windows (SetForegroundWindow + SendInput Strg+V) — **neu in 1.0** | Cross-Typecheck, manuell offen |
| Einstellungen | Sprache, Hell/Dunkel, Verlaufsgröße, KB-Limit, Bilder, Pins behalten, Autostart | ui (Speichern/Validierung) + auto (Server-Validierung) |
| Einstellungen | Ignorier-Liste: laufende App hinzufügen, manuell, entfernen | ui; native App-Liste statisch |
| Einstellungen | Kürzel-Editor (3 global + 23 In-App), Konflikterkennung, ↺, Alle zurücksetzen | ui |
| Einstellungen | Direktes Einfügen (nur Mac): Status, anfordern, Systemeinstellungen öffnen | statisch |
| Daten | Einzel-Export .txt/.png, Backup-Export JSON / verschlüsselt (CLIPONX1), Import mit Dedupe | auto (Format, Krypto, Import-Bereinigung) + ui (Dialog-Fluss) |
| Updates | Prüfung beim Start (Banner), manuell in den Einstellungen, signierte Installation | ui (Banner/Buttons); echte Installation manuell offen |
| Tray | Öffnen, Pause (synchron mit UI), Beenden; Beschriftung folgt der Sprache | statisch, manuell offen |
| Fenster | Schließen = verstecken, Dock-Klick (macOS), globales Ein-/Ausblenden | statisch, manuell offen |
| Einzelinstanz | zweiter Start holt Fenster nach vorn — **neu in 1.0** | statisch, manuell offen |
| Hilfe | First-Run-Tutorial (9 Schritte), UI-Tour, durchsuchbares Handbuch, DE/EN-Umschalter, F1 | ui |
| Darstellung | Dunkel/Hell, Quick-Picker zieht mit | ui (Screenshots geprüft) |
| Kürzel | In-App-Dispatch, Tippschutz in Eingabefeldern | ui |

UI-Szenarien (alle bestanden): Start/Badges, Auswahl+Kopieren, Doppelklick,
Filter, Suche, Farb-Umrechnung, Farbwähler, Werkzeuge inkl. JSON-Fehler, Pins,
Stack (Zusammenführen, Nächsten kopieren), Löschen aktualisiert Stack, Snippets
CRUD, Snippet-Validierung, Zahlenfelder, Limit-Klemmung, Pause bleibt beim
Speichern erhalten, Ignorier-Liste, Kürzel-Editor mit Konflikt und Reset,
Tastatur-Navigation, Tippschutz, Einzel-Export, Backup-Export/-Import mit
Passphrase-Hinweis, Leeren mit Pins, Sprachwechsel EN, Hilfe/F1/Suche/DE-EN,
Tutorial+Tour, Hell-Modus, Update-Banner, Versionsanzeige 1.0.0, Quick-Picker
(Filter, Enter, Ziffer, Esc, Hinweistext je nach Freigabe).

Zusätzlich: IPC-Vertragsprüfung (Skript) — alle 33 `invoke`-Aufrufe in
`src/api.ts` passen in Name und Argumentschlüsseln zu den Rust-Commands, alle
Commands sind registriert.

## 2. Tests & Prüfungen

| Prüfung | Ergebnis |
|---|---|
| `cargo test --workspace` | 27 Core-Tests + 6 App-Tests, alle grün |
| `pnpm build` (tsc + vite) | fehlerfrei |
| `cargo clippy --workspace --tests` | nur Stil-Hinweise (`sort_by_key`), keine Fehler |
| `cargo check --target x86_64-pc-windows-msvc --workspace --tests` | fehlerfrei, ohne Warnungen (vom Mac aus, C-Teile von `ring` nur mit Stub-Headern übersetzt — reiner Typecheck, kein Link) |
| `pnpm audit` | 1 high in `source-map-js` (nur Build-Tooling) → auf 1.2.2 gehoben, jetzt 0 |
| `cargo audit` | nicht installiert — nicht gelaufen; Versionen der Kern-Crates manuell gesichtet (tauri 2.11.5, rustls 0.23.43, ring 0.17.14, image 0.25.10, keyring 3.6.3), keine offene Lücke bekannt |

## 3. Sicherheitsbefunde

| Schwere | Befund | Status |
|---|---|---|
| Hoch | Jeder Schlüsselbund-Fehler außer „gefunden“ (gesperrter Schlüsselbund nach Login, verweigerter Zugriff) erzeugte einen **neuen** Schlüssel und überschrieb den gespeicherten — Verlauf dauerhaft unlesbar, beim nächsten Speichern überschrieben | behoben: neuer Schlüssel nur bei `NoEntry`, Wiederholung beim Start, Fallback-Datei |
| Hoch | Nicht entschlüsselbarer Verlauf wurde stillschweigend durch einen leeren ersetzt | behoben: wird als `history.clipon.unreadable-<Zeit>` beiseitegelegt |
| Hoch | Windows: Passwort-Manager-Kopien wurden **aufgezeichnet** (ConcealedType-Prüfung gab es nur auf macOS, Ignorier-Liste griff mangels Quell-App nie) | behoben: `ExcludeClipboardContentFromMonitorProcessing`, `Clipboard Viewer Ignore`, `CanIncludeInClipboardHistory = 0`; Quell-App = Zwischenablage-Besitzer, Liste mit Windows-Passwort-Managern vorbelegt |
| Mittel | Zwei Instanzen (Autostart + manueller Start, v. a. Windows) schrieben parallel in denselben Store → Datenverlust | behoben: Einzelinstanz-Sperre |
| Mittel | `export_item`/`export_data` schrieben an jeden Pfad mit jedem Namen, den die Webview übergab (bei XSS beliebiges Datei-Schreiben, z. B. Shell-Profile) | behoben: Endung wird erzwungen (.txt/.png, .json/.clipon-backup) |
| Mittel | Import übernahm Hash/Vorschau/Typ ungeprüft aus der Datei (Dedupe aushebelbar, Einträge ohne Text) | behoben: wird neu berechnet, Unsinn übersprungen, 1-GB-Limit |
| Niedrig | Capabilities zu breit (`opener:default` = http/https/mailto/tel + Ordner zeigen, `dialog:default`) | behoben: nur dialog open/save und die eine `x-apple.systempreferences`-URL |
| Niedrig | CSP ohne `object-src`/`base-uri`/`form-action`/`frame-src`/`connect-src` | behoben |
| Niedrig | Blob-Dateinamen aus dem Store ungeprüft zu Pfaden zusammengesetzt | behoben: nur `<uuid>.bin`-artige Namen |
| Niedrig | Schlüssel-Fallback-Datei mit Standardrechten | behoben: 0600 (Unix) |
| Niedrig | Einstellungen nicht atomar geschrieben, Webview-Werte ungeprüft | behoben: write+rename, serverseitige Validierung |
| Info | Krypto: AES-256-GCM mit zufälliger 96-bit-Nonce pro Schreibvorgang (Store, Blobs, Export); Export-KDF PBKDF2-HMAC-SHA256 600 000 Runden, 16-Byte-Salt; Tauri-Updater mit Minisign-Pubkey, Endpoint https | in Ordnung |
| Info | Kein `dangerouslySetInnerHTML`, keine Markdown-/HTML-Darstellung, Inhalte nur als Text; Farb-Swatch nutzt den Wert als `style.background` (CSP blockt externe Bilder) | in Ordnung |
| Info | Schlüssel und Passphrasen werden nicht geloggt; Fehlermeldungen enthalten keine Inhalte | in Ordnung |
| Offen (niedrig) | Schlüssel/Passphrase werden nach Gebrauch nicht explizit genullt (`zeroize` wäre neue Abhängigkeit) | bewusst offen |
| Offen (niedrig) | Klartext-JSON-Export ist unverschlüsselt — so dokumentiert und in der UI optional | by design |

## 4. Weitere behobene Fehler

- Stack-Ansicht blieb nach Löschen/Leeren/Import/Verdrängung veraltet (fehlendes `stack-changed`).
- Zahlenfelder „Verlaufsgröße“/„KB-Limit“: jede Eingabe sprang sofort auf 10 — „2000“ ließ sich nicht tippen.
- Speichern der Einstellungen setzte eine zwischenzeitlich (Tray/Kürzel) geänderte Pause zurück.
- Tray-Menü blieb nach Sprachwechsel in der alten Sprache.
- Standardsprache wurde nur aus `LANG` gelesen — aus Finder/Explorer gestartet fast immer Englisch; jetzt System-UI-Sprache.
- Windows: Watcher las ohne Änderungs-Gate alle 400 ms die komplette Zwischenablage (bei Bildern teuer); jetzt Sequenznummer.
- Windows: „Laufende App hinzufügen“ lieferte nichts; Quick-Picker konnte nicht direkt einfügen; Hinweistexte nannten ⌘V.
- Export-Temp-Datei `<name>.tmp` konnte eine gleichnamige Datei des Nutzers überschreiben.
- Snippet-Speicher-/Kopierfehler wurden verschluckt.

## 5. Bekannte Grenzen

- **Linux:** keine Passwort-Manager-Erkennung, keine Quell-App, kein Direkt-Einfügen (Picker kopiert nur); Wayland schränkt globale Kürzel und Zwischenablage ein.
- **Windows:** Direkt-Einfügen in Programme, die als Administrator laufen, ist durch UIPI gesperrt — dort nur kopieren. Erkennung vertraulicher Kopien setzt voraus, dass der Passwort-Manager die Formate setzt (KeePass, KeePassXC, 1Password tun das).
- **macOS:** Direkt-Einfügen braucht die Bedienungshilfen-Freigabe; bei Dev-Builds nach jedem Neubau neu erteilen.
- Kopierte Dateien werden als Pfadtext zurückkopiert, nicht als Datei.
- Quell-App kann bei sehr schnellen App-Wechseln um einen Moment danebenliegen.
- Ein unlesbarer Verlauf (z. B. Schlüsselbund verloren) wird beiseitegelegt, aber nicht automatisch wiederhergestellt.
- Dev-Build und installierte App teilen Datenordner und Einzelinstanz-Sperre.

## 6. Manuelle Testliste

### macOS (DMG aus dem CI-Build, signiert + notarisiert)

- [ ] DMG öffnet ohne Gatekeeper-Warnung, App startet, Fenster nicht schwarz
- [ ] Ersten Start: Tutorial erscheint; Schlüsselbund-Eintrag `com.lan-solo.clipon` / `history-key` angelegt
- [ ] Update von 0.4: Verlauf, Snippets, Einstellungen bleiben erhalten; kein Schlüsselbund-Dialog bzw. „Immer erlauben“ wirkt dauerhaft
- [ ] Text in TextEdit kopieren → erscheint mit Quelle „TextEdit“; erneut kopieren → kein Duplikat, Zähler 2
- [ ] Screenshot in die Zwischenablage (⌘⇧⌃4) → Bild-Eintrag; Bilddatei im Finder kopieren → Bild mit Dateiname; PDF im Finder kopieren → file-Eintrag
- [ ] 1Password/Apple-Passwörter: Passwort kopieren → erscheint **nicht**
- [ ] App zur Ignorier-Liste hinzufügen („Laufende App …“), dort kopieren → erscheint nicht
- [ ] ⌘⇧V blendet ein/aus, ⌘⇧B kopiert nächsten Stack-Eintrag aus einer anderen App
- [ ] ⌘⇧Space in TextEdit: Picker am Mauszeiger, ↩ ohne AX-Freigabe → nur kopiert (Hinweis ⌘V); Freigabe erteilen → ↩ fügt direkt ein
- [ ] Tray: Pause setzt Statuszeile in der App, Sprache auf EN → Tray-Texte englisch
- [ ] Fenster schließen → läuft weiter; Dock-Klick holt Fenster zurück; App aus /Programme erneut öffnen → kein zweites Tray-Symbol
- [ ] Autostart an → nach Ab-/Anmelden läuft clipon, Verlauf lesbar
- [ ] Backup mit Passphrase exportieren, Verlauf leeren, importieren → Einträge zurück, falsche Passphrase → Fehlermeldung
- [ ] Hell-Modus: alle Dialoge (Einstellungen, Snippet-Editor, Hilfe, Picker) lesbar
- [ ] Update-Banner bei verfügbarem Release (nach dem ersten 1.0.x)

### Windows (NSIS-Setup bzw. MSI aus dem CI-Build)

- [ ] Setup installiert, App startet, Hauptfenster **nicht schwarz** (0.4-Befund), Picker öffnet beim ersten Strg+Shift+Space
- [ ] Erststart: Sprache entspricht der Windows-Anzeigesprache; Anmeldeinformationsverwaltung enthält `com.lan-solo.clipon`
- [ ] Text in Notepad kopieren → Eintrag mit Quelle „Notepad“ (bzw. Programmname)
- [ ] Snipping Tool / Druck-Taste → Bild-Eintrag; Bilddatei im Explorer kopieren → Bild mit Dateiname; ZIP kopieren → file-Eintrag mit `C:\…`-Pfad
- [ ] KeePass oder KeePassXC: Passwort kopieren → erscheint **nicht**
- [ ] „Laufende App hinzufügen …“ listet offene Programme als `Name — Programm.exe`; ignoriertes Programm → Kopien erscheinen nicht
- [ ] Strg+Shift+V / Strg+Shift+B global
- [ ] Strg+Shift+Space in Notepad: ↩ fügt direkt ein; als Administrator gestartetes Notepad → nur kopiert
- [ ] Tray-Symbol im Infobereich: Öffnen/Pause/Beenden, Sprache folgt
- [ ] clipon erneut über Startmenü starten, während es im Tray läuft → vorhandenes Fenster kommt nach vorn, kein zweites Tray-Symbol
- [ ] Autostart an → nach Neuanmeldung läuft clipon genau einmal
- [ ] Einzel-Export ohne Endung im Dateinamen → Datei bekommt .txt/.png
- [ ] Backup-Export/-Import wie oben; Pfade mit Leerzeichen und Umlauten (`C:\Users\Jürgen\Desktop`)
- [ ] In-App-Update vom installierten Stand auf das nächste Release (NSIS)

## 7. Verifikation (unabhängige Zweitprüfung)

Zweiter, skeptischer Durchgang über den Diff `32da487..666fc8b` plus eigene Läufe.

### Nachgeprüft und bestätigt

- `cargo test --workspace`: 27 (clipon-core) + 6 (clipon) = 33 bestanden, 0 fehlgeschlagen.
- `pnpm build` (tsc + vite): OK. `cargo clippy --workspace --tests`: 0 Fehler, nur die bekannten `sort_by_key`-Stilhinweise.
- Windows-Cross-Typecheck `cargo check --target x86_64-pc-windows-msvc --workspace --tests` (Stub-Header, kein Link): OK, 0 Warnungen — auch nach den Änderungen unten.
- Version 1.0.0 in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `Cargo.lock` (Paket `clipon`) und `APP_VERSION` der Einstellungen. `clipon-core` bleibt bewusst 0.1.0 (interne Bibliothek).
- `build.yml`: nichts, was einen 1.0.0-`workflow_dispatch` auf macOS/Windows bricht; Signatur/Notarisierung unverändert. `File::try_lock` (Einzelinstanz) braucht Rust ≥ 1.89 — CI nutzt `stable`.
- Win32-FFI in `win.rs`: `INPUT`-Layout (40 Byte x64), `EnumWindows`-Callback, `QueryFullProcessImageNameW`-Puffer, Clipboard öffnen/schließen paarweise — statisch geprüft, plausibel.

### In dieser Runde zusätzlich behoben

1. **„Verlauf leeren“ löschte ohne Rückfrage — auch bei „Abbrechen“ (Datenverlust).**
   `tauri-plugin-dialog` 2.7 ersetzt `window.confirm` per Init-Skript durch eine
   *async*-Funktion, die einen Befehl `plugin:dialog|confirm` aufruft, den das Plugin gar
   nicht mehr hat. `if (window.confirm(…))` war damit immer wahr (Promise-Objekt) und
   der Verlauf wurde sofort geleert. Die Mock-Harness hatte `window.confirm = () => true`
   gesetzt und den Fehler so verdeckt. Jetzt: `confirm()` aus `@tauri-apps/plugin-dialog`
   (Buttons „Verlauf leeren“/„Abbrechen“), Capability `dialog:allow-message`.
   Harness bildet das echte Plugin-Verhalten jetzt nach; neues Szenario „Abbrechen löscht
   nichts“ schlägt auf `666fc8b` fehl (0 von 10 Einträgen übrig) und besteht mit dem Fix.
2. **Windows: Quick-Picker konnte beim ersten Strg+Shift+Space einfrieren.** Das Picker-Fenster
   wird auf Windows erst bei Bedarf gebaut — und zwar im Global-Shortcut-Handler bzw. in
   einem synchronen Befehl, also auf dem Event-Loop-Thread. Laut Tauri-Doku („Known issues“)
   blockiert `WebviewWindowBuilder::build` dort auf Windows (WebView2). Jetzt wird das
   Fenster beim ersten Aufruf auf einem eigenen Thread gebaut und danach angezeigt
   (nur ein Bau gleichzeitig); das Einfüge-Ziel wird vorher gemerkt.
3. **Verlaufsgröße verkleinern wurde nicht gespeichert, Liste nicht aktualisiert,** sobald nur
   Text-Einträge verdrängt wurden (`enforce_limit` liefert nur Bild-Blobs zurück). Jetzt
   Vergleich der Eintragszahl → speichern + `history-changed`/`stack-changed`.
4. **Stack-Ansicht nach Verdrängung weiterhin veraltet** (der Fix aus Runde 1 griff nur, wenn
   ein *Bild* verdrängt wurde, und gar nicht im Watcher). Jetzt lädt jede
   Verlaufsänderung auch den Stack neu; `add_text_and_copy` meldet `stack-changed` immer.
5. **`set_settings` brach beim ersten Fehler ab** (Autostart oder Kürzel-Registrierung), obwohl
   die Einstellungen schon gespeichert waren: Limit, Tray-Sprache und `settings-changed`
   wurden übersprungen, die UI zeigte den alten Stand. Jetzt laufen alle Schritte, Fehler
   werden gesammelt gemeldet, die UI lädt danach den tatsächlich gespeicherten Stand.

UI-Mock-Harness danach: 31/31 Szenarien bestanden, 0 Konsolenfehler.

### Widerlegt / eingeschränkt

- „Stack-Ansicht nach … Verdrängung aktualisiert“ war nur für Bild-Verdrängung richtig (s. 4).
- „30/30 Szenarien“ galt nur mit einem `window.confirm`-Stub, den es in der echten App nicht gibt (s. 1).
- Ein Harness-Szenario („Löschen eines Stack-Eintrags“) hing von der Reihenfolge ab und traf
  den zusammengeführten Eintrag statt `input@…`; korrigiert, kein App-Fehler.

### Weiterhin nur statisch geprüft (braucht Gerät)

Alles aus Abschnitt 6, zusätzlich:
- [ ] macOS + Windows: „Verlauf leeren“ → nativer Dialog; „Abbrechen“ lässt den Verlauf stehen
- [ ] Windows: allererstes Strg+Shift+Space nach dem Start → Picker erscheint (kein Hänger), ↩ fügt in Notepad ein
- [ ] Verlaufsgröße von 500 auf 10 senken → Liste sofort gekürzt, nach Neustart weiterhin 10

Hinweis: Strg+Shift+V ist unter Windows in vielen Programmen „Als Nur-Text einfügen“; als
globales Kürzel überdeckt clipon das systemweit. Im Kürzel-Editor änderbar, Standard unverändert.
