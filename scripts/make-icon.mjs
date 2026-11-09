// Regenerate every app icon from the SVG sources.
//
//   src/assets/kestrel-light.svg — the executable / window / tray / installer
//     icon (always the light artwork) and the README preview PNG.
//   src/assets/kestrel-dark.svg  — the in-app logo on dark surfaces. It is
//     picked by CSS via --app-logo (src/shared/theme.css), so this script does
//     not touch it.
//
// Run `node scripts/make-icon.mjs` after editing either SVG. Requires the
// Tauri CLI from `npm install`.
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cli = join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");
const source = join(root, "src", "assets", "kestrel-light.svg");

if (!existsSync(cli)) throw new Error("Tauri CLI missing — run npm install first");
if (!existsSync(source)) throw new Error(`missing icon source: ${source}`);

function icon(...args) {
  const result = spawnSync(process.execPath, [cli, "icon", source, ...args], {
    cwd: root,
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

// Full set: png / ico / icns / Windows Store logos / mobile icons.
icon();

// A native 1024px render for the README preview — the default set tops out at
// 512px, and upscaling it would soften the artwork.
const tmp = mkdtempSync(join(tmpdir(), "kestrel-icon-"));
try {
  icon("--png", "1024", "-o", tmp);
  copyFileSync(join(tmp, "1024x1024.png"), join(root, "icon-1024.png"));
  console.log(`[kestrel] README preview updated: ${join(root, "icon-1024.png")}`);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
