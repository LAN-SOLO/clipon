import { Dict, Lang } from './i18n';
import { formatCombo } from './shortcuts';

/** The backend reports errors as stable codes — "code" or "code: detail",
 *  several joined with " | " — never as translated prose. This maps them to
 *  the UI language; the detail (an OS or library message) is appended in
 *  parentheses. Unknown messages are shown as they are. */
export function errorText(err: unknown, t: Dict, lang: Lang): string {
  const raw = err instanceof Error ? err.message : String(err ?? '');
  return raw
    .split(' | ')
    .map((part) => one(part.trim(), t, lang))
    .filter(Boolean)
    .join(' · ');
}

function one(part: string, t: Dict, lang: Lang): string {
  const m = /^([a-z][a-z-]*)(?::\s*([\s\S]*))?$/.exec(part);
  if (!m) return part;
  const [, code, detail] = m;
  const label = (t.errors as Record<string, string>)[code];
  if (code === 'shortcut' && detail) {
    // "shortcut: CmdOrCtrl+Shift+V: <reason>"
    const i = detail.indexOf(': ');
    const combo = i === -1 ? detail : detail.slice(0, i);
    const why = i === -1 ? '' : detail.slice(i + 2);
    return t.errShortcut(formatCombo(combo, lang)) + (why ? ` (${why})` : '');
  }
  if (!label) return part;
  // nested codes ("save-failed: io: …") are translated as well
  return detail ? `${label} (${one(detail.trim(), t, lang)})` : label;
}

/** True when the error (or one of several) carries this code. */
export function hasCode(err: unknown, code: string): boolean {
  const raw = err instanceof Error ? err.message : String(err ?? '');
  return raw.split(' | ').some((p) => p.trim() === code || p.trim().startsWith(`${code}:`));
}
