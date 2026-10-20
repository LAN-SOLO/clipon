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

## Runde 2 (2026-10-20)

Basis `6c3cae1` (= origin/main, Stand der 1.0.0-Test-Builds). Gezielte Suche nach den
Fehlermustern L1–L13 aus Runde 1 der anderen Apps, plus die Restpunkte dieser App.

### Behoben

| Muster | Befund | Fix |
|---|---|---|
| L2 | Backend-Fehler kamen als deutsche Prosa („unbekanntes Element“, „Update-Prüfung fehlgeschlagen …“, „Datei zu groß …“) bzw. englische Library-Texte („wrong passphrase or corrupted file“, „invalid JSON“) roh in die UI — in der englischen Oberfläche deutsch und umgekehrt | Backend liefert nur noch stabile Codes (`code` / `code: Detail`, mehrere mit ` \| `), `src/errors.ts` übersetzt sie (DE+EN), auch verschachtelt (`save-failed: io: …`); Kürzel-Fehler nennen das Kürzel in Plattform-Schreibweise. Test `every_error_code_is_translated` prüft, dass jeder Code in beiden Sprachen in `i18n.ts` steht |
| L11 | Viele Aktionen schluckten Fehler: Kopieren, Pin, In den Stack, Löschen, Stack entfernen/leeren, „Nächsten kopieren“ (gar kein `catch` → unbehandelte Promise), Pause, Farbwähler, Update-Installation, Picker-Einfügen; „Zusammenführen“ meldete bei *jedem* Fehler „keine Text-Einträge“ | Jeder Fehler erscheint als Toast (5 s) bzw. im Picker-Fuß, in der UI-Sprache |
| L11 | Speichern des Verlaufs (`persist`) und der Einstellungen scheiterte still (nur `eprintln`) — voller Datenträger oder gesperrte Datei blieb unbemerkt | `persist()`/`settings::store()` liefern `Result`; Befehle geben den Fehler zurück, Watcher/Tray/globale Kürzel senden `save-failed` → Toast „Änderung konnte nicht gespeichert werden“ |
| L11 | Snippet löschen ohne Rückfrage (Zeilen-Symbol und Backspace-Kürzel) — selbst getippter Text sofort weg | Native Rückfrage über `confirm()` aus `@tauri-apps/plugin-dialog` (Löschen/Abbrechen) |
| L8 | Fast alle Befehle liefen synchron auf dem Haupt-/IPC-Thread: Entschlüsseln + Skalieren von Bildern (`get_item_image`), Bild in die Zwischenablage, Einzel-Export, Backup-Export inkl. PBKDF2 600k, Import inkl. Bild-Neuverschlüsselung, jedes `persist` (AES über den ganzen Verlauf) → Fenster friert bei großem Verlauf/großen Bildern ein | Schwere Befehle laufen per `spawn_blocking`, die übrigen Store-Befehle sind `async` (Tokio-Worker statt Haupt-Thread) |
| L9 | Export/Import ließ sich während eines laufenden Exports/Imports erneut starten | Knöpfe während des Laufs gesperrt, Hinweis „Läuft …“ |
| L9 | „Nächsten kopieren“ entfernte den Eintrag *vor* dem Kopieren aus dem Stack — scheiterte die Zwischenablage, war er weg | erst kopieren, dann entfernen (Befehl und globales Kürzel) |
| L5 | `rename` beim Speichern von Verlauf, Einstellungen und Exporten ohne Wiederholung — Virenscanner/OneDrive/Indexer halten die Datei unter Windows kurz offen (ERROR_SHARING_VIOLATION/ACCESS_DENIED/LOCK_VIOLATION) | `replace_file`/`read_file` mit bis zu 8 Wiederholungen (25–200 ms), Temp-Datei wird bei Fehlschlag entfernt; Test mit simuliertem Lock |
| L5 | Vorgeschlagener Export-Dateiname aus der Vorschau konnte ein reservierter Windows-Name sein (Eintrag „con“ → `con.txt`) | `exportBaseName`: reservierte Namen (CON, PRN, AUX, NUL, COM1–9, LPT1–9) bekommen `clipon-` davor |
| L4 | Import lehnte JSON mit UTF-8-BOM ab (Windows-Editor speichert so) | BOM wird entfernt; Test `import_accepts_utf8_bom_and_reports_codes` |
| L6 | Kürzel überall als „Ctrl+…“, auch in der deutschen Oberfläche; Hinweis „⌘/Ctrl oder Alt“ auf jeder Plattform; Handbuch DE mit „Ctrl“ | `formatCombo(combo, lang)`: macOS ⌘⇧V, Windows/Linux „Strg+Shift+V“ (DE) bzw. „Ctrl+Shift+V“ (EN); Hinweis plattformgerecht; Handbuch DE „Cmd/Strg“ |
| L10 | Vorgeschlagener Backup-Name nutzte das UTC-Datum (`toISOString`) — kurz nach Mitternacht in Europa der Vortag | lokales Datum |
| L7 | Farbfeld nutzte den Rohtext als `style.background`; `rgb(0 0 0) url(…)` galt als Farbe | Erkennung lässt in `rgb()/hsl()` nur Zahlen, Einheiten und Trenner zu (Core-Test), Farbfeld zeigt nur den geparsten Wert als `rgba()` neu serialisiert (CSP blockte externe Bilder ohnehin) |
| L13 | Update-Fehler ohne Grund („Update-Prüfung fehlgeschlagen“), Installationsfehler im Banner still | Fehler mit Grund in der UI-Sprache, in Einstellungen und Banner |
| Abh. | `cargo audit`: rustls 0.23.43 mit 4 Advisories (RUSTSEC-2026-0285/-0346/-0348/-0349) | `cargo update -p rustls` → 0.23.46 (Patch, nur Lockfile) |

### Geprüft, ohne Befund

- **L1:** kein `window.confirm/alert/prompt` mehr; „Verlauf leeren“ und „Snippet löschen“ über `confirm()` des Plugins (Capability `dialog:allow-message`, Plugin 2.7.2 ruft `plugin:dialog|message`). Die Harness bildet das echte Plugin nach (`window.confirm` wirft).
- **L3:** Sprache beim Erststart kommt aus der System-UI-Sprache (macOS `NSLocale`, Windows `GetUserDefaultUILanguage`), nicht aus `LANG`.
- **L12:** alle neuen Texte DE+EN; Handbuch (Verlauf, Snippets, Export & Import, Farben, Kürzel) beschreibt das neue Verhalten.
- **L13:** Updater-Endpoint `https://github.com/LAN-SOLO/clipon/releases/latest/download/latest.json`, Minisign-Pubkey gesetzt, Versionsvergleich semver über das Plugin (1.0.0 → 1.0.x).
- CSV gibt es in clipon nicht (L4-Export entfällt).

### Bewusst offen

- `cargo audit`: `zip` 4.6.1 (RUSTSEC-2026-0367/-0368/-0369, Fix erst in 9.x) kommt nur über `tauri-plugin-updater` 2.10.1 und nur unter Windows; die Lücken betreffen SUID-Bits/Symlinks beim Entpacken (Unix) und das Update-Paket wird erst nach der Minisign-Prüfung entpackt. Ein Update des Updater-Plugins (2.12) wäre ein Minor-Sprung mitten in den 1.0-Tests — nicht gemacht. Übrige Meldungen sind „unmaintained“/„unsound“-Warnungen in GTK/Linux-Abhängigkeiten von Tauri.
- Schlüssel/Passphrasen werden nicht genullt (`zeroize` wäre neue Abhängigkeit).
- Fehler-Details (Text in Klammern) stammen vom Betriebssystem bzw. von Bibliotheken und bleiben in deren Sprache (Windows liefert sie in der Systemsprache).

### Tests Runde 2

| Prüfung | Ergebnis |
|---|---|
| `cargo test --workspace` | 30 Core + 8 App = 38 grün (neu: Lock-Wiederholung, BOM, Store ersetzen, Farb-Härtung, Import-Codes, Übersetzungs-Vollständigkeit) |
| `pnpm build` (tsc + vite) | fehlerfrei |
| `cargo clippy --workspace --tests` | nur die bekannten `sort_by_key`-Hinweise |
| Windows-Cross-Typecheck (`x86_64-pc-windows-msvc`, Stub-Header, kein Link) | fehlerfrei, 0 Warnungen |
| `cargo audit` | rustls behoben; zip offen (s. o.) |
| UI-Harness (headless Chromium, gemocktes IPC, Dialog-Plugin wie echt) | 31 Szenarien Runde 1 + 13 neue (Fehler-Toasts DE/EN, Snippet-Rückfrage Abbrechen/Löschen, `save-failed`-Event, Kürzel-/Autostart-Fehler, Export-Sperre + lokales Datum, Import-/Update-Fehler, Farbfeld, Stack-Pop-Fehler, Picker-Fehler) + 3 mit Windows-Plattform (Strg/Ctrl) — alle bestanden, 0 Konsolenfehler |
| Layout 780×520 (Mindestgröße) DE/EN × Dunkel/Hell, Picker 560×420 | kein horizontaler Überlauf, keine abgeschnittenen Knöpfe in Verlauf, Farben, Stack, Snippets, Einstellungen, Hilfe; Screenshots gesichtet |

### Nur am echten Gerät prüfbar (zusätzlich zu Abschnitt 6)

- [ ] macOS + Windows: Snippet löschen → nativer Dialog; „Abbrechen“ behält es
- [ ] Windows: Kürzel-Editor und Stack-Hinweis zeigen „Strg+Shift+…“ (deutsche Oberfläche)
- [ ] Großer Verlauf (mehrere 100 Bilder): Fenster bleibt beim Scrollen, Export und Import bedienbar
- [ ] Windows mit OneDrive-/Defender-Echtzeitscan auf dem Datenordner: Speichern klappt, kein „Speichern fehlgeschlagen“
- [ ] Schreibgeschützter Datenordner bzw. voller Datenträger: Toast „Änderung konnte nicht gespeichert werden“ erscheint
- [ ] JSON-Backup im Windows-Editor öffnen, speichern (UTF-8 mit BOM), importieren
- [ ] Englische Oberfläche: falsche Passphrase, Update-Fehler (offline) → englische Meldung

## Runde 2 – Verifikation (2026-10-20, unabhängige Zweitprüfung)

Geprüft: Diff `6c3cae1..0d2ce2b` auf Regressionen (plattformübergreifend, neue
Async-/Race-Fälle), alle Tests und Typechecks erneut ausgeführt, die geänderten
Abläufe in der UI-Harness durchgespielt, L1–L13 an zusätzlichen Stellen nachgesehen.

### Nachgeprüft und bestätigt

- **L1:** `confirm()` des Dialog-Plugins 2.7.2 gegen den Rust-Code von Plugin und `rfd` 0.16 geprüft: bei eigenen Knopf-Texten liefert macOS (`NSAlert`) wie Windows (TaskDialog) `Custom("<Text>")`, der Vergleich `=== okLabel` stimmt also auf beiden Plattformen; „Abbrechen“ ergibt `false`. Capability `dialog:allow-message` vorhanden.
- **L2:** jeder `Err(...)` im Backend ist ein Code aus der Testliste oder `io:`/`clipboard:`/`image:`/`internal:`/`shortcut:` mit Detail; keine deutsche/englische Prosa mehr. `errorText` lässt unbekannte Texte unverändert, übersetzt verschachtelte Codes und reicht Details roh in Klammern durch (Harness).
- **L8/Async:** Kein Store-Lock wird über ein `await` gehalten; `persist()` serialisiert über den Store-Mutex, die Temp-Datei kann sich also nicht überschneiden. `set_settings`/`set_paused` bleiben synchron (Haupt-Thread) und schreiben die Einstellungen damit nacheinander. Gleichzeitige Exporte/Importe (z. B. Einstellungen geschlossen und neu geöffnet) laufen über den Store-Lock nacheinander; ein doppelter Import überspringt Duplikate.
- **L5:** `is_transient_busy` greift nur unter Windows (`cfg!(windows)`); Windows-Typecheck prüft den `cfg(windows)`-Code wirklich (mit einem absichtlichen Windows-only-Fehler gegengeprüft, danach entfernt).
- **L13:** Endpoint/Pubkey/Version 1.0.0 unverändert korrekt.

### In dieser Prüfung zusätzlich behoben

| Muster | Befund | Fix |
|---|---|---|
| L9/L11 | Runde 2 stellte „Nächsten kopieren“ auf „erst kopieren, dann entfernen“ um. Ließ sich der vorderste Eintrag *dauerhaft* nicht kopieren (Bilddatei fehlt/beschädigt, Entschlüsselung scheitert), blieb er für immer vorn: jeder weitere Abruf scheiterte, der Rest des Stacks war blockiert. Beim globalen Kürzel geschah das zudem **still** (Fehler wurde verworfen). | Gemeinsame Logik `pop_with` für Befehl und Kürzel: nur ein Zwischenablage-Fehler (`clipboard:`) lässt den Eintrag vorn; ein dauerhaft defekter Eintrag wird aus dem Stack genommen und der Grund gemeldet; Stack-IDs ohne Eintrag werden übersprungen. Das globale Kürzel meldet Fehler per Event `action-failed` → Toast im Hauptfenster. Test `stack_pop_keeps_entry_on_clipboard_error_and_drops_broken_ones`; Handbuch DE+EN und README ergänzt. |

### Widerlegt / eingeschränkt

- Keine Aussage aus Runde 2 widerlegt. Einschränkung: „keine stillen Fehlschläge“ galt nicht für das globale Stack-Kürzel (oben behoben). Hintergrund-Ladevorgänge (Liste, Einstellungen, Update-Prüfung beim Start) fangen Fehler weiterhin still ab — bewusst, sie haben keine Benutzeraktion.
- `cargo audit` zusätzlich: `zbus_names`/`zvariant`/`zvariant_derive` „unmaintained“ (RUSTSEC-2026-0370/-0371/-0372), nur Linux (D-Bus), keine Sicherheitslücke — nicht angefasst.

### Tests der Verifikation

| Prüfung | Ergebnis |
|---|---|
| `cargo test --workspace` | 30 Core + 9 App = 39 grün (neu: Stack-Pop-Logik) |
| `pnpm build` (tsc + vite) | fehlerfrei |
| `cargo clippy --workspace --tests` | nur die bekannten `sort_by_key`-Hinweise |
| Windows-Cross-Typecheck (`x86_64-pc-windows-msvc`, kein Link) | fehlerfrei, 0 Warnungen, vor und nach dem Fix |
| UI-Harness (headless Chromium, gemocktes IPC mit echtem Dialog-Plugin-Verhalten) | 31 (Runde 1) + 13 (Runde 2) + 3 neu (`action-failed` DE/EN, roher/verschachtelter Fehlertext) + 3 mit Windows-Plattform — alle bestanden, 0 Konsolenfehler, 0 IPC-Argumentfehler |

### Nur am echten Gerät prüfbar (zusätzlich)

- [ ] Stack mit einem Bild-Eintrag, dessen Datei im Datenordner gelöscht wurde: Cmd/Strg+Shift+B → Meldung im Fenster, der nächste Druck liefert den folgenden Eintrag
- [ ] Zwischenablage von anderer App blockiert (Windows): „Nächsten kopieren“ meldet den Fehler, der Eintrag bleibt vorn
- [ ] Native Rückfrage (Snippet löschen, Verlauf leeren) unter Windows mit eigenen Knopf-Texten „Löschen“/„Abbrechen“

## Runde 3 (2026-10-20, abschließend)

Basis `7c05322`. Jede Sachaussage aus Handbuch/Tutorial/Tour (`Help.tsx`, DE und EN), README und
den In-App-Texten (`i18n.ts`) gegen den Code geprüft; DE und EN inhaltlich abgeglichen.
Danach End-to-End-Abläufe in der UI-Harness (DE + EN, zusätzlich mit Windows-Plattformkennung)
und neue Rust-Tests für Neustart/schreibgeschützten Ordner.

### Falsche bzw. ungenaue Aussagen — behoben

| Aussage | Befund | Lösung |
|---|---|---|
| „Verlaufsgröße … (Pins zählen nicht gegen das Aufräumen)“ (Handbuch, Audit-Inventar) | **Falsch, mit Datenverlust-Folge:** `enforce_limit` zählte *alle* Einträge. Mit so vielen Pins wie dem Limit (z. B. Limit 10, 10 Pins) wurde jede neue Kopie sofort wieder verdrängt — der Verlauf nahm nichts mehr auf | Code: es zählen nur unangepinnte Einträge, Pins kommen dazu und werden nie aufgeräumt. Tests `limit_evicts_oldest_unpinned` (angepasst), `new_copy_survives_when_pins_fill_the_limit`; Mock der Harness spiegelt das. Texte DE+EN präzisiert (Tutorial, Verlauf, Pins, Einstellungen, README) |
| „Pins überleben … das Leeren des Verlaufs“ (Tutorial, Tour, Leer-Zustand „Pins“) | nur solange „Beim Leeren Pins behalten“ an ist | Text DE+EN: „(Standard)“ bzw. „außer du schaltest das ab“ |
| Quick-Picker „1–9 … sonst mit Cmd“ | Code akzeptiert Cmd **oder** Strg (`metaKey \|\| ctrlKey`); unter Windows gibt es kein Cmd | Text DE „Cmd/Strg“, EN „Cmd/Ctrl“ |
| Platzhalter `{date:%A, %d. %B}` | chrono formatiert Wochentag/Monat immer englisch („Monday“), auch in der deutschen Oberfläche | Hinweis DE+EN ergänzt |
| Platzhalter „unbekannte bleiben stehen“ | `{a {date}}` ließ auch das innere `{date}` unersetzt (Token reichte bis zur ersten `}`) | Code: innere Klammer beginnt das eigentliche Token; Test erweitert (`{a {date}}`, `{{time}}`) |
| „Beim Anmelden starten (Mac: Anmeldeobjekt)“ | Plugin nutzt `MacosLauncher::LaunchAgent` (Plist in `~/Library/LaunchAgents`), kein Anmeldeobjekt | Text DE+EN korrigiert (Windows: Run-Eintrag in der Registry) |
| „Klappt etwas nicht (… Datei nicht speicherbar …) zeigt clipon eine Meldung — auch im Hintergrund, etwa bei vollem Datenträger“ | Für **Bild-Aufnahmen** falsch: scheiterte das Schreiben der Bilddatei (voll/schreibgeschützt), wurde die Kopie still verworfen, eine Teil-Datei blieb liegen | Code: `write_blob` entfernt die Teil-Datei und meldet `save-failed` |
| dito, Meldung „im clipon-Fenster“ (Stack-Kürzel, Watcher, Tray) | Toast lief 5 s ab, während das Fenster versteckt war — niemand sah ihn | siehe Restpunkt 1 |

Zusätzlich (kein Text betroffen): Teil-Dateien bei vollem Datenträger blieben liegen — beim Verlauf
(`history.tmp`) und beim Export (`<name>.clipon-part` im Zielordner des Nutzers). Beide werden jetzt
auch entfernt, wenn schon das Schreiben scheitert.

Geprüft und zutreffend (Auswahl): Dedupe + Zähler, Volltextsuche über den vollen Text + Bild-Label,
Typ-Badges, Farb-Erkennung (#rgb/#rrggbb/#rrggbbaa, rgb[a]/hsl[a]), Werkzeuge (Trim, Groß/klein,
Titel, Zeilenumbrüche → Leerzeichen, JSON, URL RFC 3986), Zusammenführen (Zeilenumbruch, Stack bleibt,
Bilder übersprungen, nur-Bilder → Meldung), Stack-Abruf-Logik, Snippets in der verschlüsselten
Datenbank, Platzhalter-Formate, Export-Endung erzwungen, Import (1 GB, BOM, Pins/Zeitstempel erhalten,
Limit danach), Bildformate (image-Features png/jpeg/gif/bmp/webp/tiff), 64 MB / 20 Dateien pro Kopie,
Einstellungs-Grenzen 10–100.000 und 512 KB/0 = nie, Standard-Kürzel (3 global, 23 in der App),
Picker schließt bei Fokusverlust, Schließen = Verstecken, Dock-Klick (`RunEvent::Reopen`),
Updater-Endpoint/Signatur, Tutorial 9 Schritte. DE und EN sagen nach den Korrekturen dasselbe.

### Restpunkte

1. **Toast bei verstecktem Fenster.** `tauri-plugin-notification` ist **nicht** im Lockfile — eine
   Systemmitteilung wäre eine neue Abhängigkeit, daher nicht gebaut. Stattdessen: Meldungen aus dem
   Hintergrund (`save-failed`, `action-failed`) erscheinen sofort *und* werden, wenn das Fenster
   gerade nicht sichtbar/fokussiert ist (`document.hasFocus()`), gemerkt und beim nächsten Fokus bzw.
   Sichtbarwerden erneut 6 s gezeigt (nur die letzte, einmalig). Handbuch DE+EN und README nennen das.
2. **Hunderte Bilder.** Vorher forderte jede Liste *alle* Vorschaubilder gleichzeitig an (bei 400
   Bildern 800 parallele Backend-Aufrufe wegen StrictMode im Dev-Build — jeder entschlüsselt und
   skaliert das Vollbild) und behielt jede Data-URL für immer. Jetzt: Vorschauen laden erst, wenn die
   Zeile ins Bild kommt (`IntersectionObserver`, 200 px Vorlauf), höchstens 3 gleichzeitig, Cache auf
   400 Vorschauen begrenzt; das große Detailbild umgeht die Warteschlange. Unsichtbarer 36-px-Platzhalter,
   damit die Zeilen nicht springen. Harness: 400 Bilder → beim Öffnen der Liste 14 statt 800 Aufrufe (nur die
   sichtbaren Zeilen + Vorlauf), nach Scrollen ans Ende 28; Höchstwert 3 parallel (vorher 800).

### End-to-End-Abläufe (UI-Harness, headless Chromium, gemocktes IPC mit echtem Dialog-Verhalten)

Je DE und EN, zusätzlich alle mit Windows-Plattformkennung (UA/`userAgentData` = Windows):

| # | Ablauf | Ergebnis |
|---|---|---|
| J1 | Erststart: Tutorial 9 Schritte bis „Los geht’s“, Flag gesetzt | bestanden |
| J2 | Kopie erfasst → gleiche Kopie erneut → kein Duplikat, „2×“ → Doppelklick kopiert zurück | bestanden |
| J3 | 10 Pins, Limit auf 10 → neue Kopie bleibt im Verlauf | bestanden (Kernlogik: Rust-Test) |
| J4 | Verlauf leeren: nativer Dialog „Abbrechen“ → nichts weg; bestätigen → nur Pin bleibt | bestanden |
| J5 | Stack sammeln → zusammenführen → abrufen; `action-failed` bei verstecktem Fenster → nach Fokus erneut sichtbar, danach nicht noch einmal | bestanden; **schlägt auf `7c05322` fehl** |
| J6 | Hintergrund-Speichern scheitert („No space left on device (os error 28)“) bei verstecktem Fenster → Meldung beim nächsten Öffnen | bestanden; **schlägt auf `7c05322` fehl** |
| J7 | Snippet mit `{date}` anlegen → kopieren → Löschen abbrechen → löschen | bestanden |
| J8 | Backup-Export: Dialog abgebrochen → kein Aufruf, keine Meldung; schreibgeschützt (os error 30), voll (28), gesperrt (32), verweigert (13) → „Export fehlgeschlagen: Datei-Fehler (…)“; Knöpfe danach wieder frei | bestanden |
| J9 | Import: Dialog abgebrochen → nichts; falsche Passphrase → Meldung; Einzel-Export abgebrochen → kein Aufruf, kein Toast | bestanden |
| J10 | 400 Bild-Einträge: Liste öffnen, ans Ende scrollen, Detailbild | bestanden; **schlägt auf `7c05322` fehl** (800 Aufrufe/800 parallel) |
| J11 | Quick-Picker: tippen filtert, Strg/Cmd+1 bei Suche fügt ein, 1 ohne Modifier bei Suche nicht, Esc schließt | bestanden |

Neustart mitten im Speichern und schreibgeschützter Datenordner sind in der Harness nicht abbildbar
(der Mock hält alles im Speicher) und deshalb als Rust-Tests umgesetzt:
`restart_after_interrupted_save_keeps_last_good_store` (halb geschriebene Temp-Datei → letzter guter
Stand wird geladen, nächstes Speichern klappt) und `save_into_read_only_folder_fails_with_code_and_keeps_old_store`
(Unix; Fehler `io: …`, alter Verlauf intakt, keine Temp-Datei).

Alte Harness-Suiten nach den Änderungen erneut: 31 (Runde 1) + 13 (Runde 2) + 3 (Verifikation) + 3 (Windows) —
alle bestanden, 0 Konsolenfehler, 0 IPC-Argumentfehler.

### Tests Runde 3

| Prüfung | Ergebnis |
|---|---|
| `cargo test --workspace` | 33 Core + 9 App = 42 grün (neu: Pins vs. Limit, verschachtelte Platzhalter, Neustart nach abgebrochenem Speichern, schreibgeschützter Ordner) |
| `pnpm build` (tsc + vite) | fehlerfrei |
| `cargo clippy --workspace --tests` | nur die bekannten `sort_by_key`-Hinweise |
| Windows-Cross-Typecheck (`x86_64-pc-windows-msvc`, kein Link) | fehlerfrei, 0 Warnungen |
| UI-Harness | 22 neue Ablauf-Szenarien (DE/EN) + 22 mit Windows-Plattform + 50 aus Runde 1/2 — alle bestanden |

### Nur geprüft, nicht ausgeführt

- Watcher-Pfad `write_blob` (echte Zwischenablage + voller Datenträger) — nur Typecheck/Review.
- Ob WKWebView/WebView2 beim Wiedereinblenden wirklich `focus`/`visibilitychange` liefern (Harness
  simuliert das Ereignis).

### Nur am echten Gerät prüfbar (zusätzlich)

- [ ] macOS + Windows: Fenster schließen, Stack leer machen, Bild-Eintrag mit gelöschter Datei in den Stack, Cmd/Strg+Shift+B → beim Öffnen per Cmd/Strg+Shift+V erscheint die Meldung
- [ ] Datenordner schreibgeschützt machen, Screenshot kopieren → Meldung „Änderung konnte nicht gespeichert werden“ (sofort bzw. beim nächsten Öffnen), keine `.bin`-Reste
- [ ] 300+ Bilder im Verlauf (Release-Build): Filter „Bilder“ öffnet ohne Hänger, Scrollen flüssig, Vorschauen erscheinen beim Scrollen
- [ ] Limit 10 mit 10+ Pins: neue Kopien erscheinen und bleiben
- [ ] Autostart auf macOS: Eintrag unter Systemeinstellungen → Allgemein → Anmeldeobjekte („Im Hintergrund erlauben“)
