import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { applyMirror, assetFileName } from "./updater";
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
  /** Compact layout, used inside the launcher banner. */
  compact?: boolean;
}

type Phase = "idle" | "downloading" | "done" | "error";

/**
 * The three actions an update offers: install right away, only download the
 * installer, or open the release page.
 *
 * The download runs on the Rust side and reports progress through the
 * `update://progress` event, so the bar keeps moving even when the webview
 * is busy.
 */
export default function UpdateActions({
  assetUrl,
  htmlUrl,
  mirror,
  compact = false,
}: UpdateActionsProps) {
  // Re-render on a language change so every label below is translated.
  const lang = useLang();
  void lang;

  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [path, setPath] = useState("");
  const [error, setError] = useState("");

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
        const url = applyMirror(assetUrl, mirror);
        const fileName = assetFileName(assetUrl, "kestrel-update.bin");
        const id = await api.updateDownload(url, fileName);
        jobRef.current = id;
      } catch (e) {
        setPhase("error");
        setError(String(e));
      }
    },
    [assetUrl, mirror],
  );

  const cancel = useCallback(() => {
    if (jobRef.current) void api.updateCancel(jobRef.current);
  }, []);

  const install = useCallback(() => {
    if (!path || installStarted.current) return;
    installStarted.current = true;
    handOver(path);
  }, [path, handOver]);

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

  if (phase === "downloading") {
    const received = progress?.received ?? 0;
    const total = progress?.total ?? 0;
    const pct = total > 0 ? Math.min(100, Math.round((received / total) * 100)) : 0;
    const speed = progress && progress.ms > 0 ? (received / 1024 / (progress.ms / 1000)).toFixed(1) : "0";
    return (
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
          <span>{t("速度")} {speed} KB/s</span>
          <button className="btn" data-size="sm" onClick={cancel}>
            {t("取消")}
          </button>
        </div>
      </div>
    );
  }

  if (phase === "done") {
    return (
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
    );
  }

  if (phase === "error") {
    return (
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
  }

  return (
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
  );
}

/** 1234567 -> "1.2 MB" */
function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
