/**
 * Release notes shown in 设置 → 关于.
 *
 * Each item carries its own translations instead of going through the global
 * dictionary: the log is content (it grows every release) and keeping it in one
 * file makes adding an entry a single-place edit. Resolution order is
 * language → Traditional Chinese → English, so an untranslated line still reads
 * sensibly instead of showing a raw key.
 */

import type { Lang } from "./i18n";

export type ReleaseKind = "new" | "improve" | "fix";

export interface ReleaseItem {
  kind: ReleaseKind;
  text: Partial<Record<Lang, string>> & { zh: string; en: string };
}

export interface Release {
  version: string;
  date: string;
  /** Headline of the release, one line. */
  title: Partial<Record<Lang, string>> & { zh: string; en: string };
  items: ReleaseItem[];
}

export const RELEASES: Release[] = [
  {
    version: "1.2.1",
    date: "2026-10-31",
    title: {
      zh: "修复检查更新与镜像测速必失败的问题",
      en: "Fixed update checks and mirror speed tests always failing",
      ja: "更新チェックとミラー速度テストが必ず失敗する問題を修正",
      es: "Corregido: la comprobación de actualizaciones y la prueba de espejos siempre fallaban",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "修复 1.2.0 的检查更新与镜像测速在任何网络下都必然失败：安全策略未放行外部网络请求，WebView 把所有 fetch 直接拦截了",
          en: "Fixed update checks and mirror speed tests in 1.2.0 failing on every network: the content security policy blocked all outbound requests from the WebView",
          ja: "1.2.0 で更新チェックとミラー速度テストがどのネットワークでも必ず失敗する問題を修正（CSP が WebView の外部リクエストをすべてブロックしていた）",
          es: "Corregido: en la 1.2.0 las comprobaciones y pruebas de espejos fallaban en cualquier red: la política de seguridad bloqueaba todas las peticiones del WebView",
        },
      },
      {
        kind: "new",
        text: {
          zh: "检查更新增加备用源：GitHub API 不可达时自动改用 jsDelivr 查询最新版本号",
          en: "Update checks gained a fallback source: when GitHub's API is unreachable, the newest version is looked up via jsDelivr instead",
          ja: "更新チェックに予備ソースを追加：GitHub API に届かない場合は jsDelivr で最新バージョンを確認",
          es: "La comprobación de actualizaciones ahora tiene una fuente alternativa: si la API de GitHub no responde, se consulta la versión más reciente vía jsDelivr",
        },
      },
    ],
  },
  {
    version: "1.2.0",
    date: "2026-10-31",
    title: {
      zh: "检查更新：自动发现新版本，镜像加速下载",
      en: "Update check: find new releases automatically, mirror-accelerated downloads",
      ja: "更新チェック：新バージョンの自動検出とミラーダウンロード",
      es: "Comprobación de actualizaciones con descarga por espejos",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "自动检查更新：可设每天/每周/从不，支持启动时立即检查；发现新版本时启动器顶部出现提示条，点击直达下载",
          en: "Automatic update checks: daily, weekly or never, with an optional check on every launch; a banner appears in the launcher when a new version is found",
          ja: "自動更新チェック：毎日／毎週／しないから選べ、起動時の確認にも対応。新バージョン検出時はランチャー上部にバナーを表示",
          es: "Comprobaciones automáticas: cada día, cada semana o nunca, con opción al iniciar; un aviso aparece en el lanzador al hallar una versión nueva",
        },
      },
      {
        kind: "new",
        text: {
          zh: "下载镜像与测速：内置多个 GitHub 加速镜像，一键测连通速度，也可填自定义镜像前缀",
          en: "Download mirrors with a live speed test: several GitHub accelerators built in, plus a custom prefix field",
          ja: "ダウンロードミラーと速度テスト：GitHub 加速ミラーを内蔵し、独自の接頭辞も設定できます",
          es: "Espejos de descarga con prueba de velocidad: varios aceleradores de GitHub integrados y prefijo personalizado",
        },
      },
    ],
  },
  {
    version: "1.1.0",
    date: "2026-10-31",
    title: {
      zh: "快速切换：路径导航，跳转不替你确认",
      en: "Quick Switch: path navigation, jumping without confirming for you",
      ja: "クイック切替：パス移動と、確認しないジャンプ",
      es: "Cambio rápido: navegación por rutas y saltos sin confirmar",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "快速切换支持路径语法：.. 上跳一级、\\ 到盘根、D: 直接切盘、~ 回到用户目录，半截名字自动补全",
          en: "Quick Switch understands paths: .. for up one level, \\ for the drive root, D: to switch drive, ~ for home, and half-typed names complete themselves",
          ja: "クイック切替でパス構文に対応：.. で 1 つ上、\\ でドライブのルート、D: でドライブ切替、~ でホーム、途中までの名前は自動補完",
          es: "Cambio rápido entiende rutas: .. para subir, \\ para la raíz, D: para cambiar de unidad, ~ para la carpeta personal y autocompletado",
        },
      },
      {
        kind: "new",
        text: {
          zh: "快速切换可用 ← / → 在目录层级间来回走，连续下钻不必重新输入",
          en: "← / → walk the folder tree in Quick Switch, so drilling several levels deep needs no retyping",
          ja: "クイック切替で ← / → によりフォルダー階層を移動でき、深く潜っても入力し直す必要がありません",
          es: "← / → recorren el árbol de carpetas en Cambio rápido sin volver a escribir al profundizar",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "快速切换不再替你按下「打开/保存」：现在只跳转目录，要不要用由你决定",
          en: "Quick Switch no longer presses Open/Save for you: it moves the dialog and leaves the choice to you",
          ja: "クイック切替が「開く／保存」を自動で押さなくなりました。移動するだけで、使うかどうかはユーザーが決めます",
          es: "Cambio rápido ya no pulsa Abrir/Guardar por ti: mueve el diálogo y la decisión es tuya",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复快速切换栏被别的窗口盖住、或反过来浮在其它窗口之上的问题",
          en: "Fixed the Quick Switch box being buried under other windows — or floating above them",
          ja: "クイック切替バーが他のウィンドウに隠れたり、逆に前面に出続けたりする問題を修正",
          es: "Corregido: la barra quedaba detrás de otras ventanas o flotaba sobre ellas",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复文件对话框打开较慢时，快速切换栏第一次唤不出来的问题",
          en: "Fixed the Quick Switch box not appearing the first time on a slow file dialog",
          ja: "ファイルダイアログの表示が遅い場合、クイック切替バーが最初に出ない問題を修正",
          es: "Corregido: la barra no aparecía la primera vez en diálogos de apertura lentos",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复长时间只用鼠标（看文档、看视频）后，双击 Ctrl 偶发失效的问题",
          en: "Fixed double Ctrl occasionally failing after a long stretch of mouse-only use",
          ja: "マウス操作のみが続いた後に Ctrl 二度押しが効かなくなる問題を修正",
          es: "Corregido: el doble Ctrl fallaba a veces tras un uso prolongado solo con el ratón",
        },
      },
    ],
  },
  {
    version: "1.0.0",
    date: "2026-10-30",
    title: {
      zh: "可定制的界面与更稳的运行",
      en: "A more customisable UI, a steadier app",
      ja: "カスタマイズ性と安定性の向上",
      es: "Interfaz más personalizable y más estable",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "界面语言支持「跟随系统」，切换 Windows 语言后自动匹配",
          en: "Interface language can follow the system and match Windows automatically",
          ja: "表示言語が「システムに従う」に対応し、Windows の言語に自動で合わせます",
          es: "El idioma de la interfaz puede seguir al sistema y coincidir con Windows",
        },
      },
      {
        kind: "new",
        text: {
          zh: "关于页新增发布日志，可直接查看每个版本的改动",
          en: "Release notes in About, so every version's changes are visible in the app",
          ja: "「バージョン情報」にリリースノートを追加しました",
          es: "Notas de la versión en «Acerca de» para consultar los cambios",
        },
      },
      {
        kind: "new",
        text: {
          zh: "常规页新增「以管理员身份运行」，需要提权的操作不再被拒绝",
          en: "New “Run as administrator” option so privileged actions stop failing",
          ja: "「管理者として実行」を追加し、権限が必要な操作が通るようになりました",
          es: "Nueva opción «Ejecutar como administrador» para las acciones que requieren permisos",
        },
      },
      {
        kind: "new",
        text: {
          zh: "预览面板可停靠在底部，窄窗口下更省横向空间",
          en: "The preview panel can dock at the bottom, saving horizontal space",
          ja: "プレビューを下部に配置できるようになり、横方向のスペースを節約できます",
          es: "El panel de vista previa puede acoplarse abajo y ahorrar espacio horizontal",
        },
      },
      {
        kind: "new",
        text: {
          zh: "颜色主题可分别调整窗口、卡片、面板、三档文字、描边与选中色",
          en: "Colours are now adjustable per element: window, cards, panels, three text levels, borders and selection",
          ja: "配色を要素ごとに調整可能に（ウィンドウ・カード・パネル・文字3段階・罫線・選択色）",
          es: "Los colores se ajustan por elemento: ventana, tarjetas, paneles, tres niveles de texto, bordes y selección",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "修正高 DPI（如 175%）下窗口尺寸被放大、字体发虚、右栏被裁切的问题",
          en: "Fixed oversized, blurry and clipped windows on high-DPI displays (e.g. 175%)",
          ja: "高 DPI（175% など）でウィンドウが拡大され、文字がぼやけ、右端が切れる問題を修正",
          es: "Corregido el tamaño excesivo, el texto borroso y el recorte en pantallas de DPI alto",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "快速切换栏按整行对齐高度，最后一行与底栏不再被切掉一半",
          en: "The Quick Switch box now snaps to whole rows instead of slicing the last one",
          ja: "クイック切替バーの高さを行単位に揃え、最終行とフッターが欠けないようにしました",
          es: "La barra de cambio rápido ajusta su altura a filas completas y ya no corta la última",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复首次打开设置窗口时界面卡死的问题",
          en: "Fixed the app freezing the first time the settings window opened",
          ja: "設定ウィンドウを初めて開くときに固まる問題を修正",
          es: "Corregido el bloqueo al abrir la ventana de ajustes por primera vez",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复「预览位置」设置不生效的问题",
          en: "Fixed the “preview position” setting having no effect",
          ja: "「プレビューの位置」設定が効かない問題を修正",
          es: "Corregido el ajuste «posición de la vista previa», que no tenía efecto",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复启动器已显示时双击 Ctrl 无法隐藏的问题",
          en: "Fixed double Ctrl failing to hide an already visible launcher",
          ja: "ランチャー表示中に Ctrl の二度押しで隠せない問題を修正",
          es: "Corregido: doble Ctrl no ocultaba el lanzador ya visible",
        },
      },
    ],
  },
  {
    version: "0.9.0",
    date: "2026-10-28",
    title: {
      zh: "收藏夹固定项与快捷键预设",
      en: "Pinned folders and shortcut presets",
      ja: "固定フォルダーとショートカットのプリセット",
      es: "Carpetas fijas y atajos predefinidos",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "收藏夹的桌面、下载、文档、图片、音乐、视频作为固定项，可开关与修改路径，不可删除",
          en: "Desktop, Downloads, Documents, Pictures, Music and Videos are pinned: toggle or re-point them, but they cannot be deleted",
          ja: "デスクトップ・ダウンロード・ドキュメント・ピクチャ・ミュージック・ビデオを固定項目に（オン/オフとパス変更が可能、削除は不可）",
          es: "Escritorio, Descargas, Documentos, Imágenes, Música y Vídeos quedan fijados: se pueden activar o redirigir, no borrar",
        },
      },
      {
        kind: "new",
        text: {
          zh: "主快捷键增加常用预设，一键套用",
          en: "Preset shortcuts for the main hotkey, applied in one click",
          ja: "メインのショートカットにプリセットを追加し、ワンクリックで適用",
          es: "Atajos predefinidos para el atajo principal, aplicados con un clic",
        },
      },
      {
        kind: "new",
        text: {
          zh: "界面语言增加繁體中文、日本語、Español",
          en: "Added Traditional Chinese, Japanese and Spanish",
          ja: "繁体字中国語・日本語・スペイン語を追加",
          es: "Se añadieron chino tradicional, japonés y español",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "索引改为压缩存储，磁盘占用大幅下降",
          en: "The index is stored compressed, using far less disk space",
          ja: "インデックスを圧縮保存し、ディスク使用量を大幅に削減",
          es: "El índice se guarda comprimido y ocupa mucho menos disco",
        },
      },
    ],
  },
  {
    version: "0.8.0",
    date: "2026-10-27",
    title: {
      zh: "快速切换吸附文件对话框",
      en: "Quick Switch inside file dialogs",
      ja: "ファイルダイアログに追従するクイック切替",
      es: "Cambio rápido dentro de los diálogos de archivos",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "在打开/保存对话框中按 Ctrl+G，搜索栏吸附到对话框下方",
          en: "Press Ctrl+G in an open/save dialog to attach the search box underneath it",
          ja: "開く／保存ダイアログで Ctrl+G を押すと、検索ボックスが下に吸着します",
          es: "Pulsa Ctrl+G en un diálogo de abrir/guardar para acoplar el buscador debajo",
        },
      },
      {
        kind: "new",
        text: {
          zh: "托盘单击唤起启动器，双击直接打开设置",
          en: "Single-click the tray icon to open the launcher, double-click for settings",
          ja: "トレイをシングルクリックで起動、ダブルクリックで設定",
          es: "Un clic en la bandeja abre el lanzador, doble clic abre los ajustes",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "托盘双击不再误唤醒搜索栏",
          en: "Double-clicking the tray icon no longer wakes the search bar",
          ja: "トレイのダブルクリックで検索バーが誤って開かなくなりました",
          es: "El doble clic en la bandeja ya no abre el buscador por error",
        },
      },
    ],
  },
  {
    version: "0.7.0",
    date: "2026-10-26",
    title: {
      zh: "首个版本",
      en: "First release",
      ja: "最初のリリース",
      es: "Primera versión",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "极速文件索引与搜索，支持拼音、模糊匹配与路径匹配",
          en: "Fast file indexing and search with pinyin, fuzzy and path matching",
          ja: "高速なインデックスと検索。ピンイン・あいまい一致・パス一致に対応",
          es: "Indexación y búsqueda rápida con pinyin, coincidencia difusa y de rutas",
        },
      },
      {
        kind: "new",
        text: {
          zh: "内置计算器、剪贴板历史、窗口切换与自定义命令",
          en: "Built-in calculator, clipboard history, window switcher and custom commands",
          ja: "電卓・クリップボード履歴・ウィンドウ切替・カスタムコマンドを内蔵",
          es: "Calculadora, historial del portapapeles, conmutador de ventanas y comandos propios",
        },
      },
    ],
  },
];

/** Pick the best available translation for `lang`. */
export function pick(
  text: Partial<Record<Lang, string>> & { zh: string; en: string },
  lang: Lang,
): string {
  return text[lang] ?? (lang === "zh-TW" ? text.zh : undefined) ?? text.en ?? text.zh;
}
