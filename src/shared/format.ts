export function formatBytes(n: number): string {
  if (!n || n < 0) return "—";
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

export function formatDate(ts: number): string {
  if (!ts) return "—";
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}`;
}

export function formatRelative(ts: number): string {
  if (!ts) return "—";
  const diff = Date.now() / 1000 - ts;
  if (diff < 60) return "刚刚";
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  if (diff < 86400 * 30) return `${Math.floor(diff / 86400)} 天前`;
  return formatDate(ts).slice(0, 10);
}

/** Shorten a path in the middle so both ends stay readable. */
export function shortPath(path: string, max = 60): string {
  if (!path || path.length <= max) return path;
  const parts = path.split("\\");
  if (parts.length <= 2) return path;
  const head = parts[0] + "\\";
  let tail = "";
  for (let i = parts.length - 1; i > 0; i--) {
    const next = parts[i] + (tail ? "\\" + tail : "");
    if ((head + "…\\" + next).length > max) break;
    tail = next;
  }
  return `${head}…\\${tail}`;
}

export function dirOf(path: string): string {
  const i = path.lastIndexOf("\\");
  return i > 0 ? path.slice(0, i) : path;
}

export const kindLabel: Record<string, string> = {
  file: "文件",
  folder: "文件夹",
  app: "应用",
  command: "命令",
  calc: "计算",
  web: "网页",
  favorite: "收藏",
  clipboard: "剪贴板",
  window: "窗口",
};

export function resolveTheme(mode: string): "light" | "dark" {
  if (mode === "light" || mode === "dark") return mode;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
