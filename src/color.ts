// Parsing & Umrechnung für Farb-Einträge: Hex, rgb()/rgba(), hsl()/hsla().
// Genutzt von der Farb-Detailansicht (Format-Umrechnung) und dem Farbwähler.

export interface Rgba {
  r: number; // 0–255
  g: number;
  b: number;
  a: number; // 0–1
}

const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

export function parseColor(input: string): Rgba | null {
  const t = input.trim();
  if (t.startsWith('#')) return parseHex(t.slice(1));
  const m = t.match(/^(rgba?|hsla?)\(\s*([^)]*)\)$/i);
  if (!m) return null;
  const fn = m[1].toLowerCase();
  const parts = m[2]
    .split(/[,\s/]+/)
    .map((p) => p.trim())
    .filter(Boolean);
  if (parts.length < 3) return null;
  if (fn.startsWith('rgb')) {
    const [r, g, b] = parts.slice(0, 3).map((p) => channel(p));
    if ([r, g, b].some((v) => v === null)) return null;
    return { r: r!, g: g!, b: b!, a: alpha(parts[3]) };
  }
  const h = parseFloat(parts[0]);
  const s = parseFloat(parts[1]) / 100;
  const l = parseFloat(parts[2]) / 100;
  if ([h, s, l].some(Number.isNaN)) return null;
  return { ...hslToRgb(h, clamp(s, 0, 1), clamp(l, 0, 1)), a: alpha(parts[3]) };
}

function parseHex(hex: string): Rgba | null {
  if (!/^[0-9a-f]+$/i.test(hex)) return null;
  if (hex.length === 3) {
    const [r, g, b] = hex.split('').map((c) => parseInt(c + c, 16));
    return { r, g, b, a: 1 };
  }
  if (hex.length === 6 || hex.length === 8) {
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    const a = hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1;
    return { r, g, b, a };
  }
  return null;
}

/** "255" oder "100%" → 0–255 */
function channel(p: string): number | null {
  const pct = p.endsWith('%');
  const v = parseFloat(p);
  if (Number.isNaN(v)) return null;
  return Math.round(clamp(pct ? (v / 100) * 255 : v, 0, 255));
}

function alpha(p: string | undefined): number {
  if (p === undefined) return 1;
  const pct = p.endsWith('%');
  const v = parseFloat(p);
  if (Number.isNaN(v)) return 1;
  return clamp(pct ? v / 100 : v, 0, 1);
}

function hslToRgb(h: number, s: number, l: number): { r: number; g: number; b: number } {
  const hh = ((h % 360) + 360) % 360;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((hh / 60) % 2) - 1));
  const m = l - c / 2;
  const [r1, g1, b1] =
    hh < 60 ? [c, x, 0] :
    hh < 120 ? [x, c, 0] :
    hh < 180 ? [0, c, x] :
    hh < 240 ? [0, x, c] :
    hh < 300 ? [x, 0, c] : [c, 0, x];
  return {
    r: Math.round((r1 + m) * 255),
    g: Math.round((g1 + m) * 255),
    b: Math.round((b1 + m) * 255),
  };
}

export function fmtHex(c: Rgba): string {
  const h = (v: number) => v.toString(16).padStart(2, '0');
  const base = `#${h(c.r)}${h(c.g)}${h(c.b)}`;
  return c.a < 1 ? `${base}${h(Math.round(c.a * 255))}` : base;
}

export function fmtRgb(c: Rgba): string {
  return c.a < 1
    ? `rgba(${c.r}, ${c.g}, ${c.b}, ${trim(c.a)})`
    : `rgb(${c.r}, ${c.g}, ${c.b})`;
}

export function fmtHsl(c: Rgba): string {
  const r = c.r / 255;
  const g = c.g / 255;
  const b = c.b / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  let h = 0;
  let s = 0;
  if (d > 0) {
    s = d / (1 - Math.abs(2 * l - 1));
    if (max === r) h = 60 * (((g - b) / d) % 6);
    else if (max === g) h = 60 * ((b - r) / d + 2);
    else h = 60 * ((r - g) / d + 4);
    if (h < 0) h += 360;
  }
  const hs = `${Math.round(h)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%`;
  return c.a < 1 ? `hsla(${hs}, ${trim(c.a)})` : `hsl(${hs})`;
}

const trim = (a: number) => String(Math.round(a * 100) / 100);
