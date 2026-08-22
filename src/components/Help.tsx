import { useEffect, useState } from 'react';
import { Lang } from '../i18n';

// Selbstständiges Hilfe-System: schwebender ?-Button, First-Run-Tutorial
// und durchsuchbares Handbuch. Inhalte leben bewusst hier (nicht in i18n.ts),
// damit UI-Strings und Doku getrennt gepflegt werden können.

interface Step {
  title: string;
  body: string[];
}

interface Section {
  id: string;
  title: string;
  body: string[];
}

interface Content {
  labels: {
    fab: string;
    tutorial: string;
    manual: string;
    search: string;
    next: string;
    back: string;
    skip: string;
    done: string;
    stepOf: (n: number, total: number) => string;
    noResults: string;
  };
  tutorial: Step[];
  sections: Section[];
}

const de: Content = {
  labels: {
    fab: 'Hilfe & Handbuch',
    tutorial: 'Tutorial',
    manual: 'Handbuch',
    search: 'Handbuch durchsuchen …',
    next: 'Weiter',
    back: 'Zurück',
    skip: 'Überspringen',
    done: 'Los geht’s',
    stepOf: (n, total) => `Schritt ${n} von ${total}`,
    noResults: 'Keine Treffer',
  },
  tutorial: [
    {
      title: 'Willkommen bei clipon.',
      body: [
        'clipon merkt sich, was du kopierst — Text und Bilder — und bringt es durchsuchbar zurück.',
        'Alles bleibt auf deinem Rechner: Der Verlauf liegt AES-256-verschlüsselt auf der Platte, der Schlüssel im Schlüsselbund des Systems.',
        'Dieses Tutorial dauert eine Minute. Du findest es jederzeit wieder über den ?-Knopf unten rechts.',
      ],
    },
    {
      title: 'Der Verlauf',
      body: [
        'Kopiere irgendwo etwas — es taucht sofort oben in der Liste auf. Doppelklick kopiert einen Eintrag zurück in die Zwischenablage.',
        'Einfacher Klick wählt aus und zeigt rechts die Details: kompletter Text, Zeichen- und Wortzahl, wie oft kopiert.',
        '• Links filtern: Alles, Pins, Text, Links, Farben, Bilder',
        '• Oben suchen: Volltext über den ganzen Verlauf',
      ],
    },
    {
      title: 'Pins',
      body: [
        'Wichtiges pinnst du an — angepinnte Einträge überleben das Aufräumen und das Leeren des Verlaufs.',
        'Der Verlauf hält standardmäßig 500 Einträge; die ältesten unangepinnten fliegen zuerst raus.',
      ],
    },
    {
      title: 'Der Paste-Stack',
      body: [
        'Sammle mehrere Einträge über „In den Stack“ und hole sie in Reihenfolge zurück — ideal für Formulare.',
        'Cmd+Shift+B kopiert den jeweils nächsten Stack-Eintrag, ganz ohne das Fenster zu öffnen.',
      ],
    },
    {
      title: 'Snippets',
      body: [
        'Textbausteine, die du immer wieder brauchst — Adresse, Signatur, Antworten. Ein Klick, kopiert.',
        'Anlegen und bearbeiten im Bereich „Snippets“ in der Seitenleiste.',
      ],
    },
    {
      title: 'Kürzel & Tray',
      body: [
        '• Cmd+Shift+V — clipon-Fenster ein-/ausblenden, von überall',
        '• Cmd+Shift+B — Paste-Stack: nächsten Eintrag kopieren',
        'Das Schließen des Fensters beendet clipon nicht — es läuft im Tray weiter und zeichnet auf. Beenden geht über das Tray-Menü.',
        'Alle Kürzel lassen sich im Kürzel-Editor der Einstellungen frei belegen.',
      ],
    },
    {
      title: 'Privatsphäre & Pause',
      body: [
        'Die Zwischenablage ist ein sensibler Ort. Wenn clipon gerade nichts mitschneiden soll: Pause — per Seitenleiste oder Tray.',
        'Große Texte lassen sich per Limit ausschließen, Bilder komplett abschalten. Und nichts verlässt je deinen Rechner.',
      ],
    },
  ],
  sections: [
    {
      id: 'history',
      title: 'Verlauf',
      body: [
        'clipon beobachtet die Zwischenablage und legt jeden neuen Inhalt oben im Verlauf ab. Kopierst du denselben Inhalt erneut, entsteht kein Duplikat — der bestehende Eintrag rückt nach oben und zählt hoch.',
        'Bedienung:',
        '• Klick — Eintrag auswählen, Details rechts',
        '• Doppelklick oder Kopieren-Symbol — zurück in die Zwischenablage',
        '• Symbole am Eintrag (bei Maus darüber): Kopieren, Anpinnen, In den Stack, Löschen',
        'Der Typ jedes Eintrags wird automatisch erkannt und als Badge angezeigt: url (Links), @ (E-Mail), col (Farben, mit Farbfeld), { } (Code), img (Bilder), txt (alles andere).',
        'Die Verlaufsgröße ist einstellbar (Standard 500 Einträge). Ist das Limit erreicht, werden die ältesten unangepinnten Einträge entfernt.',
      ],
    },
    {
      id: 'filters',
      title: 'Filter & Suche',
      body: [
        'Die Seitenleiste filtert den Verlauf: Alles, Pins, Text, Links, Farben, Bilder.',
        'Das Suchfeld durchsucht den vollständigen Text aller Einträge (nicht nur die Vorschau). Suche und Filter lassen sich kombinieren.',
        '„Verlauf leeren“ (Papierkorb-Symbol neben der Suche) entfernt alle Einträge — angepinnte bleiben erhalten, wenn die Einstellung „Beim Leeren Pins behalten“ aktiv ist (Standard).',
      ],
    },
    {
      id: 'pins',
      title: 'Pins',
      body: [
        'Angepinnte Einträge (gelber Punkt) sind vor automatischem Aufräumen und vor „Verlauf leeren“ geschützt.',
        'Anpinnen und Lösen: über das Pin-Symbol am Eintrag oder den Knopf in der Detail-Ansicht. Der Filter „Pins“ zeigt alle auf einen Blick.',
      ],
    },
    {
      id: 'stack',
      title: 'Paste-Stack',
      body: [
        'Der Stack ist eine Warteschlange zum Einfügen in Reihenfolge: Einträge über „In den Stack“ sammeln, dann nacheinander abrufen.',
        '• „Nächsten kopieren“ im Stack-Bereich — oder von überall per Cmd+Shift+B',
        '• Jeder Abruf kopiert den vordersten Eintrag und entfernt ihn aus dem Stack',
        '• Einzelne Einträge entfernen oder den ganzen Stack leeren',
        'Typischer Ablauf: fünf Angaben aus einem Dokument kopieren, alle in den Stack, dann im Formular Feld für Feld Cmd+Shift+B → Cmd+V.',
      ],
    },
    {
      id: 'snippets',
      title: 'Snippets',
      body: [
        'Snippets sind gespeicherte Textbausteine mit Namen — unabhängig vom Verlauf, sie verfallen nie.',
        '• Anlegen: „Neues Snippet“ im Snippets-Bereich',
        '• Klick auf ein Snippet kopiert seinen Text',
        '• Bearbeiten und Löschen über die Symbole am Eintrag',
        'Snippets liegen wie der Verlauf in der verschlüsselten Datenbank.',
      ],
    },
    {
      id: 'images',
      title: 'Bilder',
      body: [
        'Kopierte Bilder (Screenshots, Grafiken) landen als Vorschau im Verlauf und lassen sich wie Text zurückkopieren.',
        'Jedes Bild wird als eigene, einzeln verschlüsselte Datei gespeichert. Wer Bilder nicht mitschneiden will, schaltet „Bilder mitschneiden“ in den Einstellungen ab.',
      ],
    },
    {
      id: 'shortcuts',
      title: 'Kürzel',
      body: [
        '• Cmd/Ctrl+Shift+V — Fenster ein-/ausblenden (global, funktioniert in jeder App)',
        '• Cmd/Ctrl+Shift+B — Paste-Stack: nächsten Eintrag kopieren (global)',
        'In der App hat jede Aktion ein Kürzel: Cmd/Ctrl+1–8 wechseln Filter und Ansichten, Pfeiltasten bewegen die Auswahl, Enter kopiert sie, Cmd/Ctrl+P pinnt, Cmd/Ctrl+S legt in den Stack, Backspace löscht, Cmd/Ctrl+E bearbeitet Snippets, Cmd/Ctrl+F springt in die Suche, F1 öffnet diese Hilfe.',
        'Alle Kürzel — global wie in der App — belegst du im Kürzel-Editor der Einstellungen frei: auf ein Kürzel klicken, neue Tastenkombination drücken. Konflikte werden erkannt, ↺ setzt einzelne zurück, „Alle zurücksetzen" den ganzen Satz.',
      ],
    },
    {
      id: 'privacy',
      title: 'Privatsphäre & Verschlüsselung',
      body: [
        'Der komplette Verlauf liegt AES-256-GCM-verschlüsselt in deinem Benutzerordner. Der Schlüssel wird beim ersten Start erzeugt und im Schlüsselbund des Systems abgelegt — nicht in einer Datei neben den Daten.',
        'clipon sendet nichts ins Netz; die einzige Verbindung ist der Update-Check gegen GitHub.',
        'Aufnahme pausieren: über die Seitenleiste oder das Tray-Menü — solange Pause aktiv ist, wird nichts mitgeschnitten.',
        'Zusätzliche Regeln in den Einstellungen: Texte über einer wählbaren Größe ignorieren, Bilder abschalten.',
      ],
    },
    {
      id: 'settings',
      title: 'Einstellungen',
      body: [
        '• Sprache — Deutsch / English',
        '• Verlaufsgröße — wie viele Einträge behalten werden (Pins zählen nicht gegen das Aufräumen)',
        '• Text ignorieren ab — sehr große Kopien gar nicht erst aufnehmen (0 = nie)',
        '• Bilder mitschneiden — Bild-Inhalte an/aus',
        '• Beim Leeren Pins behalten — Schutz für Angepinntes',
        '• Beim Anmelden starten — clipon automatisch mit dem System starten',
        '• Kürzel — Editor für alle Kürzel (global & in der App): klicken, Tasten drücken, fertig',
      ],
    },
    {
      id: 'updates',
      title: 'Updates',
      body: [
        'clipon prüft beim Start automatisch auf neue Versionen und meldet sich mit einem Hinweis, wenn eine bereitsteht — installiert wird erst nach deinem Klick.',
        'Manuell prüfen: Einstellungen → „Nach Updates suchen“. Vor der Installation siehst du das Changelog.',
        'Updates kommen signiert von GitHub (LAN-SOLO/clipon): Die App prüft die Signatur, bevor irgendetwas installiert wird. Verlauf und Einstellungen bleiben bei Updates unangetastet.',
      ],
    },
    {
      id: 'tray',
      title: 'Tray & Fensterverhalten',
      body: [
        'Das Schließen des Fensters (rotes X) versteckt es nur — clipon läuft im Hintergrund weiter und zeichnet auf.',
        'Tray-Menü (Menüleisten-Symbol): clipon öffnen, Aufnahme pausieren, clipon beenden.',
        'Auf dem Mac holt auch ein Klick auf das Dock-Symbol das Fenster zurück — oder das globale Kürzel Cmd+Shift+V.',
      ],
    },
  ],
};

const en: Content = {
  labels: {
    fab: 'Help & manual',
    tutorial: 'Tutorial',
    manual: 'Manual',
    search: 'Search the manual …',
    next: 'Next',
    back: 'Back',
    skip: 'Skip',
    done: 'Let’s go',
    stepOf: (n, total) => `Step ${n} of ${total}`,
    noResults: 'No matches',
  },
  tutorial: [
    {
      title: 'Welcome to clipon.',
      body: [
        'clipon remembers what you copy — text and images — and brings it back, searchable.',
        'Everything stays on your machine: the history is stored AES-256-encrypted on disk, the key lives in the system keychain.',
        'This tutorial takes a minute. You can reopen it anytime via the ? button in the bottom right.',
      ],
    },
    {
      title: 'The history',
      body: [
        'Copy something anywhere — it appears at the top of the list instantly. Double-click copies an item back to the clipboard.',
        'A single click selects it and shows details on the right: full text, character and word count, times copied.',
        '• Filter on the left: Everything, Pins, Text, Links, Colors, Images',
        '• Search on top: full text across the whole history',
      ],
    },
    {
      title: 'Pins',
      body: [
        'Pin what matters — pinned items survive cleanup and clearing the history.',
        'The history keeps 500 items by default; the oldest unpinned ones go first.',
      ],
    },
    {
      title: 'The paste stack',
      body: [
        'Collect items with “Add to stack” and bring them back in order — ideal for forms.',
        'Cmd+Shift+B copies the next stack item without even opening the window.',
      ],
    },
    {
      title: 'Snippets',
      body: [
        'Text blocks you need again and again — address, signature, replies. One click, copied.',
        'Create and edit them in the “Snippets” area in the sidebar.',
      ],
    },
    {
      title: 'Shortcuts & tray',
      body: [
        '• Cmd+Shift+V — show/hide the clipon window, from anywhere',
        '• Cmd+Shift+B — paste stack: copy the next item',
        'Closing the window does not quit clipon — it keeps running in the tray and keeps capturing. Quit via the tray menu.',
        'Every shortcut can be changed in the shortcut editor in Settings.',
      ],
    },
    {
      title: 'Privacy & pause',
      body: [
        'The clipboard is a sensitive place. When clipon shouldn’t capture: pause it — via the sidebar or the tray.',
        'Large texts can be excluded by a size limit, images can be turned off entirely. And nothing ever leaves your machine.',
      ],
    },
  ],
  sections: [
    {
      id: 'history',
      title: 'History',
      body: [
        'clipon watches the clipboard and files every new item at the top of the history. Copying the same content again creates no duplicate — the existing item moves to the top and its counter goes up.',
        'Controls:',
        '• Click — select item, details on the right',
        '• Double-click or the copy icon — back to the clipboard',
        '• Icons on hover: copy, pin, add to stack, delete',
        'Each item’s type is detected automatically and shown as a badge: url (links), @ (email), col (colors, with a swatch), { } (code), img (images), txt (everything else).',
        'The history size is configurable (default 500 items). When the limit is reached, the oldest unpinned items are removed.',
      ],
    },
    {
      id: 'filters',
      title: 'Filters & search',
      body: [
        'The sidebar filters the history: Everything, Pins, Text, Links, Colors, Images.',
        'The search box searches the full text of all items (not just the preview). Search and filters combine.',
        '“Clear history” (trash icon next to search) removes all items — pinned ones survive if “Keep pins when clearing” is enabled (default).',
      ],
    },
    {
      id: 'pins',
      title: 'Pins',
      body: [
        'Pinned items (yellow dot) are protected from automatic cleanup and from “Clear history”.',
        'Pin and unpin via the pin icon on an item or the button in the detail view. The “Pins” filter shows them all at a glance.',
      ],
    },
    {
      id: 'stack',
      title: 'Paste stack',
      body: [
        'The stack is a queue for pasting in order: collect items via “Add to stack”, then retrieve them one by one.',
        '• “Copy next” in the stack view — or from anywhere via Cmd+Shift+B',
        '• Each retrieval copies the front item and removes it from the stack',
        '• Remove single items or clear the whole stack',
        'Typical flow: copy five values from a document, add all to the stack, then in the form: Cmd+Shift+B → Cmd+V, field by field.',
      ],
    },
    {
      id: 'snippets',
      title: 'Snippets',
      body: [
        'Snippets are saved text blocks with a name — independent of the history, they never expire.',
        '• Create: “New snippet” in the snippets view',
        '• Click a snippet to copy its text',
        '• Edit and delete via the icons on each entry',
        'Snippets live in the same encrypted database as the history.',
      ],
    },
    {
      id: 'images',
      title: 'Images',
      body: [
        'Copied images (screenshots, graphics) appear as previews in the history and can be copied back just like text.',
        'Each image is stored as its own individually encrypted file. If you don’t want images captured, disable “Capture images” in Settings.',
      ],
    },
    {
      id: 'shortcuts',
      title: 'Shortcuts',
      body: [
        '• Cmd/Ctrl+Shift+V — show/hide the window (global, works in any app)',
        '• Cmd/Ctrl+Shift+B — paste stack: copy next item (global)',
        'Inside the app every action has a shortcut: Cmd/Ctrl+1–8 switch filters and views, arrow keys move the selection, Enter copies it, Cmd/Ctrl+P pins, Cmd/Ctrl+S adds to the stack, Backspace deletes, Cmd/Ctrl+E edits snippets, Cmd/Ctrl+F jumps to search, F1 opens this help.',
        'Every shortcut — global and in-app — is freely configurable in the shortcut editor in Settings: click a shortcut, press the new key combination. Conflicts are detected, ↺ resets one binding, “Reset all” the whole set.',
      ],
    },
    {
      id: 'privacy',
      title: 'Privacy & encryption',
      body: [
        'The entire history is stored AES-256-GCM-encrypted in your user folder. The key is generated on first launch and kept in the system keychain — not in a file next to the data.',
        'clipon sends nothing to the network; the only connection is the update check against GitHub.',
        'Pause capturing via the sidebar or the tray menu — while paused, nothing is recorded.',
        'Additional rules in Settings: ignore texts above a chosen size, disable images.',
      ],
    },
    {
      id: 'settings',
      title: 'Settings',
      body: [
        '• Language — Deutsch / English',
        '• History size — how many items to keep (pins don’t count against cleanup)',
        '• Ignore text above — skip very large copies entirely (0 = never)',
        '• Capture images — image contents on/off',
        '• Keep pins when clearing — protection for pinned items',
        '• Start at login — launch clipon with the system',
        '• Shortcuts — the editor for every binding (global & in-app): click, press keys, done',
      ],
    },
    {
      id: 'updates',
      title: 'Updates',
      body: [
        'clipon checks for new versions on launch and shows a notice when one is available — nothing installs without your click.',
        'Check manually: Settings → “Check for updates”. You see the changelog before installing.',
        'Updates come signed from GitHub (LAN-SOLO/clipon): the app verifies the signature before installing anything. History and settings are untouched by updates.',
      ],
    },
    {
      id: 'tray',
      title: 'Tray & window behavior',
      body: [
        'Closing the window (red X) only hides it — clipon keeps running and capturing in the background.',
        'Tray menu (menu bar icon): open clipon, pause capture, quit clipon.',
        'On the Mac, clicking the dock icon brings the window back — or use the global shortcut Cmd+Shift+V.',
      ],
    },
  ],
};

const SEEN_KEY = 'clipon.tutorialSeen';

export function Help({
  lang,
  onOpenChange,
}: {
  lang: Lang;
  onOpenChange?: (open: boolean) => void;
}) {
  const c = lang === 'de' ? de : en;
  const [mode, setMode] = useState<'closed' | 'tutorial' | 'manual'>(() =>
    localStorage.getItem(SEEN_KEY) ? 'closed' : 'tutorial'
  );
  const [step, setStep] = useState(0);
  const [sel, setSel] = useState(c.sections[0].id);
  const [q, setQ] = useState('');

  useEffect(() => {
    onOpenChange?.(mode !== 'closed');
  }, [mode, onOpenChange]);

  // the help shortcut in App opens the manual through this event
  useEffect(() => {
    const open = () => setMode('manual');
    window.addEventListener('clipon-open-help', open);
    return () => window.removeEventListener('clipon-open-help', open);
  }, []);

  const close = () => {
    localStorage.setItem(SEEN_KEY, '1');
    setMode('closed');
    setStep(0);
  };

  const query = q.trim().toLowerCase();
  const filtered = query
    ? c.sections.filter(
        (s) =>
          s.title.toLowerCase().includes(query) ||
          s.body.some((p) => p.toLowerCase().includes(query))
      )
    : c.sections;
  const current =
    filtered.find((s) => s.id === sel) ?? filtered[0] ?? null;

  return (
    <>
      <button className="hlp-fab" title={c.labels.fab} onClick={() => setMode('manual')}>
        ?
      </button>
      {mode !== 'closed' && (
        <div className="hlp-overlay" onClick={close}>
          <div className="hlp-modal" onClick={(e) => e.stopPropagation()}>
            <div className="hlp-head">
              <span className="hlp-brand">
                <span className="hlp-name">clipon</span>
                <span className="hlp-dot">.</span>
              </span>
              <button
                className={`hlp-tab ${mode === 'tutorial' ? 'active' : ''}`}
                onClick={() => {
                  setMode('tutorial');
                  setStep(0);
                }}
              >
                {c.labels.tutorial}
              </button>
              <button
                className={`hlp-tab ${mode === 'manual' ? 'active' : ''}`}
                onClick={() => setMode('manual')}
              >
                {c.labels.manual}
              </button>
              <span className="hlp-spacer" />
              <button className="hlp-close" onClick={close}>
                ✕
              </button>
            </div>

            {mode === 'tutorial' && (
              <div className="hlp-tut">
                <div className="hlp-step-count">
                  {c.labels.stepOf(step + 1, c.tutorial.length)}
                </div>
                <h2>{c.tutorial[step].title}</h2>
                {c.tutorial[step].body.map((p, i) =>
                  p.startsWith('• ') ? (
                    <div key={i} className="hlp-li">
                      {p.slice(2)}
                    </div>
                  ) : (
                    <p key={i}>{p}</p>
                  )
                )}
                <div className="hlp-tut-nav">
                  <button className="hlp-ghost" onClick={close}>
                    {c.labels.skip}
                  </button>
                  <span className="hlp-dots">
                    {c.tutorial.map((_, i) => (
                      <span key={i} className={i === step ? 'on' : ''} />
                    ))}
                  </span>
                  {step > 0 && (
                    <button onClick={() => setStep(step - 1)}>{c.labels.back}</button>
                  )}
                  {step < c.tutorial.length - 1 ? (
                    <button className="hlp-primary" onClick={() => setStep(step + 1)}>
                      {c.labels.next}
                    </button>
                  ) : (
                    <button className="hlp-primary" onClick={close}>
                      {c.labels.done}
                    </button>
                  )}
                </div>
              </div>
            )}

            {mode === 'manual' && (
              <div className="hlp-body">
                <div className="hlp-toc">
                  <input
                    type="text"
                    placeholder={c.labels.search}
                    value={q}
                    onChange={(e) => setQ(e.target.value)}
                  />
                  {filtered.length === 0 && (
                    <div className="hlp-empty">{c.labels.noResults}</div>
                  )}
                  {filtered.map((s) => (
                    <button
                      key={s.id}
                      className={`hlp-toc-item ${current?.id === s.id ? 'active' : ''}`}
                      onClick={() => setSel(s.id)}
                    >
                      {s.title}
                    </button>
                  ))}
                </div>
                <div className="hlp-content">
                  {current && (
                    <>
                      <h2>{current.title}</h2>
                      {current.body.map((p, i) =>
                        p.startsWith('• ') ? (
                          <div key={i} className="hlp-li">
                            {p.slice(2)}
                          </div>
                        ) : (
                          <p key={i}>{p}</p>
                        )
                      )}
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
