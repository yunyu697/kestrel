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
 */
export function applyWindowChrome(
  s: Pick<Settings, "opacity" | "blur" | "cornerRadius" | "fontScale">,
): void {
  const root = document.documentElement;
  const scale = clamp(s.fontScale, 0.85, 1.2);
  const radius = clamp(s.cornerRadius, 0, 22);

  root.style.setProperty("--font-scale", String(scale));
  root.style.setProperty("--radius-win", `${radius}px`);
  root.style.setProperty("--radius-card", `${Math.round((radius * 2) / 3)}px`);
  root.style.setProperty("--radius-ctl", `${Math.round((radius * 4) / 9)}px`);
  root.style.setProperty("--win-alpha", String(clamp(s.opacity, 0.6, 1)));
  root.setAttribute("data-blur", s.blur ? "on" : "off");
}
