// A "Run" row is a command line, not a file on disk — even when it resolves to
// one. The launcher's file actions (reveal, rename, delete) act on whatever
// `hit.path` points at, so routing a run row through them would delete or
// rename the *program* instead of the text that was typed. These assertions
// lock the two paths apart, and lock the row to the backend that runs it.
//
//   esbuild scripts/test-run.mjs --bundle --platform=node --format=esm \
//     --outfile=.tmp/run-test.mjs
//   node .tmp/run-test.mjs

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (p) => readFileSync(new URL(p, import.meta.url), "utf8");

const launcher = read("../src/launcher/Launcher.tsx");
const types = read("../src/shared/types.ts");
const settings = read("../src/settings/Settings.tsx");
const api = read("../src/shared/api.ts");
const searchRs = read("../src-tauri/src/commands/search.rs");
const runRs = read("../src-tauri/src/run.rs");
const modelRs = read("../src-tauri/src/model.rs");

// ---- the row exists, and is a first-class kind everywhere ----
assert.ok(
  /export type HitKind =[\s\S]*?\| "run"/.test(types),
  "HitKind 里没有 run，类型层面就不承认这个结果",
);
assert.ok(
  /run: "terminal"/.test(launcher),
  "run 行没有图标映射，会退化成普通文件图标",
);

// ---- keyboard: Enter runs it, Ctrl+Shift+Enter elevates it ----
const openSelected = launcher.slice(
  launcher.indexOf("const openSelected"),
  launcher.indexOf("const handleCommand"),
);
assert.ok(
  openSelected.indexOf('hit.kind === "run"') >= 0 &&
    openSelected.indexOf('hit.kind === "run"') <
      openSelected.indexOf("const path = hit.path"),
  "openSelected 没有在读取 hit.path 之前拦下 run 行",
);
assert.ok(
  /api\.runCommand\(/.test(launcher) && /runCommand: \(raw: string, admin = false\)/.test(api),
  "前端没有把命令行交给 run_command_line",
);

// ---- the context menu of a run row is about running, nothing else ----
const buildActions = launcher.slice(
  launcher.indexOf("function buildActions"),
  launcher.indexOf("export default function Launcher"),
);
assert.ok(
  /if \(hit\.kind === "run"\) \{\s*return \[/.test(buildActions),
  "run 行拿到了文件操作菜单（重命名 / 删除会作用到程序上）",
);

const runAction = launcher.slice(
  launcher.indexOf("const runAction"),
  launcher.indexOf('case "open": launch(', launcher.indexOf("const runAction")),
);
assert.ok(
  runAction.indexOf('hit.kind === "run"') >= 0 &&
    runAction.indexOf('hit.kind === "run"') < runAction.indexOf("if (!hit.path)"),
  "runAction 先按文件处理了 run 行",
);

// ---- Quick Switch must never offer to start a program ----
assert.ok(
  /!quick \|\| h\.kind !== "run"/.test(launcher),
  "快速切换里也会出现 run 行，回车会启动程序而不是填对话框",
);

// ---- settings: both switches exist and reach the backend ----
assert.ok(
  /runEnabled/.test(settings) && /runRemember/.test(settings),
  "设置页缺少运行相关开关",
);
assert.ok(
  /run_enabled: true/.test(modelRs) && /run_remember: true/.test(modelRs),
  "运行功能默认是关的，升级后旧配置会静默失效",
);

// ---- search: the row is produced, and it outranks the rest ----
assert.ok(
  /if settings\.run_enabled \{/.test(searchRs) && /run_hit\(plan/.test(searchRs),
  "搜索没有产出 run 行",
);
assert.ok(
  /if h\.kind == "run" \{\s*0/.test(searchRs),
  "run 行没有被排到最前，输入会被文件结果淹没",
);

// ---- execution: a command typed in the launcher is not elevated by accident ----
const execute = runRs.slice(runRs.indexOf("pub fn execute"), runRs.indexOf("#[cfg(test)]"));
assert.ok(
  /if admin \{\s*shell::run_as_admin_args[\s\S]{0,120}?\}\s*else \{\s*shell::spawn_program_unelevated/
    .test(execute),
  "程序没有走非提权路径：以管理员运行的 Kestrel 会把每条命令都提权",
);
assert.ok(
  /pub fn shell_folder/.test(runRs) && /SHELL_FOLDERS_USER/.test(runRs),
  "shell: 名称没有按 Shell Folders 解析",
);
assert.ok(
  /pub fn remember/.test(runRs) && /MRUList/.test(runRs),
  "没有接 Win+R 的历史记录",
);

console.log("run: 命令行能运行、能提权、不会走文件操作，历史与 Win+R 共用");
