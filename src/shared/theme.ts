/**
 * Fine-grained colour overrides.
 *
 * The design tokens live in `theme.css`; this module lets the user replace any
 * of them individually. An empty value means "keep the theme default", so a
 * stored colour is always an explicit choice and switching light/dark keeps
 * working for the slots that were never touched.
 */

import type { Settings } from "./types";
import { schemeById, schemePalette } from "./palettes";

export interface ColorSlot {
  /** Key on `Settings`. */
  key: keyof Settings;
  /** CSS custom property it overrides. */
  cssVar: string;
  /** Source string used as the i18n key. */
  label: string;
  /** Description shown under the label. */
  desc: string;
}

export const COLOR_SLOTS: ColorSlot[] = [
  {
    key: "colorBg",
    cssVar: "--bg",
    label: "窗口背景",
    desc: "启动器与设置窗口的底色",
  },
  {
    key: "colorSurface",
    cssVar: "--surface",
    label: "卡片背景",
    desc: "卡片、输入框与列表的底色",
  },
  {
    key: "colorSurface2",
    cssVar: "--surface-2",
    label: "面板背景",
    desc: "预览面板与次级区域的底色",
  },
  {
    key: "colorText",
    cssVar: "--text",
    label: "主文字",
    desc: "标题、结果名与正文",
  },
  {
    key: "colorMuted",
    cssVar: "--text-2",
    label: "次要文字",
    desc: "说明文字与路径",
  },
  {
    key: "colorFaint",
    cssVar: "--text-3",
    label: "辅助文字",
    desc: "提示、占位符与计数",
  },
  {
    key: "colorBorder",
    cssVar: "--border",
    label: "描边",
    desc: "分隔线、卡片与控件边框",
  },
  {
    key: "colorHover",
    cssVar: "--hover",
    label: "选中与悬停",
    desc: "结果选中行与按钮悬停底色",
  },
  {
    key: "colorSunken",
    cssVar: "--bg-sunken",
    label: "凹陷区背景",
    desc: "设置侧栏与标题栏下的底层",
  },
  {
    key: "colorSurface3",
    cssVar: "--surface-3",
    label: "控件底色",
    desc: "开关轨道、标签与按钮悬停",
  },
  {
    key: "colorActive",
    cssVar: "--active",
    label: "按下态",
    desc: "按钮与列表项被按下时的底色",
  },
  {
    key: "colorKbd",
    cssVar: "--kbd-bg",
    label: "键帽背景",
    desc: "底栏快捷键提示的按键底色",
  },
];

/** Solid stand-ins for the theme defaults, so an unset slot still shows a
 *  meaningful colour in the picker (the real values are partly translucent). */
export const THEME_SWATCHES: Record<"light" | "dark", Record<string, string>> = {
  light: {
    "--bg": "#FFFFFF",
    "--surface": "#FFFFFF",
    "--surface-2": "#F6F7FA",
    "--text": "#14161B",
    "--text-2": "#59616E",
    "--text-3": "#8B93A1",
    "--border": "#E4E7EE",
    "--hover": "#F2F4F8",
    "--bg-sunken": "#F4F6FA",
    "--surface-3": "#EEF1F6",
    "--active": "#ECEEF3",
    "--kbd-bg": "#F1F3F8",
  },
  dark: {
    "--bg": "#1B1D22",
    "--surface": "#202329",
    "--surface-2": "#262A31",
    "--text": "#ECEEF3",
    "--text-2": "#A4ACBA",
    "--text-3": "#767E8C",
    "--border": "#33373F",
    "--hover": "#2A2E36",
    "--bg-sunken": "#15171B",
    "--surface-3": "#2E333B",
    "--active": "#2F323A",
    "--kbd-bg": "#2E3137",
  },
};

/** Current value of a slot, falling back to the theme default. */
export function slotColor(
  s: Settings,
  slot: ColorSlot,
  theme: "light" | "dark",
): string {
  const v = String(s[slot.key] ?? "").trim();
  if (v) return v;
  return THEME_SWATCHES[theme][slot.cssVar] ?? "#888888";
}

/** Apply (or clear) every colour override on `:root`. */
export function applyColors(s: Settings, theme: "light" | "dark"): void {
  const root = document.documentElement;
  const scheme = s.colorScheme && s.colorScheme !== "custom" ? schemeById(s.colorScheme) : undefined;
  const palette = scheme ? schemePalette(scheme, theme) : null;
  for (const slot of COLOR_SLOTS) {
    let v = "";
    if (palette) {
      // A scheme owns every token: manual overrides are dormant while it is on.
      v = palette[slot.cssVar] ?? "";
    } else if (s.colorScheme === "custom") {
      v = String(s[slot.key] ?? "").trim();
    }
    if (v) root.style.setProperty(slot.cssVar, v);
    else root.style.removeProperty(slot.cssVar);
  }
  // Accent follows the active scheme so the whole UI stays one hue family;
  // only in custom/default mode does the standalone accent setting apply.
  root.style.setProperty("--accent", palette ? palette["--accent"] : s.accent);
}
