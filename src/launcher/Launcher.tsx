import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./launcher.css";
import { Icon } from "../shared/icons";
import { api } from "../shared/api";
import { setLang, t, useLang } from "../shared/i18n";
import { accentMix, formatBytes, formatDate, formatRelative, resolveTheme, shortPath } from "../shared/format";
import {
  baseName,
  isPathLike,
  matchKids,
  normFolder,
  resolvePath,
  sameFolder,
  trimEnd,
  upOne,
} from "../shared/pathnav";
import { applyColors } from "../shared/theme";
import {
  checkForUpdate,
  clearFoundUpdate,
  foundUpdate,
  mirrorPrefix,
  shouldCheckAtStartup,
  type FoundUpdate,
} from "../shared/updater";
import UpdateActions from "../shared/UpdateActions";
import type { ClipboardItem, FileInfo, SearchHit, Settings, WindowInfo } from "../shared/types";

type Mode = "search" | "clipboard" | "windows" | "notes";

const KIND_ICON: Record<string, string> = {
  file: "file",
  folder: "folder",
  app: "app",
  command: "gear",
  calc: "calc",
  web: "globe",
  favorite: "star",
  window: "window",
};

function glyphFor(hit: SearchHit): string {
  if (hit.kind === "command" && hit.meta) return (hit.meta.glyph as string) || "gear";
  if (hit.kind === "web") return "globe";
  if (hit.kind === "calc") return "calc";
  return KIND_ICON[hit.kind] || "file";
}

function buildActions(hit: SearchHit) {
  const isFolder = hit.kind === "folder" || hit.kind === "favorite";
  const isApp = hit.kind === "app";
  // A Store app is a `shell:AppsFolder` id, not a file: revealing, renaming,
  // deleting or hashing it cannot work, so those actions are not offered.
  const isUwp = isApp && hit.meta?.source === "uwp";
  const list: { id: string; label: string; icon: string; danger?: boolean; key?: string }[] = [
    { id: "open", label: t("打开"), icon: "external", key: "↵" },
    { id: "reveal", label: t("在资源管理器中显示"), icon: "folder", key: "Ctrl ↵" },
    { id: "openwith", label: t("打开方式…"), icon: "layers" },
  ];
  if (isApp) list.push({ id: "admin", label: t("以管理员身份运行"), icon: "lock", key: "Ctrl ⇧ ↵" });
  if (!isFolder) list.push({ id: "copyfile", label: t("复制文件"), icon: "copy" });
  list.push({ id: "copypath", label: t("复制路径"), icon: "copy" });
  list.push({ id: "copypathq", label: t("复制为路径（带引号）"), icon: "copy" });
  list.push({ id: "copyname", label: t("复制文件名"), icon: "copy" });
  if (!isFolder) list.push({ id: "checksum", label: t("计算校验和"), icon: "hash" });
  if (!isFolder) list.push({ id: "notepad", label: t("用记事本编辑"), icon: "file-text" });
  list.push({ id: "term", label: t("在此处打开终端"), icon: "terminal" });
  list.push({ id: "fav", label: t("添加到收藏夹"), icon: "star" });
  list.push({ id: "rename", label: t("重命名…"), icon: "pen" });
  list.push({ id: "props", label: t("属性"), icon: "info" });
  list.push({ id: "del", label: t("删除到回收站"), icon: "trash", danger: true, key: "Del" });
  if (!isUwp) return list;
  const fileOnly = new Set([
    "reveal",
    "openwith",
    "admin",
    "copyfile",
    "copypath",
    "copypathq",
    "checksum",
    "notepad",
    "term",
    "rename",
    "props",
    "del",
  ]);
  return list.filter((a) => !fileOnly.has(a.id));
}

export default function Launcher() {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [selected, setSelected] = useState(0);
  const [mode, setMode] = useState<Mode>("search");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [preview, setPreview] = useState<FileInfo | null>(null);
  const [icons, setIcons] = useState<Record<string, string>>({});
  const [hint, setHint] = useState<string | null>(null);
  // `count` is the committed index size; `scanned` is the live walk counter.
  // While a rebuild runs `count` still holds the *old* index (it is swapped in
  // only when the walk finishes), so showing it made the footer read
  // "索引中 0" for the whole first build.
  const [status, setStatus] = useState<{ state: string; count: number; scanned: number }>({
    state: "idle",
    count: 0,
    scanned: 0,
  });
  const shownCount = status.state === "scanning" ? status.scanned : status.count;
  const [toast, setToast] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [quick, setQuick] = useState(false);
  const [qsDirs, setQsDirs] = useState<string[]>([]);
  const [dirSel, setDirSel] = useState(0);
  // Where the attached dialog is standing. `folder` is what the dialog
  // reports; the box also remembers the last place it jumped to, because the
  // dialog's address bar is not always readable.
  const [qsFolder, setQsFolder] = useState<string | null>(null);
  const [qsHome, setQsHome] = useState("");
  const [qsKids, setQsKids] = useState<string[]>([]);
  const [hashModal, setHashModal] = useState<
    | { path: string; loading: true }
    | { path: string; loading: false; md5: string; sha1: string; sha256: string; size: number }
    | null
  >(null);
  // Live snapshot for event listeners that must read current state.
  const qsRef = useRef({ quick, query, qsDirs, dirSel, hits, selected, pathTarget: "" });
  // The listeners are registered above the jump handler, so they reach it
  // through a ref rather than closing over a binding declared further down.
  const qsJumpRef = useRef<(target: string, move?: boolean) => void>(() => {});
  qsRef.current = { quick, query, qsDirs, dirSel, hits, selected, pathTarget: "" };

  const [action, setAction] = useState<{ hit: SearchHit; items: ReturnType<typeof buildActions>; idx: number } | null>(null);

  const [clipboard, setClipboard] = useState<ClipboardItem[]>([]);
  const [clipSel, setClipSel] = useState(0);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [winSel, setWinSel] = useState(0);
  const [notes, setNotes] = useState("");

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const shellRef = useRef<HTMLDivElement>(null);
  const seq = useRef(0);
  const toastTimer = useRef<number | null>(null);
  const qsTimer = useRef<number | null>(null);
  const [narrow, setNarrow] = useState(false);
  // Icon cache lives in a ref so the loader callback stays stable.
  const iconCache = useRef<Record<string, string>>({});
  const iconPending = useRef<Set<string>>(new Set());

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1600);
  }, []);

  const rowH = settings?.compactMode ? 38 : 46;
  // Re-render on language change so every t() call below picks up the new one.
  const lang = useLang();
  void lang;

  // ---- update check ----
  const [updateFound, setUpdateFound] = useState<FoundUpdate | null>(null);
  // The banner stays a one-line strip until the user asks for the actions.
  const [updOpen, setUpdOpen] = useState(false);

  /** Silent update check; the banner flips on only when something is found. */
  const runUpdateCheck = useCallback(() => {
    return api
      .buildInfo()
      .then((b) => checkForUpdate(b.version))
      .then((r) => {
        if (r.hasUpdate) setUpdateFound(foundUpdate());
      })
      .catch(() => {});
  }, []);

  const dismissUpdate = useCallback(() => {
    clearFoundUpdate();
    setUpdateFound(null);
    setUpdOpen(false);
  }, []);

  // ---- theme + settings bootstrap ----
  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      applyTheme(s);
      setNotes(s.quickNotes || "");
      // Update check: a previous result may still be pending, and a fresh
      // one runs on startup when the settings ask for it. All async and
      // silent — the banner appears only when something was actually found.
      const fu = foundUpdate();
      if (fu) setUpdateFound(fu);
      if (shouldCheckAtStartup(s)) {
        void runUpdateCheck();
      }
    });
    api.indexStatus().then((st) => setStatus({ state: st.state, count: st.count, scanned: st.scanned }));
    api.clipboardList().then(setClipboard);
  }, []);

  // Periodic re-check while running: daily/weekly modes catch up here even
  // if the machine hibernates through several startup windows.
  useEffect(() => {
    if (!settings || settings.updateCheck === "off") return;
    const id = window.setInterval(() => void runUpdateCheck(), 4 * 3600_000);
    return () => window.clearInterval(id);
  }, [settings?.updateCheck, runUpdateCheck]);

  function applyTheme(s: Settings) {
    // Language first: every label below is translated at render time.
    setLang(s.language);
    const themeName = resolveTheme(s.theme);
    document.documentElement.setAttribute("data-theme", themeName);
    document.documentElement.setAttribute("data-anim", s.animations ? "on" : "off");
    // Per-element colour overrides (empty value = theme default).
    applyColors(s, themeName);
    document.documentElement.style.setProperty(
      "--accent-soft",
      accentMix(s.accent, 0.16),
    );
    document.documentElement.style.setProperty("--font-scale", String(s.fontScale));
  }

  // Chromium (WebView2) owns a few accelerators we never want here: Ctrl+F and
  // Ctrl+G open its built-in find bar, a separate popup window that lands on
  // top of the Quick Switch box and searches our own UI text (the result list
  // labels) instead of anything the user cares about. Ctrl+P would open a print
  // dialog. Swallowing them in the capture phase stops the browser default
  // while leaving our own key handling untouched.
  useEffect(() => {
    const block = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      const k = e.key.toLowerCase();
      if (k === "f" || k === "g" || k === "p") e.preventDefault();
    };
    window.addEventListener("keydown", block, true);
    return () => window.removeEventListener("keydown", block, true);
  }, []);

  /** Re-read where the attached dialog stands, and what is inside it. */
  const loadQsContext = useCallback(() => {
    void api
      .qsContext()
      .then((c) => {
        if (c.home) setQsHome(c.home);
        if (c.folder) setQsFolder(normFolder(c.folder));
        setQsKids(c.children || []);
      })
      .catch(() => {});
  }, []);

  const listenEvents = useCallback(async () => {
    const win = getCurrentWebviewWindow();
    const un = await win.listen("launcher://shown", async () => {
      if (settings?.clearQueryOnHide) {
        setQuery("");
      }
      setMode("search");
      setQuick(false);
      setMenuOpen(false);
      setAction(null);
      requestAnimationFrame(() => shellRef.current?.setAttribute("data-enter", "false"));
      setTimeout(() => inputRef.current?.focus(), 30);
      // Empty query → backend returns favorites + most-used + commands.
      try {
        const res = await api.search("");
        setHits(res.hits);
        setSelected(0);
        setHint(res.hint ?? null);
        loadIcons(res.hits);
      } catch {
        /* ignore */
      }
    });
    const uh = await win.listen("launcher://hiding", () => {
      // Park the card at the hidden state while the window is still invisible,
      // so the next show starts from the correct first frame (no flash).
      shellRef.current?.setAttribute("data-enter", "true");
    });
    const u2 = await win.listen("launcher://mode", (e: any) => {
      if (e.payload === "quickSwitch") {
        setQuick(true);
        setMode("search");
        setMenuOpen(false);
        setAction(null);
        setTimeout(() => inputRef.current?.focus(), 30);
      }
    });
    const u3 = await win.listen("index://progress", () =>
      api.indexStatus().then((s) => setStatus({ state: s.state, count: s.count, scanned: s.scanned })),
    );
    const u4 = await win.listen("index://done", () =>
      api.indexStatus().then((s) => setStatus({ state: s.state, count: s.count, scanned: s.scanned })),
    );
    const u5 = await win.listen("clipboard://changed", () => api.clipboardList().then(setClipboard));
    const u6 = await win.listen("settings://changed", (e: any) => {
      const s: Settings = e.payload;
      setSettings(s);
      applyTheme(s);
    });
    const u7 = await win.listen<string[]>("quickswitch://show", (e) => {
      setQuick(true);
      setMode("search");
      setQuery("");
      setHits([]);
      setAction(null);
      setMenuOpen(false);
      setQsDirs(Array.isArray(e.payload) ? e.payload : []);
      setDirSel(0);
      // A new dialog means a new location — drop the old one so relative
      // paths cannot quietly aim at the previous session's folder.
      setQsFolder(null);
      setQsKids([]);
      // Read where the dialog stands up front: the first keystroke already
      // needs it, and waiting for that keystroke costs a round trip.
      loadQsContext();
      requestAnimationFrame(() => shellRef.current?.setAttribute("data-enter", "false"));
      setTimeout(() => inputRef.current?.focus(), 30);
    });
    const u9 = await win.listen("quickswitch://moved", () => {
      // The dialog moved on its own (the user came back from Explorer), so
      // the box has to follow or it would resolve paths against the old
      // folder. It navigates asynchronously, hence the short wait.
      if (qsTimer.current) window.clearTimeout(qsTimer.current);
      qsTimer.current = window.setTimeout(loadQsContext, 350);
    });
    const u8 = await win.listen("quickswitch://jump", () => {
      const st = qsRef.current;
      if (!st.quick) return;
      // A resolved folder beats the result list: if the box is showing one,
      // that is what the user is aiming at.
      const target =
        st.pathTarget || (st.query === "" ? st.qsDirs[st.dirSel] : st.hits[st.selected]?.path);
      if (target) qsJumpRef.current(target, true);
    });
    return () => [un, u2, u3, u4, u5, u6, u7, u8, u9, uh].forEach((f) => f());
  }, [settings, loadQsContext]);

  useEffect(() => {
    let off: (() => void) | undefined;
    listenEvents().then((fn) => { off = fn; });
    return () => off?.();
  }, [listenEvents]);

  // ---- resize observer for narrow layout ----
  useEffect(() => {
    const el = shellRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setNarrow(el.clientWidth < 700));
    ro.observe(el);
    setNarrow(el.clientWidth < 700);
    return () => ro.disconnect();
  }, []);

  const loadIcons = useCallback((list: SearchHit[]) => {
    // Only the visible rows need an icon: decoding 60 PNGs on every
    // keystroke was a major source of lag while typing.
    const paths = list
      .slice(0, 16)
      .filter((h) => (h.kind === "file" || h.kind === "folder" || h.kind === "app" || h.kind === "favorite") && h.path)
      .map((h) => h.path as string)
      .filter((p) => !iconCache.current[p] && !iconPending.current.has(p));
    if (!paths.length) return;
    paths.forEach((p) => iconPending.current.add(p));
    api.iconsFor(paths, 32).then((map) => {
      paths.forEach((p) => iconPending.current.delete(p));
      iconCache.current = { ...iconCache.current, ...map };
      // Keep the cache bounded: a long session would otherwise keep every
      // base64 PNG it ever saw.
      const keys = Object.keys(iconCache.current);
      if (keys.length > 512) {
        const keep = new Set(keys.slice(keys.length - 512));
        const next: Record<string, string> = {};
        for (const k of keys) if (keep.has(k)) next[k] = iconCache.current[k];
        iconCache.current = next;
      }
      setIcons(iconCache.current);
    }).catch(() => {
      paths.forEach((p) => iconPending.current.delete(p));
    });
    // `iconCache` is read through a ref on purpose: keeping this callback
    // stable is what stops the "icons arrived -> search again -> icons
    // arrived" feedback loop that kept the launcher busy.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ---- Quick Switch path navigation ----
  // The attached box is sized in whole rows by the backend, so the list must
  // never claim more rows than the window was given.
  // Matches the slider in the settings page (1..8).
  const qsRows = Math.max(1, Math.min(8, settings?.qsVisibleRows || 3));

  // What the typed text means right now: the folder it resolves to, plus the
  // real subfolders that complete a half-typed name.
  const plan = useMemo(() => {
    const s = query;
    if (!quick || !s || !isPathLike(s)) return null;
    // Without a known location, only absolute input can be trusted — `..`
    // aimed at the wrong folder is worse than no path mode at all.
    const v = s.trim();
    const homeish = v === "~" || v.startsWith("~/") || v.startsWith("~\\");
    const absolute = /^[A-Za-z]:/.test(v) || /^[\\/]/.test(v) || homeish;
    if (!qsFolder && !absolute) return null;
    if (homeish && !qsHome) return null;
    const raw = resolvePath(s, qsFolder, qsHome);
    if (!raw) return null;
    const endsSep = /[\\/]$/.test(s);
    const segs = trimEnd(s).split(/[\\/]/);
    const last = segs[segs.length - 1] || "";
    const head = segs.slice(0, -1).join("\\");
    let candidates: string[] = [];
    // Set when the name was typed inside the current folder but matches
    // nothing in it — the target is then a guess, so it is shown as such.
    let tentative = false;
    // Completing only makes sense one level below where the dialog stands;
    // deeper input resolves directly instead.
    if (!endsSep && last && last !== ".." && last !== "." && !/^[A-Za-z]:$/.test(last)) {
      const headResolved = resolvePath(head, qsFolder, qsHome);
      if (headResolved && qsFolder && sameFolder(headResolved, qsFolder)) {
        candidates = matchKids(qsKids, last).slice(0, qsRows);
        tentative = candidates.length === 0;
      }
    }
    return { raw, candidates, tentative };
  }, [query, quick, qsFolder, qsHome, qsKids, qsRows]);

  // The folder Enter / → would take the dialog to.
  const pathTarget = plan
    ? plan.candidates.length
      ? plan.candidates[Math.min(selected, plan.candidates.length - 1)]
      : plan.raw
    : "";
  qsRef.current.pathTarget = pathTarget;

  /**
   * Move the dialog to `target` and re-read where it ended up.
   *
   * The box records the target itself, not just what the dialog later reports:
   * the dialog's address bar is not always readable, but we always know what
   * we asked for, so navigation keeps working even on a stubborn dialog. The
   * re-read happens on a delay because the dialog navigates asynchronously.
   */
  const qsJump = useCallback(async (target: string, move = false) => {
    if (!target) return;
    setQuery("");
    setHits([]);
    setSelected(0);
    setDirSel(0);
    if (move) setQsFolder(normFolder(target));
    setQsKids([]);
    // Not awaited: the navigation itself takes ~200ms of injected keystrokes
    // and the box must stay responsive while it happens.
    void api.quickSwitchApply(target);
    if (qsTimer.current) window.clearTimeout(qsTimer.current);
    qsTimer.current = window.setTimeout(loadQsContext, 300);
  }, [loadQsContext]);
  qsJumpRef.current = qsJump;

  // ---- search ----
  const runSearch = useCallback(
    async (q: string) => {
      const s = ++seq.current;
      try {
        const res = await api.search(q);
        if (s !== seq.current) return;
        setHits(res.hits);
        setSelected(0);
        setHint(res.hint ?? null);
        loadIcons(res.hits);
      } catch {
        /* ignore */
      }
    },
    [loadIcons],
  );

  useEffect(() => {
    // A typed path resolves locally, so skip the search round trip entirely.
    if (quick && isPathLike(query)) {
      setSelected(0);
      return;
    }
    const id = window.setTimeout(() => runSearch(query), 35);
    return () => window.clearTimeout(id);
  }, [query, runSearch, quick]);

  // ---- preview ----
  const selHit = hits[selected];
  useEffect(() => {
    if (!selHit) {
      setPreview(null);
      return;
    }
    if (selHit.kind === "file" || selHit.kind === "folder" || selHit.kind === "favorite" || selHit.kind === "app") {
      const p = selHit.path;
      if (!p) return setPreview(null);
      const id = window.setTimeout(() => api.fileInfo(p).then(setPreview).catch(() => setPreview(null)), 40);
      return () => window.clearTimeout(id);
    }
    setPreview(null);
  }, [selHit]);

  // ---- scroll selected into view ----
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const row = list.querySelector(`[data-idx="${selected}"]`) as HTMLElement | null;
    row?.scrollIntoView({ block: "nearest" });
  }, [selected, hits]);

  // ---- clipboard / windows panels data ----
  useEffect(() => {
    if (mode === "clipboard") api.clipboardList().then(setClipboard);
    if (mode === "windows") api.listWindows().then((w) => { setWindows(w); setWinSel(0); });
  }, [mode]);

  const openSelected = useCallback(
    async (shiftKey = false, ctrlKey = false) => {
      const hit = hits[selected];
      if (!hit) return;
      if (hit.kind === "calc" && hit.meta) {
        const v = String(hit.meta.display ?? hit.title);
        await api.clipboardPushText(v);
        showToast(t("已复制结果到剪贴板"));
        if (settings?.closeAfterOpen) api.hideLauncher();
        return;
      }
      if (hit.kind === "web" && hit.meta) {
        await api.openUrl(hit.meta.url as string);
        if (settings?.closeAfterOpen) api.hideLauncher();
        return;
      }
      if (hit.kind === "command" && hit.meta) {
        const id = hit.meta.commandId as string;
        await handleCommand(id);
        if (settings?.closeAfterOpen) api.hideLauncher();
        return;
      }
      const path = hit.path;
      if (!path) return;
      // Quick Switch: move the captured file dialog instead of opening.
      if (quick) {
        api.recordQuery(query);
        // Not marked as a move: a file only fills the name box and the dialog
        // stays put; the location is re-read from the dialog either way.
        await qsJump(path);
        return;
      }
      if (ctrlKey && !shiftKey) {
        await api.revealPath(path);
      } else if (shiftKey) {
        await api.openWith(path);
      } else {
        await api.openPath(path);
      }
      api.recordQuery(query);
      if (settings?.closeAfterOpen) api.hideLauncher();
    },
    [hits, selected, settings, query, quick, showToast, qsJump],
  );

  const handleCommand = useCallback(
    async (id: string) => {
      if (id === "cmd.clipboard") { setMode("clipboard"); return; }
      if (id === "cmd.notes") { setMode("notes"); return; }
      if (id === "cmd.about") { await api.openSettingsWindow(); return; }
      await api.runBuiltin(id);
    },
    [],
  );

  // ---- keyboard ----
  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (action) {
        if (e.key === "Escape") { setAction(null); return; }
        if (e.key === "ArrowDown") { e.preventDefault(); setAction((a) => a && { ...a, idx: Math.min(a.idx + 1, a.items.length - 1) }); }
        else if (e.key === "ArrowUp") { e.preventDefault(); setAction((a) => a && { ...a, idx: Math.max(a.idx - 1, 0) }); }
        else if (e.key === "Enter") { e.preventDefault(); runAction(action.items[action.idx]); }
        return;
      }
      if (mode === "clipboard") return clipKeyDown(e);
      if (mode === "windows") return winKeyDown(e);

      if (e.key === "Escape") {
        e.preventDefault();
        if (menuOpen) setMenuOpen(false);
        // Inside Quick Switch the first Escape clears what was typed and the
        // second one puts the box away, matching how a search box behaves.
        else if (quick && query) setQuery("");
        else api.hideLauncher();
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (quick && query === "") setDirSel((s) => Math.min(s + 1, Math.max(0, qsDirs.length - 1)));
        else if (plan && plan.candidates.length)
          setSelected((s) => Math.min(s + 1, plan.candidates.length - 1));
        else setSelected((s) => Math.min(s + 1, Math.max(0, hits.length - 1)));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        if (quick && query === "") setDirSel((s) => Math.max(s - 1, 0));
        else setSelected((s) => Math.max(s - 1, 0));
      } else if (e.key === "ArrowRight") {
        // → walks into a folder. Outside Quick Switch it belongs to the caret.
        if (!quick) return;
        e.preventDefault();
        const into = pathTarget || (query === "" ? qsDirs[dirSel] : "");
        if (into) void qsJump(into, true);
      } else if (e.key === "ArrowLeft") {
        // ← climbs one level, but only on an empty box: while the user is
        // typing, the key belongs to the caret. Each press goes one level up
        // because jumping empties the box again.
        if (!quick || query !== "" || !qsFolder) return;
        e.preventDefault();
        void qsJump(upOne(qsFolder), true);
      } else if (e.key === "Tab" || (e.key === "k" && e.ctrlKey)) {
        e.preventDefault();
        if (!quick && hits[selected]) setAction({ hit: hits[selected], items: buildActions(hits[selected]), idx: 0 });
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (quick && query === "") {
          const d = qsDirs[dirSel];
          if (d) void qsJump(d, true);
          return;
        }
        // A typed path wins over the result list.
        if (quick && pathTarget) {
          void qsJump(pathTarget, true);
          return;
        }
        openSelected(e.shiftKey, e.ctrlKey || e.metaKey);
      } else if (e.key === "," && e.ctrlKey) {
        e.preventDefault();
        api.openSettingsWindow();
      } else if (e.key === "1" && e.ctrlKey) {
        setMode((m) => (m === "clipboard" ? "search" : "clipboard"));
      }
    },
    [
      action,
      mode,
      hits,
      selected,
      menuOpen,
      quick,
      query,
      qsDirs,
      dirSel,
      openSelected,
      plan,
      pathTarget,
      qsFolder,
      qsJump,
    ],
  );

  const clipKeyDown = async (e: React.KeyboardEvent) => {
    if (e.key === "Escape") { e.preventDefault(); setMode("search"); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); setClipSel((s) => Math.min(s + 1, clipboard.length - 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setClipSel((s) => Math.max(s - 1, 0)); }
    else if (e.key === "Enter") {
      e.preventDefault();
      const it = clipboard[clipSel];
      if (it) { await api.clipboardCopy(it.id); showToast(t("已粘贴到剪贴板")); api.hideLauncher(); }
    } else if (e.key === "Delete") {
      const it = clipboard[clipSel];
      if (it) { api.clipboardDelete(it.id); setClipboard((c) => c.filter((x) => x.id !== it.id)); }
    } else if (e.key.toLowerCase() === "p") {
      const it = clipboard[clipSel];
      if (it) { api.clipboardPin(it.id, !it.pinned); setClipboard((c) => c.map((x) => (x.id === it.id ? { ...x, pinned: !x.pinned } : x))); }
    }
  };

  const winKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") { e.preventDefault(); setMode("search"); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); setWinSel((s) => Math.min(s + 1, windows.length - 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setWinSel((s) => Math.max(s - 1, 0)); }
    else if (e.key === "Enter") { e.preventDefault(); const w = windows[winSel]; if (w) { api.activateWindow(w.hwnd); api.hideLauncher(); } }
  };

  const runAction = async (item: { id: string; label: string }) => {
    const hit = action?.hit;
    if (!hit || !hit.path) return setAction(null);
    const p = hit.path;
    setAction(null);
    switch (item.id) {
      case "open": await api.openPath(p); break;
      case "reveal": await api.revealPath(p); break;
      case "openwith": await api.openWith(p); break;
      case "admin": if (hit.kind === "app") await api.runAsAdmin(p); else await api.runAsAdmin(p); break;
      case "copyfile": await api.copyFileRefs([p]); showToast(t("已复制文件")); break;
      case "copypath": await api.clipboardPushText(p); showToast(t("已复制路径")); break;
      case "copypathq": await api.clipboardPushText(`"${p}"`); showToast(t("已复制带引号路径")); break;
      case "copyname": await api.clipboardPushText(p.split("\\").pop() || p); showToast(t("已复制文件名")); break;
      case "checksum":
        setHashModal({ path: p, loading: true });
        api
          .fileHashes(p)
          .then((h) => setHashModal({ path: p, loading: false, ...h }))
          .catch(() => { setHashModal(null); showToast(t("计算校验和失败")); });
        return;
      case "notepad": await api.spawnProgram("notepad.exe", p); break;
      case "term": await api.openTerminal(p); break;
      case "fav": await api.favoritesAdd(p, hit.title); showToast(t("已加入收藏夹")); api.hideLauncher(); return;
      case "rename": {
        const nm = window.prompt(t("重命名为："), p.split("\\").pop() || "");
        if (nm) { const np = await api.renamePath(p, nm); if (np) showToast(t("已重命名")); runSearch(query); }
        return;
      }
      case "props": await api.showProperties(p); break;
      case "del": {
        if (window.confirm(t("确定将 “{name}” 删除到回收站？", { name: hit.title }))) {
          const n = await api.deleteToTrash([p]);
          showToast(t("已删除 {n} 项", { n }));
          runSearch(query);
        }
        return;
      }
    }
    if (settings?.closeAfterOpen && item.id !== "reveal" && item.id !== "props" && item.id !== "openwith" && item.id !== "term")
      api.hideLauncher();
  };

  const dropdown = [
    { id: "clip", label: t("剪贴板历史"), icon: "clipboard", key: "Ctrl 1", act: () => setMode("clipboard") },
    { id: "win", label: t("窗口切换"), icon: "window", act: () => setMode("windows") },
    { id: "note", label: t("快捷笔记"), icon: "note", act: () => setMode("notes") },
    { id: "color", label: t("颜色取色"), icon: "palette", act: async () => { const c = await api.pickColor(); await api.clipboardPushText(c.hex); showToast(t("已复制 {text}", { text: c.hex })); } },
    { id: "settings", label: t("设置…"), icon: "gear", key: "Ctrl ,", act: () => api.openSettingsWindow() },
    {
      id: "reindex",
      label: t("重建索引"),
      icon: "refresh",
      act: async () => {
        // A rebuild walks every indexed drive: minutes of disk churn and
        // temporarily incomplete file results. Never do that on a stray click.
        if (!window.confirm(t("重建索引会重新扫描所有索引磁盘，期间文件搜索结果可能不完整。确定继续？"))) return;
        await api.rebuildIndex();
        showToast(t("开始重建索引"));
      },
    },
  ];

  // The sidebar is driven by one setting only, so it can never flicker on and
  // off: the footer button and the settings page toggle the same value.
  const isPanelMode = ["clipboard", "windows", "notes"].includes(mode);
  const showPreview = !!settings?.showPreview && !narrow && !isPanelMode;
  const togglePreview = () => {
    if (!settings) return;
    const next = { ...settings, showPreview: !settings.showPreview };
    setSettings(next);
    api.saveSettings(next);
  };

  return (
    <div
      ref={shellRef}
      className="shell"
      data-narrow={narrow}
      data-compact={settings?.compactMode}
      data-qs={quick ? "true" : "false"}
      data-enter="true"
      data-preview={
        // The Quick Switch box is short and wide: a bottom dock would eat its
        // few result rows, so it always keeps the side panel.
        !quick && settings?.previewPosition === "bottom" ? "bottom" : "right"
      }
    >
      <div className="card">
        {updateFound && (
          <div className="upd-panel">
            <div className="upd-bar">
              <button
                className="upd-bar-main"
                onClick={() => setUpdOpen((v) => !v)}
                title={t("更新")}
              >
                <Icon name="refresh" size={13} />
                <span>
                  {t("发现新版本")} <b>v{updateFound.version}</b>
                </span>
              </button>
              <button className="upd-bar-x" onClick={dismissUpdate} title={t("关闭")}>
                <Icon name="close" size={12} />
              </button>
            </div>
            {updOpen && (
              <div className="upd-panel-body">
                <UpdateActions
                  assetUrl={updateFound.assetUrl}
                  htmlUrl={updateFound.htmlUrl}
                  mirror={mirrorPrefix({ updateMirror: settings?.updateMirror ?? "" })}
                  compact
                />
              </div>
            )}
          </div>
        )}
        <header className="bar">
          <div className="bar-icon" data-active={mode === "search" && query.length > 0}>
            <Icon name="search" size={20} />
          </div>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => { setQuery(e.target.value); if (mode !== "search") setMode("search"); }}
            onKeyDown={onKeyDown}
            placeholder={mode === "search" ? t("搜索文件、应用、命令、计算…") : ""}
            spellCheck={false}
            autoComplete="off"
          />
          {query && mode === "search" && (
            <button className="icon-btn" onClick={() => { setQuery(""); inputRef.current?.focus(); }} title={t("清除")}>
              <Icon name="close" size={16} />
            </button>
          )}
          {quick && (
            <span className="qk-tag">
              <Icon name="corner-down-left" size={13} />{t("快速切换")}</span>
          )}
          <div className="bar-right">
            <button className="icon-btn" data-active={menuOpen} onClick={() => setMenuOpen((v) => !v)} title={t("更多")}>
              <Icon name="layers" size={18} />
            </button>
          </div>
        </header>

        <div
          className="hairline"
          data-show={
            (mode === "search" &&
              (hits.length > 0 || !!plan || (quick && query === "" && qsDirs.length > 0))) ||
            mode !== "search"
          }
        />

        <div className="body">
          {mode === "search" && (
            <div className="list-wrap">
              <div className="list" ref={listRef} data-accent={selected >= 0 && hits[selected]?.kind === "command" ? "true" : "false"}>
                <div
                  className="pill"
                  data-hidden={hits.length === 0}
                  style={{ transform: `translateY(${selected * rowH}px)`, height: rowH, opacity: hits.length ? 1 : 0 }}
                />
                {hits.length === 0 && !(quick && (query === "" || plan)) && (
                  <div className="empty">
                    {status.state === "scanning" ? (
                      <>
                        <div className="big">{t("正在建立索引…")}</div>
                        <div className="small">{t("已扫描 {n} 个文件。你可以先搜索应用与命令，文件结果会随索引完善逐步出现。", { n: shownCount.toLocaleString() })}</div>
                      </>
                    ) : query ? (
                      <>
                        <div className="big">{t("没有匹配的结果")}</div>
                        <div className="small">{t("试试更简短的关键词，或使用筛选语法：")}<kbd>ext:pdf</kbd> <kbd>dm:today</kbd> <kbd>in:Documents</kbd></div>
                      </>
                    ) : (
                      <>
                        <div className="big">{t("开始搜索")}</div>
                        <div className="small">{t("双击")}<kbd>Ctrl</kbd>{t("或按")}<kbd>Alt</kbd>+<kbd>Space</kbd>{t("随时唤起。输入")}<kbd>g</kbd>{t("可搜索网页。")}</div>
                      </>
                    )}
                  </div>
                )}
                {/* Typed path: show where it lands, or the folders completing it. */}
                {plan && (
                  <>
                    <div className="qs-loc">
                      <span>{qsFolder ? shortPath(qsFolder) : t("路径")}</span>
                    </div>
                    {plan.candidates.length > 0 ? (
                      plan.candidates.map((c, i) => (
                        <button
                          key={c}
                          className="qs-dir"
                          data-selected={i === selected}
                          onMouseEnter={() => setSelected(i)}
                          onClick={() => void qsJump(c, true)}
                        >
                          <span className="qs-ic"><Icon name="folder" size={18} /></span>
                          <span className="qs-txt">
                            <span className="n">{baseName(c)}</span>
                            <span className="p">{c}</span>
                          </span>
                          <span className="qs-key">{t("↵ 跳转")}</span>
                        </button>
                      ))
                    ) : (
                      <button
                        className="qs-dir"
                        data-selected="true"
                        data-tentative={plan.tentative ? "true" : undefined}
                        onClick={() => void qsJump(plan.raw, true)}
                      >
                        <span className="qs-ic"><Icon name="folder" size={18} /></span>
                        <span className="qs-txt">
                          <span className="n">{baseName(plan.raw)}</span>
                          <span className="p">{plan.raw}</span>
                        </span>
                        <span className="qs-key">{t("↵ 跳转")}</span>
                      </button>
                    )}
                  </>
                )}
                {!plan && quick && query === "" && qsDirs.length > 0 && (
                  <>
                    <div className="qs-head">{t("最近访问的目录")}</div>
                    {qsDirs.map((d, i) => (
                      <button
                        key={d}
                        className="qs-dir"
                        data-selected={i === dirSel}
                        onMouseEnter={() => setDirSel(i)}
                        onClick={() => void qsJump(d, true)}
                      >
                        <span className="qs-ic"><Icon name="folder" size={18} /></span>
                        <span className="qs-txt">
                          <span className="n">{baseName(d)}</span>
                          <span className="p">{d}</span>
                        </span>
                        <span className="qs-key">{t("↵ 跳转")}</span>
                      </button>
                    ))}
                  </>
                )}
                {!plan && !(quick && query === "") && hits.map((h, i) => (
                  <Row
                    key={h.id}
                    hit={h}
                    idx={i}
                    selected={i === selected}
                    icon={h.path ? icons[h.path] : undefined}
                    onHover={() => setSelected(i)}
                    onOpen={() => { setSelected(i); openSelected(false, false); }}
                    onMenu={() => {
                      setSelected(i);
                      setAction({ hit: h, items: buildActions(h), idx: 0 });
                    }}
                  />
                ))}
              </div>
            </div>
          )}

          {mode === "clipboard" && (
            <div className="panel">
              <div className="panel-head">
                <Icon name="clipboard" size={16} />
                <span className="t">{t("剪贴板历史")}</span>
                <span className="count">{t("{n} 项", { n: clipboard.length })}</span>
                <span className="spacer" />
                <button className="btn" data-size="sm" onClick={() => { api.clipboardClear(); setClipboard([]); }}>{t("清空")}</button>
              </div>
              <div className="clip-list">
                {clipboard.length === 0 && <div className="empty"><div className="big">{t("暂无记录")}</div><div className="small">{t("复制文字后会出现在这里。")}</div></div>}
                {clipboard.map((c, i) => (
                  <button key={c.id} className="clip-item" data-selected={i === clipSel} onClick={() => { setClipSel(i); api.clipboardCopy(c.id); showToast(t("已粘贴")); api.hideLauncher(); }}>
                    {c.kind === "image" && c.image ? <img src={c.image} alt="" /> : <Icon name="file-text" size={18} />}
                    <div className="body">
                      <div className="txt">{c.kind === "image" ? t("图片") : c.text}</div>
                      <div className="time">{formatRelative(c.at)}{c.pinned ? t("· 已固定") : ""}</div>
                    </div>
                    {c.pinned && <Icon className="pin" name="pin" size={14} />}
                  </button>
                ))}
              </div>
            </div>
          )}

          {mode === "windows" && (
            <div className="panel">
              <div className="panel-head">
                <Icon name="window" size={16} />
                <span className="t">{t("窗口切换")}</span>
                <span className="count">{t("{n} 个", { n: windows.length })}</span>
              </div>
              <div className="clip-list">
                {windows.map((w, i) => (
                  <button key={w.hwnd} className="win-item" data-selected={i === winSel} onClick={() => { setWinSel(i); api.activateWindow(w.hwnd); api.hideLauncher(); }}>
                    <div className="ico"><Icon name="window" size={20} /></div>
                    <div className="meta2">
                      <div className="n">{w.title}</div>
                      <div className="p">{w.process}</div>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}

          {mode === "notes" && (
            <div className="panel">
              <div className="panel-head">
                <Icon name="note" size={16} />
                <span className="t">{t("快捷笔记")}</span>
                <span className="spacer" />
                <span className="count">{t("自动保存")}</span>
              </div>
              <div className="notes">
                <textarea
                  value={notes}
                  placeholder={t("随手记点什么，内容会自动保存在本地…")}
                  onChange={(e) => {
                    setNotes(e.target.value);
                    if (settings) {
                      const s = { ...settings, quickNotes: e.target.value };
                      setSettings(s);
                      api.saveSettings(s);
                    }
                  }}
                />
              </div>
            </div>
          )}

          {showPreview && (
            <PreviewPanel hit={selHit} info={preview} onCopyPath={(p) => { api.clipboardPushText(p); showToast(t("已复制路径")); }} onOpen={(p) => api.openPath(p)} onReveal={(p) => api.revealPath(p)} />
          )}
        </div>

        <footer className="foot">
          {mode === "search" && quick ? (
            <>
              <span className="hint"><kbd>↑</kbd><kbd>↓</kbd>{t("选择")}</span>
              <span className="hint"><kbd>↵</kbd>{t("跳转")}</span>
              <span className="hint optional"><kbd>←</kbd><kbd>→</kbd>{t("上级 / 进入")}</span>
              <span className="hint optional"><kbd>Esc</kbd>{t("取消")}</span>
              <span className="grow" />
              <span className="stat">
                <span className="dot" data-state={status.state} />
                {status.state === "scanning" ? t("索引中 {n}", { n: shownCount.toLocaleString() }) : t("{n} 个文件", { n: shownCount.toLocaleString() })}
              </span>
            </>
          ) : mode === "search" ? (
            <>
              <span className="hint"><kbd>↑</kbd><kbd>↓</kbd>{t("选择")}</span>
              <span className="hint optional"><kbd>Tab</kbd>{t("操作")}</span>
              <span className="hint optional"><kbd>↵</kbd>{t("打开 ·")}<kbd>Ctrl↵</kbd>{t("定位")}</span>
              <span className="hint optional"><kbd>Esc</kbd>{t("关闭")}</span>
              <span className="grow" />
              {hint && <span className="hint optional">{hint}</span>}
              <span className="stat">
                <span className="dot" data-state={status.state} />
                {status.state === "scanning" ? t("索引中 {n}", { n: shownCount.toLocaleString() }) : t("{n} 个文件", { n: shownCount.toLocaleString() })}
              </span>
              <button
                className="icon-btn"
                data-active={!!settings?.showPreview}
                onClick={togglePreview}
                title={settings?.showPreview ? t("隐藏预览") : t("显示预览")}
              >
                <Icon name="eye" size={15} />
              </button>
              <button className="icon-btn" onClick={() => api.openSettingsWindow()} title={t("设置")}>
                <Icon name="gear" size={15} />
              </button>
            </>
          ) : (
            <>
              <span className="hint"><kbd>↑</kbd><kbd>↓</kbd>{t("选择")}</span>
              <span className="hint"><kbd>↵</kbd>{t("确认 ·")}<kbd>Esc</kbd>{t("返回")}</span>
              <span className="grow" />
              <button className="btn" data-size="sm" onClick={() => setMode("search")}>{t("返回搜索")}</button>
            </>
          )}
        </footer>
      </div>

      {menuOpen && (
        <div className="scrim" data-soft="true" onMouseDown={(e) => { e.preventDefault(); setMenuOpen(false); }}>
          <div className="actions menu-pop" onMouseDown={(e) => e.stopPropagation()}>
            <div className="head">
              <div className="ico"><Icon name="layers" size={18} /></div>
              <div className="t">
                <div className="n">{t("更多")}</div>
              </div>
            </div>
            <div className="items">
              {dropdown.map((d) => (
                <button key={d.id} className="item" onClick={() => { setMenuOpen(false); d.act(); }}>
                  <Icon name={d.icon} size={16} />
                  <span>{d.label}</span>
                  {d.key && <span className="k"><kbd>{d.key}</kbd></span>}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {action && (
        <div className="scrim" onMouseDown={(e) => { e.preventDefault(); setAction(null); }}>
          <div className="actions" onMouseDown={(e) => e.stopPropagation()}>
            <div className="head">
              <div className="ico">{action.hit.path && icons[action.hit.path] ? <img src={icons[action.hit.path]} alt="" /> : <Icon name={glyphFor(action.hit)} size={18} />}</div>
              <div className="t">
                <div className="n">{action.hit.title}</div>
                <div className="p">{action.hit.path || action.hit.subtitle}</div>
              </div>
            </div>
            <div className="items">
              {action.items.map((it, i) => (
                <button key={it.id} className="item" data-selected={i === action.idx} data-danger={it.danger} onMouseEnter={() => setAction((a) => a && { ...a, idx: i })} onClick={() => runAction(it)}>
                  <Icon name={it.icon} size={16} />
                  <span>{it.label}</span>
                  {it.key && <span className="k"><kbd>{it.key}</kbd></span>}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {toast && <div className="toast">{toast}</div>}

      {hashModal && (
        <>
          <div className="scrim" onMouseDown={() => setHashModal(null)} />
          <div className="hash-modal" onMouseDown={(e) => e.stopPropagation()}>
            <div className="hm-head">
              <Icon name="hash" size={15} />
              <span className="t">{t("校验和")}</span>
              <span className="f">{hashModal.path.split("\\").pop()}</span>
              <span className="spacer" />
              <button className="icon-btn" onClick={() => setHashModal(null)}>
                <Icon name="close" size={14} />
              </button>
            </div>
            {hashModal.loading ? (
              <div className="hm-body"><span className="spin-inline">{t("计算中…")}</span></div>
            ) : (
              <div className="hm-body">
                {([
                  ["MD5", hashModal.md5],
                  ["SHA-1", hashModal.sha1],
                  ["SHA-256", hashModal.sha256],
                ] as const).map(([label, val]) => (
                  <div className="hm-row" key={label}>
                    <span className="hm-label">{label}</span>
                    <span className="hm-value" title={val}>{val}</span>
                    <button
                      className="btn"
                      data-size="sm"
                      onClick={() => { api.clipboardPushText(val); showToast(t("已复制 {text}", { text: label })); }}
                    >{t("复制")}</button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

function Row(props: {
  hit: SearchHit;
  idx: number;
  selected: boolean;
  icon?: string;
  onHover: () => void;
  onOpen: () => void;
  onMenu: () => void;
}) {
  const { hit, idx, selected, icon, onHover, onOpen, onMenu } = props;
  const sub = hit.kind === "file" || hit.kind === "folder" || hit.kind === "favorite" || hit.kind === "app"
    ? shortPath(hit.subtitle || hit.path || "")
    : hit.subtitle;
  return (
    <div
      data-idx={idx}
      data-selected={selected}
      data-anim={idx < 40}
      className="row"
      style={{ animationDelay: `${Math.min(idx * 7, 160)}ms` }}
      onMouseMove={onHover}
      onClick={onOpen}
      onContextMenu={(e) => {
        e.preventDefault();
        onHover();
        onMenu();
      }}
    >
      <div className="ico" data-glyph={!icon}>
        {icon ? <img src={icon} alt="" /> : <Icon name={glyphFor(hit)} size={17} />}
      </div>
      <div className="texts">
        <div className="title">{hit.title}</div>
        <div className="sub">{sub}</div>
      </div>
      <div className="tail">
        {hit.badge && <span className="badge">{hit.badge}</span>}
        <span className="enter-hint"><Icon name="corner-down-left" size={15} /></span>
      </div>
    </div>
  );
}

function PreviewPanel(props: {
  hit?: SearchHit;
  info: FileInfo | null;
  onCopyPath: (p: string) => void;
  onOpen: (p: string) => void;
  onReveal: (p: string) => void;
}) {
  const { hit, info } = props;
  // Never unmount the panel: an empty result list must not make the sidebar
  // disappear (that looked like "the sidebar only shows sometimes").
  if (!hit) {
    return (
      <aside className="preview preview-empty">
        <div className="ph">
          <Icon name="layers" size={28} />
          <span>{t("选择一项查看详情")}</span>
        </div>
      </aside>
    );
  }

  if (hit.kind === "calc" && hit.meta) {
    return (
      <aside className="preview calc-preview">
        <div className="val">{hit.title}</div>
        <div className="exp">{hit.subtitle}</div>
      </aside>
    );
  }
  if (hit.kind === "web" && hit.meta) {
    return (
      <aside className="preview">
        <div className="hero">
          <div className="ico" data-glyph><Icon name="globe" size={56} /></div>
          <div className="name">{hit.title}</div>
          <div className="type">{t("网页搜索")}</div>
        </div>
        <div className="meta">
          <div className="item"><span className="k">{t("地址")}</span><span className="v mono">{hit.subtitle}</span></div>
        </div>
        <div className="quick">
          <button className="btn" data-variant="primary" onClick={() => api.openUrl(hit.meta?.url as string)}>{t("打开")}</button>
        </div>
      </aside>
    );
  }
  if (hit.kind === "command" && hit.meta) {
    return (
      <aside className="preview">
        <div className="hero">
          <div className="ico" data-glyph><Icon name={(hit.meta.glyph as string) || "gear"} size={56} /></div>
          <div className="name">{hit.title}</div>
          <div className="type">{hit.badge || t("命令")}</div>
        </div>
        <div className="meta">
          <div className="item"><span className="k">{t("说明")}</span><span className="v">{hit.subtitle}</span></div>
        </div>
        <div className="quick">
          <button className="btn" data-variant="primary" onClick={() => {}}>{t("运行")}</button>
        </div>
      </aside>
    );
  }
  if (!info) {
    return (
      <aside className="preview">
        <div className="hero">
          <div className="ico" data-glyph><Icon name={glyphFor(hit)} size={56} /></div>
          <div className="name">{hit.title}</div>
        </div>
      </aside>
    );
  }
  return (
    <aside className="preview">
      {info.thumb ? (
        <div className="thumb"><img src={info.thumb} alt="" /></div>
      ) : (
        <div className="hero">
          <div className="ico" data-glyph><Icon name={info.isDir ? "folder" : "file"} size={56} /></div>
          <div className="name">{info.name}</div>
          <div className="type">{info.typeName}</div>
        </div>
      )}
      <div className="meta">
        {!info.thumb && <div className="item"><span className="k">{t("类型")}</span><span className="v">{info.typeName}</span></div>}
        {!info.isDir && <div className="item"><span className="k">{t("大小")}</span><span className="v">{formatBytes(info.size)}</span></div>}
        {info.isDir && info.itemCount != null && <div className="item"><span className="k">{t("项目")}</span><span className="v">{info.itemCount} 个</span></div>}
        <div className="item"><span className="k">{t("修改")}</span><span className="v">{formatDate(info.modified)}</span></div>
        <div className="item"><span className="k">{t("路径")}</span><span className="v mono">{info.path}</span></div>
      </div>
      <div className="quick">
        <button className="btn" data-variant="primary" onClick={() => props.onOpen(info.path)}>{t("打开")}</button>
        <button className="btn" onClick={() => props.onReveal(info.path)}>{t("定位")}</button>
        <button className="btn" data-variant="ghost" onClick={() => props.onCopyPath(info.path)}><Icon name="copy" size={15} /></button>
      </div>
    </aside>
  );
}
