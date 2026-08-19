import { useState } from 'react';
import { api, Settings, UpdateInfo } from '../api';
import { Dict } from '../i18n';

const APP_VERSION = '0.1.0';

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

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setS((prev) => ({ ...prev, [key]: value }));

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
        <label className="field">
          <span>{t.shortcutToggle}</span>
          <input
            type="text"
            value={s.shortcutToggle}
            onChange={(e) => set('shortcutToggle', e.target.value)}
          />
        </label>
        <label className="field">
          <span>{t.shortcutStackPop}</span>
          <input
            type="text"
            value={s.shortcutStackPop}
            onChange={(e) => set('shortcutStackPop', e.target.value)}
          />
        </label>
        <div className="note">{t.shortcutHint}</div>

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
