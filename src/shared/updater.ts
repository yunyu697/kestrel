// GitHub release update check.
//
// Runs entirely in the WebView: api.github.com answers browser fetches with
// CORS headers, so no Rust HTTP dependency is needed. Release downloads are
// plain github.com links; users behind slow GitHub routes can prefix them
// with a mirror (the standard `https://mirror/https://github.com/...` form).

import type { Settings } from "./types";

export const UPDATE_REPO = "yunyu697/kestrel";

const LS_LAST = "kestrel.update.lastCheck";
const LS_FOUND = "kestrel.update.found";

export interface UpdateMirror {
  id: string;
  name: string;
  /** URL prefix; "" means direct GitHub access. */
  prefix: string;
}

/** Built-in mirrors for release downloads. Latency varies a lot by region
 *  and over time, so the settings pane can test them live. */
export const MIRRORS: UpdateMirror[] = [
  { id: "direct", name: "官方直连", prefix: "" },
  { id: "ghfast", name: "ghfast.top", prefix: "https://ghfast.top/" },
  { id: "ghproxy", name: "gh-proxy.com", prefix: "https://gh-proxy.com/" },
  { id: "ghproxynet", name: "ghproxy.net", prefix: "https://ghproxy.net/" },
];

export function mirrorPrefix(settings: Pick<Settings, "updateMirror">): string {
  return settings.updateMirror.replace(/\/?$/, "/");
}

export function applyMirror(url: string, prefix: string): string {
  if (!prefix) return url;
  // Normalize here too: callers may pass a raw mirror string that lacks the
  // trailing slash, and the prefix form requires exactly one.
  return prefix.replace(/\/?$/, "/") + url;
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

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  const res = await fetch(`https://api.github.com/repos/${UPDATE_REPO}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json" },
  });
  // 404 = the repo simply has no published release yet; that is "no update",
  // not a failure.
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const d = await res.json();
  return {
    version: String(d.tag_name || "").replace(/^v/i, ""),
    notes: String(d.body || ""),
    htmlUrl: String(d.html_url || `https://github.com/${UPDATE_REPO}/releases/latest`),
    assets: (d.assets || []).map((a: { name: string; browser_download_url: string; size: number }) => ({
      name: a.name,
      url: a.browser_download_url,
      size: a.size,
    })),
  };
}

/** Fallback when api.github.com is unreachable (common on some networks):
 *  jsDelivr's data API lists the repo's git tags, answers CORS, and is
 *  usually reachable where GitHub's API is not. It carries no assets or
 *  notes, so the caller only gets a version plus the release page link. */
async function fetchLatestViaJsDelivr(): Promise<ReleaseInfo | null> {
  const res = await fetch(`https://data.jsdelivr.com/v1/packages/gh/${UPDATE_REPO}`, {
    headers: { Accept: "application/json" },
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const d = await res.json();
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

/** A tiny fixed file used to time each mirror. */
const SPEEDTEST_URL = "https://raw.githubusercontent.com/github/gitignore/main/README.md";

/** Rough reachability + latency for one mirror. Uses a no-cors request, so
 *  the response is opaque — we only keep the timing. null = unreachable. */
export async function testMirrorLatency(prefix: string, timeoutMs = 8000): Promise<number | null> {
  const url = applyMirror(SPEEDTEST_URL, prefix);
  const t0 = performance.now();
  try {
    const ctl = new AbortController();
    const timer = window.setTimeout(() => ctl.abort(), timeoutMs);
    await fetch(url, { mode: "no-cors", cache: "no-store", signal: ctl.signal });
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
      const asset = pickAsset(r.info.assets);
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
