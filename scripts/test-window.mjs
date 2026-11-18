// The four window settings used to reach settings.json and stop there: nothing
// read them, so the sliders moved and the window never changed. This locks the
// values to the CSS custom properties that actually paint the window, so a
// future refactor cannot quietly disconnect them again.
//
//   esbuild scripts/test-window.mjs --bundle --platform=node --format=esm \
//     --outfile=.tmp/window-test.mjs
//   node .tmp/window-test.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const vars = new Map();
const attrs = new Map();

globalThis.document = {
  documentElement: {
    style: {
      setProperty: (k, v) => vars.set(k, String(v)),
      removeProperty: (k) => vars.delete(k),
    },
    setAttribute: (k, v) => attrs.set(k, String(v)),
  },
};

const { applyWindowChrome } = await import("../src/shared/theme.ts");

const ctl = (v) => Math.round((v * 4) / 9);
const card = (v) => Math.round((v * 2) / 3);

function apply(s) {
  vars.clear();
  attrs.clear();
  applyWindowChrome(s);
}

// Defaults shipped in Settings::default(): the radius family must stay exactly
// 18 / 12 / 8 so an untouched install looks identical to before.
apply({ opacity: 0.98, blur: true, cornerRadius: 18, fontScale: 1 });
assert.equal(vars.get("--radius-win"), "18px");
assert.equal(vars.get("--radius-card"), "12px");
assert.equal(vars.get("--radius-ctl"), "8px");
assert.equal(vars.get("--win-alpha"), "0.98");
assert.equal(vars.get("--font-scale"), "1");
assert.equal(attrs.get("data-blur"), "on");

// Slider extremes.
apply({ opacity: 0.6, blur: false, cornerRadius: 0, fontScale: 0.85 });
assert.equal(vars.get("--radius-win"), "0px");
assert.equal(vars.get("--radius-card"), "0px");
assert.equal(vars.get("--radius-ctl"), "0px");
assert.equal(vars.get("--win-alpha"), "0.6");
assert.equal(vars.get("--font-scale"), "0.85");
assert.equal(attrs.get("data-blur"), "off");

apply({ opacity: 1, blur: true, cornerRadius: 22, fontScale: 1.2 });
assert.equal(vars.get("--radius-win"), "22px");
assert.equal(vars.get("--radius-card"), `${card(22)}px`);
assert.equal(vars.get("--radius-ctl"), `${ctl(22)}px`);
assert.equal(vars.get("--win-alpha"), "1");

// A hand-edited config must not be able to push the window into a state the
// UI cannot recover from.
apply({ opacity: 0.1, blur: true, cornerRadius: 90, fontScale: 4 });
assert.equal(vars.get("--win-alpha"), "0.6");
assert.equal(vars.get("--radius-win"), "22px");
assert.equal(vars.get("--font-scale"), "1.2");

apply({ opacity: Number.NaN, blur: true, cornerRadius: Number.NaN, fontScale: Number.NaN });
assert.equal(vars.get("--win-alpha"), "0.6");
assert.equal(vars.get("--radius-win"), "0px");
assert.equal(vars.get("--font-scale"), "0.85");

// The type ramp has to ride --font-scale; a hardcoded size would ignore the
// slider the same way it did before.
const css = readFileSync(new URL("../src/shared/theme.css", import.meta.url), "utf8");
const ramp = [...css.matchAll(/--fs-[\w-]+:\s*([^;]+);/g)];
assert.ok(ramp.length >= 6, `type ramp looks wrong: ${ramp.length} steps`);
for (const [, value] of ramp) {
  assert.ok(
    value.includes("var(--font-scale"),
    `字号没有跟随 --font-scale：${value.trim()}`,
  );
}

// Both windows must apply the chrome; the launcher and the settings page each
// had their own copy of applyTheme and each forgot the same four fields.
for (const page of ["src/launcher/Launcher.tsx", "src/settings/Settings.tsx"]) {
  const src = readFileSync(new URL(`../${page}`, import.meta.url), "utf8");
  assert.ok(
    src.includes("applyWindowChrome("),
    `${page} 没有调用 applyWindowChrome`,
  );
}

console.log(`window chrome: ${ramp.length} 字号档位跟随缩放，四个设置均已接线`);
