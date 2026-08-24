import { invoke } from '@tauri-apps/api/core';

export type ItemKind = 'text' | 'image';
export type Detected = 'plain' | 'url' | 'email' | 'color' | 'code' | 'file';
export type Filter = 'all' | 'pinned' | 'text' | 'links' | 'images' | 'colors';
export type Transform =
  | 'trim'
  | 'lower'
  | 'upper'
  | 'title'
  | 'stripbreaks'
  | 'jsonpretty'
  | 'urlencode'
  | 'urldecode';

export interface ClipItem {
  id: string;
  kind: ItemKind;
  preview: string;
  chars: number;
  detected: Detected;
  pinned: boolean;
  timesCopied: number;
  createdAt: string;
  lastCopiedAt: string;
  sourceAppId: string | null;
  sourceAppName: string | null;
}

export interface Snippet {
  id: string;
  name: string;
  text: string;
}

export interface Settings {
  language: 'de' | 'en';
  historyLimit: number;
  captureImages: boolean;
  maxTextKb: number;
  keepPinnedOnClear: boolean;
  autostart: boolean;
  shortcutToggle: string;
  shortcutStackPop: string;
  shortcutPicker: string;
  /** In-app shortcut overrides (action id → combo); defaults live in shortcuts.ts. */
  keymap: Record<string, string>;
  paused: boolean;
  /** Bundle ids whose copies are never recorded. */
  ignoredApps: string[];
}

export interface UpdateInfo {
  version: string;
  notes: string | null;
  date: string | null;
}

export interface RunningApp {
  bundleId: string;
  name: string;
}

export interface ImportStats {
  itemsAdded: number;
  itemsSkipped: number;
  snippetsAdded: number;
  snippetsSkipped: number;
}

export const api = {
  getSettings: () => invoke<Settings>('get_settings'),
  setSettings: (s: Settings) => invoke<void>('set_settings', { new: s }),
  setPaused: (paused: boolean) => invoke<void>('set_paused', { paused }),
  listRunningApps: () => invoke<RunningApp[]>('list_running_apps'),
  listItems: (query: string, filter: Filter) =>
    invoke<ClipItem[]>('list_items', { query, filter }),
  getItemText: (id: string) => invoke<string>('get_item_text', { id }),
  getItemImage: (id: string, maxDim?: number) =>
    invoke<string>('get_item_image', { id, maxDim: maxDim ?? null }),
  copyItem: (id: string) => invoke<void>('copy_item', { id }),
  /** Adds text to the history AND puts it on the clipboard (color picker, format conversion). */
  addTextItem: (text: string) => invoke<void>('add_text_item', { text }),
  /** Derived text → new history entry + clipboard. */
  transformItem: (id: string, op: Transform) => invoke<void>('transform_item', { id, op }),
  mergeStack: () => invoke<void>('merge_stack'),
  exportItem: (id: string, path: string) => invoke<void>('export_item', { id, path }),
  exportData: (
    path: string,
    includeHistory: boolean,
    includeSnippets: boolean,
    passphrase: string | null
  ) => invoke<void>('export_data', { path, includeHistory, includeSnippets, passphrase }),
  importData: (path: string, passphrase: string | null) =>
    invoke<ImportStats>('import_data', { path, passphrase }),
  pinItem: (id: string, pinned: boolean) => invoke<void>('pin_item', { id, pinned }),
  deleteItem: (id: string) => invoke<void>('delete_item', { id }),
  clearHistory: () => invoke<void>('clear_history'),
  listSnippets: () => invoke<Snippet[]>('list_snippets'),
  saveSnippet: (id: string | null, name: string, text: string) =>
    invoke<string>('save_snippet', { id, name, text }),
  deleteSnippet: (id: string) => invoke<void>('delete_snippet', { id }),
  copySnippet: (id: string) => invoke<void>('copy_snippet', { id }),
  stackList: () => invoke<ClipItem[]>('stack_list'),
  stackAdd: (id: string) => invoke<void>('stack_add', { id }),
  stackRemove: (id: string) => invoke<void>('stack_remove', { id }),
  stackClear: () => invoke<void>('stack_clear'),
  stackPopCopy: () => invoke<ClipItem | null>('stack_pop_copy'),
  showPicker: () => invoke<void>('show_picker'),
  hidePicker: () => invoke<void>('hide_picker'),
  /** Resolves true when pasted directly, false when only copied (no Accessibility permission). */
  pickerPaste: (id: string) => invoke<boolean>('picker_paste', { id }),
  accessibilityStatus: () => invoke<boolean>('accessibility_status'),
  requestAccessibility: () => invoke<boolean>('request_accessibility'),
  checkUpdate: () => invoke<UpdateInfo | null>('check_update'),
  installUpdate: () => invoke<void>('install_update'),
};
