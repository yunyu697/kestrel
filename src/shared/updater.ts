// GitHub release update check.
//
// The request runs on the Rust side (WinHTTP) instead of the WebView, for
// two reasons that both bite exactly where it matters:
//
//   - the WebView's content security policy can block cross-origin fetches
//     without saying anything in the UI, and
//   - Chromium follows the Windows proxy settings but ignores the
//     `http_proxy` / `https_proxy` environment variables a lot of users set,
//     so the WebView cannot reach the network while curl and git can.
//
// The WebView's own fetch stays as a fallback for builds that do not have
// the Rust command yet. Release downloads are plain github.com links; users
// behind slow GitHub routes can prefix them with a mirror (the standard
// `https://mirror/https://github.com/...` form).

import { api } from "./api";
import type { HttpProbeResult, Settings } from "./types";

export const UPDATE_REPO = "yunyu697/kestrel";

const LS_LAST = "kestrel.update.lastCheck";
const LS_FOUND = "kestrel.update.found";
/** URL of the newest installer we know about — the speed test measures it. */
const LS_LATEST_ASSET = "kestrel.update.latestAsset";

export interface UpdateMirror {
  id: string;
  name: string;
  /** URL prefix; "" means direct GitHub access. */
  prefix: string;
}

/** Built-in mirrors for release downloads. Latency varies a lot by region
 *  and over time, so the settings pane can test them live. The list only
 *  keeps mirrors that actually answer — the ones that died are gone. */
export const MIRRORS: UpdateMirror[] = [
  { id: "direct", name: "官方直连", prefix: "" },
  { id: "ghproxycom", name: "gh-proxy.com", prefix: "https://gh-proxy.com/" },
  { id: "ghproxynet2", name: "gh-proxy.net", prefix: "https://gh-proxy.net/" },
  { id: "ghproxynet", name: "ghproxy.net", prefix: "https://ghproxy.net/" },
];

/**
 * The prefix to put in front of a release URL, with exactly one trailing
 * slash — or `""` for the official direct route.
 *
 * The empty string is a *meaning*, not an empty prefix. Appending a slash to
 * it produced `"/"`, which joined to a GitHub URL as `"/https://github.com/…"`
 * and made every download fail with "unsupported protocol: /https" — the
 * default setting, so the updater could not download at all.
 */
export function mirrorPrefix(settings: Pick<Settings, "updateMirror">): string {
  const raw = settings.updateMirror.trim();
  if (!raw) return "";
  return raw.replace(/\/+$/, "") + "/";
}

/**
 * Join a mirror prefix and a release URL.
 *
 * A prefix has to be an absolute URL, or the result stops being one. A bare
 * host (`gh-proxy.com`) is treated as https, and anything that still is not
 * http(s) — a stale value, a typo, a scheme this client cannot speak — is
 * ignored so the download falls back to the direct route instead of failing
 * with a protocol error the user cannot act on.
 */
export function applyMirror(url: string, prefix: string): string {
  const base = prefix.trim();
  if (!base) return url;
  const absolute = /^[a-z][a-z0-9+.-]*:\/\//i.test(base) ? base : `https://${base}`;
  const trimmed = absolute.replace(/\/+$/, "");
  // It has to be an http(s) URL with a real host. A bare "/" turns into
  // "https:" after trimming, and a host with a space is not a host — joining
  // either of those would put a second scheme inside the URL.
  if (!/^https?:\/\/[^/]/i.test(trimmed) || /\s/.test(trimmed)) return url;
  return trimmed + "/" + url;
}

export interface ReleaseAsset {
  name: string;
  url: string;
  size: number;
}

export interface ReleaseInfo {
  /** Tag without the leading "v". */
  version: string;
  notes: string;
  htmlUrl: string;
  assets: ReleaseAsset[];
}

/** Probe a URL through the Rust side. Returns null when the command is not
 *  there (an older build), so the caller can fall back to the WebView. */
async function probeViaRust(
  url: string,
  timeoutMs: number,
  rangeBytes?: number,
): Promise<HttpProbeResult | null> {
  try {
    // Never let a stuck IPC call freeze the UI. The Rust side has a timeout
    // of its own; this is only the safety net.
    const guard = new Promise<null>((resolve) =>
      window.setTimeout(() => resolve(null), timeoutMs + 3000),
    );
    const call = api.httpProbe(url, timeoutMs, rangeBytes).catch(() => null);
    return await Promise.race([call, guard]);
  } catch {
    return null;
  }
}

interface Fetched {
  status: number;
  text: string;
  ms: number;
}

/** GET a URL: Rust first, WebView second. */
async function fetchText(url: string, timeoutMs: number): Promise<Fetched | null> {
  const rust = await probeViaRust(url, timeoutMs);
  if (rust) {
    if (rust.status > 0) {
      return { status: rust.status, text: rust.body ?? "", ms: rust.ms };
    }
    // The Rust side could not reach the server either — but the WebView may
    // still succeed, so keep going instead of giving up here.
  }

  const ctl = new AbortController();
  const timer = window.setTimeout(() => ctl.abort(), timeoutMs);
  const t0 = performance.now();
  try {
    const res = await fetch(url, { cache: "no-store", signal: ctl.signal });
    const text = await res.text();
    return { status: res.status, text, ms: Math.round(performance.now() - t0) };
  } catch {
    return null;
  } finally {
    window.clearTimeout(timer);
  }
}

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  const r = await fetchText(
    `https://api.github.com/repos/${UPDATE_REPO}/releases/latest`,
    15_000,
  );
  if (!r) throw new Error("network unavailable");
  // 404 = the repo simply has no published release yet; that is "no update",
  // not a failure.
  if (r.status === 404) return null;
  if (r.status < 200 || r.status >= 300) throw new Error(`HTTP ${r.status}`);
  const d = JSON.parse(r.text);
  const assets: ReleaseAsset[] = (d.assets || []).map(
    (a: { name: string; browser_download_url: string; size: number }) => ({
      name: a.name,
      url: a.browser_download_url,
      size: a.size,
    }),
  );
  // Remember the newest installer: the mirror speed test measures it.
  const asset = pickAsset(assets);
  if (asset) localStorage.setItem(LS_LATEST_ASSET, asset.url);
  return {
    version: String(d.tag_name || "").replace(/^v/i, ""),
    notes: String(d.body || ""),
    htmlUrl: String(d.html_url || `https://github.com/${UPDATE_REPO}/releases/latest`),
    assets,
  };
}

/** Fallback when api.github.com is unreachable (common on some networks):
 *  jsDelivr's data API lists the repo's git tags, answers CORS, and is
 *  usually reachable where GitHub's API is not. It carries no assets or
 *  notes, so the caller only gets a version plus the release page link. */
async function fetchLatestViaJsDelivr(): Promise<ReleaseInfo | null> {
  const r = await fetchText(
    `https://data.jsdelivr.com/v1/packages/gh/${UPDATE_REPO}`,
    15_000,
  );
  if (!r) throw new Error("network unavailable");
  if (r.status < 200 || r.status >= 300) throw new Error(`HTTP ${r.status}`);
  const d = JSON.parse(r.text);
  const tags: string[] = (d.versions || [])
    .map((v: { version?: string }) => String(v.version || "").replace(/^v/i, ""))
    .filter((t: string) => /^\d+(\.\d+)*$/.test(t));
  if (!tags.length) return null;
  const latest = tags.sort(compareVersions).pop() as string;
  return {
    version: latest,
    notes: "",
    htmlUrl: `https://github.com/${UPDATE_REPO}/releases/tag/v${latest}`,
    assets: [],
  };
}

/** 1 when a is newer than b, -1 when older, 0 when equal. */
export function compareVersions(a: string, b: string): number {
  const pa = String(a).split(".").map((n) => parseInt(n, 10) || 0);
  const pb = String(b).split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d !== 0) return d > 0 ? 1 : -1;
  }
  return 0;
}

/** The asset a normal user should get: MSI installer, then NSIS setup,
 *  then the portable zip as a last resort. */
export function pickAsset(assets: ReleaseAsset[]): ReleaseAsset | null {
  return (
    assets.find((a) => a.name.endsWith(".msi")) ??
    assets.find((a) => a.name.includes("setup") && a.name.endsWith(".exe")) ??
    assets.find((a) => a.name.endsWith(".zip")) ??
    null
  );
}

/** The installer that replaces *this* installation.
 *
 *  A per-machine copy (MSI → Program Files) has to be upgraded with the MSI,
 *  a current-user copy (NSIS → AppData) with the NSIS setup — running the
 *  other one would install a second copy next to the running one instead of
 *  replacing it. */
export function pickInstallAsset(
  assets: ReleaseAsset[],
  mode: string,
): ReleaseAsset | null {
  if (mode === "currentUser") {
    return (
      assets.find((a) => a.name.includes("setup") && a.name.endsWith(".exe")) ??
      assets.find((a) => a.name.endsWith(".zip")) ??
      null
    );
  }
  return pickAsset(assets);
}

/** The file name of a download URL — what the installer is saved as. */
export function assetFileName(url: string, fallback: string): string {
  try {
    const clean = url.split("?")[0].split("#")[0];
    const name = decodeURIComponent(clean.slice(clean.lastIndexOf("/") + 1));
    return name || fallback;
  } catch {
    return fallback;
  }
}

/** The file the speed test measures: the newest installer we know about.
 *  Mirrors only proxy release assets — asking one for a release *page* gets
 *  a 403 — so the test has to point at a real download. */
function speedTestTarget(currentVersion?: string): string {
  const known = localStorage.getItem(LS_LATEST_ASSET);
  if (known) return known;
  const v = (currentVersion || "").trim();
  if (v && v !== "0.0.0") {
    return `https://github.com/${UPDATE_REPO}/releases/download/v${v}/Kestrel_${v}_x64_zh-CN.msi`;
  }
  return `https://github.com/${UPDATE_REPO}/releases/latest`;
}

/** Rough reachability + latency for one mirror. A range request keeps the
 *  transfer tiny, and any HTTP answer counts as reachable — only a transport
 *  error means the mirror is down. null = unreachable. */
export async function testMirrorLatency(
  prefix: string,
  currentVersion?: string,
  timeoutMs = 12_000,
): Promise<number | null> {
  const url = applyMirror(speedTestTarget(currentVersion), prefix);

  const rust = await probeViaRust(url, timeoutMs, 1024);
  if (rust) {
    return rust.status > 0 ? rust.ms : null;
  }

  // Fallback for builds without the Rust command: a HEAD request, so the
  // browser asks no more than the headers and never pulls the installer.
  const t0 = performance.now();
  try {
    const ctl = new AbortController();
    const timer = window.setTimeout(() => ctl.abort(), timeoutMs);
    await fetch(url, {
      method: "HEAD",
      mode: "no-cors",
      cache: "no-store",
      signal: ctl.signal,
    });
    window.clearTimeout(timer);
    return Math.round(performance.now() - t0);
  } catch {
    return null;
  }
}

export interface UpdateCheckResult {
  checkedAt: number;
  current: string;
  latest: string | null;
  hasUpdate: boolean;
  info: ReleaseInfo | null;
  error: boolean;
}

export async function checkForUpdate(current: string): Promise<UpdateCheckResult> {
  try {
    // Primary source first; when GitHub's API is unreachable, jsDelivr can
    // still tell us the newest tag. If that throws too, we report an error.
    let info: ReleaseInfo | null;
    try {
      info = await fetchLatestRelease();
    } catch {
      info = await fetchLatestViaJsDelivr();
    }
    if (!info) {
      const r: UpdateCheckResult = {
        checkedAt: Date.now(),
        current,
        latest: null,
        hasUpdate: false,
        info: null,
        error: false,
      };
      localStorage.setItem(LS_LAST, String(r.checkedAt));
      localStorage.removeItem(LS_FOUND);
      return r;
    }
    const r: UpdateCheckResult = {
      checkedAt: Date.now(),
      current,
      latest: info.version,
      hasUpdate: compareVersions(info.version, current) > 0,
      info,
      error: false,
    };
    localStorage.setItem(LS_LAST, String(r.checkedAt));
    if (r.hasUpdate && r.info) {
      // The installer has to match how this copy was installed, otherwise
      // the update installs a second Kestrel next to the running one.
      const mode = await api.installMode().catch(() => "perMachine");
      const asset = pickInstallAsset(r.info.assets, mode);
      localStorage.setItem(
        LS_FOUND,
        JSON.stringify({
          version: info.version,
          assetUrl: asset?.url ?? "",
          htmlUrl: info.htmlUrl,
        }),
      );
    } else {
      localStorage.removeItem(LS_FOUND);
    }
    return r;
  } catch {
    const r: UpdateCheckResult = {
      checkedAt: Date.now(),
      current,
      latest: null,
      hasUpdate: false,
      info: null,
      error: true,
    };
    localStorage.setItem(LS_LAST, String(r.checkedAt));
    return r;
  }
}

export function lastCheckAt(): number {
  return parseInt(localStorage.getItem(LS_LAST) || "0", 10) || 0;
}

export interface FoundUpdate {
  version: string;
  assetUrl: string;
  htmlUrl: string;
}

/** What a previous check found, if anything. */
export function foundUpdate(): FoundUpdate | null {
  try {
    const raw = localStorage.getItem(LS_FOUND);
    return raw ? (JSON.parse(raw) as FoundUpdate) : null;
  } catch {
    return null;
  }
}

export function clearFoundUpdate(): void {
  localStorage.removeItem(LS_FOUND);
}

/** Decide whether a startup check should run right now. */
export function shouldCheckAtStartup(
  s: Pick<Settings, "updateCheck" | "startupCheck">,
): boolean {
  if (s.updateCheck === "off") return false;
  if (s.startupCheck) return true;
  const last = lastCheckAt();
  if (!last) return true; // never checked: once is fair
  const interval = s.updateCheck === "weekly" ? 7 * 86400_000 : 86400_000;
  return Date.now() - last >= interval;
}
