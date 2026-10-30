// Bumps the build number on every build/dev run and records version + timestamp.
// The Rust build script reads build-info.json and exposes it to the backend.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pkgPath = resolve(root, "package.json");
const infoPath = resolve(root, "build-info.json");

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const prev = existsSync(infoPath)
  ? JSON.parse(readFileSync(infoPath, "utf8"))
  : { build: 0, version: pkg.version };

const now = new Date();
const pad = (n, w = 2) => String(n).padStart(w, "0");
const stamp =
  `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ` +
  `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;

const info = {
  name: "Kestrel",
  version: pkg.version,
  build: (Number(prev.build) || 0) + 1,
  builtAt: stamp,
  builtAtIso: now.toISOString(),
};

writeFileSync(infoPath, JSON.stringify(info, null, 2) + "\n", "utf8");
console.log(`[kestrel] v${info.version} build ${info.build} @ ${stamp}`);
