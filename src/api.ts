import { invoke } from '@tauri-apps/api/core';

export type ItemKind = 'text' | 'image';
export type Detected = 'plain' | 'url' | 'email' | 'color' | 'code';
export type Filter = 'all' | 'pinned' | 'text' | 'links' | 'images' | 'colors';

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
  /** In-app shortcut overrides (action id → combo); defaults live in shortcuts.ts. */
  keymap: Record<string, string>;
  paused: boolean;
}

export interface UpdateInfo {
  version: string;
  notes: string | null;
  date: string | null;
}

export const api = {
  getSettings: () => invoke<Settings>('get_settings'),
  setSettings: (s: Settings) => invoke<void>('set_settings', { new: s }),
  setPaused: (paused: boolean) => invoke<void>('set_paused', { paused }),
  listItems: (query: string, filter: Filter) =>
    invoke<ClipItem[]>('list_items', { query, filter }),
  getItemText: (id: string) => invoke<string>('get_item_text', { id }),
  getItemImage: (id: string, maxDim?: number) =>
    invoke<string>('get_item_image', { id, maxDim: maxDim ?? null }),
  copyItem: (id: string) => invoke<void>('copy_item', { id }),
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
  checkUpdate: () => invoke<UpdateInfo | null>('check_update'),
  installUpdate: () => invoke<void>('install_update'),
};
