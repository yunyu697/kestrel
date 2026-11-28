/**
 * Applying a colour scheme.
 *
 * `theme.css` owns the neutral defaults. When a scheme other than "follow
 * theme" is active, every semantic token is replaced at once by the preset's
 * palette — the point is to change the whole scheme, not to tint a handful of
 * controls. Each preset (see palettes.ts) ships light and dark variants and is
 * contrast-checked before it is ever applied.
 *
 * `colorScheme` values:
 *   "" / "neutral"   → theme.css defaults, nothing is overridden
 *   a preset id      → that scheme's full palette
 *   "auto"/"custom"  → legacy free-form modes: migrated to the closest mono
 *   old multi ids    → "harbor"/"aurora"/"grove" removed; migrated to their
 *                      nearest mono by accent (mist / violet / mint)
 *   unknown ids      → migrated to the closest mono by the stored accent
 *
 * Legacy hue-preset ids (mist, celadon, …) are still valid preset ids, so an
 * existing config keeps its colour and simply gains the readability fixes.
 */

import type { Settings } from "./types";
import {
  NEUTRAL_SCHEME,
  nearestMono,
  schemeById,
  schemePalette,
  type Palette,
  type SchemeDef,
} from "./palettes";

export { THEME_SWATCHES } from "./palettes";

/** Normalise any stored value to a real scheme id. */
export function normalizeSchemeId(id: string, accent = "#4C7DFF"): string {
  const key = (id || "").trim();
  if (!key || key === "neutral" || key === "default" || key === "theme") return "neutral";
  if (schemeById(key)) return key;
  // "auto"/"custom" were the free-form generator modes; both are gone. Pick
  // the closest ready-made single-hue scheme from the accent the user had.
  return nearestMono(accent || "#4C7DFF").id;
}

/** The scheme currently in effect, after legacy migration. */
export function schemeFor(s: Settings): SchemeDef {
  return schemeById(normalizeSchemeId(s.colorScheme, s.accent)) ?? NEUTRAL_SCHEME;
}

/** The palette on screen, used by the Settings preview as well. */
export function currentPalette(s: Settings, theme: "light" | "dark"): Palette {
  return schemePalette(schemeFor(s), theme);
}

/** Every CSS custom property a scheme owns. Clearing these restores the
 *  theme.css defaults exactly, including translucent borders and hover tints. */
const MANAGED = [
  "--bg",
  "--bg-sunken",
  "--surface",
  "--surface-2",
  "--surface-3",
  "--text",
  "--text-2",
  "--text-3",
  "--border",
  "--border-strong",
  "--hover",
  "--active",
  "--kbd-bg",
  "--accent",
  "--accent-text",
  "--accent-ink",
  "--accent-soft",
];

/** Apply (or clear) the whole scheme on `:root`. */
export function applyColors(s: Settings, theme: "light" | "dark"): void {
  const root = document.documentElement;
  for (const key of MANAGED) root.style.removeProperty(key);

  const def = schemeFor(s);
  if (def.family === "neutral") return;

  const pal = schemePalette(def, theme);
  for (const [key, value] of Object.entries(pal)) {
    if (value) root.style.setProperty(key, value);
  }
}

/** Clamp to the range a slider offers, so a hand-edited config cannot push
 *  the window past something the UI can recover from. */
function clamp(v: number, lo: number, hi: number): number {
  return Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : lo;
}

/**
 * Window chrome: scale, corners and translucency.
 *
 * These four settings are stored in the config and shown in the settings page,
 * but nothing used to read them — the values only ever reached the file. They
 * live here rather than in each window's own `applyTheme` because the launcher
 * and the settings page had drifted apart once already: both copies forgot the
 * same four fields.
 *
 * Radius ratios keep the current defaults exact: 18px window, 12px card, 8px
 * control. Scaling the window radius therefore scales the whole roundness
 * family instead of only the outer shell.
 *
 * Transparency is a *background* alpha, not an element opacity. Fading the
 * whole surface faded the text with it, which is why the slider stopped being
 * useful below about 70%. The floor here is the alpha at which body text still
 * clears 4.5:1 over a light desktop; below it the slider would trade
 * legibility for decoration.
 *
 * The blur field is kept in Settings for config compatibility but has no
 * visual effect: Windows' system acrylic cannot round a layered window, so
 * enabling it produced square corners and a patchy backdrop. The setting
 * page no longer offers the switch.
 */
const MIN_READABLE_ALPHA = 0.82;

/**
 * Style layer.
 *
 * Four interface languages can be selected in the settings page. The layer is
 * deliberately orthogonal to colour: a style owns shape, density and the way
 * hierarchy is expressed, never a colour token. That is what lets
 * 4 styles × 2 themes × 16 schemes all be valid combinations.
 *
 * The window/card/control radii come from here rather than from a single
 * number, because the four languages do not share a roundness family: Ant is
 * nearly square, Material rounds the window more than the card, Apple rounds
 * controls into pills at CSS level. `cornerRadius` is still honoured, but as a
 * *scale* of the style's own base radii (18 = the style as drawn).
 */
export type StyleId = "fluent" | "apple" | "material" | "ant";

export const STYLE_IDS: StyleId[] = ["fluent", "apple", "material", "ant"];

/** [window, card, control] base radii, in px. */
const STYLE_RADII: Record<StyleId, [number, number, number]> = {
  fluent: [18, 12, 8],
  apple: [20, 16, 10],
  material: [22, 12, 12],
  ant: [8, 8, 6],
};

export function normalizeStyle(id: string | undefined): StyleId {
  return id === "apple" || id === "material" || id === "ant" ? id : "fluent";
}

/** Write the style onto `:root`; every per-style rule keys off `data-style`. */
export function applyStyle(id: string | undefined): StyleId {
  const style = normalizeStyle(id);
  document.documentElement.setAttribute("data-style", style);
  return style;
}

export function applyWindowChrome(
  s: Pick<Settings, "opacity" | "blur" | "cornerRadius" | "fontScale" | "style">,
): void {
  const root = document.documentElement;
  const scale = clamp(s.fontScale, 0.85, 1.2);
  const radius = clamp(s.cornerRadius, 0, 22);
  const alpha = clamp(s.opacity, 0.6, 1);
  // Map the slider's 0.6–1 range onto the readable 0.82–1 range.
  const surfaceAlpha = MIN_READABLE_ALPHA + (alpha - 0.6) / 0.4 * (1 - MIN_READABLE_ALPHA);

  // 18px is the reference: at that value each style keeps its drawn radii.
  const factor = radius / 18;
  const [win, card, ctl] = STYLE_RADII[normalizeStyle(s.style)];

  root.style.setProperty("--font-scale", String(scale));
  root.style.setProperty("--radius-win", `${Math.round(win * factor)}px`);
  root.style.setProperty("--radius-card", `${Math.round(card * factor)}px`);
  root.style.setProperty("--radius-ctl", `${Math.round(ctl * factor)}px`);
  root.style.setProperty("--win-alpha", String(alpha));
  root.style.setProperty("--surface-alpha", surfaceAlpha.toFixed(3));
}
