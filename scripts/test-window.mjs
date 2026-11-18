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
// The slider must never become an element opacity: that faded the text too.
assert.ok(vars.has("--surface-alpha"), "没有写入 --surface-alpha");
const defaultAlpha = Number(vars.get("--surface-alpha"));
assert.ok(defaultAlpha <= 1, `背景 alpha 越界：${defaultAlpha}`);

// Slider extremes.
apply({ opacity: 0.6, blur: false, cornerRadius: 0, fontScale: 0.85 });
assert.equal(vars.get("--radius-win"), "0px");
assert.equal(vars.get("--radius-card"), "0px");
assert.equal(vars.get("--radius-ctl"), "0px");
assert.equal(vars.get("--win-alpha"), "0.6");
assert.equal(vars.get("--font-scale"), "0.85");

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

// Readability floor: at the lowest slider position the surface must still be
// at least 82% opaque, or body text stops clearing 4.5:1 on a bright desktop.
// blur is a legacy config field with no visual effect, so both states match.
for (const blur of [true, false]) {
  for (const opacity of [0.6, 0.75, 0.98, 1]) {
    apply({ opacity, blur, cornerRadius: 18, fontScale: 1 });
    const a = Number(vars.get("--surface-alpha"));
    assert.ok(a >= 0.82 - 1e-9 && a <= 1, `alpha 越界 blur=${blur} opacity=${opacity}: ${a}`);
  }
}
apply({ opacity: 0.6, blur: false, cornerRadius: 18, fontScale: 1 });
const floorLow = Number(vars.get("--surface-alpha"));
apply({ opacity: 1, blur: false, cornerRadius: 18, fontScale: 1 });
assert.equal(floorLow, 0.82, `最低透明度应为 0.82：${floorLow}`);
assert.equal(Number(vars.get("--surface-alpha")), 1, "最高透明度应为 1");
apply({ opacity: 0.75, blur: true, cornerRadius: 18, fontScale: 1 });
const withBlur = Number(vars.get("--surface-alpha"));
apply({ opacity: 0.75, blur: false, cornerRadius: 18, fontScale: 1 });
assert.equal(Number(vars.get("--surface-alpha")), withBlur, "blur 状态改变了表面透明度");

// Neither window may use element opacity for the slider, and both must paint
// their glass from --surface-alpha. A single hardcoded opacity would fade the
// text again; a missing background would show no glass at all.
for (const page of ["src/launcher/launcher.css", "src/settings/settings.css"]) {
  const css = readFileSync(new URL(`../${page}`, import.meta.url), "utf8");
  assert.ok(
    !/opacity:\s*var\(--win-alpha/.test(css),
    `${page} 又把透明度接到了元素 opacity`,
  );
  assert.ok(
    /background:\s*color-mix\([^;]+var\(--surface-alpha/.test(css),
    `${page} 的玻璃背景没有使用 --surface-alpha`,
  );
}

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

// System acrylic has to cover the whole window rectangle, which cannot be
// rounded on a layered window — it produced square corners and a patchy
// backdrop. The setting page no longer offers the switch and the backend no
// longer applies the effect.
const settingsRs = readFileSync(
  new URL("../src-tauri/src/commands/settings.rs", import.meta.url),
  "utf8",
);
assert.ok(
  !/set_effects|Effect::Acrylic|apply_window_effects/.test(settingsRs),
  "系统毛玻璃又接回来了",
);
const settingsTsx = readFileSync(new URL("../src/settings/Settings.tsx", import.meta.url), "utf8");
assert.ok(
  !/t\("背景模糊"\)/.test(settingsTsx),
  "设置页又出现了无效的模糊开关",
);
const libRs = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
assert.ok(
  !/apply_window_regions|WindowEvent::Resized/.test(libRs),
  "窗口区域裁剪没有清理干净",
);
// The settings window must stay a normal (non-layered) frameless window so
// Win11 rounds its corners; a transparent one cannot be rounded.
const settingsBlock = libRs.slice(libRs.indexOf("ensure_settings_window"));
assert.ok(
  !/\.transparent\(true\)/.test(settingsBlock.slice(0, 1500)),
  "设置窗口又变成透明分层窗口，圆角会失效",
);

console.log(`window chrome: ${ramp.length} 字号档位跟随缩放，透明度有可读下限，无模糊`);
