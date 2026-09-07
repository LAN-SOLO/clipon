import { useEffect, useState } from 'react';
import { open, save } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';
import { api, RunningApp, Settings, UpdateInfo } from '../api';
import { Dict } from '../i18n';
import {
  ACTIONS,
  ActionDef,
  comboFromEvent,
  formatCombo,
  GLOBAL_DEFAULTS,
  hasRealModifier,
  isMac,
  resolveKeymap,
} from '../shortcuts';

const APP_VERSION = '0.4.0';
const AX_SETTINGS_URL =
  'x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility';

type GlobalRow = 'toggle' | 'stackPop' | 'picker';
const GLOBAL_FIELD: Record<GlobalRow, keyof typeof GLOBAL_DEFAULTS> = {
  toggle: 'shortcutToggle',
  stackPop: 'shortcutStackPop',
  picker: 'shortcutPicker',
};

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
  // shortcut editor: which row is recording (global row key | action id)
  const [recording, setRecording] = useState<string | null>(null);
  const [scError, setScError] = useState<string | null>(null);
  // privacy: ignore list editing
  const [running, setRunning] = useState<RunningApp[] | null>(null);
  const [manualApp, setManualApp] = useState('');
  // data: export / import
  const [expHistory, setExpHistory] = useState(true);
  const [expSnippets, setExpSnippets] = useState(true);
  const [passphrase, setPassphrase] = useState('');
  const [dataMsg, setDataMsg] = useState<{ text: string; kind: 'ok' | 'error' } | null>(null);
  const [axOk, setAxOk] = useState<boolean | null>(null);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

  useEffect(() => {
    if (isMac) api.accessibilityStatus().then(setAxOk).catch(() => setAxOk(false));
  }, []);

  // ---- shortcut editor ----------------------------------------------------
  const keymap = resolveKeymap(s);

  /** All current bindings with display labels, for conflict checks. */
  const allBindings = (): { key: string; combo: string; label: string }[] => [
    { key: 'toggle', combo: s.shortcutToggle, label: t.shortcutToggle },
    { key: 'stackPop', combo: s.shortcutStackPop, label: t.shortcutStackPop },
    { key: 'picker', combo: s.shortcutPicker, label: t.shortcutPicker },
    ...ACTIONS.map((a) => ({ key: a.id, combo: keymap[a.id], label: a.label(t) })),
  ];

  const assign = (rowKey: string, combo: string) => {
    const conflict = allBindings().find((b) => b.key !== rowKey && b.combo === combo);
    if (conflict) {
      setScError(t.scConflict(conflict.label));
      return;
    }
    setScError(null);
    if (rowKey in GLOBAL_FIELD) {
      if (!hasRealModifier(combo)) {
        setScError(t.scNeedsModifier);
        return;
      }
      set(GLOBAL_FIELD[rowKey as GlobalRow], combo);
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
      shortcutPicker: GLOBAL_DEFAULTS.shortcutPicker,
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

  // ---- privacy: ignored apps ---------------------------------------------
  const addIgnored = (id: string) => {
    const clean = id.trim();
    if (!clean) return;
    if (!s.ignoredApps.some((x) => x.toLowerCase() === clean.toLowerCase())) {
      set('ignoredApps', [...s.ignoredApps, clean]);
    }
    setManualApp('');
  };

  const loadRunning = () => {
    api
      .listRunningApps()
      .then((apps) => setRunning(apps.filter((a) => !s.ignoredApps.includes(a.bundleId))))
      .catch(() => setRunning([]));
  };

  // ---- data: export / import ---------------------------------------------
  const pass = passphrase.trim() ? passphrase : null;

  const doExport = () => {
    setDataMsg(null);
    const date = new Date().toISOString().slice(0, 10);
    const ext = pass ? 'clipon-backup' : 'json';
    save({
      defaultPath: `clipon-backup-${date}.${ext}`,
      filters: [{ name: 'clipon backup', extensions: ['clipon-backup', 'json'] }],
    })
      .then((path) => {
        if (!path) return;
        return api
          .exportData(path, expHistory, expSnippets, pass)
          .then(() => setDataMsg({ text: t.exportDone, kind: 'ok' }));
      })
      .catch((e) => setDataMsg({ text: `${t.exportError} ${String(e)}`, kind: 'error' }));
  };

  const doImport = () => {
    setDataMsg(null);
    open({
      multiple: false,
      directory: false,
      filters: [{ name: 'clipon backup', extensions: ['clipon-backup', 'json'] }],
    })
      .then((path) => {
        if (!path || Array.isArray(path)) return;
        return api
          .importData(path, pass)
          .then((r) =>
            setDataMsg({
              text: t.importResult(r.itemsAdded, r.itemsSkipped + r.snippetsSkipped, r.snippetsAdded),
              kind: 'ok',
            })
          );
      })
      .catch((e) => {
        const msg = String(e);
        setDataMsg({
          text: msg === 'passphrase-required' ? t.importNeedsPass : `${t.importError} ${msg}`,
          kind: 'error',
        });
      });
  };

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

        <div className="row2">
          <label className="field grow1">
            <span>{t.language}</span>
            <select value={s.language} onChange={(e) => set('language', e.target.value as 'de' | 'en')}>
              <option value="de">Deutsch</option>
              <option value="en">English</option>
            </select>
          </label>
          <label className="field grow1">
            <span>{t.themeLabel}</span>
            <select
              value={s.theme}
              onChange={(e) => set('theme', e.target.value as Settings['theme'])}
            >
              <option value="dark">{t.themeDark}</option>
              <option value="light">{t.themeLight}</option>
            </select>
          </label>
        </div>

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
        <div className="fieldlabel">{t.privacy}</div>
        <div className="scgroup">{t.ignoredApps}</div>
        <div className="note">{t.ignoredAppsHint}</div>
        <div className="applist">
          {s.ignoredApps.length === 0 && <div className="note">{t.ignoredAppsEmpty}</div>}
          {s.ignoredApps.map((id) => (
            <div className="approw" key={id}>
              <code>{id}</code>
              <button
                className="ghost icon"
                title={t.remove}
                onClick={() =>
                  set(
                    'ignoredApps',
                    s.ignoredApps.filter((x) => x !== id)
                  )
                }
              >
                ✕
              </button>
            </div>
          ))}
        </div>
        <div className="addrow">
          {running === null ? (
            <button onClick={loadRunning}>{t.ignoredAppsAddRunning}</button>
          ) : (
            <select
              value=""
              onChange={(e) => {
                addIgnored(e.target.value);
                setRunning(null);
              }}
            >
              <option value="">{t.ignoredAppsAddRunning}</option>
              {running.map((a) => (
                <option key={a.bundleId} value={a.bundleId}>
                  {a.name} — {a.bundleId}
                </option>
              ))}
            </select>
          )}
        </div>
        <div className="addrow">
          <input
            type="text"
            placeholder={t.ignoredAppsManual}
            value={manualApp}
            onChange={(e) => setManualApp(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') addIgnored(manualApp);
            }}
          />
          <button disabled={!manualApp.trim()} onClick={() => addIgnored(manualApp)}>
            {t.add}
          </button>
        </div>

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
        {shortcutRow('picker', t.shortcutPicker, s.shortcutPicker, GLOBAL_DEFAULTS.shortcutPicker)}
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

        {isMac && (
          <>
            <div className="sep" />
            <div className="fieldlabel">{t.axTitle}</div>
            <div className="updatebox">
              <span className={`pill ${axOk ? 'ok' : 'warn'}`}>
                {axOk === null ? '…' : axOk ? t.axGranted : t.axMissing}
              </span>
              {!axOk && (
                <button
                  onClick={() =>
                    api
                      .requestAccessibility()
                      .then(setAxOk)
                      .catch(() => {})
                  }
                >
                  {t.axRequest}
                </button>
              )}
              <button className="ghost" onClick={() => openUrl(AX_SETTINGS_URL).catch(() => {})}>
                {t.axOpenSettings}
              </button>
            </div>
            <div className="note">{t.axExplain}</div>
          </>
        )}

        <div className="sep" />
        <div className="fieldlabel">{t.dataSection}</div>
        <label className="check">
          <input type="checkbox" checked={expHistory} onChange={(e) => setExpHistory(e.target.checked)} />
          {t.exportIncludeHistory}
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={expSnippets}
            onChange={(e) => setExpSnippets(e.target.checked)}
          />
          {t.exportIncludeSnippets}
        </label>
        <label className="field">
          <span>{t.exportPassphrase}</span>
          <input
            type="password"
            value={passphrase}
            onChange={(e) => setPassphrase(e.target.value)}
            autoComplete="off"
          />
        </label>
        <div className="updatebox">
          <button disabled={!expHistory && !expSnippets} onClick={doExport}>
            {t.exportData}
          </button>
          <button onClick={doImport}>{t.importData}</button>
        </div>
        {dataMsg && (
          <div className={`note ${dataMsg.kind === 'error' ? 'scerror' : 'ok'}`}>{dataMsg.text}</div>
        )}

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
            disabled={
              !s.shortcutToggle.trim() || !s.shortcutStackPop.trim() || !s.shortcutPicker.trim()
            }
            onClick={() => onSave(s)}
          >
            {t.save}
          </button>
        </div>
      </div>
    </div>
  );
}
