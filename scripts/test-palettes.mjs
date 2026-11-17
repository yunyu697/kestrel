// Guarantees that every shipped colour scheme is readable in both modes.
// The old hue presets could reach 2.78:1 on faint text and 2.69:1 on a white
// button label; this test fails the build if any preset regresses.
//
//   node_modules/.bin/esbuild scripts/test-palettes.mjs --bundle --platform=node
//     --format=esm --outfile=.tmp/palettes-test.mjs
//   node .tmp/palettes-test.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  CONTRAST_TARGETS,
  MONO_SCHEME_LIST,
  NEUTRAL_SCHEME,
  SCHEMES,
  THEME_SWATCHES,
  accentInk,
  contrastRatio,
  hexToLch,
  paletteReport,
  schemeById,
  schemePalette,
  schemeSwatches,
  textBackdrops,
} from "../src/shared/palettes.ts";
import { currentPalette, normalizeSchemeId, schemeFor } from "../src/shared/theme.ts";

let checks = 0;
const ok = (cond, msg) => {
  checks++;
  assert.ok(cond, msg);
};
const hueDelta = (a, b) => {
  const d = Math.abs(a - b) % 360;
  return d > 180 ? 360 - d : d;
};
// Worst case across every surface the token can sit on: window, cards,
// panels, sunken sidebar, raised pills, hover/selection fills, keycaps and the
// accent-soft blend used by the selected row. Measuring only bg/surface/
// surface-2 is what let 11px captions sit at 3.85:1 on a selected row.
const worstText = (pal, token) =>
  Math.min(...textBackdrops(pal).map((bg) => contrastRatio(pal[token], bg)));

// ---- scheme inventory -----------------------------------------------------
ok(SCHEMES.length === 16, `expected 16 schemes (1 neutral + 15 mono), got ${SCHEMES.length}`);
ok(new Set(SCHEMES.map((s) => s.id)).size === SCHEMES.length, "scheme ids must be unique");
ok(MONO_SCHEME_LIST.length === 15, "fifteen single-hue schemes");
ok(NEUTRAL_SCHEME.family === "neutral", "neutral scheme is the theme default");
ok(SCHEMES.every((s) => s.family !== "multi"), "multi-hue schemes are gone");
for (const def of SCHEMES) {
  ok(schemeById(def.id) === def, `lookup by id works for ${def.id}`);
  ok(typeof def.label === "string" && def.label.length > 0, `${def.id} has a label`);
  ok(typeof def.desc === "string" && def.desc.length > 0, `${def.id} has a description`);
}

// ---- every token, every scheme, light + dark ------------------------------
const HEX = /^#[0-9A-F]{6}$/;
const SOLID_TOKENS = [
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
];

for (const theme of ["light", "dark"]) {
  for (const def of SCHEMES) {
    const pal = schemePalette(def, theme);
    const where = `${def.id}/${theme}`;

    for (const token of SOLID_TOKENS) {
      ok(HEX.test(pal[token] ?? ""), `${where}: ${token} must be a hex colour`);
    }
    ok(/^rgba\(/.test(pal["--accent-soft"] ?? ""), `${where}: --accent-soft must be rgba`);

    const report = paletteReport(pal);
    ok(report.length === 5, `${where}: five contrast checks`);
    for (const c of report) {
      ok(c.ok, `${where}: ${c.label} = ${c.ratio.toFixed(2)}:1 (below target)`);
    }

    // The three text levels must stay ordered, not just individually pass.
    const r1 = worstText(pal, "--text");
    const r2 = worstText(pal, "--text-2");
    const r3 = worstText(pal, "--text-3");
    ok(r1 > r2 && r2 > r3, `${where}: text hierarchy ${r1.toFixed(1)} > ${r2.toFixed(1)} > ${r3.toFixed(1)}`);
    ok(
      contrastRatio(pal["--accent-ink"], pal["--accent"]) >= CONTRAST_TARGETS.label - 0.01,
      `${where}: accent label must clear ${CONTRAST_TARGETS.label}:1`,
    );
    ok(
      pal["--accent-ink"] === accentInk(pal["--accent"]),
      `${where}: accent ink is the better of black/white`,
    );
    ok(schemeSwatches(pal).length === 4, `${where}: four preview swatches`);
  }
}

// ---- contrast floor on every backdrop, every scheme -----------------------
// paletteReport already aggregates these, but an explicit per-token assertion
// names the failing token instead of only printing a ratio.
for (const theme of ["light", "dark"]) {
  for (const def of SCHEMES) {
    const pal = schemePalette(def, theme);
    const where = `${def.id}/${theme}`;
    const targets = [
      ["--text", CONTRAST_TARGETS.text],
      ["--text-2", CONTRAST_TARGETS.text2],
      ["--text-3", CONTRAST_TARGETS.text3],
    ];
    for (const [token, target] of targets) {
      const worst = worstText(pal, token);
      ok(worst >= target - 0.01, `${where}: ${token} worst ${worst.toFixed(2)}:1, want ${target}:1`);
    }
  }
}
// Accent used as text (selected nav label, match highlight, badges) needs the
// small-text floor, not the 3:1 that is enough for a fill or an indicator.
for (const theme of ["light", "dark"]) {
  for (const def of SCHEMES) {
    const pal = schemePalette(def, theme);
    const where = `${def.id}/${theme}`;
    const worst = Math.min(
      ...[pal["--bg"], pal["--surface"], pal["--surface-2"]].map((bg) =>
        contrastRatio(pal["--accent-text"], bg),
      ),
    );
    ok(
      worst >= CONTRAST_TARGETS.accentText - 0.01,
      `${where}: accent used as text is only ${worst.toFixed(2)}:1, want ${CONTRAST_TARGETS.accentText}:1`,
    );
    // The fill colour may stay as vivid as it likes, so the two must be
    // allowed to differ; what must not happen is accent-text being *less*
    // readable than the fill.
    ok(
      contrastRatio(pal["--accent-text"], pal["--bg"]) >=
        contrastRatio(pal["--accent"], pal["--bg"]) - 0.01,
      `${where}: accent-text must never be worse than accent`,
    );
  }
}

// Captions are 11-12px, so their floor is WCAG AA for small text, not 4:1.
ok(CONTRAST_TARGETS.text3 >= 4.5, "caption target must be at least WCAG AA (4.5:1)");
ok(CONTRAST_TARGETS.text2 >= 4.5, "secondary text target must be at least WCAG AA");

// ---- the grid reads as a hue walk ----------------------------------------
// Settings renders MONO_SCHEME_LIST in array order, so the array *is* the
// picker. It must advance monotonically around the OKLCH wheel from the
// shipped blue; achromatic graphite is pinned last because its hue means
// nothing.
const blueHue = hexToLch("#4C7DFF").H;
const walk = MONO_SCHEME_LIST.filter((s) => s.id !== "graphite").map((s) => {
  const rel = (hexToLch(s.seed).H - blueHue) % 360;
  return rel < 0 ? rel + 360 : rel;
});
for (let i = 1; i < walk.length; i++) {
  ok(
    walk[i] > walk[i - 1],
    `scheme order must follow the hue wheel: ${MONO_SCHEME_LIST[i].id} (${walk[i].toFixed(1)}°) after ${MONO_SCHEME_LIST[i - 1].id} (${walk[i - 1].toFixed(1)}°)`,
  );
}
ok(MONO_SCHEME_LIST.at(-1).id === "graphite", "achromatic graphite stays last");
ok(MONO_SCHEME_LIST[0].id === "mist", "the default blue leads the list");

// ---- theme.css and THEME_SWATCHES must not drift -------------------------
// The neutral scheme is hand-written in two places: the live CSS and the
// preview built from THEME_SWATCHES. A fix applied to one and not the other is
// how neutral kept 3.91:1 captions while every preset was corrected.
const cssText = readFileSync(new URL("../src/shared/theme.css", import.meta.url), "utf8");
function cssBlock(selector) {
  const at = cssText.indexOf(selector);
  ok(at >= 0, `theme.css contains ${selector}`);
  const open = cssText.indexOf("{", at);
  const close = cssText.indexOf("}", open);
  const vars = {};
  for (const line of cssText.slice(open + 1, close).split("\n")) {
    const m = line.match(/^\s*(--[a-z0-9-]+):\s*([^;]+);/i);
    if (m) vars[m[1]] = m[2].trim();
  }
  return vars;
}
const rootVars = cssBlock(":root {");
const MIRRORED = [
  "--bg", "--bg-sunken", "--surface", "--surface-2", "--surface-3",
  "--text", "--text-2", "--text-3", "--kbd-bg", "--accent", "--accent-text",
];
const rgbOf = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
/** CSS declares some structure tokens as translucent white/black, which the
 *  preview can only stand in for with a solid colour. Flatten over the window
 *  background and allow a small delta for those; hex tokens must match. */
function flattenCss(value, backdrop) {
  if (/^#[0-9a-f]{6}$/i.test(value ?? "")) return { hex: value, exact: true };
  const m = (value ?? "").match(/^rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/i);
  if (!m) return { hex: null, exact: false };
  const a = m[4] === undefined ? 1 : Number(m[4]);
  const base = rgbOf(backdrop);
  const mix = [Number(m[1]), Number(m[2]), Number(m[3])].map((c, i) =>
    Math.round(c * a + base[i] * (1 - a)),
  );
  return { hex: "#" + mix.map((v) => v.toString(16).padStart(2, "0")).join(""), exact: false };
}
for (const theme of ["light", "dark"]) {
  const live = { ...rootVars, ...cssBlock(`:root[data-theme="${theme}"]`) };
  for (const token of MIRRORED) {
    const { hex, exact } = flattenCss(live[token], live["--bg"]);
    ok(hex !== null, `theme defaults/${theme}: ${token} could not be read from theme.css`);
    const swatch = THEME_SWATCHES[theme][token] ?? "";
    const same = exact
      ? hex.toLowerCase() === swatch.toLowerCase()
      : rgbOf(hex).every((v, i) => Math.abs(v - rgbOf(swatch)[i]) <= 10);
    ok(
      same,
      `theme defaults/${theme}: ${token} drifted — theme.css ${live[token]} (${hex}) vs THEME_SWATCHES ${swatch}`,
    );
  }
}

// ---- family identity ------------------------------------------------------
for (const theme of ["light", "dark"]) {
  for (const def of MONO_SCHEME_LIST) {
    const pal = schemePalette(def, theme);
    const d = hueDelta(hexToLch(pal["--surface-2"]).H, hexToLch(pal["--accent"]).H);
    ok(d < 15, `${def.id}/${theme}: single hue, got ${d.toFixed(1)}° apart`);
  }
}

// No two single-hue presets may share a hue: users asked that new schemes not
// duplicate existing ones. Compare the seed hues directly (the accent token is
// the seed), keeping a small tolerance for OKLCH round-tripping.
for (let i = 0; i < MONO_SCHEME_LIST.length; i++) {
  for (let j = i + 1; j < MONO_SCHEME_LIST.length; j++) {
    const a = MONO_SCHEME_LIST[i], b = MONO_SCHEME_LIST[j];
    if (a.id === "graphite" || b.id === "graphite") continue; // near-grey, hue meaningless
    const d = hueDelta(hexToLch(a.seed).H, hexToLch(b.seed).H);
    ok(d >= 12, `hue collision: ${a.id} vs ${b.id} only ${d.toFixed(1)}° apart`);
  }
}

// Neutral swatches must mirror the shipped theme.css defaults closely enough
// that the settings preview is not lying about what "follow theme" looks like.
for (const theme of ["light", "dark"]) {
  const pal = THEME_SWATCHES[theme];
  ok(paletteReport(pal).every((c) => c.ok), `theme defaults/${theme}: readable`);
}

// ---- migration of old stored values --------------------------------------
ok(normalizeSchemeId("") === "neutral", "empty = follow theme");
ok(normalizeSchemeId("neutral") === "neutral", "explicit neutral");
ok(normalizeSchemeId("mist") === "mist", "legacy hue ids are still valid presets");
for (const legacy of ["auto", "custom", "something-else"]) {
  const id = normalizeSchemeId(legacy, "#8B5CF6");
  ok(schemeById(id)?.family === "mono", `${legacy} migrates to a real mono preset (${id})`);
}
// Removed multi-hue schemes migrate by their stored accent, each landing on a
// *different* mono preset so long-time users don't all collapse to one colour.
const migrated = [
  normalizeSchemeId("harbor", "#4C7DFF"),
  normalizeSchemeId("aurora", "#8B5CF6"),
  normalizeSchemeId("grove", "#2FB45C"),
];
ok(migrated[0] === "mist" && migrated[1] === "violet" && migrated[2] === "mint",
  `old multi schemes migrate to mist/violet/mint, got ${migrated.join("/")}`);
ok(new Set(migrated).size === 3, "the three old schemes migrate to three distinct presets");
ok(schemeFor({ colorScheme: "", accent: "#4C7DFF" }).id === "neutral", "schemeFor default");
ok(
  schemeFor({ colorScheme: "auto", accent: "#4C7DFF" }).family === "mono",
  "schemeFor migrates legacy free-form mode",
);
for (const theme of ["light", "dark"]) {
  const viaCurrent = currentPalette({ colorScheme: "terracotta", accent: "#4C7DFF" }, theme);
  const viaScheme = schemePalette(schemeById("terracotta"), theme);
  ok(viaCurrent["--accent"] === viaScheme["--accent"], `currentPalette/${theme} matches preset`);
}

console.log(`palette tests: ${checks} assertions passed across ${SCHEMES.length * 2} scheme variants`);
