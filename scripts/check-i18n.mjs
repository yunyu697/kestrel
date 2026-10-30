// Verifies that every key used via t("...") exists in all language
// dictionaries in src/shared/i18n.ts. Exits 1 when a translation is missing,
// so CI and pre-commit can gate on it. Run: node scripts/check-i18n.mjs
import fs from "fs";

const src = fs.readFileSync("src/shared/i18n.ts", "utf8");

// collect keys used via t("...")
const files = [];
function walk(d) {
  for (const f of fs.readdirSync(d)) {
    const p = d + "/" + f;
    const st = fs.statSync(p);
    if (st.isDirectory()) {
      if (f !== "node_modules") walk(p);
    } else if (/\.(tsx|ts)$/.test(f)) files.push(p);
  }
}
walk("src");
const used = new Set();
for (const f of files) {
  const c = fs.readFileSync(f, "utf8");
  for (const m of c.matchAll(/\bt\(\s*"((?:[^"\\]|\\.)*)"/g)) used.add(m[1]);
}

// parse dict blocks
function block(name) {
  const i = src.indexOf(`const ${name}: Dict = {`);
  if (i < 0) return null;
  let j = src.indexOf("{", i + `const ${name}: Dict = `.length - 1);
  let depth = 0,
    k = j;
  for (; k < src.length; k++) {
    if (src[k] === "{") depth++;
    else if (src[k] === "}") {
      depth--;
      if (depth === 0) {
        k++;
        break;
      }
    }
  }
  return src.slice(j, k);
}

let failures = 0;
for (const d of ["en", "ja", "es", "zhTW"]) {
  const b = block(d);
  if (!b) {
    console.log(d, "NOT FOUND");
    failures++;
    continue;
  }
  const keys = new Set();
  for (const m of b.matchAll(/"((?:[^"\\]|\\.)*)"\s*:/g)) keys.add(m[1]);
  const missing = [...used].filter((k) => !keys.has(k));
  const status = missing.length ? "FAIL" : "ok";
  console.log(
    `${d}: ${keys.size} keys, missing ${missing.length} [${status}]`,
    missing.slice(0, 12),
  );
  if (missing.length) failures++;
}
console.log("used total:", used.size);
process.exit(failures ? 1 : 0);
