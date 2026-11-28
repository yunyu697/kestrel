// The style layer has two failure modes that a type checker cannot see, and
// both are silent in the UI until someone opens the right window:
//
//   1. A style exists in the theme but not in a stylesheet (or the reverse), so
//      choosing it renders the default shell with a stray `data-style` value.
//   2. A style makes the Quick Switch search bar taller than 58px. The QS
//      window is sized in Rust from that number before the page paints, so the
//      footer gets clipped and the last row is cut — the exact bug the geometry
//      comment in monitor.rs warns about.
//
// These assertions read the same files the app compiles.
//
//   esbuild scripts/test-styles.mjs --bundle --platform=node --format=esm \
//     --outfile=.tmp/styles-test.mjs
//   node .tmp/styles-test.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (p) => readFileSync(new URL(p, import.meta.url), "utf8");

const theme = read("../src/shared/theme.ts");
const stylesTs = read("../src/shared/styles.ts");
const i18n = read("../src/shared/i18n.ts");
const settingsCss = read("../src/settings/styles.css");
const launcherCss = read("../src/launcher/styles.css");
const launcherTsx = read("../src/launcher/Launcher.tsx");
const monitorRs = read("../src-tauri/src/win/monitor.rs");
const modelRs = read("../src-tauri/src/model.rs");

const IDS = ["fluent", "apple", "material", "ant"];

// ---- one list of ids, used everywhere ----
assert.ok(
  /export const STYLE_IDS: StyleId\[\] = \[[^\]]*"fluent"[^\]]*"apple"[^\]]*"material"[^\]]*"ant"/.test(
    theme,
  ),
  "theme.ts 的 STYLE_IDS 不是四套",
);
for (const id of IDS) {
  assert.ok(
    theme.includes(`"${id}"`) && theme.includes(`  ${id}: [`),
    `theme.ts 里缺少 ${id} 的半径基准`,
  );
  assert.ok(
    new RegExp(`id: "${id}"`).test(stylesTs),
    `styles.ts 里缺少 ${id}，选择器上就没有这张卡片`,
  );
  assert.ok(
    settingsCss.includes(`[data-style="${id}"]`),
    `settings/styles.css 里没有 ${id} 的规则，选了它只会得到默认外壳`,
  );
  assert.ok(
    launcherCss.includes(`[data-style="${id}"]`),
    `launcher/styles.css 里没有 ${id} 的规则，启动器不会跟着变`,
  );
  assert.ok(
    launcherTsx.includes(`  ${id}: `),
    `Launcher.tsx 的 STYLE_ROW_H 里缺少 ${id} 的行高`,
  );
}
assert.equal(
  (theme.match(/\[number, number, number\]/g) || []).length,
  1,
  "半径表被拆成了多份，容易出现半套",
);

// ---- the style is persisted and applied, not just styled ----
assert.ok(
  /pub style: String/.test(modelRs),
  "model.rs 没有 style 字段，切换后不会写进配置文件",
);
assert.ok(
  /style: style_default\(\)/.test(modelRs),
  "model.rs 的默认值漏了 style",
);
assert.ok(
  /export function applyStyle\(/.test(theme) && /setAttribute\("data-style"/.test(theme),
  "theme.ts 没有把风格写到 data-style 上",
);
assert.ok(
  /applyStyle\(s\.style\)/.test(read("../src/settings/Settings.tsx")),
  "设置页没有调用 applyStyle，切换不会立即生效",
);
assert.ok(
  /applyStyle\(s\.style\)/.test(launcherTsx),
  "启动器没有调用 applyStyle，下一个窗口仍是旧风格",
);

// ---- the Quick Switch height contract ----
// monitor.rs computes the window from these constants; the CSS cannot exceed
// them without clipping the footer.
const qsChrome = monitorRs.match(/let chrome = ([\d.]+) \+ ([\d.]+) \+ ([\d.]+) \+ ([\d.]+) \+ ([\d.]+);/);
assert.ok(qsChrome, "monitor.rs 的 QS 尺寸公式变了，这里的断言需要跟着改");
const barLimit = Number(qsChrome[1]);
assert.equal(barLimit, 58, "QS 的搜索栏基准不是 58，样式层的上限需要重新核对");

const qsBarHeights = [...launcherCss.matchAll(/\[data-qs="true"\][^{]*\.bar\s*\{[^}]*?height:\s*(\d+)px/g)];
for (const m of qsBarHeights) {
  assert.ok(
    Number(m[1]) <= barLimit,
    `QS 搜索栏高 ${m[1]}px 超过了后端算好的 ${barLimit}px，页脚会被裁掉`,
  );
}

// A style that sets a bar height for the launcher must also set one for QS, or
// the launcher-only value leaks into the attached box.
const launcherBar = [...launcherCss.matchAll(/\[data-style="(\w+)"\]\s*\.shell\[data-qs="false"\]\s*\.bar/g)].map(
  (m) => m[1],
);
const qsBar = [...launcherCss.matchAll(/\[data-style="(\w+)"\]\s*\.shell\[data-qs="true"\]\s*\.bar/g)].map(
  (m) => m[1],
);
for (const id of launcherBar) {
  assert.ok(
    qsBar.includes(id),
    `${id} 只给了启动器的搜索栏高度，快速切换会沿用那个尺寸`,
  );
}

// ---- the row height is written from one place ----
assert.ok(
  /--row-h/.test(launcherTsx) && /style=\{\{ "--row-h"/.test(launcherTsx),
  "行高没有写到 .shell 上，选中块会和行错位",
);
assert.ok(
  !/--row-h:\s*\d+px/.test(read("../src/launcher/launcher.css")),
  "launcher.css 里又出现了写死的 --row-h，和 JS 里的数字会打架",
);

// ---- every style is described in every language ----
for (const id of IDS) {
  const block = stylesTs.slice(stylesTs.indexOf(`id: "${id}"`));
  const desc = block.match(/desc:\s*"([^"]+)"/);
  assert.ok(desc, `${id} 没有说明文字`);
  const key = desc[1];
  // Four dictionaries: zh uses the key as its own text, zh-TW / en / ja / es
  // each carry a translation.
  const occurrences = i18n.split(`"${key}"`).length - 1;
  assert.equal(occurrences, 4, `${id} 的说明在四个字典里不齐（找到 ${occurrences} 处）`);
}

console.log(
  `styles: ${IDS.length} 套风格都有外壳与启动器规则；QS 搜索栏 ≤ ${barLimit}px；说明五语齐全`,
);
