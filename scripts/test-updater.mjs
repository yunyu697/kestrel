// Exercises the frontend update logic outside the WebView.
//
// The Tauri invoke is stubbed out on purpose, so this run exercises exactly
// the path a build without the Rust command would take: the WebView fallback.
// It proves the parsing, the mirror handling and the timing still work.
//
//   node_modules/.bin/esbuild scripts/test-updater.mjs --bundle --platform=node
//     --format=esm --external:@tauri-apps/* --outfile=.tmp/updater-test.mjs
//   node .tmp/updater-test.mjs

// --- browser globals the module expects -----------------------------------
const store = new Map();
globalThis.window = {
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  clearTimeout: (id) => clearTimeout(id),
  // Stand in for a build without the Rust command: every invoke rejects, so
  // the module has to fall back to the WebView's own fetch.
  __TAURI_INTERNALS__: {
    invoke: () => Promise.reject(new Error("no tauri in this environment")),
    transformCallback: (cb) => cb,
  },
};
globalThis.localStorage = {
  getItem: (k) => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => store.set(k, String(v)),
  removeItem: (k) => store.delete(k),
};

const {
  fetchLatestRelease,
  testMirrorLatency,
  MIRRORS,
  applyMirror,
  pickInstallAsset,
  assetFileName,
  compareVersions,
} = await import("../src/shared/updater.ts");

let failures = 0;
function check(name, cond, detail = "") {
  console.log(`${cond ? "OK  " : "FAIL"} ${name}${detail ? " — " + detail : ""}`);
  if (!cond) failures++;
}

// 1) The release API must answer and parse.
const info = await fetchLatestRelease();
check("fetchLatestRelease 返回版本", !!info && !!info.version, info ? `v${info.version}` : "null");
check("版本号格式正确", !!info && /^\d+\.\d+\.\d+$/.test(info.version), info?.version);
check(
  "资产列表非空",
  !!info && info.assets.length > 0,
  info ? `${info.assets.length} 个资产` : "",
);
check(
  "能挑出 MSI 安装包",
  !!info && !!info.assets.find((a) => a.name.endsWith(".msi")),
);

// 2) The mirror speed test must produce a timing for a reachable mirror.
//    Node's undici stalls on a no-cors request that redirects across origins,
//    so only the mirror case is asserted here. The real app goes through the
//    Rust channel, which follows redirects properly — that path is covered by
//    the Rust tests (cargo test -- --ignored).
const ghproxy = await testMirrorLatency("https://gh-proxy.com/", info?.version);
check("gh-proxy.com 测速返回延迟", typeof ghproxy === "number" && ghproxy > 0, `${ghproxy}ms`);

// 3) Mirror prefix handling.
check("镜像前缀拼接正确", applyMirror("https://x/y", "https://m/") === "https://m/https://x/y");
check("无前缀时原样返回", applyMirror("https://x/y", "") === "https://x/y");
check("镜像列表非空", MIRRORS.length >= 2, `${MIRRORS.length} 个`);

// 4) The installer must match how this copy was installed, otherwise the
//    update installs a second Kestrel instead of replacing this one.
const perMachine = pickInstallAsset(info?.assets ?? [], "perMachine");
const currentUser = pickInstallAsset(info?.assets ?? [], "currentUser");
check(
  "机器级安装选用 MSI",
  !!perMachine && perMachine.name.endsWith(".msi"),
  perMachine?.name,
);
check(
  "当前用户安装选用 NSIS",
  !!currentUser && currentUser.name.includes("setup"),
  currentUser?.name,
);
check(
  "两种安装方式选出的文件不同",
  !!perMachine && !!currentUser && perMachine.url !== currentUser.url,
);

// 5) The download file name comes from the URL.
check(
  "从下载地址解析文件名",
  assetFileName("https://github.com/o/r/releases/download/v1.2.4/Kestrel_1.2.4_x64_zh-CN.msi", "x") ===
    "Kestrel_1.2.4_x64_zh-CN.msi",
);
check("空地址回退到默认名", assetFileName("", "kestrel-update.bin") === "kestrel-update.bin");

// 6) Version comparison drives the "有更新" decision.
check("版本比较：新的更大", compareVersions("1.2.4", "1.2.3") === 1);
check("版本比较：相同为 0", compareVersions("1.2.3", "1.2.3") === 0);
check("版本比较：旧的更小", compareVersions("1.2.2", "1.2.3") === -1);
check("版本比较：位数不同", compareVersions("1.10.0", "1.2.3") === 1);

console.log(failures === 0 ? "\n全部通过" : `\n${failures} 项失败`);
process.exit(failures === 0 ? 0 : 1);
