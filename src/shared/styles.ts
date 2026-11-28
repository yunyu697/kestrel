/**
 * The four interface styles offered in the settings page.
 *
 * A style is a *skeleton*, not a skin: it changes how navigation is drawn,
 * how rows are grouped, how a control is shaped and how tightly the page is
 * packed. Colours are never part of it — those come from `theme` and
 * `colorScheme`, which is what makes the two axes independent.
 *
 * The base radii live in `theme.ts` because they feed the shape tokens; this
 * file only carries what the picker needs to render and label the choice.
 */

import type { StyleId } from "./theme";

export type StyleMeta = {
  id: StyleId;
  /** Proper noun — the same in every language, never translated. */
  label: string;
  /** Chinese source key, translated through `t()`. */
  desc: string;
};

export const STYLES: StyleMeta[] = [
  {
    id: "fluent",
    label: "Fluent 2",
    desc: "克制而工具化：侧栏竖条选中，每行一张独立卡片",
  },
  {
    id: "apple",
    label: "Apple HIG",
    desc: "安静留白：整组容器，行间发丝线，按钮为药丸形",
  },
  {
    id: "material",
    label: "Material 3",
    desc: "块面表达：图标导航栏，表面色分层，药丸选中",
  },
  {
    id: "ant",
    label: "Ant Design",
    desc: "密度与规则：菜单侧栏，逐行分隔线，紧凑行高",
  },
];

export function styleMeta(id: string | undefined): StyleMeta {
  return STYLES.find((s) => s.id === id) ?? STYLES[0];
}
