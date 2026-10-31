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
    version: "1.2.4",
    date: "2026-10-31",
    title: {
      zh: "应用内更新：更新 / 仅下载 / 打开下载页面",
      en: "In-app updates: install, download only, or open the release page",
      ja: "アプリ内更新：更新 / ダウンロードのみ / リリースページを開く",
      es: "Actualizaciones dentro de la app: instalar, solo descargar o abrir la página de la versión",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "发现新版本后提供三个操作：「更新」直接下载并在完成后自动完成安装；「仅下载」只下载安装包，下载完提示是否安装；「打开下载页面」打开该版本的 release 页面",
          en: "Three actions once a new version is found: «Update» downloads and installs automatically, «Download only» fetches the installer and then asks whether to install, and «Open release page» opens that release on GitHub",
          ja: "新バージョンを検出すると 3 つの操作：「更新」はダウンロード後に自動でインストール、「仅下载（ダウンロードのみ）」はインストーラーを保存するだけで完了後にインストールするか確認、「打开下载页面（リリースページを開く）」はその版の release ページを開きます",
          es: "Tres acciones al detectar una versión nueva: «Actualizar» descarga e instala automáticamente, «Solo descargar» guarda el instalador y luego pregunta si instalarlo, y «Abrir la página de la versión» abre esa versión en GitHub",
        },
      },
      {
        kind: "new",
        text: {
          zh: "下载在应用内完成，走系统网络组件（WinHTTP），带进度条、已下载大小与实时速度；可随时取消",
          en: "The download runs inside the app through the system network stack (WinHTTP) with a progress bar, byte counter and live speed; it can be cancelled at any time",
          ja: "ダウンロードはアプリ内でシステムのネットワーク層（WinHTTP）経由で行い、進捗バー・取得済みサイズ・実測速度を表示。いつでも中断できます",
          es: "La descarga se hace dentro de la app usando la pila de red del sistema (WinHTTP), con barra de progreso, tamaño descargado y velocidad en vivo; se puede cancelar en cualquier momento",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复「检查更新」找到新版本后按钮点了没反应：此前只是把下载链接丢给浏览器，现在改为应用内直接下载并安装",
          en: "Fixed the update button doing nothing after a new version was found: it used to hand the download link to the browser, now the app downloads and installs it itself",
          ja: "新バージョン検出後に更新ボタンが反応しなかった問題を修正。従来はダウンロードリンクをブラウザーに渡すだけでしたが、アプリ内で直接ダウンロード・インストールします",
          es: "Se corrigió que el botón de actualización no hiciera nada tras detectar una versión nueva: antes solo se pasaba el enlace al navegador; ahora la app descarga e instala por su cuenta",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "安装器会等应用完全退出后再执行，避免「文件被占用」导致安装失败；装完自动启动新版本",
          en: "The installer now waits until the app has fully exited before it runs, so it cannot fail with «file in use»; the new version starts itself afterwards",
          ja: "インストーラーはアプリが完全に終了してから実行されるため「ファイルが使用中」で失敗しません。完了後に新バージョンを自動で起動します",
          es: "El instalador espera a que la app se cierre por completo antes de ejecutarse, así no falla con «el archivo está en uso»; al terminar se inicia la versión nueva",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "更新时自动选择与当前安装方式匹配的安装包（MSI / NSIS），不会再装出第二份 Kestrel",
          en: "The update now picks the installer that matches how this copy was installed (MSI / NSIS), so it never installs a second Kestrel next to the running one",
          ja: "現在のインストール形態に合ったインストーラー（MSI / NSIS）を選ぶようになり、Kestrel が二重に導入されなくなりました",
          es: "La actualización elige ahora el instalador que corresponde al tipo de instalación actual (MSI / NSIS), de modo que nunca instala un segundo Kestrel junto al que está en marcha",
        },
      },
    ],
  },
  {
    version: "1.2.3",
    date: "2026-10-31",
    title: {
      zh: "QS 栏只在真正的打开/保存对话框中弹出",
      en: "The Quick Switch now appears only in real Open/Save dialogs",
      ja: "QS バーは実際の保存/開くダイアログでのみ表示",
      es: "El Quick Switch ahora aparece solo en los diálogos reales de abrir/guardar",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "修复 QS 栏在 IDM 下载完成提示等非文件对话框下方弹出：此前只凭「有输入框 + 有列表」判断，而 IDM 的主窗口恰好两者都有。现在必须同时具备外壳视图与文件名组合框，才会判定为文件对话框",
          en: "Fixed the Quick Switch popping up under non-file dialogs such as IDM's download notification: the old check only looked for 'an edit box and a list', which IDM's main window happens to have. A dialog now has to carry both the shell view and the file name combo",
          ja: "IDM のダウンロード完了通知など、ファイルダイアログ以外でも QS バーが出ていた問題を修正。「入力欄＋リスト」だけでは不十分で、外壳ビューとファイル名コンボの両方がある時だけファイルダイアログと判定します",
          es: "Se corrigió que el Quick Switch apareci bajo diálogos que no son de archivo, como el aviso de descarga de IDM: la comprobación anterior solo buscaba «un campo de edición y una lista», que la ventana principal de IDM cumple. Ahora hace falta tanto la vista del shell como el combo del nombre de archivo",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "「另存为」对话框同样能被正确识别（它的文件名框不是 ComboBoxEx32，改用普通 ComboBox）",
          en: "The Save-as dialog is recognised too — its file name box is a plain ComboBox, not a ComboBoxEx32",
          ja: "保存ダイアログも正しく認識されます（ファイル名欄が ComboBoxEx32 ではなく通常の ComboBox のため）",
          es: "El diálogo «Guardar como» también se reconoce: su campo de nombre es un ComboBox normal, no un ComboBoxEx32",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "新增对话框识别测试：把真实系统对话框的控件结构固化为样本，防止将来再次误判",
          en: "Added tests that pin the real system dialogs' control layout, so the detection cannot silently regress",
          ja: "実際のシステムダイアログの控件構造をテストとして固定し、判定の退行を防ぎます",
          es: "Se añadieron pruebas que fijan la estructura real de los diálogos del sistema para que la detección no retroceda en silencio",
        },
      },
    ],
  },
  {
    version: "1.2.2",
    date: "2026-10-31",
    title: {
      zh: "更新检查改由系统网络栈发起，镜像测速对准真实下载",
      en: "Update checks now go through the system network stack; mirror tests hit the real download",
      ja: "更新チェックをシステムのネットワーク層経由に変更、ミラー速度テストを実際のダウンロードに",
      es: "Las comprobaciones usan ahora la pila de red del sistema y las pruebas de espejos miden la descarga real",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "彻底修复检查更新与镜像测速失败：请求改由系统网络组件（WinHTTP）直接发出，不再受页面安全策略限制，也不受浏览器忽略代理设置的影响",
          en: "Update checks and mirror tests now run through the system network stack (WinHTTP): they are no longer held back by the page's security policy or by browsers ignoring the proxy settings",
          ja: "更新チェックとミラー速度テストをシステムネットワーク（WinHTTP）経由に変更。ページセキュリティポリシーやブラウザのプロキシ無視の影響を受けません",
          es: "Las comprobaciones y pruebas de espejos pasan por la pila de red del sistema (WinHTTP): ya no dependen de la política de la página ni de que el navegador ignore el proxy",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复页面自带的安全策略未放行外部请求的问题（这是此前更新功能始终不可用的真正原因）",
          en: "Fixed the page's own security policy blocking outbound requests — the real reason the update feature never worked",
          ja: "ページ自带のセキュリティポリシーが外部リクエストを遮断していた問題を修正（更新機能が使えなかった真因）",
          es: "Corregida la política de seguridad de la página que bloqueaba las peticiones salientes: la causa real del fallo",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "镜像测速改为探测真实安装包地址（只取 1KB），并移除已下线的镜像；可用的镜像会显示真实延迟",
          en: "The mirror test now probes the real installer URL (1 KB only) and the dead mirrors are gone; working ones show their true latency",
          ja: "ミラー速度テストを実際のインストーラー URL の一部（1KB）に変更し、終了したミラーを削除。動作するミラーは実際の遅延を表示",
          es: "La prueba de espejos usa ahora la URL real del instalador (1 KB) y se retiraron los espejos caídos; los que funcionan muestran su latencia real",
        },
      },
    ],
  },
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
