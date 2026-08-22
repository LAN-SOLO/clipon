import { Settings } from './api';
import { Dict } from './i18n';

/** In-app keyboard shortcuts: every clickable action has an action id, a
 *  default combo and an entry in the settings editor. Combos are stored in
 *  the same syntax the two GLOBAL shortcuts already use ("CmdOrCtrl+Shift+V"),
 *  so one recorder serves both kinds; only overrides land in
 *  `settings.keymap` — new actions in future versions get their defaults. */

export type ActionId =
  | 'filterAll'
  | 'filterPinned'
  | 'filterText'
  | 'filterLinks'
  | 'filterColors'
  | 'filterImages'
  | 'viewStack'
  | 'viewSnippets'
  | 'focusSearch'
  | 'togglePause'
  | 'openSettings'
  | 'openHelp'
  | 'clearHistory'
  | 'selectPrev'
  | 'selectNext'
  | 'copySelected'
  | 'pinSelected'
  | 'stackAddSelected'
  | 'deleteSelected'
  | 'editSelected'
  | 'stackPopNext'
  | 'stackClear'
  | 'newSnippet';

export interface ActionDef {
  id: ActionId;
  label: (t: Dict) => string;
  def: string;
  group: 'nav' | 'list' | 'tools';
}

export const ACTIONS: ActionDef[] = [
  // navigation
  { id: 'filterAll', label: (t) => t.all, def: 'CmdOrCtrl+1', group: 'nav' },
  { id: 'filterPinned', label: (t) => t.pinned, def: 'CmdOrCtrl+2', group: 'nav' },
  { id: 'filterText', label: (t) => t.text, def: 'CmdOrCtrl+3', group: 'nav' },
  { id: 'filterLinks', label: (t) => t.links, def: 'CmdOrCtrl+4', group: 'nav' },
  { id: 'filterColors', label: (t) => t.colors, def: 'CmdOrCtrl+5', group: 'nav' },
  { id: 'filterImages', label: (t) => t.images, def: 'CmdOrCtrl+6', group: 'nav' },
  { id: 'viewStack', label: (t) => t.stack, def: 'CmdOrCtrl+7', group: 'nav' },
  { id: 'viewSnippets', label: (t) => t.snippets, def: 'CmdOrCtrl+8', group: 'nav' },
  { id: 'focusSearch', label: (t) => t.scFocusSearch, def: 'CmdOrCtrl+F', group: 'nav' },
  { id: 'openSettings', label: (t) => t.settings, def: 'CmdOrCtrl+Comma', group: 'nav' },
  { id: 'openHelp', label: (t) => t.scOpenHelp, def: 'F1', group: 'nav' },
  // current list / selection
  { id: 'selectPrev', label: (t) => t.scSelectPrev, def: 'ArrowUp', group: 'list' },
  { id: 'selectNext', label: (t) => t.scSelectNext, def: 'ArrowDown', group: 'list' },
  { id: 'copySelected', label: (t) => t.scCopySelected, def: 'Enter', group: 'list' },
  { id: 'pinSelected', label: (t) => `${t.pin} / ${t.unpin}`, def: 'CmdOrCtrl+P', group: 'list' },
  { id: 'stackAddSelected', label: (t) => t.toStack, def: 'CmdOrCtrl+S', group: 'list' },
  { id: 'editSelected', label: (t) => t.edit, def: 'CmdOrCtrl+E', group: 'list' },
  { id: 'deleteSelected', label: (t) => t.delete, def: 'Backspace', group: 'list' },
  // tools
  { id: 'togglePause', label: (t) => t.scTogglePause, def: 'CmdOrCtrl+Shift+P', group: 'tools' },
  { id: 'clearHistory', label: (t) => t.clear, def: 'CmdOrCtrl+Shift+Backspace', group: 'tools' },
  { id: 'stackPopNext', label: (t) => t.popNext, def: 'CmdOrCtrl+Shift+N', group: 'tools' },
  { id: 'stackClear', label: (t) => t.stackClear, def: 'CmdOrCtrl+Shift+X', group: 'tools' },
  { id: 'newSnippet', label: (t) => t.newSnippet, def: 'CmdOrCtrl+N', group: 'tools' },
];

/** Defaults of the two system-wide shortcuts (registered with the OS). */
export const GLOBAL_DEFAULTS = {
  shortcutToggle: 'CmdOrCtrl+Shift+V',
  shortcutStackPop: 'CmdOrCtrl+Shift+B',
} as const;

export const isMac = /mac/i.test(
  (navigator as { userAgentData?: { platform?: string } }).userAgentData?.platform ??
    navigator.platform
);

/** Full action-id → combo map: defaults overlaid with saved overrides. */
export function resolveKeymap(settings: Settings): Record<ActionId, string> {
  const map = {} as Record<ActionId, string>;
  for (const a of ACTIONS) map[a.id] = settings.keymap?.[a.id] || a.def;
  return map;
}

const PUNCT: Record<string, string> = {
  ',': 'Comma',
  '.': 'Period',
  '/': 'Slash',
  ';': 'Semicolon',
  "'": 'Quote',
  '-': 'Minus',
  '=': 'Equal',
  '[': 'BracketLeft',
  ']': 'BracketRight',
  '\\': 'Backslash',
  '`': 'Backquote',
};

/** Layout-safe key name for an event: letters/digits via e.code (so
 *  Shift+1 stays "1"), everything else via e.key / the punctuation table.
 *  Returns null for pure modifier presses. */
function keyName(e: KeyboardEvent): string | null {
  const code = e.code || '';
  const m = /^(?:Key([A-Z])|Digit([0-9]))$/.exec(code);
  if (m) return m[1] ?? m[2];
  const k = e.key;
  if (k === 'Shift' || k === 'Control' || k === 'Alt' || k === 'Meta' || k === 'Dead') return null;
  if (k === ' ') return 'Space';
  if (PUNCT[k]) return PUNCT[k];
  if (k.length === 1) return k.toUpperCase();
  if (k === 'Escape') return null; // reserved: cancels recording, closes modals
  return k; // named keys: Enter, Backspace, Delete, ArrowUp, F1 …
}

/** Canonical combo string for a key event, null when not bindable. */
export function comboFromEvent(e: KeyboardEvent): string | null {
  const key = keyName(e);
  if (!key) return null;
  const parts: string[] = [];
  if (e.metaKey || e.ctrlKey) parts.push('CmdOrCtrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  parts.push(key);
  return parts.join('+');
}

export const hasRealModifier = (combo: string): boolean =>
  combo.includes('CmdOrCtrl+') || combo.includes('Alt+');

const KEY_GLYPHS: Record<string, string> = {
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
  Enter: '↩',
  Backspace: '⌫',
  Delete: '⌦',
  Space: '␣',
  Comma: ',',
  Period: '.',
  Slash: '/',
  Semicolon: ';',
  Quote: "'",
  Minus: '-',
  Equal: '=',
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  Backquote: '`',
};

/** Human-readable form: "⌘⇧V" on macOS, "Ctrl+Shift+V" elsewhere. */
export function formatCombo(combo: string): string {
  const parts = combo.split('+');
  const key = parts[parts.length - 1];
  const mods = parts.slice(0, -1);
  const shown = KEY_GLYPHS[key] ?? key;
  if (isMac) {
    return (
      mods
        .map((mod) => (mod === 'CmdOrCtrl' ? '⌘' : mod === 'Alt' ? '⌥' : mod === 'Shift' ? '⇧' : mod))
        .join('') + shown
    );
  }
  return [...mods.map((mod) => (mod === 'CmdOrCtrl' ? 'Ctrl' : mod)), shown].join('+');
}
