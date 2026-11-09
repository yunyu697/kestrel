// Guarantees that every shipped colour scheme is readable in both modes.
// The old hue presets could reach 2.78:1 on faint text and 2.69:1 on a white
// button label; this test fails the build if any preset regresses.
//
//   node_modules/.bin/esbuild scripts/test-palettes.mjs --bundle --platform=node
//     --format=esm --outfile=.tmp/palettes-test.mjs
//   node .tmp/palettes-test.mjs

import assert from "node:assert/strict";
import {
  CONTRAST_TARGETS,
  MONO_SCHEME_LIST,
  MULTI_SCHEME_LIST,
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
const worstText = (pal, token) =>
  Math.min(
    ...[pal["--bg"], pal["--surface"], pal["--surface-2"]].map((bg) =>
      contrastRatio(pal[token], bg),
    ),
  );

// ---- scheme inventory -----------------------------------------------------
ok(SCHEMES.length === 11, `expected 11 schemes, got ${SCHEMES.length}`);
ok(new Set(SCHEMES.map((s) => s.id)).size === SCHEMES.length, "scheme ids must be unique");
ok(MONO_SCHEME_LIST.length === 7, "seven single-hue schemes");
ok(MULTI_SCHEME_LIST.length === 3, "three multi-hue schemes");
ok(NEUTRAL_SCHEME.family === "neutral", "neutral scheme is the theme default");
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

// ---- family identity ------------------------------------------------------
for (const theme of ["light", "dark"]) {
  for (const def of MONO_SCHEME_LIST) {
    const pal = schemePalette(def, theme);
    const d = hueDelta(hexToLch(pal["--surface-2"]).H, hexToLch(pal["--accent"]).H);
    ok(d < 15, `${def.id}/${theme}: single hue, got ${d.toFixed(1)}° apart`);
  }
  for (const def of MULTI_SCHEME_LIST) {
    const pal = schemePalette(def, theme);
    const d = hueDelta(hexToLch(pal["--kbd-bg"]).H, hexToLch(pal["--accent"]).H);
    ok(d > 25, `${def.id}/${theme}: spot colour is a different hue, got ${d.toFixed(1)}°`);
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
ok(schemeFor({ colorScheme: "", accent: "#4C7DFF" }).id === "neutral", "schemeFor default");
ok(
  schemeFor({ colorScheme: "auto", accent: "#4C7DFF" }).family === "mono",
  "schemeFor migrates legacy free-form mode",
);
for (const theme of ["light", "dark"]) {
  const viaCurrent = currentPalette({ colorScheme: "aurora", accent: "#4C7DFF" }, theme);
  const viaScheme = schemePalette(schemeById("aurora"), theme);
  ok(viaCurrent["--accent"] === viaScheme["--accent"], `currentPalette/${theme} matches preset`);
}

console.log(`palette tests: ${checks} assertions passed across ${SCHEMES.length * 2} scheme variants`);
