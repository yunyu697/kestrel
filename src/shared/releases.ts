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
    version: "1.2.10",
    date: "2026-11-02",
    title: {
      zh: "索引占用的内存少了一半以上",
      en: "The index takes less than half the memory it used to",
      ja: "インデックスのメモリ使用量が半分以下になりました",
      es: "El índice ocupa menos de la mitad de memoria que antes",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "每条索引记录以前把同一份东西存了两遍到三遍：完整路径在记录里存一份、在路径索引表里又存一份；父目录单独存一份，而它本来就是路径的前半段；文件名的小写副本对本来就全小写的文件名也照存不误。三处都去掉了——路径只留一份，两张表共用；父目录改成用到时从路径里现切；小写副本只在真和原名不同时才留。71.5 万个文件，主进程内存从 718 MB 降到 320 MB",
          en: "Every index entry used to store the same thing two or three times: the full path once in the entry and again as the lookup-table key, the parent folder as its own copy even though it is just the first half of the path, and a lowercased name even for names that were already lowercase. All three are gone — the path is stored once and shared by both tables, the parent folder is sliced off the path when needed, and the lowercase copy only exists when it actually differs. For 715k files that takes the main process from 718 MB down to 320 MB",
          ja: "インデックスの各項目が同じ内容を 2〜3 回保存していました。完全なパスを項目内と検索テーブルのキーに二重保存し、パスの前半そのものである親フォルダーを別途保存し、もともと小文字だけの名前にも小文字コピーを持っていました。3 つとも廃止しました。パスは 1 回だけ保存して両方の表で共有し、親フォルダーは必要時にパスから切り出し、小文字コピーは実際に異なる場合のみ保持します。71.5 万ファイルで、メインプロセスのメモリが 718 MB から 320 MB になりました",
          es: "Cada entrada del índice guardaba lo mismo dos o tres veces: la ruta completa una vez en la entrada y otra como clave de la tabla de búsqueda, la carpeta padre como copia propia aunque es solo la primera mitad de la ruta, y un nombre en minúsculas incluso para nombres que ya lo estaban. Las tres se han eliminado: la ruta se guarda una vez y la comparten ambas tablas, la carpeta padre se recorta de la ruta cuando hace falta y la copia en minúsculas solo existe si realmente difiere. Con 715 mil archivos, el proceso principal pasa de 718 MB a 320 MB",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "保存索引以前要把整份索引复制一遍再压缩，中途瞬间多占四百多 MB；读取时又先把解压出来的两百多 MB 整个摊在内存里再解析。现在保存是一边序列化一边压缩、读取是一边解压一边解析，中间不再有大块缓冲",
          en: "Saving the index used to copy the whole thing before compressing it, spiking memory by over 400 MB, and loading first unpacked the entire 200-plus MB payload before parsing it. Saving now compresses while it serialises, and loading parses while it decompresses, so neither step needs a large buffer in between",
          ja: "インデックスの保存時に全体を複製してから圧縮していたため、一時的に 400 MB 以上を余分に使っていました。読み込み時も、解凍した 200 MB 超を丸ごとメモリに広げてから解析していました。現在は保存はシリアライズしながら圧縮し、読み込みは解凍しながら解析するため、途中に大きなバッファを取りません",
          es: "Guardar el índice copiaba todo antes de comprimirlo, con un pico de más de 400 MB, y al cargarlo se descomprimían los más de 200 MB completos antes de analizarlos. Ahora se comprime mientras se serializa y se analiza mientras se descomprime, sin búferes grandes por medio",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "索引文件本身也小了，18.8 MB 变成 16.4 MB。另外升级后第一次启动会重建一次索引（约一分钟），期间搜索会提示正在建立索引；这次格式变了，只此一次",
          en: "The index file itself got smaller too, from 18.8 MB to 16.4 MB. Upgrading also rebuilds the index once on first launch (about a minute), with search showing the indexing hint meanwhile — the format changed this once",
          ja: "インデックス ファイル自体も 18.8 MB から 16.4 MB に小さくなりました。また、アップグレード後の初回起動時に一度だけインデックスを再構築します（約 1 分）。その間は検索にインデックス作成中の表示が出ます。形式が変わったのは今回だけです",
          es: "El archivo del índice también es más pequeño: de 18,8 MB a 16,4 MB. Además, tras actualizar se reconstruye una vez en el primer arranque (alrededor de un minuto), y mientras tanto la búsqueda muestra el aviso de indexación; el formato cambió solo esta vez",
        },
      },
    ],
  },
  {
    version: "1.2.9",
    date: "2026-11-02",
    title: {
      zh: "QS 栏和全局搜索栏可以互相接替，不再互相顶掉",
      en: "Quick Switch and the launcher hand the window back and forth",
      ja: "クイックスイッチとランチャーがウィンドウを譲り合うようになりました",
      es: "Quick Switch y el lanzador se ceden la ventana en lugar de anularse",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "对话框开着 QS 栏时去用全局搜索栏，QS 会话以前会被直接结束，回到对话框就再也回不来。现在两个会话共用一个窗口：搜索栏出现时 QS 只是让位，关掉搜索栏 QS 栏就回到原对话框",
          en: "Using the global search bar while a Quick Switch box was attached used to end the Quick Switch session outright, so returning to the dialog never brought it back. The two sessions now share the window: the launcher only takes it over, and closing the launcher puts the box back under the same dialog",
          ja: "クイックスイッチ表示中にグローバル検索バーを使うとセッションが終了してしまい、ダイアログに戻っても復活しませんでした。2 つのセッションがウィンドウを共有するようになり、検索バーを閉じると元のダイアログに戻ります",
          es: "Usar la barra de búsqueda global con un cuadro de Quick Switch adjunto terminaba la sesión, así que al volver al diálogo ya no reaparecía. Ahora ambas sesiones comparten la ventana: el lanzador solo la ocupa y, al cerrarlo, el cuadro vuelve bajo el mismo diálogo",
        },
      },
      {
        kind: "new",
        text: {
          zh: "7-Zip 这类「只给一个文件夹输入框」的对话框也能吸附 QS 栏了：解压到、复制到、移动到都能直接选目标文件夹。识别靠输入框里装的是文件夹还是文件名，所以「压缩文件名」这种框不会被误判",
          en: "Dialogs that ask for a folder with nothing but a text field are now supported too — 7-Zip's \"extract to\", \"copy to\" and \"move to\" boxes all accept a destination folder. Recognition is based on whether the field holds a folder or a file name, so an \"archive name\" box is never mistaken for one",
          ja: "フォルダーを 1 つの入力欄だけで尋ねるダイアログにも対応しました。7-Zip の「展開先」「コピー先」「移動先」で保存先フォルダーを選べます。判定は入力欄がフォルダーかファイル名かで行うため、「圧縮ファイル名」を誤認しません",
          es: "Los diálogos que piden una carpeta con solo un campo de texto ya se admiten: las cajas «extraer en», «copiar en» y «mover a» de 7-Zip aceptan la carpeta de destino. La detección mira si el campo contiene una carpeta o un nombre de archivo, así que «nombre del archivo» nunca se confunde",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "升级到新版本后仍然提示「检测到新版本」的问题修好了：那条记录存在浏览器本地，以前不看当前版本号，装完新版本也照样显示；现在只要发现它不比正在运行的版本新就直接清掉",
          en: "The \"new version found\" banner no longer survives the upgrade: the record lives in local storage and was never compared against the running build, so installing the new version kept showing it. It is now dropped as soon as it is not newer than what is running",
          ja: "新しいバージョンに更新しても「新しいバージョンがあります」と表示され続ける問題を修正しました。記録はローカル保存で、実行中のバージョンと比較していなかったためです。今は新しくない記録は即座に破棄されます",
          es: "El aviso de «nueva versión encontrada» ya no sobrevive a la actualización: el registro se guardaba localmente y nunca se comparaba con la versión en ejecución. Ahora se descarta en cuanto deja de ser más reciente",
        },
      },
    ],
  },
  {
    version: "1.2.8",
    date: "2026-11-02",
    title: {
      zh: "刚装好的软件立刻就能搜到",
      en: "Programs you just installed are searchable right away",
      ja: "インストールしたばかりのアプリがすぐ検索できます",
      es: "Los programas recién instalados se encuentran al instante",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "应用列表以前只在启动时扫描一次，装完软件必须重启 Kestrel 才搜得到。现在开始菜单和桌面是常驻监听的，安装完成后一两秒内就能搜到，不用重启",
          en: "The app list used to be scanned once at startup, so a program installed afterwards stayed invisible until Kestrel was restarted. The Start Menu and both desktops are now watched continuously: a new shortcut becomes searchable within a second or two, no restart needed",
          ja: "アプリ一覧は起動時に一度だけスキャンされていたため、後からインストールしたアプリは Kestrel を再起動するまで検索できませんでした。スタート メニューとデスクトップを常時監視するようになり、インストールから 1〜2 秒で検索できます",
          es: "La lista de aplicaciones se escaneaba una sola vez al arrancar, así que un programa instalado después no aparecía hasta reiniciar Kestrel. Ahora se vigilan el menú Inicio y ambos escritorios: un acceso nuevo se puede buscar en uno o dos segundos, sin reiniciar",
        },
      },
      {
        kind: "new",
        text: {
          zh: "支持 Microsoft Store 应用：记事本、计算器、设置、终端、画图、照片、截图工具等 30 多个以前完全搜不到的商店应用，现在都能搜到并直接启动，图标也是真实的",
          en: "Microsoft Store apps are supported: Notepad, Calculator, Settings, Terminal, Paint, Photos, Snipping Tool and the other 30-odd Store apps that used to be entirely absent can now be found and launched, with their real icons",
          ja: "Microsoft Store アプリに対応しました。メモ帳、電卓、設定、ターミナル、ペイント、フォト、切り取りツールなど、以前はまったく検索できなかった 30 以上のストア アプリを検索して起動できます。アイコンも本物です",
          es: "Se admiten aplicaciones de Microsoft Store: Notepad, Calculadora, Configuración, Terminal, Paint, Fotos, Recortes y las más de 30 aplicaciones de la Store que antes no aparecían ahora se encuentran y se abren, con sus iconos reales",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "公共桌面（`%PUBLIC%\\Desktop`）以前没有被扫描。只在公共桌面放图标的程序，无论重启多少次都搜不到",
          en: "The all-users desktop (`%PUBLIC%\\Desktop`) was never scanned. A program that only put its shortcut there could not be found no matter how often you restarted",
          ja: "共有デスクトップ（`%PUBLIC%\\Desktop`）がスキャン対象外でした。そこにしかショートカットを置かないアプリは、何度再起動しても検索できませんでした",
          es: "El escritorio de todos los usuarios (`%PUBLIC%\\Desktop`) no se escaneaba. Un programa que solo dejaba ahí su acceso directo no se encontraba por muchas veces que reiniciaras",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "打开搜索栏时，如果距上次扫描已超过 10 秒，会顺手在后台补扫一次——即使文件系统通知漏掉了，也不会出现「明明装了却搜不到」",
          en: "Opening the search box rescans in the background when the list is more than ten seconds old, so even a missed filesystem notification cannot leave an installed program unsearchable",
          ja: "検索バーを開いたとき、前回のスキャンから 10 秒以上経っていればバックグラウンドで再スキャンします。ファイル システム通知を取りこぼしても「入れたのに出てこない」ことはありません",
          es: "Al abrir el buscador se vuelve a escanear en segundo plano si la lista tiene más de diez segundos, de modo que ni siquiera una notificación perdida deja un programa sin encontrar",
        },
      },
    ],
  },
  {
    version: "1.2.7",
    date: "2026-11-01",
    title: {
      zh: "修复更新下载失败：不支持的协议",
      en: "Fixes update downloads failing with an unsupported-protocol error",
      ja: "更新のダウンロードが「サポートされていないプロトコル」で失敗する問題を修正",
      es: "Corrige los fallos de descarga de la actualización por protocolo no admitido",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "默认「官方直连」时下载地址被拼成 `/https://github.com/...`，导致 1.2.0 之后的版本点下载一律失败并提示「不支持的协议」。空前缀现在会被正确理解为「不加速」，而不是一个 `/`",
          en: "With the default «direct» route the download URL was built as `/https://github.com/...`, so every download since 1.2.0 failed with an unsupported-protocol error. An empty prefix now means «no mirror» instead of turning into a slash",
          ja: "既定の「公式直結」ではダウンロード URL が `/https://github.com/...` になってしまい、1.2.0 以降はダウンロードが必ず「サポートされていないプロトコル」で失敗していました。空のプレフィックスは `/` ではなく「ミラーなし」として扱われます",
          es: "Con la ruta «directa» predeterminada la URL se construía como `/https://github.com/...`, así que toda descarga desde 1.2.0 fallaba con un error de protocolo no admitido. Un prefijo vacío ahora significa «sin espejo» en lugar de convertirse en una barra",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "自定义镜像如果只填了域名（缺 `https://`）或填了不合法的内容，不再拼出非法地址，而是自动补全或回退到官方直连",
          en: "A custom mirror typed without `https://`, or with something that is not a URL at all, no longer produces an unusable address: the scheme is filled in, or the download falls back to the direct route",
          ja: "カスタム ミラーに `https://` のないドメインや URL でない文字列を入力しても、不正なアドレスを作らず、スキームを補完するか公式直結に戻ります",
          es: "Un espejo personalizado escrito sin `https://`, o con algo que no es una URL, ya no genera una dirección inservible: se completa el esquema o se vuelve a la ruta directa",
        },
      },
    ],
  },
  {
    version: "1.2.6",
    date: "2026-11-01",
    title: {
      zh: "同一时间只允许运行一个实例",
      en: "Only one copy runs at a time",
      ja: "同時に起動できるのは 1 つだけ",
      es: "Solo se ejecuta una copia a la vez",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "同时启动多个副本时可能留下两个实例。守卫现在在任何窗口创建之前就获取，抢不到的副本直接退出——不会再有 WebView2 配置冲突导致的崩溃弹窗",
          en: "Launching several copies at once could leave two instances running. The guard is now taken before any window exists, so the copy that loses simply exits — no more crash dialog from a WebView2 profile conflict",
          ja: "同時に複数のコピーを起動すると 2 つのインスタンスが残ることがありました。ガードはウィンドウを作る前に取得するようになり、取れなかったコピーはそのまま終了します。WebView2 のプロファイル競合によるクラッシュ ダイアログはもう出ません",
          es: "Lanzar varias copias a la vez podía dejar dos instancias en marcha. La guarda se toma ahora antes de crear ninguna ventana, así que la copia que pierde simplemente sale: se acabó el diálogo de error por conflicto de perfil de WebView2",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "已经开着的时候再点一次图标，会正常弹出搜索栏。以前在启动的头一两秒内点，是完全没有反应的",
          en: "Clicking the icon again while it is already open now brings up the search box. Before, clicking during the first second or two of startup did nothing at all",
          ja: "起動済みのときにもう一度アイコンを押すと、検索バーがちゃんと表示されます。以前は起動直後の 1〜2 秒に押しても何も起きませんでした",
          es: "Volver a pulsar el icono con la app ya abierta ahora muestra el buscador. Antes, pulsar durante el primer segundo o dos del arranque no hacía absolutamente nada",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "`--no-single-instance` 不再是通用绕过开关，只对「以管理员身份重启」这一次交接有效",
          en: "`--no-single-instance` is no longer a general bypass; it only applies to the hand-off when restarting as administrator",
          ja: "`--no-single-instance` は汎用のバイパスではなくなり、「管理者として再起動」の引き継ぎ時だけ有効になります",
          es: "`--no-single-instance` ya no es un atajo universal: solo se aplica al traspaso al reiniciar como administrador",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "单实例按登录会话隔离：同一台电脑上两个用户各自登录，可以各开一份",
          en: "The lock is per sign-in session, so two users logged in on the same PC can each run their own copy",
          ja: "単一インスタンスはログオン セッション単位です。同じ PC に 2 人がログオンしていれば、それぞれ 1 つずつ起動できます",
          es: "El bloqueo es por sesión de inicio, así que dos usuarios con sesión abierta en el mismo PC pueden tener cada uno su copia",
        },
      },
    ],
  },
  {
    version: "1.2.5",
    date: "2026-11-01",
    title: {
      zh: "QS 栏进入应用自己的选文件夹对话框，并在文件夹树里直接跳转",
      en: "The Quick Switch now works in an app's own folder dialog, jumping inside its folder tree",
      ja: "QS バーがアプリ独自のフォルダー選択ダイアログでも使え、フォルダーツリー内を直接移動します",
      es: "El Quick Switch ya funciona en el diálogo de carpetas de otras apps y salta dentro de su árbol de carpetas",
    },
    items: [
      {
        kind: "new",
        text: {
          zh: "WinRAR「解压路径和选项」这类对话框现在也会弹出 QS 栏。选中文件夹后是**在树里选中那个节点**，路径框由对话框自己同步，不再往输入框里填字符串",
          en: "Dialogs like WinRAR's «extract path and options» get the Quick Switch too. Picking a folder now selects that node in the tree and the dialog syncs its own path field, instead of a string being pasted into an input box",
          ja: "WinRAR の「解凍先とオプション」のようなダイアログでも QS バーが表示されます。フォルダーを選ぶとツリー内のそのノードが選択され、パス欄はダイアログ自身が同期します。入力欄に文字列を流し込むことはもうありません",
          es: "Diálogos como «ruta y opciones de extracción» de WinRAR ya muestran el Quick Switch. Al elegir una carpeta se selecciona ese nodo en el árbol y el diálogo sincroniza su propio campo de ruta, en lugar de pegar una cadena en una caja de texto",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "树里显示的是系统本地化名称（`Documents` 显示为「文档」、`D:\\` 显示为「Data (D:)」），跳转会先向系统查询该文件夹的显示名再匹配，因此中文系统下也能正确定位",
          en: "Trees show the shell's localised names (Documents appears as 「文档」, D:\\ as 「Data (D:)」), so the jump asks the shell for the folder's display name before matching — it lands correctly on a Chinese system too",
          ja: "ツリーには OS のローカライズ名が表示される（Documents は「文档」、D:\\ は「Data (D:)」）ため、移動時にまずシステムへ表示名を問い合わせてから照合します。中国語環境でも正しく到達します",
          es: "Los árboles muestran los nombres localizados del shell (Documents aparece como 「文档」, D:\\ como 「Data (D:)」), así que el salto pregunta primero al sistema por el nombre visible de la carpeta; también acierta en un sistema en chino",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "打开/保存对话框的跳转更快：不再需要按 Ctrl+L，地址栏不会闪现，也不抢焦点，实测耗时从约 520 ms 降到约 270 ms",
          en: "Jumps in Open/Save dialogs are faster: no Ctrl+L, no address bar flashing, no focus stealing — measured at roughly 520 ms down to 270 ms",
          ja: "開く／保存ダイアログへの移動が速くなりました。Ctrl+L 不要、アドレスバーも表示されず、フォーカスも奪いません。実測で約 520 ms から約 270 ms に短縮",
          es: "Los saltos en los diálogos de abrir/guardar son más rápidos: sin Ctrl+L, sin que parpadee la barra de direcciones y sin robar el foco; medido, de unos 520 ms a unos 270 ms",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复在自动跳转途中关闭对话框时，Ctrl+L 被发送到应用自己的主窗口（例如 Notepad3 的编辑区）的问题",
          en: "Fixed Ctrl+L being sent into the application's own main window (Notepad3's editor, for instance) when the dialog was closed mid-jump",
          ja: "自動移動の途中でダイアログを閉じると、Ctrl+L がアプリ自身のメインウィンドウ（Notepad3 の編集領域など）へ送られてしまう問題を修正",
          es: "Se corrigió que Ctrl+L se enviara a la ventana principal de la propia aplicación (el editor de Notepad3, por ejemplo) al cerrar el diálogo en mitad del salto",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复「当前目录」读取总是失败的问题：此前取到的是导航按钮所在的工具栏，导致「已经在该目录」的判断失效、每次都会重复跳转",
          en: "Fixed reading the current folder always failing: it picked up the navigation-buttons toolbar, so «already in that folder» never triggered and every jump ran again",
          ja: "「現在のフォルダー」の取得が常に失敗していた問題を修正。ナビゲーションボタンのツールバーを読んでいたため「すでにそのフォルダーにいる」判定が効かず、毎回移動し直していました",
          es: "Se corrigió que la lectura de la carpeta actual fallara siempre: tomaba la barra de botones de navegación, así que «ya estás en esa carpeta» nunca se detectaba y el salto se repetía cada vez",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "对话框识别更精确：路径框必须是一个**可编辑**的组合框（内部含 Edit 子窗口），因此 IDM 主窗口这类只有输入框加列表的窗口仍不会被误判",
          en: "Dialog detection is tighter: the path field has to be an **editable** combo (one with an Edit child window), so windows like IDM's main one — an edit box plus a list — stay rejected",
          ja: "ダイアログ判定をより厳密にしました。パス欄は **編集可能** なコンボ（内部に Edit 子ウィンドウを持つ）である必要があるため、IDM のメインウィンドウのような「入力欄＋リスト」だけの画面は引き続き除外されます",
          es: "La detección de diálogos es más estricta: el campo de ruta tiene que ser un combo **editable** (con una ventana Edit hija), así que ventanas como la principal de IDM —una caja de texto y una lista— siguen descartándose",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复 QS 栏被对话框盖住：此前搜索栏在置顶层、对话框在普通层，对话框会挡住搜索栏。现在两者同层，搜索栏紧贴在对话框上方一格",
          en: "Fixed the Quick Switch being covered by the dialog: the box sat in the topmost band while the dialog did not, so the dialog could hide it. Both are in the normal band now, with the box one slot above the dialog",
          ja: "QS バーがダイアログに隠れる問題を修正。検索バーは最前面、ダイアログは通常層にいたため隠れていました。現在は同じ層で、検索バーがダイアログのすぐ上に位置します",
          es: "Se corrigió que el diálogo tapara el Quick Switch: la caja estaba en la banda superior y el diálogo no, así que podía ocultarla. Ahora ambos están en la banda normal y la caja queda justo encima del diálogo",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复首次建立索引时进度一直显示 0、百分比恒为 100%、取消重建会把旧索引换成残缺索引、启动后底部显示「0 个文件」这几个问题",
          en: "Fixed the index showing 0 while the first scan ran, the percentage stuck at 100%, cancelling a rebuild replacing the old index with a partial one, and the footer reading «0 files» after startup",
          ja: "初回インデックス作成中に 0 のまま、進捗率が常に 100%、再構築のキャンセルで不完全なインデックスに置き換わる、起動後に「0 個のファイル」と表示される問題を修正",
          es: "Se corrigió que el índice mostrara 0 durante el primer escaneo, que el porcentaje se quedara en 100 %, que cancelar una reconstrucción sustituyera el índice anterior por uno parcial y que el pie mostrara «0 archivos» al arrancar",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "重建索引前会先确认；安装系统服务需要管理员权限，未提权时会明确提示，而不是静默失败",
          en: "Rebuilding the index now asks for confirmation first, and installing the background service says plainly that administrator rights are required instead of failing silently",
          ja: "インデックス再構築の前に確認を表示します。バックグラウンドサービスは管理者権限が必要で、権限がない場合は黙って失敗せず明示します",
          es: "Reconstruir el índice pide confirmación primero, y la instalación del servicio en segundo plano avisa de que hacen falta permisos de administrador en lugar de fallar en silencio",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复应用内安装时路径含正斜杠导致 Windows Installer 报 1619 的问题",
          en: "Fixed the in-app installer failing with Windows Installer error 1619 when the path contained forward slashes",
          ja: "アプリ内インストールでパスにスラッシュが含まれると Windows Installer が 1619 で失敗する問題を修正",
          es: "Se corrigió que el instalador interno fallara con el error 1619 de Windows Installer cuando la ruta contenía barras inclinadas",
        },
      },
    ],
  },
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
