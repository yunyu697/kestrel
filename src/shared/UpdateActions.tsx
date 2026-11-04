import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { applyMirror, assetFileName, MIRRORS, mirrorIdFor, mirrorPrefix } from "./updater";
import type { UpdateProgress } from "./types";
import { t, useLang } from "./i18n";
import "./update.css";

export interface UpdateActionsProps {
  /** Direct download URL of the installer (without a mirror prefix). */
  assetUrl: string;
  /** The release page — what "打开下载页面" opens. */
  htmlUrl: string;
  /** Mirror prefix from the settings ("" = direct GitHub). */
  mirror: string;
  /** Persist a mirror chosen here, so the settings page agrees next time. */
  onMirrorChange?: (prefix: string) => void;
  /** Compact layout, used inside the launcher banner. */
  compact?: boolean;
}

type Phase = "idle" | "downloading" | "done" | "error";

/** "https://gh-proxy.com/" -> "gh-proxy.com" for the drop-down label. */
function hostOf(prefix: string): string {
  try {
    return new URL(prefix).host;
  } catch {
    return prefix.replace(/^https?:\/\//i, "").replace(/\/+$/, "");
  }
}

/**
 * The three actions an update offers: install right away, only download the
 * installer, or open the release page — plus a drop-down that picks the
 * mirror both download buttons will use.
 *
 * The download runs on the Rust side and reports progress through the
 * `update://progress` event, so the bar keeps moving even when the webview
 * is busy.
 */
export default function UpdateActions({
  assetUrl,
  htmlUrl,
  mirror,
  onMirrorChange,
  compact = false,
}: UpdateActionsProps) {
  // Re-render on a language change so every label below is translated.
  const lang = useLang();
  void lang;

  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [path, setPath] = useState("");
  const [error, setError] = useState("");
  // The mirror chosen here. It starts from the settings and is written back
  // through onMirrorChange, so the launcher and the settings page stay in sync.
  const [prefix, setPrefix] = useState(mirror);
  const [customOpen, setCustomOpen] = useState(mirrorIdFor(mirror) === "custom");

  useEffect(() => {
    setPrefix(mirror);
    setCustomOpen(mirrorIdFor(mirror) === "custom");
  }, [mirror]);

  // The job id arrives with the answer of the start command, but the first
  // progress event can beat it — so the id is adopted from the event too.
  const jobRef = useRef<string | null>(null);
  const modeRef = useRef<"install" | "download">("download");
  // The installer may only be started once per download, however many
  // progress events arrive (a double-mounted effect would do it twice).
  const installStarted = useRef(false);

  useEffect(() => {
    let dispose: (() => void) | null = null;
    const pending = listen<UpdateProgress>("update://progress", (ev) => {
      const p = ev.payload;
      if (jobRef.current && p.id !== jobRef.current) return;
      if (!jobRef.current) jobRef.current = p.id;
      setProgress(p);
      if (p.status === "done") {
        setPhase("done");
        setPath(p.path);
        if (modeRef.current === "install" && !installStarted.current) {
          installStarted.current = true;
          handOver(p.path);
        }
      } else if (p.status === "error") {
        setPhase("error");
        setError(p.error || t("下载失败"));
      } else if (p.status === "canceled") {
        setPhase("idle");
        setProgress(null);
      }
    });
    pending
      .then((un) => {
        dispose = un;
      })
      .catch(() => {});
    return () => {
      dispose?.();
    };
  }, []);

  /**
   * Give the installer to the helper process and quit: the installer cannot
   * replace our own files while we are still running.
   */
  const handOver = useCallback((file: string) => {
    void api.updateInstall(file).then((ok) => {
      if (ok) {
        void api.quit();
      } else {
        installStarted.current = false;
        setPhase("error");
        setError(t("无法启动安装程序"));
      }
    });
  }, []);

  const start = useCallback(
    async (mode: "install" | "download") => {
      if (!assetUrl) return;
      modeRef.current = mode;
      jobRef.current = null;
      installStarted.current = false;
      setPhase("downloading");
      setError("");
      setProgress(null);
      try {
        const url = applyMirror(assetUrl, prefix);
        const fileName = assetFileName(assetUrl, "kestrel-update.bin");
        const id = await api.updateDownload(url, fileName, mode);
        jobRef.current = id;
      } catch (e) {
        setPhase("error");
        setError(String(e));
      }
    },
    [assetUrl, prefix],
  );

  const cancel = useCallback(() => {
    if (jobRef.current) void api.updateCancel(jobRef.current);
  }, []);

  const install = useCallback(() => {
    if (!path || installStarted.current) return;
    installStarted.current = true;
    handOver(path);
  }, [path, handOver]);

  const chooseMirror = (id: string) => {
    if (id === "custom") {
      // Keep a real custom prefix when there is one; otherwise start empty so
      // the user can type one without erasing a built-in choice.
      if (mirrorIdFor(prefix) !== "custom") setPrefix("");
      setCustomOpen(true);
      return;
    }
    setCustomOpen(false);
    const next = MIRRORS.find((m) => m.id === id)?.prefix ?? "";
    setPrefix(next);
    onMirrorChange?.(next);
  };

  const mirrorId = mirrorIdFor(prefix);
  const mirrorLabel =
    mirrorId === "custom"
      ? `${t("自定义")}${hostOf(prefix) ? ` — ${hostOf(prefix)}` : ""}`
      : mirrorId === "direct"
        ? t("官方直连")
        : (MIRRORS.find((m) => m.id === mirrorId)?.name ?? t("官方直连"));

  const mirrorPicker = (
    <div className="upd-mirror">
      <label className="upd-mirror-label" htmlFor="upd-mirror-select">
        {t("下载镜像")}
      </label>
      <select
        id="upd-mirror-select"
        className="select upd-mirror-select"
        value={mirrorId}
        onChange={(e) => chooseMirror(e.target.value)}
      >
        {MIRRORS.map((m) => (
          <option key={m.id} value={m.id}>
            {m.id === "direct" ? t(m.name) : m.name}
          </option>
        ))}
        <option value="custom">{t("自定义")}…</option>
      </select>
      {customOpen && (
        <input
          className="input upd-mirror-custom"
          value={prefix}
          placeholder={t("自定义镜像前缀")}
          spellCheck={false}
          onChange={(e) => setPrefix(e.target.value)}
          onBlur={() => {
            // Empty means "no custom prefix yet" — don't overwrite the saved
            // setting with it, or the drop-down would jump back to Direct.
            if (prefix.trim()) onMirrorChange?.(mirrorPrefix({ updateMirror: prefix }));
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && prefix.trim()) {
              onMirrorChange?.(mirrorPrefix({ updateMirror: prefix }));
              e.currentTarget.blur();
            }
          }}
        />
      )}
    </div>
  );

  if (!assetUrl) {
    // Nothing to install (the version was found through the fallback source,
    // which carries no assets) — the release page is all we can offer.
    return (
      <div className="upd-actions">
        <button className="btn" data-variant="primary" data-size="sm" onClick={() => void api.openUrl(htmlUrl)}>
          {t("打开下载页面")}
        </button>
      </div>
    );
  }

  let body: ReactNode;
  if (phase === "downloading") {
    const received = progress?.received ?? 0;
    const total = progress?.total ?? 0;
    const pct = total > 0 ? Math.min(100, Math.round((received / total) * 100)) : 0;
    const speed = progress && progress.ms > 0 ? (received / 1024 / (progress.ms / 1000)).toFixed(1) : "0";
    body = (
      <div className="upd-prog" data-compact={compact}>
        <div className="upd-prog-head">
          <span>{t("正在下载更新")}</span>
          <span className="upd-prog-num">
            {total > 0 ? `${pct}%` : ""} {formatBytes(received)} / {total > 0 ? formatBytes(total) : "?"}
          </span>
        </div>
        <div className="upd-prog-bar">
          <div className="upd-prog-fill" style={{ width: `${total > 0 ? pct : 8}%` }} />
        </div>
        <div className="upd-prog-foot">
          <span>{mirrorLabel} · {t("速度")} {speed} KB/s</span>
          <button className="btn" data-size="sm" onClick={cancel}>
            {t("取消")}
          </button>
        </div>
      </div>
    );
  } else if (phase === "done") {
    // "仅下载" ends here with the installer in the user's Downloads folder.
    // Offer the same install button as the automatic path, plus the folder.
    body = (
      <div className="upd-done-wrap">
        <div className="upd-done">
          <span className="upd-done-text">{t("下载完成")}</span>
          <div className="upd-actions">
            <button className="btn" data-variant="primary" data-size="sm" onClick={install}>
              {t("立即安装")}
            </button>
            <button className="btn" data-size="sm" onClick={() => void api.updateReveal(path)}>
              {t("打开所在文件夹")}
            </button>
          </div>
        </div>
        {path && (
          <div className="upd-path" title={path}>
            {t("已保存到")} {path}
          </div>
        )}
      </div>
    );
  } else if (phase === "error") {
    body = (
      <div className="upd-done">
        <span className="upd-done-text upd-err">{t("下载失败")}：{error}</span>
        <div className="upd-actions">
          <button className="btn" data-variant="primary" data-size="sm" onClick={() => void start(modeRef.current)}>
            {t("重试")}
          </button>
          <button className="btn" data-size="sm" onClick={() => void api.openUrl(htmlUrl)}>
            {t("打开下载页面")}
          </button>
        </div>
      </div>
    );
  } else {
    body = (
      <>
        <div className="upd-actions">
          <button className="btn" data-variant="primary" data-size="sm" onClick={() => void start("install")}>
            {t("更新")}
          </button>
          <button className="btn" data-size="sm" onClick={() => void start("download")}>
            {t("仅下载")}
          </button>
          <button className="btn" data-size="sm" onClick={() => void api.openUrl(htmlUrl)}>
            {t("打开下载页面")}
          </button>
        </div>
        <div className="upd-hint">
          {t("「仅下载」会保存到系统的「下载」文件夹，完成后可随时安装")}
        </div>
      </>
    );
  }

  return (
    <div className="upd-block" data-compact={compact}>
      {/* Idle: pick before pressing 更新 / 仅下载. Error: swap a mirror that
          failed before retrying. During a download or after it finished the
          picker would only be misleading, so it stays hidden. */}
      {(phase === "idle" || phase === "error") && mirrorPicker}
      {body}
    </div>
  );
}

/** 1234567 -> "1.2 MB" */
function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
