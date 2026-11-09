/**
 * Ready-made colour schemes with a readability floor.
 *
 * Every scheme is a *whole* set of semantic colours — surfaces, three text
 * levels, interaction states and the accent — for both light and dark modes.
 * The old approach took a hue and pushed a fixed HSL lightness ladder through
 * every token, which dyed even the neutral surfaces and left some hues (yellow,
 * green) with unreadable secondary text and white-on-accent labels.
 *
 * Here the lightness ladder is expressed in OKLCH, which is perceptually
 * uniform, and is *fixed*: a scheme only chooses hue and chroma, never how far
 * apart text and background are. After composition, every text token is
 * measured against all three backgrounds it can sit on and pushed along the L
 * axis until it clears its WCAG target; the accent gets the black/white label
 * with the better contrast. That is why a multi-hue scheme is just as legible
 * as a single-hue one.
 *
 * Schemes are intentionally fixed presets — there is no free-form colour
 * picker. Anything a user can dial in by hand is something that can be dialled
 * into an unreadable state.
 */

export type SchemeFamily = "neutral" | "mono" | "multi";

export interface Layer {
  /** Seed colour supplying the hue. */
  seed: string;
  /** How much of that hue the neutral surfaces carry, 0..1. */
  tint: number;
}

/** A multi-hue scheme splits the UI into four independently coloured groups. */
export interface SchemeLayers {
  /** Window background, sunken areas, text and borders. */
  shell: Layer;
  /** Cards and panels. */
  surface: Layer;
  /** Hover / active states and the accent. */
  action: Layer;
  /** Control fills and keycaps — the small "third colour" accents. */
  spot: Layer;
}

export interface SchemeDef {
  id: string;
  family: SchemeFamily;
  /** Chinese source string, used as an i18n key. */
  label: string;
  /** Chinese source string, shown under the card. */
  desc: string;
  /** Single-hue schemes: seed + chroma strength. */
  seed?: string;
  tint?: number;
  /** Multi-hue schemes: one layer per semantic group. */
  layers?: SchemeLayers;
}

export type Palette = Record<string, string>;

/** Minimum contrast ratios guaranteed by every scheme, in both modes. */
export const CONTRAST_TARGETS = {
  text: 12,
  text2: 5.5,
  text3: 4,
  label: 4.5,
  accent: 3,
  border: 1.25,
} as const;

const MONO_TINT = 0.7;
const MONO_DESC = "单一色相，整体统一";

/** One hue for the entire interface. These ids are also the legacy preset ids,
 *  so an existing config keeps (and improves) its chosen colour on upgrade. */
const MONO_SCHEMES: SchemeDef[] = [
  { id: "mist", family: "mono", label: "雾蓝", desc: MONO_DESC, seed: "#4C7DFF", tint: MONO_TINT },
  { id: "celadon", family: "mono", label: "青瓷", desc: MONO_DESC, seed: "#0E9F8E", tint: MONO_TINT },
  { id: "mint", family: "mono", label: "薄荷", desc: MONO_DESC, seed: "#2FB45C", tint: MONO_TINT },
  { id: "sand", family: "mono", label: "暖砂", desc: MONO_DESC, seed: "#C98A3C", tint: MONO_TINT },
  { id: "rose", family: "mono", label: "玫瑰", desc: MONO_DESC, seed: "#E2596F", tint: MONO_TINT },
  { id: "violet", family: "mono", label: "藤紫", desc: MONO_DESC, seed: "#8B5CF6", tint: MONO_TINT },
  { id: "graphite", family: "mono", label: "石墨", desc: MONO_DESC, seed: "#6E7681", tint: 0.45 },
];

/** Different hue families working together. The groups map onto real CSS
 *  tokens (see composeLayers), so the difference is visible in the app, not
 *  just in a preview swatch. */
const MULTI_SCHEMES: SchemeDef[] = [
  {
    id: "harbor",
    family: "multi",
    label: "港湾",
    desc: "中性底 · 靛蓝 · 琥珀点缀",
    layers: {
      shell: { seed: "#808080", tint: 0 },
      surface: { seed: "#4C7DFF", tint: 0.8 },
      action: { seed: "#4C7DFF", tint: 0.85 },
      spot: { seed: "#F59E0B", tint: 0.8 },
    },
  },
  {
    id: "aurora",
    family: "multi",
    label: "极光",
    desc: "冷蓝底 · 暖砂卡片 · 藤紫",
    layers: {
      shell: { seed: "#3B82F6", tint: 0.85 },
      surface: { seed: "#F59E0B", tint: 0.9 },
      action: { seed: "#8B5CF6", tint: 0.9 },
      spot: { seed: "#0E9F8E", tint: 0.8 },
    },
  },
  {
    id: "grove",
    family: "multi",
    label: "林间",
    desc: "青绿底 · 暖橙 · 玫瑰点缀",
    layers: {
      shell: { seed: "#0E9F8E", tint: 0.8 },
      surface: { seed: "#C98A3C", tint: 0.9 },
      action: { seed: "#2FB45C", tint: 0.9 },
      spot: { seed: "#E2596F", tint: 0.8 },
    },
  },
];

export const NEUTRAL_SCHEME: SchemeDef = {
  id: "neutral",
  family: "neutral",
  label: "跟随主题",
  desc: "沿用系统默认配色",
};

export const SCHEMES: SchemeDef[] = [NEUTRAL_SCHEME, ...MONO_SCHEMES, ...MULTI_SCHEMES];
export const MONO_SCHEME_LIST = MONO_SCHEMES;
export const MULTI_SCHEME_LIST = MULTI_SCHEMES;

export function schemeById(id: string): SchemeDef | undefined {
  return SCHEMES.find((s) => s.id === id);
}

/** Strings passed to the translator dynamically, via a variable rather than a
 *  literal key. The i18n checker reads this list so a preset name or contrast
 *  label cannot ship untranslated. */
export const DYNAMIC_I18N_KEYS = [
  "主文字",
  "次要文字",
  "辅助文字",
  "按钮文字",
  "强调色",
  MONO_DESC,
  ...SCHEMES.flatMap((s) => [s.label, s.desc]),
];

/** Solid stand-ins for the theme.css defaults, used by previews only. */
export const THEME_SWATCHES: Record<"light" | "dark", Palette> = {
  light: {
    "--bg": "#FFFFFF",
    "--bg-sunken": "#F4F6FA",
    "--surface": "#FFFFFF",
    "--surface-2": "#F6F7FA",
    "--surface-3": "#EEF1F6",
    "--text": "#14161B",
    "--text-2": "#59616E",
    "--text-3": "#6B7280",
    "--border": "#E4E7EE",
    "--border-strong": "#D2D7E1",
    "--hover": "#F2F4F8",
    "--active": "#ECEEF3",
    "--kbd-bg": "#F1F3F8",
    "--accent": "#3B6BE8",
    "--accent-ink": "#FFFFFF",
    "--accent-soft": "rgba(59, 107, 232, 0.16)",
  },
  dark: {
    "--bg": "#1B1D22",
    "--bg-sunken": "#15171B",
    "--surface": "#202329",
    "--surface-2": "#262A31",
    "--surface-3": "#2E333B",
    "--text": "#ECEEF3",
    "--text-2": "#A4ACBA",
    "--text-3": "#949CAA",
    "--border": "#33373F",
    "--border-strong": "#454A54",
    "--hover": "#2A2E36",
    "--active": "#2F323A",
    "--kbd-bg": "#2E3137",
    "--accent": "#3B6BE8",
    "--accent-ink": "#FFFFFF",
    "--accent-soft": "rgba(59, 107, 232, 0.16)",
  },
};

interface Lch {
  L: number;
  C: number;
  H: number;
}

type Ramp = Record<string, [lightness: number, chroma: number]>;

/** `[token, OKLCH lightness, chroma at tint=1]`. These ramps are deliberately
 *  independent of the seed: only hue is inherited from it. */
const LIGHT_RAMP: Ramp = {
  "--bg": [0.995, 0.012],
  "--surface": [1.0, 0.006],
  "--surface-2": [0.965, 0.030],
  "--surface-3": [0.930, 0.048],
  "--bg-sunken": [0.970, 0.030],
  "--hover": [0.958, 0.036],
  "--active": [0.925, 0.054],
  "--border": [0.870, 0.048],
  "--kbd-bg": [0.945, 0.036],
  // Text keeps only a whisper of hue: its job is to stay legible.
  "--text": [0.20, 0.030],
  "--text-2": [0.43, 0.027],
  "--text-3": [0.55, 0.021],
};

const DARK_RAMP: Ramp = {
  "--bg": [0.155, 0.030],
  "--surface": [0.185, 0.030],
  "--surface-2": [0.220, 0.036],
  "--surface-3": [0.260, 0.048],
  "--bg-sunken": [0.125, 0.024],
  "--hover": [0.240, 0.042],
  "--active": [0.285, 0.054],
  "--border": [0.330, 0.048],
  "--kbd-bg": [0.250, 0.036],
  "--text": [0.960, 0.018],
  "--text-2": [0.760, 0.021],
  "--text-3": [0.620, 0.018],
};

const WHITE = "#FFFFFF";
const NEAR_BLACK = "#0B0E14";

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}

function parseHex(hex: string): [number, number, number] {
  const h = hex.trim().replace(/^#/, "");
  const full = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  const s = /^[0-9a-fA-F]{6}$/.test(full) ? full : "4C7DFF";
  return [
    parseInt(s.slice(0, 2), 16) / 255,
    parseInt(s.slice(2, 4), 16) / 255,
    parseInt(s.slice(4, 6), 16) / 255,
  ];
}

function toHex(rgb: [number, number, number]): string {
  return (
    "#" +
    rgb
      .map((v) => {
        const c = v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055;
        return Math.round(clamp(c, 0, 1) * 255).toString(16).padStart(2, "0");
      })
      .join("")
      .toUpperCase()
  );
}

/** sRGB → OKLab → OKLCH. */
export function hexToLch(hex: string): Lch {
  const [r, g, b] = parseHex(hex).map((v) =>
    v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4),
  ) as [number, number, number];
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return { L, C: Math.hypot(a, bb), H: (Math.atan2(bb, a) * 180) / Math.PI };
}

function oklabToRgb(L: number, a: number, b: number): [number, number, number] {
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

function inGamut(rgb: [number, number, number]): boolean {
  return rgb.every((v) => v >= -0.0005 && v <= 1.0005);
}

/** OKLCH → sRGB, reducing chroma until the colour fits the sRGB gamut. */
function lchToHex({ L, C, H }: Lch): string {
  const rad = (H * Math.PI) / 180;
  let lo = 0;
  let hi = C;
  let best: [number, number, number] = [0, 0, 0];
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    const rgb = oklabToRgb(L, mid * Math.cos(rad), mid * Math.sin(rad));
    if (inGamut(rgb)) {
      best = rgb;
      lo = mid;
    } else {
      hi = mid;
    }
  }
  return toHex(best.map((v) => clamp(v, 0, 1)) as [number, number, number]);
}

/** WCAG 2.1 relative luminance + contrast ratio. */
function luminance(hex: string): number {
  const [r, g, b] = parseHex(hex).map((v) =>
    v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4),
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(a: string, b: string): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/** Push a foreground along OKLCH L until it clears `target` on *every*
 *  background it can appear over. Hue and chroma are preserved. */
function readableOn(fg: string, backgrounds: string[], target: number, dark: boolean): string {
  const lch = hexToLch(fg);
  let out = fg;
  const step = dark ? 0.01 : -0.01;
  for (let i = 0; i < 90; i++) {
    if (backgrounds.every((bg) => contrastRatio(out, bg) >= target)) break;
    const next = clamp(lch.L + step, 0, 1);
    if (next === lch.L) break;
    lch.L = next;
    out = lchToHex(lch);
  }
  return out;
}

export function accentInk(accent: string): string {
  return contrastRatio(WHITE, accent) >= contrastRatio(NEAR_BLACK, accent) ? WHITE : NEAR_BLACK;
}

export function accentSoft(accent: string): string {
  const [r, g, b] = parseHex(accent).map((v) => Math.round(v * 255));
  return `rgba(${r}, ${g}, ${b}, 0.16)`;
}

/** Fit a seed into a usable accent: visible on the background and with a
 *  legible label on top of it. Extremely light seeds are darkened, extremely
 *  dark ones lightened; the hue is preserved. */
function fitAccent(seed: string, bg: string, dark: boolean): string {
  const lch = hexToLch(seed);
  let accent = seed;
  const step = dark ? 0.02 : -0.02;
  for (let i = 0; i < 60; i++) {
    const okBg = contrastRatio(accent, bg) >= CONTRAST_TARGETS.accent;
    const okInk = contrastRatio(accentInk(accent), accent) >= CONTRAST_TARGETS.label;
    if (okBg && okInk) break;
    const next = clamp(lch.L + step, 0, 1);
    if (next === lch.L) break;
    lch.L = next;
    accent = lchToHex(lch);
  }
  return accent;
}

/** Final pass applied to every scheme: text must clear its target on the
 *  window background *and* on cards/panels, borders must separate, and the
 *  accent must remain visible with a legible label. */
function enforceContrast(pal: Palette, dark: boolean): Palette {
  const backgrounds = [pal["--bg"], pal["--surface"], pal["--surface-2"]];
  pal["--text"] = readableOn(pal["--text"], backgrounds, CONTRAST_TARGETS.text, dark);
  pal["--text-2"] = readableOn(pal["--text-2"], backgrounds, CONTRAST_TARGETS.text2, dark);
  pal["--text-3"] = readableOn(pal["--text-3"], backgrounds, CONTRAST_TARGETS.text3, dark);
  pal["--border"] = readableOn(pal["--border"], [pal["--bg"]], CONTRAST_TARGETS.border, dark);
  pal["--border-strong"] = readableOn(pal["--border"], [pal["--bg"]], 1.6, dark);
  const accent = fitAccent(pal["--accent"], pal["--bg"], dark);
  pal["--accent"] = accent;
  pal["--accent-ink"] = accentInk(accent);
  pal["--accent-soft"] = accentSoft(accent);
  return pal;
}

/** Build a single-hue palette for one appearance mode. */
export function buildPalette(
  seed: string,
  tint: number,
  theme: "light" | "dark",
): Palette {
  const dark = theme === "dark";
  const ramp = dark ? DARK_RAMP : LIGHT_RAMP;
  const strength = clamp(Number.isFinite(tint) ? tint : MONO_TINT, 0, 1);
  const lch = hexToLch(seed);
  const out: Palette = {};
  for (const [token, [L, chroma]] of Object.entries(ramp)) {
    out[token] = lchToHex({ L, C: chroma * strength, H: lch.H });
  }
  out["--accent"] = seed;
  return enforceContrast(out, dark);
}

/** Compose a multi-hue scheme from four independently coloured groups. */
function composeLayers(layers: SchemeLayers, theme: "light" | "dark"): Palette {
  const shell = buildPalette(layers.shell.seed, layers.shell.tint, theme);
  const surface = buildPalette(layers.surface.seed, layers.surface.tint, theme);
  const action = buildPalette(layers.action.seed, layers.action.tint, theme);
  const spot = buildPalette(layers.spot.seed, layers.spot.tint, theme);
  const out: Palette = {
    ...shell,
    "--surface": surface["--surface"],
    "--surface-2": surface["--surface-2"],
    "--hover": action["--hover"],
    "--active": action["--active"],
    "--accent": action["--accent"],
    "--surface-3": spot["--surface-3"],
    "--kbd-bg": spot["--kbd-bg"],
  };
  // buildPalette already clamps each layer against its own background; run the
  // pass once more because the composed surfaces now have different hues.
  return enforceContrast(out, theme === "dark");
}

/** The complete palette for a scheme in one appearance mode. */
export function schemePalette(
  def: SchemeDef,
  theme: "light" | "dark",
): Palette {
  if (def.family === "neutral") return { ...THEME_SWATCHES[theme] };
  if (def.family === "multi" && def.layers) return composeLayers(def.layers, theme);
  return buildPalette(def.seed ?? "#4C7DFF", def.tint ?? MONO_TINT, theme);
}

/** Swatches that explain a scheme at a glance: window → panel → active → accent. */
export function schemeSwatches(pal: Palette): string[] {
  return [pal["--bg"], pal["--surface-2"], pal["--active"], pal["--accent"]];
}

/** Contrast read-out shown under the current scheme in Settings. */
export interface ContrastCheck {
  label: string;
  ratio: number;
  ok: boolean;
}

export function paletteReport(pal: Palette): ContrastCheck[] {
  const backgrounds = [pal["--bg"] ?? WHITE, pal["--surface"] ?? WHITE, pal["--surface-2"] ?? WHITE];
  const worst = (fg: string) => Math.min(...backgrounds.map((bg) => contrastRatio(fg, bg)));
  const accent = pal["--accent"] ?? "#4C7DFF";
  const ink = pal["--accent-ink"] ?? WHITE;
  const checks: [string, number, number][] = [
    ["主文字", worst(pal["--text"] ?? NEAR_BLACK), CONTRAST_TARGETS.text],
    ["次要文字", worst(pal["--text-2"] ?? NEAR_BLACK), CONTRAST_TARGETS.text2],
    ["辅助文字", worst(pal["--text-3"] ?? NEAR_BLACK), CONTRAST_TARGETS.text3],
    ["按钮文字", contrastRatio(ink, accent), CONTRAST_TARGETS.label],
    ["强调色", contrastRatio(accent, pal["--bg"] ?? WHITE), CONTRAST_TARGETS.accent],
  ];
  return checks.map(([label, ratio, target]) => ({
    label,
    ratio,
    ok: ratio >= target - 0.01,
  }));
}

/** Map an accent colour to the closest single-hue preset. Used only when
 *  migrating the old free-form "auto" / "custom" settings. */
export function nearestMono(hex: string): SchemeDef {
  const target = hexToLch(hex);
  let best = MONO_SCHEMES[0];
  let bestDistance = Infinity;
  for (const def of MONO_SCHEMES) {
    const c = hexToLch(def.seed!);
    let delta = Math.abs(c.H - target.H);
    if (delta > 180) delta = 360 - delta;
    // A near-grey accent should land on graphite rather than a vivid hue.
    const distance = target.C < 0.02 ? (def.id === "graphite" ? 0 : 180) : delta + Math.abs(c.C - target.C) * 120;
    if (distance < bestDistance) {
      bestDistance = distance;
      best = def;
    }
  }
  return best;
}
