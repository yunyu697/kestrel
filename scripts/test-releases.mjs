// Keeps the in-app changelog and the README history readable.
//
// Both used to drift into release-engineering prose: exit code 1603, install
// paths, assertion counts, "the helper is staged into a temp directory". A
// user reading 关于 → 更新日志 cannot act on any of that. The rules live in
// docs/release-notes.md; this file enforces the mechanical half of them so the
// drift cannot come back silently.
//
//   esbuild scripts/test-releases.mjs --bundle --platform=node --format=esm \
//     --outfile=.tmp/releases-test.mjs
//   node .tmp/releases-test.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { RELEASES } from "../src/shared/releases.ts";

/** Entries older than this predate the spec and are left as they were. */
const SINCE = "1.2.19";
const ZH_MAX = 40;
const README_MAX = 60;
const MAX_ITEMS = 5;
const LANGS = ["zh", "en", "ja", "es"];

/** Things a user cannot act on, or should not have to read. */
const BANNED = [
  ["错误码", /\b(1603|1619|1625|1730|1925|11730)\b/],
  ["盘符路径", /[A-Za-z]:[\\/]/],
  ["环境变量", /%[A-Za-z]+%/],
  ["文件名", /\.(exe|dll|msi|log|rs|tsx?|json|css|toml)\b/i],
  ["命令行参数", /(^|\s)--[a-z]/],
  ["开发者指标", /条断言|\bassertions?\b/i],
];

/** 1.2.9 < 1.2.19, which string comparison would get backwards. */
function cmp(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] - pb[i];
  }
  return 0;
}

function scan(where, text) {
  for (const [label, re] of BANNED) {
    assert.ok(!re.test(text), `${where} 出现了${label}：${text}`);
  }
}

let items = 0;
const live = RELEASES.filter((r) => cmp(r.version, SINCE) >= 0);

for (const r of live) {
  const where = `v${r.version}`;
  assert.ok(
    r.items.length >= 1 && r.items.length <= MAX_ITEMS,
    `${where}：${r.items.length} 条，上限 ${MAX_ITEMS} 条——超出就合并同类或删掉内部改动`,
  );
  for (const lang of LANGS) {
    assert.ok(r.title[lang], `${where}：标题缺 ${lang}`);
  }
  r.items.forEach((item, i) => {
    const at = `${where} 第 ${i + 1} 条`;
    for (const lang of LANGS) {
      assert.ok(item.text[lang]?.trim(), `${at}：缺 ${lang}`);
    }
    assert.ok(
      item.text.zh.length <= ZH_MAX,
      `${at}：中文 ${item.text.zh.length} 字，上限 ${ZH_MAX} 字——先砍定语，别砍信息`,
    );
    for (const lang of LANGS) scan(at, item.text[lang]);
    items++;
  });
}

const readme = readFileSync(new URL("../README.md", import.meta.url), "utf8");
let rows = 0;
for (const line of readme.split("\n")) {
  const m = /^\| (1\.2\.\d+) \| \d{4}-\d{2}-\d{2} \| (.*) \|$/.exec(line);
  if (!m || cmp(m[1], SINCE) < 0) continue;
  rows++;
  assert.ok(
    m[2].length <= README_MAX,
    `README ${m[1]}：${m[2].length} 字，上限 ${README_MAX} 字`,
  );
  scan(`README ${m[1]}`, m[2]);
}

assert.equal(
  rows,
  live.length,
  `README 有 ${rows} 行、releases.ts 有 ${live.length} 版（${SINCE} 起），应当一致`,
);

console.log(`更新日志检查通过：${live.length} 版 / ${items} 条 / README ${rows} 行`);
