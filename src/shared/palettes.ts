/**
 * Hue-based colour schemes.
 *
 * Instead of making the user pick twelve colours one by one, a scheme derives
 * the whole surface/text ramp from a single hue: every token sits on the same
 * hue with only its saturation and lightness varying, which is what makes a
 * palette look like one family rather than twelve unrelated swatches.
 *
 * Each scheme carries its own light and dark ramp, so switching the appearance
 * mode keeps the family intact.
 */

export interface SchemeDef {
  id: string;
  /** Hue in degrees (0-360). */
  hue: number;
  /** Base saturation in percent; every token scales it. */
  sat: number;
  /** Accent that belongs to the family. */
  accent: string;
}

export const SCHEMES: SchemeDef[] = [
  { id: "mist", hue: 218, sat: 34, accent: "#4C7DFF" },
  { id: "celadon", hue: 172, sat: 26, accent: "#0E9F8E" },
  { id: "mint", hue: 146, sat: 30, accent: "#2FB45C" },
  { id: "sand", hue: 33, sat: 40, accent: "#C98A3C" },
  { id: "rose", hue: 348, sat: 34, accent: "#E2596F" },
  { id: "violet", hue: 268, sat: 32, accent: "#8B5CF6" },
  { id: "graphite", hue: 220, sat: 9, accent: "#6E7681" },
];

/** `[cssVar, saturation × factor, lightness %]`, light ramp. */
const LIGHT: [string, number, number][] = [
  ["--bg", 0.55, 98.2],
  ["--surface", 0.4, 99.6],
  ["--surface-2", 0.75, 96.2],
  ["--surface-3", 0.9, 92.4],
  ["--bg-sunken", 0.8, 94.4],
  ["--hover", 0.85, 95.0],
  ["--active", 1.0, 90.4],
  ["--border", 0.9, 87.2],
  ["--kbd-bg", 0.7, 93.0],
  ["--text", 0.35, 13],
  ["--text-2", 0.3, 40],
  ["--text-3", 0.25, 58],
];

/** Same tokens, dark ramp: same hue, inverted lightness ladder. */
const DARK: [string, number, number][] = [
  ["--bg", 0.45, 10.5],
  ["--surface", 0.5, 13.5],
  ["--surface-2", 0.55, 16.6],
  ["--surface-3", 0.6, 21.0],
  ["--bg-sunken", 0.4, 7.8],
  ["--hover", 0.6, 18.6],
  ["--active", 0.65, 23.0],
  ["--border", 0.5, 25.0],
  ["--kbd-bg", 0.5, 20.0],
  ["--text", 0.3, 93],
  ["--text-2", 0.25, 70],
  ["--text-3", 0.2, 52],
];

function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}

/** HSL (h in degrees, s/l in percent) to `#RRGGBB`. */
function hslToHex(h: number, s: number, l: number): string {
  const hue = ((h % 360) + 360) % 360;
  const sat = clamp(s, 0, 100) / 100;
  const lig = clamp(l, 0, 100) / 100;
  const c = (1 - Math.abs(2 * lig - 1)) * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = lig - c / 2;
  const seg = Math.floor(hue / 60) % 6;
  const rgb: [number, number, number] =
    seg === 0
      ? [c, x, 0]
      : seg === 1
        ? [x, c, 0]
        : seg === 2
          ? [0, c, x]
          : seg === 3
            ? [0, x, c]
            : seg === 4
              ? [x, 0, c]
              : [c, 0, x];
  return (
    "#" +
    rgb
      .map((v) => Math.round((v + m) * 255).toString(16).padStart(2, "0"))
      .join("")
      .toUpperCase()
  );
}

export function schemeById(id: string): SchemeDef | undefined {
  return SCHEMES.find((s) => s.id === id);
}

/** Every CSS custom property of a scheme, for one appearance mode. */
export function schemePalette(
  def: SchemeDef,
  theme: "light" | "dark",
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [cssVar, factor, lightness] of theme === "dark" ? DARK : LIGHT) {
    out[cssVar] = hslToHex(def.hue, def.sat * factor, lightness);
  }
  // The accent belongs to the family too: a rose scheme must not keep a blue
  // accent, otherwise it is not actually "one colour family".
  out["--accent"] = def.accent;
  return out;
}
