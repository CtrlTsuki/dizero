// Theme resolution: base tokens → theme plugin tokens → accent override →
// high-contrast adjustment. Every value is validated as a hex colour, so
// themes can never inject arbitrary CSS.

export const BASE_TOKENS = {
  background: "#101014",
  surface: "#17171d",
  surfaceSecondary: "#1d1d25",
  surfaceElevated: "#24242e",
  accent: "#9278f5",
  accentHover: "#a48dff",
  accentMuted: "#6655a8",
  textPrimary: "#f4f2fa",
  textSecondary: "#aaa8b7",
  textMuted: "#777584",
  border: "#2c2b35",
  success: "#65c99a",
  warning: "#e8b96c",
  danger: "#e57883",
} as const;

export type TokenName = keyof typeof BASE_TOKENS;

export function normalizeHex(input: string): string | null {
  const m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(input.trim());
  if (!m || !m[1]) return null;
  const h = m[1].length === 3 ? [...m[1]].map((c) => c + c).join("") : m[1];
  return `#${h.toLowerCase()}`;
}

export function hexToRgb(hex: string): [number, number, number] | null {
  const n = normalizeHex(hex);
  if (!n) return null;
  return [parseInt(n.slice(1, 3), 16), parseInt(n.slice(3, 5), 16), parseInt(n.slice(5, 7), 16)];
}

const toHex = (c: number) => Math.round(Math.min(255, Math.max(0, c))).toString(16).padStart(2, "0");

/** Linear blend from `a` (t=0) to `b` (t=1). */
export function mix(a: string, b: string, t: number): string {
  const x = hexToRgb(a);
  const y = hexToRgb(b);
  if (!x || !y) return a;
  return `#${toHex(x[0] + (y[0] - x[0]) * t)}${toHex(x[1] + (y[1] - x[1]) * t)}${toHex(x[2] + (y[2] - x[2]) * t)}`;
}

function luminance(hex: string): number {
  const rgb = hexToRgb(hex) ?? [0, 0, 0];
  const [r, g, b] = rgb.map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const cssVar = (name: string) => `--dz-${name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`;

function ensureContrast(fg: string, bg: string, target: number, toward: string): string {
  let out = fg;
  for (let i = 1; i <= 10 && contrastRatio(out, bg) < target; i++) out = mix(fg, toward, i / 10);
  return out;
}

export interface ThemeInput {
  tokens: Record<string, string>;
  accentOverride: string | null;
  highContrast: boolean;
}

export function resolveTheme(input: ThemeInput): Record<string, string> {
  const t: Record<TokenName, string> = { ...BASE_TOKENS };
  for (const [k, v] of Object.entries(input.tokens)) {
    const hex = normalizeHex(v);
    if (hex && k in t) t[k as TokenName] = hex;
  }
  const themedAccent = "accent" in input.tokens && normalizeHex(input.tokens.accent ?? "") !== null;
  const override = input.accentOverride ? normalizeHex(input.accentOverride) : null;
  if (override) {
    t.accent = override;
    t.accentHover = mix(override, "#ffffff", 0.18);
    t.accentMuted = mix(override, t.background, 0.45);
  } else if (themedAccent) {
    if (!normalizeHex(input.tokens.accentHover ?? "")) t.accentHover = mix(t.accent, "#ffffff", 0.18);
    if (!normalizeHex(input.tokens.accentMuted ?? "")) t.accentMuted = mix(t.accent, t.background, 0.45);
  }
  if (input.highContrast) {
    t.textSecondary = ensureContrast(mix(t.textSecondary, t.textPrimary, 0.5), t.surface, 7, t.textPrimary);
    t.textMuted = ensureContrast(mix(t.textMuted, t.textPrimary, 0.45), t.surface, 4.5, t.textPrimary);
    t.border = mix(t.border, t.textPrimary, 0.28);
  }

  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(t)) out[cssVar(k)] = v;
  const accentRgb = hexToRgb(t.accent) ?? [146, 120, 245];
  out["--dz-accent-rgb"] = accentRgb.join(", ");
  out["--dz-accent-soft"] = `rgba(${accentRgb.join(", ")}, 0.16)`;
  out["--dz-accent-ring"] = `rgba(${accentRgb.join(", ")}, 0.55)`;
  out["--dz-on-accent"] = contrastRatio("#ffffff", t.accent) >= contrastRatio("#101014", t.accent) ? "#ffffff" : "#101014";
  out["--dz-hover"] = mix(t.surfaceSecondary, t.textPrimary, 0.045);
  out["--dz-active"] = mix(t.surfaceSecondary, t.textPrimary, 0.085);
  out["--dz-sidebar-hover"] = mix(t.surface, t.textPrimary, 0.05);
  out["--dz-sidebar-active"] = mix(t.surface, t.textPrimary, 0.09);
  out["--dz-border-strong"] = mix(t.border, t.textPrimary, 0.16);
  out["--dz-input"] = mix(t.background, t.surfaceSecondary, 0.5);
  return out;
}

export function applyTheme(vars: Record<string, string>, root: HTMLElement = document.documentElement): void {
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
}
