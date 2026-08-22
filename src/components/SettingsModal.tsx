import { useEffect, useState } from 'react';
import { api, Settings, UpdateInfo } from '../api';
import { Dict } from '../i18n';
import {
  ACTIONS,
  ActionDef,
  comboFromEvent,
  formatCombo,
  GLOBAL_DEFAULTS,
  hasRealModifier,
  resolveKeymap,
} from '../shortcuts';

const APP_VERSION = '0.2.0';

export function SettingsModal({
  settings,
  t,
  onClose,
  onSave,
}: {
  settings: Settings;
  t: Dict;
  onClose: () => void;
  onSave: (s: Settings) => void;
}) {
  const [s, setS] = useState<Settings>({ ...settings });
  const [updState, setUpdState] = useState<'idle' | 'checking' | 'none' | 'error'>('idle');
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  // shortcut editor: which row is recording ('toggle' | 'stackPop' | action id)
  const [recording, setRecording] = useState<string | null>(null);
  const [scError, setScError] = useState<string | null>(null);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

  // ---- shortcut editor ----------------------------------------------------
  const keymap = resolveKeymap(s);

  /** All current bindings with display labels, for conflict checks. */
  const allBindings = (): { key: string; combo: string; label: string }[] => [
    { key: 'toggle', combo: s.shortcutToggle, label: t.shortcutToggle },
    { key: 'stackPop', combo: s.shortcutStackPop, label: t.shortcutStackPop },
    ...ACTIONS.map((a) => ({ key: a.id, combo: keymap[a.id], label: a.label(t) })),
  ];

  const assign = (rowKey: string, combo: string) => {
    const conflict = allBindings().find((b) => b.key !== rowKey && b.combo === combo);
    if (conflict) {
      setScError(t.scConflict(conflict.label));
      return;
    }
    setScError(null);
    if (rowKey === 'toggle' || rowKey === 'stackPop') {
      if (!hasRealModifier(combo)) {
        setScError(t.scNeedsModifier);
        return;
      }
      set(rowKey === 'toggle' ? 'shortcutToggle' : 'shortcutStackPop', combo);
      return;
    }
    const def = ACTIONS.find((a) => a.id === rowKey)?.def;
    setS((prev) => {
      const next = { ...(prev.keymap ?? {}) };
      if (combo === def) delete next[rowKey];
      else next[rowKey] = combo;
      return { ...prev, keymap: next };
    });
  };

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === 'Escape') {
        setRecording(null);
        return;
      }
      const combo = comboFromEvent(e);
      if (!combo) return; // pure modifier — keep listening
      assign(recording, combo);
      setRecording(null);
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recording, s]);

  const resetAll = () => {
    setScError(null);
    setRecording(null);
    setS((prev) => ({
      ...prev,
      shortcutToggle: GLOBAL_DEFAULTS.shortcutToggle,
      shortcutStackPop: GLOBAL_DEFAULTS.shortcutStackPop,
      keymap: {},
    }));
  };

  const shortcutRow = (rowKey: string, label: string, combo: string, def: string) => (
    <div className="scrow" key={rowKey}>
      <span className="sclabel">{label}</span>
      {combo !== def && recording !== rowKey && (
        <button
          className="ghost screset"
          title="↺"
          onClick={() => {
            setScError(null);
            assign(rowKey, def);
          }}
        >
          ↺
        </button>
      )}
      <button
        className={`sckey ${recording === rowKey ? 'recording' : ''}`}
        onClick={() => {
          setScError(null);
          setRecording(recording === rowKey ? null : rowKey);
        }}
      >
        {recording === rowKey ? t.scRecording : formatCombo(combo)}
      </button>
    </div>
  );

  const actionRows = (group: ActionDef['group']) =>
    ACTIONS.filter((a) => a.group === group).map((a) =>
      shortcutRow(a.id, a.label(t), keymap[a.id], a.def)
    );

  const checkUpdates = () => {
    setUpdState('checking');
    setUpdate(null);
    api
      .checkUpdate()
      .then((u) => {
        if (u) {
          setUpdate(u);
          setUpdState('idle');
        } else {
          setUpdState('none');
        }
      })
      .catch(() => setUpdState('error'));
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>
          <span className="brand">
            <span className="name">clipon</span>
            <span className="dot">.</span>
          </span>{' '}
          — {t.settings}
        </h2>

        <label className="field">
          <span>{t.language}</span>
          <select value={s.language} onChange={(e) => set('language', e.target.value as 'de' | 'en')}>
            <option value="de">Deutsch</option>
            <option value="en">English</option>
          </select>
        </label>

        <label className="field">
          <span>{t.historyLimit}</span>
          <input
            type="number"
            min={10}
            max={10000}
            value={s.historyLimit}
            onChange={(e) => set('historyLimit', Math.max(10, Number(e.target.value) || 10))}
          />
        </label>

        <label className="field">
          <span>{t.maxTextKb}</span>
          <input
            type="number"
            min={0}
            value={s.maxTextKb}
            onChange={(e) => set('maxTextKb', Math.max(0, Number(e.target.value) || 0))}
          />
        </label>

        <label className="check">
          <input
            type="checkbox"
            checked={s.captureImages}
            onChange={(e) => set('captureImages', e.target.checked)}
          />
          {t.captureImages}
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={s.keepPinnedOnClear}
            onChange={(e) => set('keepPinnedOnClear', e.target.checked)}
          />
          {t.keepPinnedOnClear}
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={s.autostart}
            onChange={(e) => set('autostart', e.target.checked)}
          />
          {t.autostart}
        </label>

        <div className="sep" />
        <div className="fieldlabel">{t.shortcuts}</div>
        <div className="note">{t.shortcutHint}</div>
        {scError && <div className="note scerror">{scError}</div>}
        <div className="scgroup">{t.scGroupGlobal}</div>
        {shortcutRow('toggle', t.shortcutToggle, s.shortcutToggle, GLOBAL_DEFAULTS.shortcutToggle)}
        {shortcutRow(
          'stackPop',
          t.shortcutStackPop,
          s.shortcutStackPop,
          GLOBAL_DEFAULTS.shortcutStackPop
        )}
        <div className="scgroup">{t.scGroupNav}</div>
        {actionRows('nav')}
        <div className="scgroup">{t.scGroupList}</div>
        {actionRows('list')}
        <div className="scgroup">{t.scGroupTools}</div>
        {actionRows('tools')}
        <div className="scrow">
          <span className="sclabel" />
          <button className="ghost" onClick={resetAll}>
            {t.scResetAll}
          </button>
        </div>

        <div className="sep" />
        <div className="fieldlabel">{t.updates}</div>
        <div className="updatebox">
          <span>
            {t.version} {APP_VERSION}
          </span>
          <button onClick={checkUpdates} disabled={updState === 'checking'}>
            {updState === 'checking' ? t.checking : t.checkUpdates}
          </button>
          {updState === 'none' && <span>{t.upToDate}</span>}
          {updState === 'error' && <span style={{ color: 'var(--red)' }}>{t.updateError}</span>}
          {update && (
            <>
              <span>
                {t.updateAvailable} <strong>{update.version}</strong>
              </span>
              <button
                className="primary"
                disabled={installing}
                onClick={() => {
                  setInstalling(true);
                  api.installUpdate().catch(() => setInstalling(false));
                }}
              >
                {t.installUpdate}
              </button>
            </>
          )}
        </div>
        {update?.notes && <div className="note">{update.notes}</div>}

        <div className="note">{t.privacyNote}</div>

        <div className="btnrow">
          <button onClick={onClose}>{t.cancel}</button>
          <button
            className="primary"
            disabled={!s.shortcutToggle.trim() || !s.shortcutStackPop.trim()}
            onClick={() => onSave(s)}
          >
            {t.save}
          </button>
        </div>
      </div>
    </div>
  );
}
