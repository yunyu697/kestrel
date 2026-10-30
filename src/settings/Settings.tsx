import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm as confirmDialog, open as dialogOpen } from "@tauri-apps/plugin-dialog";
import { Icon } from "../shared/icons";
import { api } from "../shared/api";
import { LANGS, setLang, systemLang, t, useLang } from "../shared/i18n";
import { resolveTheme } from "../shared/format";
import { applyColors, COLOR_SLOTS, slotColor, THEME_SWATCHES } from "../shared/theme";
import { SCHEMES, schemeById, schemePalette } from "../shared/palettes";
import { pick, RELEASES, type ReleaseKind } from "../shared/releases";
import type {
  AppEntry,
  BuildInfo,
  CustomCommand,
  Favorite,
  IndexStatus,
  Settings,
  WebEngine,
} from "../shared/types";

const ACCENTS = [
  "#4C7DFF",
  "#2ECC71",
  "#9B59F6",
  "#FF6B6B",
  "#FF9F0A",
  "#00BCD4",
  "#E91E8C",
  "#607D8B",
];

/** Ready-made combinations for the main shortcut. */
const HOTKEY_PRESETS = [
  "Alt+Space",
  "Ctrl+Space",
  "Alt+Enter",
  "Ctrl+Alt+Space",
  "Ctrl+K",
];

const NAV: { group: string; items: { id: string; label: string; icon: string }[] }[] = [
  {
    group: "偏好",
    items: [
      { id: "general", label: "常规", icon: "sliders" },
      { id: "appearance", label: "外观", icon: "palette" },
      { id: "hotkeys", label: "快捷键", icon: "keyboard" },
    ],
  },
  {
    group: "搜索",
    items: [
      { id: "search", label: "搜索", icon: "search" },
      { id: "index", label: "索引", icon: "database" },
      { id: "favorites", label: "收藏夹", icon: "star" },
      { id: "web", label: "搜索引擎", icon: "globe" },
      { id: "commands", label: "自定义命令", icon: "terminal" },
    ],
  },
  {
    group: "功能",
    items: [
      { id: "clipboard", label: "剪贴板", icon: "clipboard" },
      { id: "features", label: "实用工具", icon: "cpu" },
      { id: "apps", label: "应用", icon: "app" },
    ],
  },
  {
    group: "关于",
    items: [{ id: "about", label: "关于", icon: "info" }],
  },
];

function applyTheme(s: Settings) {
  // Language first: every label is translated while rendering.
  setLang(s.language);
  const root = document.documentElement;
  const theme = resolveTheme(s.theme);
  root.setAttribute("data-theme", theme);
  // Per-element colour overrides (empty value = theme default).
  applyColors(s, theme);
  root.style.setProperty("--font-scale", String(s.fontScale));
  root.setAttribute("data-anim", s.animations ? "on" : "off");
}

export default function Settings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [active, setActive] = useState("general");
  const [status, setStatus] = useState<IndexStatus | null>(null);
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [build, setBuild] = useState<BuildInfo | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const saveTimer = useRef<number | null>(null);
  const loaded = useRef(false);
  // Re-render every label when the language changes.
  const lang = useLang();
  void lang;

  const load = useCallback(async () => {
    const s = await api.getSettings();
    setSettings(s);
    applyTheme(s);
    loaded.current = true;
  }, []);

  useEffect(() => {
    load();
    api.indexStatus().then(setStatus).catch(() => {});
    api.buildInfo().then(setBuild).catch(() => {});
  }, [load]);

  useEffect(() => {
    const un = listenShown();
    return () => {
      un.then((f) => f());
    };
  }, []);

  async function listenShown() {
    const { listen } = await import("@tauri-apps/api/event");
    return listen("settings://shown", () => {
      load();
      api.indexStatus().then(setStatus).catch(() => {});
    });
  }

  // debounced save
  const save = useCallback((next: Settings) => {
    if (!loaded.current) return;
    applyTheme(next);
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      api.saveSettings(next).then(
        () => flashToast(t("已保存")),
        () => flashToast(t("保存失败"))
      );
    }, 350);
  }, []);

  const flashToast = useCallback((msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast((cur) => (cur === msg ? null : cur)), 1600);
  }, []);

  const update = useCallback(
    <K extends keyof Settings>(key: K, value: Settings[K]) => {
      if (!settings) return;
      const next = { ...settings, [key]: value };
      setSettings(next);
      save(next);
    },
    [settings, save]
  );

  // Bulk variant: several fields must change in one save (e.g. resetting every
  // colour override). Sequential `update` calls would all start from the same
  // stale snapshot and only the last one would survive.
  const updateMany = useCallback(
    (patch: Partial<Settings>) => {
      if (!settings) return;
      const next = { ...settings, ...patch };
      setSettings(next);
      save(next);
    },
    [settings, save]
  );

  if (!settings) {
    return (
      <div className="settings-root">
        <div style={{ margin: "auto", color: "var(--text-3)" }}>{t("加载中…")}</div>
      </div>
    );
  }

  return (
    <div className="settings-root">
      <div className="sb-title" data-tauri-drag-region>
        <div className="brand">
          <div className="logo">
            <Icon name="search" size={14} strokeWidth={2.2} />
          </div>
          <span>{t("Kestrel 设置")}{(lang === "zh" || lang === "zh-TW") && <small>{t("隼")}</small>}
          </span>
        </div>
        <div className="spacer" />
        <button className="win-ctl" title={t("关闭")} data-close onClick={() => getCurrentWindow().hide()}>
          <Icon name="close" size={16} />
        </button>
      </div>

      <div className="sb-body">
        <nav className="sb-side">
          {NAV.map((g) => (
            <div key={g.group}>
              <div className="nav-group-label">{t(g.group)}</div>
              {g.items.map((it) => (
                <div
                  key={it.id}
                  className="sb-nav-item"
                  data-active={active === it.id}
                  onClick={() => setActive(it.id)}
                >
                  <Icon name={it.icon} size={17} />
                  <span className="nav-label">{t(it.label)}</span>
                </div>
              ))}
            </div>
          ))}
        </nav>

        <main className="sb-content">
          <div className="sb-pane">
            {active === "general" && (
              <GeneralPane s={settings} update={update} flash={flashToast} />
            )}
            {active === "appearance" && (
              <AppearancePane s={settings} update={update} updateMany={updateMany} />
            )}
            {active === "hotkeys" && <HotkeysPane s={settings} update={update} flash={flashToast} />}
            {active === "search" && <SearchPane s={settings} update={update} />}
            {active === "index" && (
              <IndexPane s={settings} update={update} status={status} setStatus={setStatus} />
            )}
            {active === "favorites" && <FavoritesPane s={settings} update={update} />}
            {active === "web" && <WebPane s={settings} update={update} />}
            {active === "commands" && <CommandsPane s={settings} update={update} />}
            {active === "clipboard" && <ClipboardPane s={settings} update={update} />}
            {active === "features" && <FeaturesPane s={settings} update={update} />}
            {active === "apps" && <AppsPane apps={apps} setApps={setApps} />}
            {active === "about" && <AboutPane build={build} s={settings} />}
          </div>
        </main>
      </div>

      <div className="sb-footer">
        <div className="save-state">
          <span className="status-dot" data-state="ready" />{t("设置实时保存")}</div>
        <div className="spacer" />
        <button
          className="btn"
          onClick={async () => {
            const fresh = await api.resetSettings();
            setSettings(fresh);
            applyTheme(fresh);
            flashToast(t("已恢复默认"));
          }}
        >
          <Icon name="refresh" size={15} />{t("恢复默认")}</button>
      </div>

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

/* ============================ shared bits ============================ */

type Updater = <K extends keyof Settings>(key: K, value: Settings[K]) => void;

function Toggle({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button className="switch" data-on={on} onClick={() => onChange(!on)} aria-pressed={on}>
      <span />
    </button>
  );
}

function Row({
  label,
  desc,
  children,
}: {
  label: string;
  desc?: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="row">
      <div className="row-main">
        <div className="row-label">{label}</div>
        {desc && <div className="row-desc">{desc}</div>}
      </div>
      <div className="row-ctl">{children}</div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="section">
      <div className="section-title">
        {title}
        <span className="line" />
      </div>
      {children}
    </div>
  );
}

function Slider({
  value,
  min,
  max,
  step,
  fmt,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  step: number;
  fmt?: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <div className="slider">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
      />
      <span className="slider-val">{fmt ? fmt(value) : value}</span>
    </div>
  );
}

function Seg<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { v: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="seg sz-lg">
      {options.map((o) => (
        <button key={o.v} data-active={value === o.v} onClick={() => onChange(o.v)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/* ============================ panes ============================ */

function GeneralPane({
  s,
  update,
  flash,
}: {
  s: Settings;
  update: Updater;
  flash: (msg: string) => void;
}) {
  const [svc, setSvc] = useState<boolean | null>(null);
  const [elevated, setElevated] = useState<boolean | null>(null);
  useEffect(() => {
    api.serviceStatus().then(setSvc).catch(() => setSvc(false));
    api.isElevated().then(setElevated).catch(() => setElevated(false));
  }, []);

  const onToggleAdmin = async (v: boolean) => {
    // Only a real change of privilege needs the restart round trip.
    if (elevated === null || v === elevated) {
      update("runAsAdmin", v);
      return;
    }
    const ok = await confirmDialog(
      v
        ? t("将以管理员身份重启 Kestrel，Windows 会弹出 UAC 授权窗口。是否继续？")
        : t("将取消管理员权限并重启 Kestrel。是否继续？"),
      { title: t("以管理员身份运行"), kind: "warning" },
    ).catch(() => false);
    if (!ok) return;
    const status = await api.setRunAsAdmin(v).catch(() => "failed" as const);
    if (status === "failed") {
      // Nothing was stored, so the toggle must not pretend it changed.
      flash(
        v
          ? t("已取消提权，设置未更改")
          : t("无法取消管理员权限，请手动重启 Kestrel"),
      );
      return;
    }
    update("runAsAdmin", v);
  };

  return (
    <>
      <h1>{t("常规")}</h1>
      <p className="pane-sub">{t("启动行为、交互方式与语言。")}</p>

      <Section title={t("启动")}>
        <div className="card">
          <Row label={t("开机自启动")} desc={t("登录 Windows 后自动运行 Kestrel")}>
            <Toggle on={s.launchAtStartup} onChange={(v) => update("launchAtStartup", v)} />
          </Row>
          <Row label={t("启动后最小化")} desc={t("启动时不弹出启动器")}>
            <Toggle on={s.startMinimized} onChange={(v) => update("startMinimized", v)} />
          </Row>
          <Row
            label={t("安装为服务")}
            desc={
              svc === null
                ? t("读取守护状态中…")
                : svc
                  ? t("已启用：Kestrel 被意外关闭时会自动重启；关闭开关即卸载")
                  : t("注册守护任务，防止程序被意外关闭（在任务计划中注册，无需管理员）")
            }
          >
            <Toggle
              on={svc === true}
              onChange={async (v) => {
                const ok = v ? await api.serviceInstall() : await api.serviceUninstall();
                if (ok) setSvc(v);
              }}
            />
          </Row>
        </div>
      </Section>

      <Section title={t("交互")}>
        <div className="card">
          <Row label={t("打开后自动隐藏")} desc={t("选择结果后立即隐藏启动器")}>
            <Toggle on={s.closeAfterOpen} onChange={(v) => update("closeAfterOpen", v)} />
          </Row>
          <Row label={t("隐藏时清空查询")} desc={t("每次打开都从空白查询开始")}>
            <Toggle on={s.clearQueryOnHide} onChange={(v) => update("clearQueryOnHide", v)} />
          </Row>
          <Row label={t("记住上次查询")} desc={t("打开时恢复上一次的搜索词")}>
            <Toggle on={s.rememberLastQuery} onChange={(v) => update("rememberLastQuery", v)} />
          </Row>
          <Row label={t("单击打开")} desc={t("在结果列表单击即可打开（默认双击）")}>
            <Toggle on={s.singleClickOpen} onChange={(v) => update("singleClickOpen", v)} />
          </Row>
        </div>
      </Section>

      <Section title={t("权限")}>
        <div className="card">
          <Row
            label={t("以管理员身份运行")}
            desc={
              elevated === null
                ? t("读取当前权限中…")
                : elevated
                  ? t("当前已以管理员权限运行，安装服务、写入系统目录等操作会直接执行")
                  : t("开启后 Kestrel 将以管理员权限重启，需要提权的操作不再被拒绝；每次启动 Windows 都会请求 UAC 授权")
            }
          >
            {/* An already elevated process (e.g. started from an elevated
                shell) shows ON even when the setting is off: that is the
                state the user is actually in, and switching it off performs
                the de-elevating restart. */}
            <Toggle on={s.runAsAdmin || elevated === true} onChange={onToggleAdmin} />
          </Row>
        </div>
      </Section>

      <Section title={t("语言")}>
        <div className="card">
          <Row
            label={t("界面语言")}
            desc={
              s.language === "system"
                ? t("跟随 Windows 显示语言（当前：{lang}）", {
                    lang: LANGS.find((l) => l.id === systemLang())?.label ?? "",
                  })
                : t("切换后立即生效")
            }
          >
            <div className="seg seg-lang">
              <button
                data-active={s.language === "system"}
                onClick={() => update("language", "system")}
              >
                {t("跟随系统")}
              </button>
              {LANGS.map((l) => (
                <button
                  key={l.id}
                  data-active={s.language === l.id}
                  onClick={() => update("language", l.id)}
                >
                  {l.label}
                </button>
              ))}
            </div>
          </Row>
        </div>
      </Section>
    </>
  );
}

function ColorRow({
  s,
  update,
  slot,
  theme,
}: {
  s: Settings;
  update: Updater;
  slot: (typeof COLOR_SLOTS)[number];
  theme: "light" | "dark";
}) {
  const raw = String(s[slot.key] ?? "").trim();
  const shown = slotColor(s, slot, theme);
  // Every colour slot is a plain string on `Settings`; the cast keeps the
  // generic `update` signature happy without widening it to `any`.
  const setColor = (v: string) => update(slot.key as "colorBg", v);
  return (
    <Row label={t(slot.label)} desc={t(slot.desc)}>
      <div className="color-ctl">
        <label className="color-chip" style={{ background: shown }} title={shown}>
          <input
            type="color"
            value={shown}
            onChange={(e) => setColor(e.target.value.toUpperCase())}
          />
        </label>
        <span className="color-hex" data-default={!raw}>
          {raw ? raw.toUpperCase() : t("默认")}
        </span>
        <button
          className="btn"
          data-size="sm"
          data-variant="ghost"
          disabled={!raw}
          onClick={() => setColor("")}
          title={t("恢复默认")}
        >
          <Icon name="refresh" size={13} />
        </button>
      </div>
    </Row>
  );
}

function SchemeCard({
  label,
  pal,
  accent,
  active,
  onPick,
}: {
  label: string;
  pal: Record<string, string>;
  accent: string;
  active: boolean;
  onPick: () => void;
}) {
  return (
    <button className="scheme" data-active={active} onClick={onPick} title={label}>
      <span
        className="scheme-preview"
        style={{ background: pal["--bg"], borderColor: pal["--border"] }}
      >
        <span className="scheme-strip" style={{ background: pal["--surface-2"] }} />
        <span className="scheme-body">
          <span className="scheme-line" style={{ background: pal["--text"] }} />
          <span
            className="scheme-line"
            data-dim="true"
            style={{ background: pal["--text-3"] }}
          />
        </span>
        <span className="scheme-dot" style={{ background: accent }} />
        {active && (
          <span className="scheme-tick">
            <Icon name="check" size={10} />
          </span>
        )}
      </span>
      <span className="scheme-name">{label}</span>
    </button>
  );
}

function AppearancePane({
  s,
  update,
  updateMany,
}: {
  s: Settings;
  update: Updater;
  updateMany: (patch: Partial<Settings>) => void;
}) {
  const theme = resolveTheme(s.theme);
  const overrides = COLOR_SLOTS.filter((slot) => String(s[slot.key] ?? "").trim());
  const scheme = schemeById(s.colorScheme);
  const custom = s.colorScheme === "custom";
  // Palette shown on a card: the scheme's ramp, the per-slot values for
  // 自定义, or the theme's own tokens when nothing is chosen.
  const paletteOf = (id: string): Record<string, string> => {
    const def = schemeById(id);
    if (def) return schemePalette(def, theme);
    if (id === "custom") {
      const out: Record<string, string> = {};
      for (const slot of COLOR_SLOTS) out[slot.cssVar] = slotColor(s, slot, theme);
      return out;
    }
    return THEME_SWATCHES[theme];
  };
  const resetAll = () => {
    const patch: Record<string, string> = { colorScheme: "" };
    for (const slot of COLOR_SLOTS) patch[String(slot.key)] = "";
    updateMany(patch as Partial<Settings>);
  };
  const hint = scheme
    ? t("当前方案：{name}，浅色与深色各一套", { name: t(scheme.id) })
    : custom
      ? overrides.length
        ? t("已自定义 {n} 项颜色，其余沿用主题默认", { n: overrides.length })
        : t("全部沿用当前主题的默认配色")
      : t("全部沿用当前主题的默认配色");
  return (
    <>
      <h1>{t("外观")}</h1>
      <p className="pane-sub">{t("主题、强调色与窗口观感。")}</p>

      <Section title={t("主题")}>
        <div className="card">
          <Row label={t("外观模式")} desc={t("跟随系统会随 Windows 主题切换")}>
            <Seg
              value={s.theme as "light" | "dark" | "system"}
              options={[
                { v: "light", label: t("浅色") },
                { v: "dark", label: t("深色") },
                { v: "system", label: t("系统") },
              ]}
              onChange={(v) => update("theme", v)}
            />
          </Row>
          <Row label={t("强调色")} desc={t("用于高亮、开关与主按钮")}>
            <div className="swatches">
              {ACCENTS.map((c) => (
                <button
                  key={c}
                  className="swatch"
                  data-active={s.accent.toUpperCase() === c.toUpperCase()}
                  style={{ background: c }}
                  onClick={() => update("accent", c)}
                  title={c}
                />
              ))}
              <label className="swatch-custom" title={t("自定义颜色")} style={{ background: s.accent }}>
                <input
                  type="color"
                  value={s.accent}
                  onChange={(e) => update("accent", e.target.value.toUpperCase())}
                />
              </label>
            </div>
          </Row>
        </div>
      </Section>

      <Section title={t("配色方案")}>
        <p className="scheme-note">{t("整套同色系配色，一键切换；浅色与深色各有一套。")}</p>
        <div className="schemes">
          <SchemeCard
            label={t("跟随主题")}
            pal={paletteOf("")}
            accent={s.accent}
            active={s.colorScheme === ""}
            onPick={() => update("colorScheme", "")}
          />
          {SCHEMES.map((def) => (
            <SchemeCard
              key={def.id}
              label={t(def.id)}
              pal={schemePalette(def, theme)}
              accent={def.accent}
              active={s.colorScheme === def.id}
              onPick={() => updateMany({ colorScheme: def.id, accent: def.accent })}
            />
          ))}
          <SchemeCard
            label={t("自定义")}
            pal={paletteOf("custom")}
            accent={s.accent}
            active={custom}
            onPick={() => update("colorScheme", "custom")}
          />
        </div>
        {custom && (
          <div className="card">
            {COLOR_SLOTS.map((slot) => (
              <ColorRow
                key={String(slot.key)}
                s={s}
                update={update}
                slot={slot}
                theme={theme}
              />
            ))}
          </div>
        )}
        <div className="color-actions">
          <span className="color-hint">{hint}</span>
          <button
            className="btn"
            data-size="sm"
            disabled={!scheme && !overrides.length}
            onClick={resetAll}
          >
            <Icon name="refresh" size={13} />
            {t("恢复默认配色")}
          </button>
        </div>
      </Section>

      <Section title={t("窗口")}>
        <div className="card">
          <Row label={t("窗口透明度")} desc={t("数值越低越通透（需关闭模糊效果最佳）")}>
            <Slider
              value={s.opacity}
              min={0.6}
              max={1}
              step={0.01}
              fmt={(v) => `${Math.round(v * 100)}%`}
              onChange={(v) => update("opacity", v)}
            />
          </Row>
          <Row label={t("背景模糊")} desc={t("毛玻璃效果，提升层次感")}>
            <Toggle on={s.blur} onChange={(v) => update("blur", v)} />
          </Row>
          <Row label={t("圆角半径")} desc={t("窗口与卡片的圆角大小")}>
            <Slider
              value={s.cornerRadius}
              min={0}
              max={22}
              step={1}
              fmt={(v) => `${Math.round(v)}px`}
              onChange={(v) => update("cornerRadius", v)}
            />
          </Row>
          <Row label={t("字体缩放")} desc={t("整体界面字号")}>
            <Slider
              value={s.fontScale}
              min={0.85}
              max={1.2}
              step={0.01}
              fmt={(v) => `${Math.round(v * 100)}%`}
              onChange={(v) => update("fontScale", v)}
            />
          </Row>
        </div>
      </Section>

      <Section title={t("布局与动效")}>
        <div className="card">
          <Row label={t("紧凑模式")} desc={t("缩小行高，一屏显示更多结果")}>
            <Toggle on={s.compactMode} onChange={(v) => update("compactMode", v)} />
          </Row>
          <Row label={t("结果预览")} desc={t("在侧面展示文件信息与缩略图")}>
            <Toggle on={s.showPreview} onChange={(v) => update("showPreview", v)} />
          </Row>
          {s.showPreview && (
            <Row label={t("预览位置")} desc={t("预览面板停靠方向")}>
              <Seg
                value={s.previewPosition as "right" | "bottom"}
                options={[
                  { v: "right", label: t("右侧") },
                  { v: "bottom", label: t("底部") },
                ]}
                onChange={(v) => update("previewPosition", v)}
              />
            </Row>
          )}
          <Row label={t("动画效果")} desc={t("过渡与微交互；性能较弱设备可关闭")}>
            <Toggle on={s.animations} onChange={(v) => update("animations", v)} />
          </Row>
        </div>
      </Section>
    </>
  );
}

function HotkeysPane({
  s,
  update,
  flash,
}: {
  s: Settings;
  update: Updater;
  flash: (m: string) => void;
}) {
  // Register the combination right away: the backend reports whether the
  // system accepted it, so a conflict is visible instead of silent.
  const apply = async (v: string) => {
    if (!v) return;
    const ok = await api.applyHotkey(v).catch(() => false);
    if (ok) {
      update("hotkeyToggle", v);
      flash(t("快捷键已更新"));
    } else {
      flash(t("快捷键注册失败，可能已被其他程序占用"));
    }
  };

  return (
    <>
      <h1>{t("快捷键")}</h1>
      <p className="pane-sub">{t("全局呼出与文件对话框快速切换。")}</p>

      <Section title={t("呼出启动器")}>
        <div className="card">
          <Row label={t("主快捷键")} desc={t("随时呼出 / 隐藏 Kestrel（需含修饰键）")}>
            <HotkeyCapture value={s.hotkeyToggle} onChange={(v) => apply(v)} />
          </Row>
          <Row label={t("常用预设")} desc={t("点击套用常用组合，也可在上方手动录制")}>
            <div className="hk-presets">
              {HOTKEY_PRESETS.map((p) => (
                <button
                  key={p}
                  className="hk-preset"
                  data-active={s.hotkeyToggle === p}
                  onClick={() => apply(p)}
                >
                  {p}
                </button>
              ))}
            </div>
          </Row>
        </div>
      </Section>

      <Section title={t("文件对话框快速切换")}>
        <div className="card">
          <Row label={t("启用快速切换")} desc={t("在打开/保存对话框中直接定位文件")}>
            <Toggle on={s.quickSwitchEnabled} onChange={(v) => update("quickSwitchEnabled", v)} />
          </Row>
          <Row label={t("自动吸附搜索框")} desc={t("文件对话框打开时自动在其下方显示搜索框，随对话框移动")}>
            <Toggle on={s.qsAutoShow} onChange={(v) => update("qsAutoShow", v)} />
          </Row>
          <Row label={t("自动跳转目录")} desc={t("从资源管理器回到对话框时，自动跳到最后浏览的目录")}>
            <Toggle on={s.qsAutoJump} onChange={(v) => update("qsAutoJump", v)} />
          </Row>
          <Row label={t("目录记录数量")} desc={t("吸附框中直接展示的历史记录条数，其余可滚动查看")}>
            <Slider
              value={s.qsVisibleRows}
              min={1}
              max={8}
              step={1}
              fmt={(v) => t("{v} 条", { v })}
              onChange={(v) => update("qsVisibleRows", Math.round(v))}
            />
          </Row>
          <Row label={t("快速切换快捷键")} desc={t("在文件对话框内按下以唤起选择")}>
            <HotkeyCapture
              value={s.hotkeyQuickSwitch}
              onChange={(v) => update("hotkeyQuickSwitch", v)}
            />
          </Row>
        </div>
      </Section>

      <Section title={t("双击 Ctrl")}>
        <div className="card">
          <Row label={t("双击 Ctrl 呼出")} desc={t("快速连按两次 Ctrl 键激活启动器")}>
            <Toggle on={s.doubleCtrl} onChange={(v) => update("doubleCtrl", v)} />
          </Row>
          <Row label={t("判定间隔")} desc={t("两次按键小于此间隔视为双击")}>
            <Slider
              value={s.doubleCtrlMs}
              min={120}
              max={900}
              step={10}
              fmt={(v) => `${Math.round(v)}ms`}
              onChange={(v) => update("doubleCtrlMs", v)}
            />
          </Row>
        </div>
      </Section>

      <Section title={t("替换 Windows 搜索")}>
        <div className="card">
          <Row
            label={t("Win+S / Win+Q 打开 Kestrel")}
            desc={t("拦截系统搜索快捷键并呼出 Kestrel。仅在本应用内生效，关闭开关或退出程序后立即恢复系统原行为，不修改任何系统组件")}
          >
            <Toggle on={s.replaceWinSearch} onChange={(v) => update("replaceWinSearch", v)} />
          </Row>
        </div>
      </Section>
    </>
  );
}

function HotkeyCapture({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [capturing, setCapturing] = useState(false);

  useEffect(() => {
    if (!capturing) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const ctrl = e.ctrlKey;
      const alt = e.altKey;
      const shift = e.shiftKey;
      const meta = e.metaKey;
      let key = "";
      if (e.code.startsWith("Key")) key = e.code.slice(3);
      else if (e.code.startsWith("Digit")) key = e.code.slice(5);
      else if (e.code === "Space") key = "Space";
      else if (e.code === "Enter") key = "Enter";
      else if (e.code.startsWith("Arrow")) key = e.code.slice(5);
      else if (e.code === "Backquote") key = "`";
      else if (e.code === "Minus") key = "-";
      else if (e.code === "Equal") key = "=";
      else key = e.key.length === 1 ? e.key.toUpperCase() : "";

      if (e.code === "Escape") {
        setCapturing(false);
        return;
      }
      if (!ctrl && !alt && !meta) return; // require a modifier
      const parts: string[] = [];
      if (ctrl) parts.push("Ctrl");
      if (alt) parts.push("Alt");
      if (meta) parts.push("Super");
      if (shift && key && key !== "Shift") parts.push("Shift");
      if (key) parts.push(key);
      onChange(parts.join("+"));
      setCapturing(false);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [capturing, onChange]);

  return (
    <button
      className="hk"
      data-capturing={capturing}
      onClick={() => setCapturing((c) => !c)}
    >
      {capturing ? (
        <>
          <Icon name="keyboard" size={15} />{t("按下快捷键…")}</>
      ) : (
        value || t("未设置")
      )}
    </button>
  );
}

function SearchPane({ s, update }: { s: Settings; update: Updater }) {
  return (
    <>
      <h1>{t("搜索")}</h1>
      <p className="pane-sub">{t("匹配策略、结果数量与历史记录。")}</p>

      <Section title={t("结果")}>
        <div className="card">
          <Row label={t("最大结果数")} desc={t("单次搜索返回的上限")}>
            <Slider
              value={s.maxResults}
              min={10}
              max={120}
              step={5}
              fmt={(v) => `${Math.round(v)}`}
              onChange={(v) => update("maxResults", v)}
            />
          </Row>
        </div>
      </Section>

      <Section title={t("匹配")}>
        <div className="card">
          <Row label={t("拼音搜索")} desc={t("支持拼音全拼与首字母（如 bbb→百度）")}>
            <Toggle on={s.pinyinSearch} onChange={(v) => update("pinyinSearch", v)} />
          </Row>
          <Row label={t("模糊匹配")} desc={t("子序列匹配，容忍缺失字符")}>
            <Toggle on={s.fuzzySearch} onChange={(v) => update("fuzzySearch", v)} />
          </Row>
          <Row label={t("路径匹配")} desc={t("命中文件路径中的关键词")}>
            <Toggle on={s.matchPath} onChange={(v) => update("matchPath", v)} />
          </Row>
        </div>
      </Section>

      <Section title={t("来源")}>
        <div className="card">
          <Row label={t("搜索应用")} desc={t("包含已安装程序")}>
            <Toggle on={s.searchApps} onChange={(v) => update("searchApps", v)} />
          </Row>
          <Row label={t("搜索命令")} desc={t("内置系统命令（控制面板、终端等）")}>
            <Toggle on={s.searchCommands} onChange={(v) => update("searchCommands", v)} />
          </Row>
          <Row label={t("文件内容")} desc={t("实验性：索引文件内容（较慢）")}>
            <Toggle on={s.searchFileContent} onChange={(v) => update("searchFileContent", v)} />
          </Row>
        </div>
      </Section>

      <Section title={t("历史")}>
        <div className="card">
          <Row label={t("记录搜索历史")} desc={t("空查询时展示常用条目")}>
            <Toggle on={s.historyEnabled} onChange={(v) => update("historyEnabled", v)} />
          </Row>
          <Row label={t("历史保留条数")} desc={t("超出后自动丢弃最旧记录")}>
            <Slider
              value={s.historyLimit}
              min={50}
              max={500}
              step={10}
              fmt={(v) => `${Math.round(v)}`}
              onChange={(v) => update("historyLimit", v)}
            />
          </Row>
        </div>
      </Section>
    </>
  );
}

function IndexPane({
  s,
  update,
  status,
  setStatus,
}: {
  s: Settings;
  update: Updater;
  status: IndexStatus | null;
  setStatus: (v: IndexStatus | null) => void;
}) {
  const [busy, setBusy] = useState(false);

  const refreshStatus = useCallback(() => {
    api.indexStatus().then(setStatus).catch(() => {});
  }, [setStatus]);

  useEffect(() => {
    if (busy) {
      const t = window.setInterval(refreshStatus, 700);
      return () => window.clearInterval(t);
    }
  }, [busy, refreshStatus]);

  const toggleDrive = (d: string) => {
    const has = s.indexedDrives.includes(d);
    const next = has ? s.indexedDrives.filter((x) => x !== d) : [...s.indexedDrives, d];
    update("indexedDrives", next);
  };

  const addDrive = async () => {
    const p = await dialogOpen({ directory: true, multiple: false });
    if (typeof p === "string") {
      const root = p.endsWith("\\") ? p : p + "\\";
      if (!s.indexedDrives.includes(root)) update("indexedDrives", [...s.indexedDrives, root]);
    }
  };

  const removeExclude = (d: string) =>
    update("excludeDirs", s.excludeDirs.filter((x) => x !== d));
  const addExclude = async () => {
    const p = await dialogOpen({ directory: true, multiple: false });
    if (typeof p === "string" && !s.excludeDirs.includes(p))
      update("excludeDirs", [...s.excludeDirs, p]);
  };

  const scanning = status?.state === "scanning";
  const pct = status && status.scanned > 0 ? Math.min(100, Math.round((status.scanned / Math.max(1, status.scanned)) * 100)) : 0;

  return (
    <>
      <h1>{t("索引")}</h1>
      <p className="pane-sub">{t("选择需要检索的磁盘与排除目录，重建索引使变更生效。")}</p>

      <Section title={t("索引状态")}>
        <div className="status-bar">
          <span
            className="status-dot"
            data-state={(status?.state as string) || "idle"}
          />
          <span>
            {scanning
              ? t("正在索引：{name}", { name: status?.current || "" })
              : status?.state === "ready"
              ? t("索引就绪")
              : t("空闲")}
          </span>
          <span className="spacer" />
          <span style={{ color: "var(--text-3)" }}>
            {t("已收录 {n} 项", { n: (status?.count ?? 0).toLocaleString() })}
          </span>
        </div>
        {scanning && (
          <div className="progress">
            <i style={{ width: `${pct}%` }} />
          </div>
        )}
        <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
          <button
            className="btn"
            data-variant="primary"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              await api.rebuildIndex();
              window.setTimeout(refreshStatus, 600);
              window.setTimeout(() => setBusy(false), 4000);
            }}
          >
            <Icon name="refresh" size={15} className={busy ? "spin" : ""} />{t("重建索引")}</button>
          <button
            className="btn"
            onClick={async () => {
              await api.cancelIndex();
              refreshStatus();
            }}
          >{t("取消")}</button>
        </div>
      </Section>

      <Section title={t("索引磁盘")}>
        <div className="card">
          <div className="row" style={{ flexWrap: "wrap" }}>
            <div className="chips">
              {s.indexedDrives.map((d) => (
                <button
                  key={d}
                  className="chip"
                  data-on={true}
                  onClick={() => toggleDrive(d)}
                  title={t("点击取消索引")}
                >
                  <Icon name="hard-drive" size={14} />
                  {d.replace(/\\/g, "")}
                  <span className="chip-x">
                    <Icon name="close" size={12} />
                  </span>
                </button>
              ))}
              <button className="chip" onClick={addDrive}>
                <Icon name="plus" size={14} />{t("添加磁盘")}</button>
            </div>
          </div>
          <Row label={t("包含隐藏文件")} desc={t("索引以点开头的隐藏文件")}>
            <Toggle on={s.indexHiddenFiles} onChange={(v) => update("indexHiddenFiles", v)} />
          </Row>
        </div>
      </Section>

      <Section title={t("排除目录")}>
        <div className="card">
          <div className="row" style={{ flexWrap: "wrap" }}>
            <div className="chips">
              {s.excludeDirs.map((d) => (
                <button
                  key={d}
                  className="chip"
                  data-on={false}
                  onClick={() => removeExclude(d)}
                  title={t("点击移除排除")}
                >
                  <Icon name="folder" size={14} />
                  {d}
                  <span className="chip-x">
                    <Icon name="close" size={12} />
                  </span>
                </button>
              ))}
              <button className="chip" onClick={addExclude}>
                <Icon name="plus" size={14} />{t("添加排除")}</button>
            </div>
          </div>
        </div>
      </Section>
    </>
  );
}

function FavoritesPane({ s, update }: { s: Settings; update: Updater }) {
  const add = async () => {
    const p = await dialogOpen({ directory: false, multiple: true });
    const paths = Array.isArray(p) ? p : p ? [p] : [];
    let list = s.favorites;
    for (const path of paths) {
      const res = await api.favoritesAdd(path);
      list = res;
    }
    update("favorites", list);
  };
  const remove = async (id: string) => update("favorites", await api.favoritesRemove(id));
  // Switch a favorite on or off — it stays in the list but disappears from
  // the launcher results.
  const toggle = async (f: Favorite) =>
    update("favorites", await api.favoritesSetEnabled(f.id, !f.enabled));
  // Point a favorite at another folder or file.
  const repoint = async (f: Favorite) => {
    const p = await dialogOpen({ directory: f.isDir, multiple: false });
    if (!p || Array.isArray(p)) return;
    update("favorites", await api.favoritesSetPath(f.id, p));
  };

  const fixed = s.favorites.filter((f) => f.pinned);
  const custom = s.favorites.filter((f) => !f.pinned);

  const row = (f: Favorite) => (
    <div className="list-row" key={f.id} data-disabled={!f.enabled}>
      <div className="lr-icon">
        <Icon name={f.isDir ? "folder" : "file"} size={16} />
      </div>
      <div className="lr-body">
        <div className="lr-title">
          {f.name}
          {f.pinned && <span className="lr-tag">{t("固定")}</span>}
        </div>
        <div className="lr-sub">{f.path}</div>
      </div>
      <div className="lr-actions">
        <Toggle on={f.enabled} onChange={() => toggle(f)} />
        <button
          className="btn"
          data-size="sm"
          title={t("修改路径")}
          onClick={() => repoint(f)}
        >
          <Icon name="folder" size={14} />
        </button>
        <button
          className="btn"
          data-size="sm"
          title={t("打开")}
          onClick={() => api.openPath(f.path)}
        >
          <Icon name="external" size={14} />
        </button>
        {!f.pinned && (
          <button
            className="btn"
            data-variant="danger"
            data-size="sm"
            title={t("删除")}
            onClick={() => remove(f.id)}
          >
            <Icon name="trash" size={14} />
          </button>
        )}
      </div>
    </div>
  );

  return (
    <>
      <h1>{t("收藏夹")}</h1>
      <p className="pane-sub">{t("固定常用文件夹与文件，随时一键直达。")}</p>

      <Section title={t("固定项")}>
        <div className="list">{fixed.map(row)}</div>
        <div className="lr-note">{t("固定项不可删除：可用开关停用，或修改路径指向其他位置。")}</div>
      </Section>

      <Section title={t("自定义收藏")}>
        <div className="list">
          {custom.length === 0 && <div className="lr-empty">{t("暂无收藏，点击下方添加。")}</div>}
          {custom.map(row)}
        </div>
        <button className="btn" style={{ marginTop: 12 }} onClick={add}>
          <Icon name="plus" size={15} />{t("添加收藏")}
        </button>
      </Section>
    </>
  );
}

function WebPane({ s, update }: { s: Settings; update: Updater }) {
  const [editId, setEditId] = useState<string | null>(null);

  const upsert = (eng: WebEngine) => {
    const list = s.webEngines.slice();
    const i = list.findIndex((e) => e.id === eng.id);
    if (i >= 0) list[i] = eng;
    else list.push(eng);
    update("webEngines", list);
    setEditId(null);
  };
  const remove = (id: string) =>
    update("webEngines", s.webEngines.filter((e) => e.id !== id));
  const toggle = (id: string) =>
    update(
      "webEngines",
      s.webEngines.map((e) => (e.id === id ? { ...e, enabled: !e.enabled } : e))
    );

  return (
    <>
      <h1>{t("搜索引擎")}</h1>
      <p className="pane-sub">{t("输入「关键字 + 空格 + 词」即可直接搜索。在搜索框使用如 g 猫 触发。")}</p>
      <Section title={t("引擎列表")}>
        <div className="list">
          {s.webEngines.map((e) => (
            <div key={e.id}>
              {editId === e.id ? (
                <EngineEditor
                  eng={e}
                  onSave={upsert}
                  onCancel={() => setEditId(null)}
                />
              ) : (
                <div className="list-row">
                  <div className="lr-icon">
                    <Icon name="globe" size={16} />
                  </div>
                  <div className="lr-body">
                    <div className="lr-title">
                      {e.name}{" "}
                      <span className="badge">{e.keyword || "—"}</span>
                    </div>
                    <div className="lr-sub">{e.url}</div>
                  </div>
                  <div className="lr-actions">
                    <Toggle on={e.enabled} onChange={() => toggle(e.id)} />
                    <button className="btn" data-size="sm" onClick={() => setEditId(e.id)}>
                      <Icon name="pen" size={14} />
                    </button>
                    <button
                      className="btn"
                      data-variant="danger"
                      data-size="sm"
                      onClick={() => remove(e.id)}
                    >
                      <Icon name="trash" size={14} />
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
        <button
          className="btn"
          style={{ marginTop: 12 }}
          onClick={() =>
            upsert({
              id: `eng-${Date.now()}`,
              name: t("新引擎"),
              keyword: "",
              url: "https://www.google.com/search?q={query}",
              enabled: true,
            })
          }
        >
          <Icon name="plus" size={15} />{t("添加引擎")}</button>
      </Section>
    </>
  );
}

function EngineEditor({
  eng,
  onSave,
  onCancel,
}: {
  eng: WebEngine;
  onSave: (e: WebEngine) => void;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState(eng);
  return (
    <div className="lr-edit">
      <div className="grid2">
        <input
          className="input"
          placeholder={t("名称")}
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        />
        <input
          className="input"
          placeholder={t("关键字（如 g）")}
          value={draft.keyword}
          onChange={(e) => setDraft({ ...draft, keyword: e.target.value })}
        />
      </div>
      <input
        className="input"
        placeholder={t("搜索 URL，用 {query} 占位")}
        value={draft.url}
        onChange={(e) => setDraft({ ...draft, url: e.target.value })}
      />
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn" data-size="sm" onClick={onCancel}>{t("取消")}</button>
        <button className="btn" data-variant="primary" data-size="sm" onClick={() => onSave(draft)}>{t("保存")}</button>
      </div>
    </div>
  );
}

function CommandsPane({ s, update }: { s: Settings; update: Updater }) {
  const [editId, setEditId] = useState<string | null>(null);

  const upsert = (c: CustomCommand) => {
    const list = s.customCommands.slice();
    const i = list.findIndex((x) => x.id === c.id);
    if (i >= 0) list[i] = c;
    else list.push(c);
    update("customCommands", list);
    setEditId(null);
  };
  const remove = (id: string) =>
    update("customCommands", s.customCommands.filter((x) => x.id !== id));

  return (
    <>
      <h1>{t("自定义命令")}</h1>
      <p className="pane-sub">{t("添加自己的快捷指令：打开网址、运行程序或执行命令。")}</p>
      <Section title={t("命令列表")}>
        <div className="list">
          {s.customCommands.map((c) => (
            <div key={c.id}>
              {editId === c.id ? (
                <CommandEditor cmd={c} onSave={upsert} onCancel={() => setEditId(null)} />
              ) : (
                <div className="list-row">
                  <div className="lr-icon">
                    <Icon
                      name={c.action === "url" ? "globe" : c.action === "shell" ? "terminal" : "app"}
                      size={16}
                    />
                  </div>
                  <div className="lr-body">
                    <div className="lr-title">
                      {c.name}{" "}
                      <span className="badge">{c.keyword || "—"}</span>
                    </div>
                    <div className="lr-sub">
                      {c.action} · {c.target} {c.args}
                    </div>
                  </div>
                  <div className="lr-actions">
                    <Toggle
                      on={c.enabled}
                      onChange={() =>
                        update(
                          "customCommands",
                          s.customCommands.map((x) =>
                            x.id === c.id ? { ...x, enabled: !x.enabled } : x
                          )
                        )
                      }
                    />
                    <button className="btn" data-size="sm" onClick={() => setEditId(c.id)}>
                      <Icon name="pen" size={14} />
                    </button>
                    <button
                      className="btn"
                      data-variant="danger"
                      data-size="sm"
                      onClick={() => remove(c.id)}
                    >
                      <Icon name="trash" size={14} />
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))}
          {s.customCommands.length === 0 && (
            <div className="lr-empty">{t("暂无自定义命令。")}</div>
          )}
        </div>
        <button
          className="btn"
          style={{ marginTop: 12 }}
          onClick={() =>
            upsert({
              id: `cmd-${Date.now()}`,
              name: t("新命令"),
              keyword: "",
              action: "program",
              target: "",
              args: "",
              description: "",
              enabled: true,
            })
          }
        >
          <Icon name="plus" size={15} />{t("添加命令")}</button>
      </Section>
    </>
  );
}

function CommandEditor({
  cmd,
  onSave,
  onCancel,
}: {
  cmd: CustomCommand;
  onSave: (c: CustomCommand) => void;
  onCancel: () => void;
}) {
  const [d, setD] = useState(cmd);
  return (
    <div className="lr-edit">
      <div className="grid2">
        <input
          className="input"
          placeholder={t("名称")}
          value={d.name}
          onChange={(e) => setD({ ...d, name: e.target.value })}
        />
        <input
          className="input"
          placeholder={t("关键字")}
          value={d.keyword}
          onChange={(e) => setD({ ...d, keyword: e.target.value })}
        />
      </div>
      <div className="grid2">
        <select
          className="select"
          value={d.action}
          onChange={(e) => setD({ ...d, action: e.target.value as CustomCommand["action"] })}
        >
          <option value="program">{t("程序")}</option>
          <option value="url">{t("网址")}</option>
          <option value="shell">{t("命令行")}</option>
          <option value="path">{t("打开路径")}</option>
          <option value="kestrel">{t("Kestrel 内置")}</option>
        </select>
        <input
          className="input"
          placeholder={t("目标（路径/URL/命令）")}
          value={d.target}
          onChange={(e) => setD({ ...d, target: e.target.value })}
        />
      </div>
      <input
        className="input"
        placeholder={t("参数（可选）")}
        value={d.args}
        onChange={(e) => setD({ ...d, args: e.target.value })}
      />
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn" data-size="sm" onClick={onCancel}>{t("取消")}</button>
        <button className="btn" data-variant="primary" data-size="sm" onClick={() => onSave(d)}>{t("保存")}</button>
      </div>
    </div>
  );
}

function ClipboardPane({ s, update }: { s: Settings; update: Updater }) {
  return (
    <>
      <h1>{t("剪贴板")}</h1>
      <p className="pane-sub">{t("在启动器中按 Tab 切换至剪贴板历史，随时回溯与粘贴。")}</p>
      <Section title={t("历史记录")}>
        <div className="card">
          <Row label={t("启用剪贴板历史")} desc={t("监听复制内容并建立历史")}>
            <Toggle on={s.clipboardEnabled} onChange={(v) => update("clipboardEnabled", v)} />
          </Row>
          <Row label={t("记录图片")} desc={t("捕获复制的图片（占用更多内存）")}>
            <Toggle on={s.clipboardImages} onChange={(v) => update("clipboardImages", v)} />
          </Row>
          <Row label={t("历史保留条数")} desc={t("超出后自动丢弃最旧记录")}>
            <Slider
              value={s.clipboardLimit}
              min={50}
              max={500}
              step={10}
              fmt={(v) => `${Math.round(v)}`}
              onChange={(v) => update("clipboardLimit", v)}
            />
          </Row>
        </div>
      </Section>
    </>
  );
}

function FeaturesPane({ s, update }: { s: Settings; update: Updater }) {
  return (
    <>
      <h1>{t("实用工具")}</h1>
      <p className="pane-sub">{t("随启动器一同提供的扩展功能。")}</p>
      <Section title={t("功能开关")}>
        <div className="card">
          <Row label={t("计算器")} desc={t("在搜索框直接输入算式，实时得出结果")}>
            <Toggle on={s.calculatorEnabled} onChange={(v) => update("calculatorEnabled", v)} />
          </Row>
          <Row label={t("窗口切换器")} desc={t("快速浏览并切换已打开的窗口")}>
            <Toggle
              on={s.windowSwitcherEnabled}
              onChange={(v) => update("windowSwitcherEnabled", v)}
            />
          </Row>
        </div>
      </Section>

      <Section title={t("快捷笔记")}>
        <div className="field">
          <label>{t("随启随记的便签文本")}</label>
          <textarea
            className="input"
            value={s.quickNotes}
            placeholder={t("写下常用片段、模板或临时备忘…")}
            onChange={(e) => update("quickNotes", e.target.value)}
          />
          <div className="hint">{t("在启动器中切换到「笔记」标签即可查看与复制。")}</div>
        </div>
      </Section>
    </>
  );
}

function AppsPane({
  apps,
  setApps,
}: {
  apps: AppEntry[];
  setApps: (a: AppEntry[]) => void;
}) {
  const [loading, setLoading] = useState(false);
  const [appIcons, setAppIcons] = useState<Record<string, string>>({});
  useEffect(() => {
    api.listApps().then(setApps).catch(() => {});
  }, [setApps]);
  // Extract real icons for the first batch of apps; fall back to a glyph.
  useEffect(() => {
    if (!apps.length) return;
    const paths = apps
      .slice(0, 150)
      .map((a) => a.target || a.lnk)
      .filter(Boolean);
    if (!paths.length) return;
    api.iconsFor(paths, 48).then(setAppIcons).catch(() => {});
  }, [apps]);

  return (
    <>
      <h1>{t("应用")}</h1>
      <p className="pane-sub">{t("Kestrel 索引到的可搜索程序（来自开始菜单与桌面）。")}</p>
      <Section title={t("已索引应用")}>
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <button
            className="btn"
            data-variant="primary"
            disabled={loading}
            onClick={async () => {
              setLoading(true);
              const list = await api.refreshApps();
              setApps(list);
              setLoading(false);
            }}
          >
            <Icon name="refresh" size={15} className={loading ? "spin" : ""} />{t("重新扫描")}</button>
          <span style={{ color: "var(--text-3)", fontSize: 13, alignSelf: "center" }}>
            {t("{n} 个应用", { n: apps.length })}
          </span>
        </div>
        <div className="apps-grid">
          {apps.map((a) => {
            const key = a.target || a.lnk;
            const ico = key ? appIcons[key] : undefined;
            return (
              <div className="app-tile" key={a.id} title={a.target}>
                {ico ? (
                  <img className="at-ico" src={ico} alt="" />
                ) : (
                  <div className="at-ico at-fallback">
                    <Icon name="app" size={22} />
                  </div>
                )}
                <span className="at-name">{a.name}</span>
              </div>
            );
          })}
          {apps.length === 0 && <div className="lr-empty">{t("尚未扫描到应用。")}</div>}
        </div>
      </Section>
    </>
  );
}

const RELEASE_KIND: Record<ReleaseKind, string> = {
  new: "新增",
  improve: "改进",
  fix: "修复",
};

function AboutPane({ build, s }: { build: BuildInfo | null; s: Settings }) {
  const lang = useLang();
  // Newest release starts expanded; older ones open on demand.
  const [open, setOpen] = useState<string | null>(RELEASES[0]?.version ?? null);
  return (
    <>
      <h1>{t("关于")}</h1>
      <p className="pane-sub">{t("Kestrel（隼）— 极速文件定位与命令启动器。")}</p>
      <div className="about-hero">
        <div className="big-logo">
          <Icon name="search" size={28} strokeWidth={2.2} />
        </div>
        <div>
          <div className="ah-name">{t("Kestrel 隼")}</div>
          <div className="ah-tag">{t("瞬达万千文件，一击即至。")}</div>
        </div>
      </div>
      <dl className="kv">
        <dt>{t("版本")}</dt>
        <dd>{build?.version ?? "—"}</dd>
        <dt>{t("构建号")}</dt>
        <dd>{build?.build ?? "—"}</dd>
        <dt>{t("构建时间")}</dt>
        <dd>{build?.builtAt ?? "—"}</dd>
        <dt>{t("架构")}</dt>
        <dd>{build?.arch ?? "—"}</dd>
        <dt>{t("系统")}</dt>
        <dd>{build?.os ?? "—"}</dd>
        <dt>{t("索引条目")}</dt>
        <dd>{(build?.indexCount ?? 0).toLocaleString()}</dd>
        <dt>{t("索引根")}</dt>
        <dd>{(build?.indexRoots ?? s.indexedDrives).join("、") || "—"}</dd>
      </dl>

      <Section title={t("发布日志")}>
        <div className="releases">
          {RELEASES.map((r) => {
            const isCurrent = !!build && r.version === build.version;
            const expanded = open === r.version;
            return (
              <div className="release" key={r.version} data-current={isCurrent}>
                <button
                  className="release-head"
                  data-open={expanded}
                  onClick={() => setOpen(expanded ? null : r.version)}
                >
                  <span className="rv">v{r.version}</span>
                  {isCurrent && <span className="rbadge">{t("当前版本")}</span>}
                  <span className="rtitle">{pick(r.title, lang)}</span>
                  <span className="rdate">{r.date}</span>
                  <Icon name={expanded ? "chevron-down" : "chevron-right"} size={14} />
                </button>
                {expanded && (
                  <ul className="release-items">
                    {r.items.map((it, i) => (
                      <li key={i} data-kind={it.kind}>
                        <span className="rk">{t(RELEASE_KIND[it.kind])}</span>
                        <span className="rt">{pick(it.text, lang)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            );
          })}
        </div>
      </Section>
    </>
  );
}
