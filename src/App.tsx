import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, ClipItem, Filter, Settings, Snippet } from './api';
import { dicts, Dict, Lang } from './i18n';
import {
  IconCopy,
  IconEdit,
  IconGear,
  IconPause,
  IconPin,
  IconPlay,
  IconPlus,
  IconStack,
  IconTrash,
  IconX,
} from './icons';
import { SettingsModal } from './components/SettingsModal';

type View = 'history' | 'stack' | 'snippets';

const thumbCache = new Map<string, string>();

function Thumb({ id, maxDim, className }: { id: string; maxDim: number; className?: string }) {
  const [src, setSrc] = useState<string | null>(thumbCache.get(`${id}:${maxDim}`) ?? null);
  useEffect(() => {
    let alive = true;
    const key = `${id}:${maxDim}`;
    if (thumbCache.has(key)) {
      setSrc(thumbCache.get(key)!);
      return;
    }
    api
      .getItemImage(id, maxDim)
      .then((data) => {
        thumbCache.set(key, data);
        if (alive) setSrc(data);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [id, maxDim]);
  return src ? <img className={className} src={src} alt="" /> : null;
}

function fmtTime(iso: string, lang: Lang): string {
  const d = new Date(iso);
  const diff = (Date.now() - d.getTime()) / 1000;
  if (diff < 60) return lang === 'de' ? 'gerade' : 'now';
  if (diff < 3600) return `${Math.floor(diff / 60)} min`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h`;
  return d.toLocaleDateString(lang === 'de' ? 'de-DE' : 'en-US', {
    day: '2-digit',
    month: '2-digit',
  });
}

function typeLabel(item: ClipItem, t: Dict): string {
  if (item.kind === 'image') return t.typeImage;
  switch (item.detected) {
    case 'url':
      return t.typeUrl;
    case 'email':
      return t.typeEmail;
    case 'color':
      return t.typeColor;
    case 'code':
      return t.typeCode;
    default:
      return t.typePlain;
  }
}

function badge(item: ClipItem): { cls: string; label: string } {
  if (item.kind === 'image') return { cls: 'image', label: 'img' };
  switch (item.detected) {
    case 'url':
      return { cls: 'url', label: 'url' };
    case 'email':
      return { cls: 'url', label: '@' };
    case 'color':
      return { cls: 'color', label: 'col' };
    case 'code':
      return { cls: 'code', label: '{ }' };
    default:
      return { cls: '', label: 'txt' };
  }
}

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [view, setView] = useState<View>('history');
  const [filter, setFilter] = useState<Filter>('all');
  const [query, setQuery] = useState('');
  const [items, setItems] = useState<ClipItem[]>([]);
  const [stack, setStack] = useState<ClipItem[]>([]);
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detailText, setDetailText] = useState<string | null>(null);
  const [paused, setPaused] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [editSnippet, setEditSnippet] = useState<Snippet | 'new' | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const lang: Lang = settings?.language ?? 'de';
  const t = dicts[lang];

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1400);
  }, []);

  const refreshItems = useCallback(() => {
    api.listItems(query, filter).then(setItems).catch(() => {});
  }, [query, filter]);
  const refreshStack = useCallback(() => {
    api.stackList().then(setStack).catch(() => {});
  }, []);
  const refreshSnippets = useCallback(() => {
    api.listSnippets().then(setSnippets).catch(() => {});
  }, []);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setPaused(s.paused);
    });
    refreshStack();
    refreshSnippets();
  }, [refreshStack, refreshSnippets]);

  useEffect(refreshItems, [refreshItems]);

  useEffect(() => {
    const subs = [
      listen('history-changed', refreshItems),
      listen('stack-changed', refreshStack),
      listen<boolean>('paused-changed', (e) => setPaused(e.payload)),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, [refreshItems, refreshStack]);

  const selected = items.find((i) => i.id === selectedId) ?? null;

  useEffect(() => {
    setDetailText(null);
    if (selected?.kind === 'text') {
      api.getItemText(selected.id).then(setDetailText).catch(() => {});
    }
  }, [selectedId, selected?.kind, selected?.id]);

  const copyItem = (id: string) => {
    api
      .copyItem(id)
      .then(() => showToast(t.copied))
      .catch(() => {});
  };

  const togglePaused = () => {
    api.setPaused(!paused).then(() => setPaused(!paused));
  };

  const navFilters: { key: Filter; label: string }[] = [
    { key: 'all', label: t.all },
    { key: 'pinned', label: t.pinned },
    { key: 'text', label: t.text },
    { key: 'links', label: t.links },
    { key: 'colors', label: t.colors },
    { key: 'images', label: t.images },
  ];

  if (!settings) return null;

  return (
    <div className="app">
      <div className="sidebar">
        <div className="brand">
          <span className="name">clipon</span>
          <span className="dot">.</span>
        </div>
        {navFilters.map((f) => (
          <button
            key={f.key}
            className={`navbtn ${view === 'history' && filter === f.key ? 'active' : ''}`}
            onClick={() => {
              setView('history');
              setFilter(f.key);
            }}
          >
            {f.label}
          </button>
        ))}
        <div className="section">// tools</div>
        <button
          className={`navbtn ${view === 'stack' ? 'active' : ''}`}
          onClick={() => setView('stack')}
        >
          <IconStack /> {t.stack}
          {stack.length > 0 && <span className="count">{stack.length}</span>}
        </button>
        <button
          className={`navbtn ${view === 'snippets' ? 'active' : ''}`}
          onClick={() => setView('snippets')}
        >
          <IconEdit /> {t.snippets}
          {snippets.length > 0 && <span className="count">{snippets.length}</span>}
        </button>
        <div className="spacer" />
        <div className={`statusline ${paused ? 'paused' : ''}`}>
          <span className="led" />
          {paused ? t.paused : t.capturing}
        </div>
        <button className="navbtn" onClick={togglePaused}>
          {paused ? <IconPlay /> : <IconPause />}
          {paused ? t.resume : t.pause}
        </button>
        <button className="navbtn" onClick={() => setShowSettings(true)}>
          <IconGear /> {t.settings}
        </button>
      </div>

      {view === 'history' && (
        <>
          <div className="main">
            <div className="toolbar">
              <input
                type="text"
                placeholder={t.search}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <button
                className="ghost"
                title={t.clear}
                onClick={() => {
                  if (window.confirm(t.clearConfirm)) api.clearHistory();
                }}
              >
                <IconTrash />
              </button>
            </div>
            <div className="list">
              {items.length === 0 && (
                <div className="empty">
                  {query || filter !== 'all' ? t.emptyFiltered : t.empty}
                </div>
              )}
              {items.map((item) => {
                const b = badge(item);
                return (
                  <div
                    key={item.id}
                    className={`row ${selectedId === item.id ? 'selected' : ''}`}
                    onClick={() => setSelectedId(item.id)}
                    onDoubleClick={() => copyItem(item.id)}
                  >
                    <span className={`kbadge ${b.cls}`}>{b.label}</span>
                    {item.detected === 'color' && (
                      <span className="swatch" style={{ background: item.preview }} />
                    )}
                    {item.kind === 'image' ? (
                      <>
                        <Thumb id={item.id} maxDim={96} className="thumb" />
                        <span className="preview mono">{item.preview}</span>
                      </>
                    ) : (
                      <span
                        className={`preview ${
                          item.detected === 'code' || item.detected === 'color' ? 'mono' : ''
                        }`}
                      >
                        {item.preview}
                      </span>
                    )}
                    {item.pinned && <span className="pinmark">●</span>}
                    <span className="meta">{fmtTime(item.lastCopiedAt, lang)}</span>
                    <span className="actions">
                      <button className="ghost icon" title={t.copy} onClick={(e) => { e.stopPropagation(); copyItem(item.id); }}>
                        <IconCopy />
                      </button>
                      <button className="ghost icon" title={item.pinned ? t.unpin : t.pin} onClick={(e) => { e.stopPropagation(); api.pinItem(item.id, !item.pinned); }}>
                        <IconPin />
                      </button>
                      <button className="ghost icon" title={t.toStack} onClick={(e) => { e.stopPropagation(); api.stackAdd(item.id); }}>
                        <IconStack />
                      </button>
                      <button className="ghost icon" title={t.delete} onClick={(e) => { e.stopPropagation(); if (selectedId === item.id) setSelectedId(null); api.deleteItem(item.id); }}>
                        <IconTrash />
                      </button>
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="detail">
            {!selected && <div className="placeholder">{t.select}</div>}
            {selected && (
              <>
                <div className="body">
                  {selected.kind === 'image' ? (
                    <Thumb id={selected.id} maxDim={640} />
                  ) : (
                    <div
                      className={`content ${
                        selected.detected === 'code' ? 'mono' : ''
                      }`}
                    >
                      {detailText ?? '…'}
                    </div>
                  )}
                </div>
                <div className="stats">
                  <span className="k">{t.type}</span>
                  <span>{typeLabel(selected, t)}</span>
                  {selected.kind === 'text' && (
                    <>
                      <span className="k">{t.chars}</span>
                      <span>
                        {selected.chars}
                        {detailText
                          ? ` · ${detailText.trim().split(/\s+/).filter(Boolean).length} ${t.words}`
                          : ''}
                      </span>
                    </>
                  )}
                  <span className="k">{t.copiedTimes}</span>
                  <span>{selected.timesCopied}×</span>
                  <span className="k">{t.added}</span>
                  <span>{fmtTime(selected.createdAt, lang)}</span>
                  <span className="k">{t.lastUsed}</span>
                  <span>{fmtTime(selected.lastCopiedAt, lang)}</span>
                </div>
                <div className="btns">
                  <button className="primary" onClick={() => copyItem(selected.id)}>
                    {t.copy}
                  </button>
                  <button onClick={() => api.pinItem(selected.id, !selected.pinned)}>
                    {selected.pinned ? t.unpin : t.pin}
                  </button>
                  <button onClick={() => api.stackAdd(selected.id)}>{t.toStack}</button>
                  <button
                    className="danger"
                    onClick={() => {
                      setSelectedId(null);
                      api.deleteItem(selected.id);
                    }}
                  >
                    {t.delete}
                  </button>
                </div>
              </>
            )}
          </div>
        </>
      )}

      {view === 'stack' && (
        <div className="main">
          <div className="panelhead">
            <span className="hint">
              {t.stackHint} <kbd>{settings.shortcutStackPop}</kbd>
            </span>
            <button
              className="primary"
              disabled={stack.length === 0}
              onClick={() =>
                api.stackPopCopy().then((item) => {
                  if (item) showToast(`${t.copied}: ${item.preview.slice(0, 40)}`);
                })
              }
            >
              {t.popNext}
            </button>
            <button disabled={stack.length === 0} onClick={() => api.stackClear()}>
              {t.stackClear}
            </button>
          </div>
          <div className="list">
            {stack.length === 0 && <div className="empty">{t.stackEmpty}</div>}
            {stack.map((item, idx) => {
              const b = badge(item);
              return (
                <div key={item.id} className="row" onDoubleClick={() => copyItem(item.id)}>
                  <span className="meta">{idx + 1}.</span>
                  <span className={`kbadge ${b.cls}`}>{b.label}</span>
                  {item.kind === 'image' && <Thumb id={item.id} maxDim={96} className="thumb" />}
                  <span className="preview">{item.preview}</span>
                  <span className="actions">
                    <button className="ghost icon" title={t.remove} onClick={() => api.stackRemove(item.id)}>
                      <IconX />
                    </button>
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {view === 'snippets' && (
        <div className="main">
          <div className="panelhead">
            <span className="hint">{t.snippetsHint}</span>
            <button className="primary" onClick={() => setEditSnippet('new')}>
              <IconPlus /> {t.newSnippet}
            </button>
          </div>
          <div className="list">
            {snippets.length === 0 && <div className="empty">{t.snippetsEmpty}</div>}
            {snippets.map((s) => (
              <div
                key={s.id}
                className="row"
                onClick={() => api.copySnippet(s.id).then(() => showToast(t.copied))}
              >
                <span className="kbadge">txt</span>
                <span className="preview">
                  <strong>{s.name}</strong>
                  {'  —  '}
                  {s.text.split('\n')[0].slice(0, 80)}
                </span>
                <span className="actions">
                  <button className="ghost icon" title={t.edit} onClick={(e) => { e.stopPropagation(); setEditSnippet(s); }}>
                    <IconEdit />
                  </button>
                  <button className="ghost icon" title={t.delete} onClick={(e) => { e.stopPropagation(); api.deleteSnippet(s.id).then(refreshSnippets); }}>
                    <IconTrash />
                  </button>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {showSettings && (
        <SettingsModal
          settings={settings}
          t={t}
          onClose={() => setShowSettings(false)}
          onSave={(s) => {
            api.setSettings(s).then(() => {
              setSettings(s);
              setPaused(s.paused);
              setShowSettings(false);
            });
          }}
        />
      )}

      {editSnippet && (
        <SnippetEditor
          t={t}
          snippet={editSnippet === 'new' ? null : editSnippet}
          onClose={() => setEditSnippet(null)}
          onSaved={() => {
            setEditSnippet(null);
            refreshSnippets();
          }}
        />
      )}

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

function SnippetEditor({
  t,
  snippet,
  onClose,
  onSaved,
}: {
  t: Dict;
  snippet: Snippet | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(snippet?.name ?? '');
  const [text, setText] = useState(snippet?.text ?? '');
  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>{snippet ? t.edit : t.newSnippet}</h2>
        <label className="field">
          <span>{t.snippetName}</span>
          <input type="text" value={name} onChange={(e) => setName(e.target.value)} autoFocus />
        </label>
        <label className="field">
          <span>{t.snippetText}</span>
          <textarea rows={6} value={text} onChange={(e) => setText(e.target.value)} />
        </label>
        <div className="btnrow">
          <button onClick={onClose}>{t.cancel}</button>
          <button
            className="primary"
            disabled={!name.trim() || !text}
            onClick={() => api.saveSnippet(snippet?.id ?? null, name, text).then(onSaved)}
          >
            {t.save}
          </button>
        </div>
      </div>
    </div>
  );
}
