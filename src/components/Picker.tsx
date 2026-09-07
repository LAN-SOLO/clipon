import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, ClipItem, Settings } from '../api';
import { dicts, Lang } from '../i18n';
import { IconLogo } from '../icons';

const MAX_RESULTS = 10;

/** Compact history picker in its own always-on-top window, opened by the
 *  global shortcut. Keyboard-first: type to filter, ↑↓ select, ↩ paste,
 *  1–9 pick directly, Esc closes. */
export function Picker() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [query, setQuery] = useState('');
  const [items, setItems] = useState<ClipItem[]>([]);
  const [idx, setIdx] = useState(0);
  const [axOk, setAxOk] = useState<boolean | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  const lang: Lang = settings?.language ?? 'de';
  const t = dicts[lang];

  const refresh = useCallback((q: string) => {
    api
      .listItems(q, 'all')
      .then((list) => {
        setItems(list.slice(0, MAX_RESULTS));
        setIdx(0);
      })
      .catch(() => {});
  }, []);

  const reset = useCallback(() => {
    setQuery('');
    refresh('');
    api.getSettings().then(setSettings).catch(() => {});
    api.accessibilityStatus().then(setAxOk).catch(() => setAxOk(false));
    window.setTimeout(() => inputRef.current?.focus(), 0);
  }, [refresh]);

  useEffect(reset, [reset]);

  // theme/language follow the main window's settings, also while hidden
  useEffect(() => {
    if (!settings) return;
    document.documentElement.dataset.theme = settings.theme;
  }, [settings]);

  useEffect(() => {
    const subs = [
      listen('picker-open', reset),
      listen('history-changed', () => refresh(query)),
      listen('settings-changed', () => {
        api.getSettings().then(setSettings).catch(() => {});
      }),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, [reset, refresh, query]);

  useEffect(() => refresh(query), [query, refresh]);

  const paste = (item: ClipItem | undefined) => {
    if (!item) return;
    api.pickerPaste(item.id).catch(() => {});
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      api.hidePicker();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setIdx((i) => Math.min(items.length - 1, i + 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setIdx((i) => Math.max(0, i - 1));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      paste(items[idx]);
    } else if (/^[1-9]$/.test(e.key) && (e.metaKey || e.ctrlKey || query === '')) {
      e.preventDefault();
      paste(items[Number(e.key) - 1]);
    }
  };

  useEffect(() => {
    document.querySelector('.picker .row.selected')?.scrollIntoView({ block: 'nearest' });
  }, [idx]);

  return (
    <div className="picker" onKeyDown={onKey}>
      <div className="pickerhead">
        <IconLogo size={16} />
        <input
          ref={inputRef}
          type="text"
          placeholder={t.pickerSearch}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          autoFocus
        />
      </div>
      <div className="list">
        {items.length === 0 && <div className="empty">{t.pickerEmpty}</div>}
        {items.map((item, i) => (
          <div
            key={item.id}
            className={`row ${i === idx ? 'selected' : ''}`}
            onMouseEnter={() => setIdx(i)}
            onClick={() => paste(item)}
          >
            <span className="meta">{i + 1}</span>
            <span className={`kbadge ${item.kind === 'image' ? 'image' : ''}`}>
              {item.kind === 'image' ? 'img' : item.detected === 'url' ? 'url' : 'txt'}
            </span>
            {item.detected === 'color' && (
              <span className="swatch" style={{ background: item.preview }} />
            )}
            <span className="preview">{item.preview}</span>
            {item.sourceAppName && <span className="meta">{item.sourceAppName}</span>}
          </div>
        ))}
      </div>
      <div className="pickerfoot">
        {axOk === null ? '' : axOk ? t.pickerPasteHint : t.pickerCopyHint}
      </div>
    </div>
  );
}
