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
    version: "1.2.22",
    date: "2026-11-17",
    title: {
      zh: "更新不再可能装重一份，索引升级不再空窗",
      en: "Updates can no longer duplicate the install; index upgrades no longer blank search",
      ja: "更新で二重インストールする余地をなくし、索引更新時の空振りを解消",
      es: "Las actualizaciones ya no duplican la instalación ni vacían el índice",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "修复应用内更新可能装成第二份的问题：判断「这份是用哪种方式装的」时，如果拿不到结果，原先会默认按机器级安装处理，于是给当前用户安装的副本递上 MSI，在 Program Files 里又装一份，而不是替换原来那份。现在三种情况（设置页、后台检查、程序自身）都改为「未知就是未知」，不再猜；拿不到安装方式时只提供「打开下载页面」，由你自己选对应的安装包",
          en: "Fixed in-app updates sometimes installing a second copy. When the app could not determine how this copy was installed, it fell back to treating it as a machine-wide install and handed the MSI to a per-user copy — which lays down a second Kestrel under Program Files instead of replacing the original. All three places that decide this (settings pane, background check, the app itself) now answer \"unknown\" instead of guessing, and an unknown mode offers the release page so you pick the matching installer yourself",
          ja: "アプリ内更新が二重インストールになることがある問題を修正しました。このコピーがどの方式でインストールされたかを判定できなかった場合、従来はマシン単位のインストールとみなし、ユーザー単位のコピーに MSI を渡していました。その結果 Program Files にもう一つ入るだけで、元のコピーは置き換わりません。判定する三か所（設定画面・バックグラウンド確認・アプリ本体）はすべて「不明は不明」と返すようになり、不明なときはダウンロードページを案内するので、適切なインストーラーを自分で選べます",
          es: "Corregido que la actualización pudiera instalar una segunda copia. Cuando no se podía determinar cómo estaba instalada esta copia, se asumía instalación para todo el equipo y se entregaba el MSI a una copia por usuario, lo que coloca un segundo Kestrel en Program Files en lugar de reemplazar el original. Los tres puntos que lo deciden (panel de ajustes, comprobación en segundo plano y la propia app) ahora responden «desconocido» en vez de adivinar, y en ese caso se ofrece la página de descarga para que elijas el instalador correcto",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "索引格式升级后不再出现「搜索空白几分钟」：以前版本号一变就先把旧的索引文件删掉再全量重建，重建期间搜不到任何东西（本机实测六分钟），中途取消或失败还会一份索引都不剩、下次启动重新扫一遍。现在旧文件保留到新索引写成为止——保存本来就是「先写临时文件再原子替换」，成功才退役旧文件；写入失败也会记日志，不再静默",
          en: "A format upgrade no longer leaves search blank for minutes. A version bump used to delete the old index first and then rebuild from scratch, so nothing was searchable in the meantime (six minutes here), and a cancelled or failed rebuild left no index at all — every later start paying the full scan again. The old file now stays until the new one lands: saving already writes a temp file and renames it into place, so a successful rebuild retires the stale index atomically. A failed save is logged instead of swallowed",
          ja: "索引形式の更新後に「検索が数分空になる」現象を解消しました。従来はバージョンが変わるとまず旧索引ファイルを削除してから全件を再構築するため、その間は何も検索できず（本機では 6 分）、途中で中止や失敗をすると索引が一つも残らず、次回起動でまた全走査になっていました。旧ファイルは新しい索引が書き上がるまで残すようにし、保存は従来どおり一時ファイル＋アトミックな置換なので、成功したときだけ旧索引が退役します。保存失敗も記録するようになりました",
          es: "Una actualización del formato ya no deja la búsqueda vacía durante minutos. Un cambio de versión borraba primero el índice antiguo y reconstruía desde cero, así que no se podía buscar nada mientras tanto (seis minutos en este equipo), y una reconstrucción cancelada o fallida dejaba sin índice alguno, obligando a repetir el escaneo completo en cada arranque. Ahora el archivo antiguo se conserva hasta que llega el nuevo: el guardado ya escribe un temporal y lo renombra, de modo que una reconstrucción correcta retira el índice obsoleto de forma atómica. Un guardado fallido se registra en lugar de ignorarse",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "热键挂掉后自愈更快：守护线程原先要等三十分钟无输入才判定快捷键失效并重建，现在改为两分钟。这一判定只在用户确实在打字时才生效，所以不会误伤空闲状态；日志里那次「stalled 1800s」意味着快捷键实打实死了半小时",
          en: "A dead hotkey now recovers in two minutes instead of thirty. The watchdog used to wait half an hour of silence before rebuilding the keyboard hook; it now acts after two minutes. The check only runs while you are actually typing, so idleness cannot trigger it — the \"stalled 1800s\" line in the log meant the hotkey really was dead for half an hour",
          ja: "ホットキーが死んだあとの復帰を速くしました。監視スレッドは従来 30 分間入力がないと判断してからフックを再構築していましたが、2 分で判断するようになりました。この判定は実際に入力している間だけ働くため、アイドル状態で誤作動しません。ログの「stalled 1800s」は、ホットキーが本当に 30 分使えなかったことを意味していました",
          es: "Un atajo que deja de responder se recupera en dos minutos en lugar de treinta. El vigilante esperaba media hora de silencio antes de reconstruir el hook del teclado; ahora actúa a los dos minutos. La comprobación solo corre mientras escribes de verdad, así que la inactividad no la dispara: la línea «stalled 1800s» del registro significaba que el atajo estuvo muerto media hora",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "在打开/保存对话框里吸附搜索栏更可靠：对话框弹出得慢时，程序原本只等八秒就放弃，之后必须再点一次对话框才生效；现在等三十秒。Windows 每次激活只发一次前台事件，所以早放弃就等于白等。放弃时日志会记下对话框是否可见和它的窗口类名，下次出问题能直接定位，不用靠猜",
          en: "Attaching the search bar to open/save dialogs is more reliable. When a dialog painted slowly, the app gave up after eight seconds and the box only appeared after you clicked the dialog again; it now waits thirty seconds. Windows sends one foreground event per activation, so giving up early wastes the only signal there is. The give-up line now records whether the dialog was visible and its window class, so the next report can be diagnosed from the log instead of guessed at",
          ja: "開く／保存ダイアログへの吸着がより確実になりました。ダイアログの描画が遅いと 8 秒で諦めてしまい、もう一度ダイアログをクリックしないと表示されませんでしたが、30 秒待つようになりました。Windows はアクティブ化ごとに前面イベントを一度しか送らないため、早く諦めることは唯一の合図を捨てるのと同じです。諦めた際はダイアログが見えていたかとウィンドウクラスを記録するので、次回はログから原因を追えます",
          es: "Adjuntar la barra de búsqueda a los diálogos de abrir/guardar es más fiable. Si un diálogo tardaba en pintarse, la app se rendía a los ocho segundos y la caja solo aparecía tras volver a pulsar el diálogo; ahora espera treinta. Windows envía un único evento de primer plano por activación, así que rendirse pronto descarta la única señal disponible. Al rendirse se registra si el diálogo era visible y su clase de ventana, para poder diagnosticarlo desde el registro",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "顺手清理了旧版本留在程序目录里的退出标志文件（早期版本把它写在程序旁边，装在 Program Files 时删不掉也写不进）。它已经没人读了，但看上去像是还在生效，容易误导排查。清理需要管理员权限，所以只在真的删掉时记一行日志",
          en: "The quit flag file that older builds left next to the executable is now cleaned up. Early versions wrote it there, which is exactly the directory an installed copy cannot write to. Nothing reads it any more, but it looks live and misleads anyone debugging a shutdown problem. Removing it needs elevation, so the cleanup logs only when it actually succeeds",
          ja: "旧バージョンが実行ファイルの隣に残した終了フラグを掃除するようにしました。初期の実装はそこに書いており、Program Files にインストールすると書き込めない場所でもあります。現在は誰も読んでいませんが、有効に見えるため終了まわりの調査を誤らせます。削除には管理者権限が要るので、実際に消せたときだけ記録します",
          es: "Ahora se limpia el archivo de bandera de salida que las versiones antiguas dejaban junto al ejecutable. Las primeras versiones lo escribían ahí, justo el directorio donde una copia instalada no puede escribir. Ya nadie lo lee, pero parece activo y confunde al depurar problemas de cierre. Borrarlo requiere elevación, así que solo se registra cuando se consigue",
        },
      },
    ],
  },
  {
    version: "1.2.21",
    date: "2026-11-17",
    title: {
      zh: "配色按色相重排，小字对比度全面达标",
      en: "Colour schemes ordered by hue; small-text contrast brought up to spec",
      ja: "配色を色相順に並べ替え、小さい文字のコントラストを全体的に改善",
      es: "Paletas ordenadas por tono y contraste del texto pequeño corregido",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "设置页的配色方案改为按色相排列：从应用主色蓝（雾蓝）起，沿 OKLCH 色相角顺时针走一圈——雾蓝、靛紫、藤紫、玫紫、樱粉、玫瑰、赤陶、暖砂、秋香、青柠、薄荷、青瓷、松石、天青。历史上「先七个老方案、再八个新方案」的堆放顺序取消，选择器现在读起来是一条连续的光谱；「跟随主题」固定第一（默认值，不属于任何色相），无彩的石墨固定最后",
          en: "The scheme picker is now ordered by hue: starting from the app's own blue (Mist) and walking the OKLCH wheel clockwise — Mist, Indigo, Violet, Magenta, Blush, Rose, Terracotta, Amber, Lime, Mint, Celadon, Turquoise, Azure. The historical pile-up of seven legacy presets followed by eight newer ones is gone, so the grid reads as one continuous spectrum. Theme-following stays first because it is the default and has no hue, and achromatic Graphite stays last",
          ja: "設定画面の配色を色相順に並べ替えました。アプリの基調色である青（ミスト）から OKLCH 色相環を時計回りに一周します——ミスト、インディゴ、バイオレット、マゼンタ、ブラッシュ、ローズ、テラコッタ、アンバー、ライム、ミント、セラドン、ターコイズ、アジュール。従来の「旧 7 件套＋新 8 件套」という並びを廃止し、選択画面が連続したスペクトルとして読めるようになりました。「テーマに従う」は既定値で色相を持たないため先頭、無彩色のグラファイトは末尾です",
          es: "El selector de paletas ahora se ordena por tono: desde el azul propio de la app (Niebla) recorriendo la rueda OKLCH en sentido horario — Niebla, Índigo, Violeta, Magenta, Rosado, Rosa, Terracota, Ámbar, Lima, Menta, Celadón, Turquesa, Azur. Se elimina el antiguo amontonamiento de siete presets heredados seguidos de ocho nuevos, de modo que la rejilla se lee como un espectro continuo. «Seguir el tema» queda primero por ser el valor por defecto y no tener tono, y el Grafito acromático queda al final",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复选中行、键帽、徽标等位置的辅助文字对比度不足：原先只按窗口底色、卡片、面板三个背景校验，而 11–12px 的辅助文字真正被读的地方往往是选中行（强调色淡底叠加窗口底色）、悬停行和键帽，实测在选中行上只有 3.85:1，低于小字所需的 4.5:1。现在校验覆盖全部八种背景加强调色淡底共九处，并逐 token 复核，16 套方案在明暗两态下的不达标项已全部修掉",
          en: "Fixed insufficient contrast for secondary text on selected rows, keycaps and badges: contrast used to be checked only against the window background, cards and panels, but 11–12px captions are actually read on selected rows (accent tint over the window background), hover fills and keycaps — where they measured as low as 3.85:1, under the 4.5:1 small text requires. Every token is now verified against all eight surfaces plus the accent tint, and all sixteen presets pass in both light and dark mode",
          ja: "選択行・キーキャップ・バッジ上の補助文字のコントラスト不足を修正しました。従来はウィンドウ背景・カード・パネルの 3 か所だけを検証していましたが、11〜12px の補助文字が実際に読まれるのは選択行（ウィンドウ背景に強調色の淡い塗りを重ねたもの）、ホバー行、キーキャップであり、実測では選択行で 3.85:1 と小さい文字に必要な 4.5:1 を下回っていました。検証対象を 8 種類の背景＋強調色の淡い塗りの計 9 か所に広げ、16 件套すべてが明暗両方で基準を満たすようになりました",
          es: "Corregido el contraste insuficiente del texto secundario en filas seleccionadas, teclas y distintivos: antes solo se comprobaba contra el fondo de la ventana, las tarjetas y los paneles, pero los textos de 11–12 px se leen sobre todo en filas seleccionadas (tinte de acento sobre el fondo), filas con hover y teclas, donde medían 3,85:1, por debajo del 4,5:1 exigido al texto pequeño. Ahora cada token se verifica contra las ocho superficies más el tinte de acento, y las dieciséis paletas cumplen en modo claro y oscuro",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "强调色当文字时改用单独的色阶：原先选中标签、命中高亮、徽标这 21 处直接拿强调色当文字色，而强调色的下限是为填充和指示器定的 3:1，导致十六套方案里有十三套的 11px 强调色文字只有 3.05–4.49:1。现在填充仍用原色保留饱和度，文字改用新增的 --accent-text，同色相沿明度轴推到 4.5:1",
          en: "Accent-coloured text now uses its own step: selected labels, match highlights and badges — 21 places — drew straight from the accent, whose floor is the 3:1 set for fills and indicators, leaving 11px accent text at 3.05–4.49:1 in thirteen of the sixteen presets. Fills keep the original colour for saturation, while text uses a new --accent-text token pushed along the lightness axis to 4.5:1 in the same hue",
          ja: "強調色を文字として使う箇所には専用の色段を用意しました。選択中ラベル、一致ハイライト、バッジの 21 か所は強調色をそのまま文字色にしていましたが、強調色の下限は塗りとインジケーター用の 3:1 であるため、16 件套のうち 13 件套で 11px の強調色文字が 3.05〜4.49:1 にとどまっていました。塗りは彩度を保つため元の色のまま、文字は新しい --accent-text を使い、同じ色相のまま明度方向に 4.5:1 まで引き上げています",
          es: "El texto en color de acento usa ahora su propio escalón: las etiquetas seleccionadas, los resaltados de coincidencia y los distintivos —21 sitios— tomaban el acento directamente, cuyo mínimo es el 3:1 fijado para rellenos e indicadores, dejando el texto de acento de 11 px en 3,05–4,49:1 en trece de las dieciséis paletas. Los rellenos conservan el color original por saturación y el texto usa el nuevo token --accent-text, desplazado en luminosidad hasta 4,5:1 dentro del mismo tono",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "「跟随主题」这一套的色值是手写的，不走配色生成管线，因此长期存在与设置页预览色卡不一致、且对比度偏低的风险（本次核出 6 处）。现在亮色的次要文字与辅助文字、暗色的正文，以及两态的强调色文字都重新取值，并加了逐 token 比对断言：改一处而忘另一处会让测试直接失败",
          en: "Theme-following is hand-written rather than generated, so its values had drifted from the swatch shown in settings and sat below target in six places. Its light-mode secondary and caption text, dark-mode body text and both modes' accent text have been re-derived, and a token-by-token assertion now fails the build if one copy is edited without the other",
          ja: "「テーマに従う」の色値は生成パイプラインを通らない手書きのため、設定画面の見本との不一致やコントラスト不足（今回は 6 か所）が起きやすい状態でした。明色の副次文字・補助文字、暗色の本文、両モードの強調色文字を取り直し、トークン単位の比較アサーションを追加しました。片方だけ直すとテストが失敗します",
          es: "«Seguir el tema» está escrito a mano y no pasa por el generador, por lo que sus valores se habían desviado de la muestra de ajustes y quedaban por debajo del objetivo en seis puntos. Se recalcularon el texto secundario y las leyendas en modo claro, el cuerpo en modo oscuro y el texto de acento en ambos, y una comprobación token por token ahora falla si se edita una copia sin la otra",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "配色测试从 1020 条断言扩到 1276 条：逐 token 在所有背景上复核、断言强调色文字下限，并校验选择器的色相走位是单调的，方案顺序不会在后续改动中被无声打乱",
          en: "The palette suite grows from 1020 to 1276 assertions: every token is re-checked on every surface, the accent-text floor is asserted, and the picker's hue walk is verified to be monotonic so the ordering cannot silently drift",
          ja: "配色テストを 1020 件から 1276 件のアサーションに拡張しました。全トークンを全背景で再検証し、強調色文字の下限を断言し、選択画面の色相の並びが単調であることを検証するため、順序が黙って崩れることはありません",
          es: "La batería de paletas pasa de 1020 a 1276 aserciones: cada token se revisa en cada superficie, se comprueba el mínimo del texto de acento y se verifica que el recorrido de tonos del selector sea monótono, de modo que el orden no pueda desviarse en silencio",
        },
      },
    ],
  },
  {
    version: "1.2.20",
    date: "2026-11-17",
    title: {
      zh: "修复「安装为服务」关掉又自启，配色精简为 15 套同色系",
      en: "Fix the watchdog respawning after quit; streamline to 15 single-hue schemes",
      ja: "「サービスとしてインストール」の再起動ループを修正、配色を同色系 15 件套に整理",
      es: "Corregido el reinicio tras salir; paletas reducidas a 15 esquemas monocromos",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "修复「安装为服务」正常退出后又被自动拉起的问题：守护进程的退出/停止标志原先写在程序目录，安装版位于 Program Files 时普通权限无法写入且失败被静默忽略，导致每次主动退出后又被重启。标志文件改到用户可写的 %LOCALAPPDATA%\\Kestrel 目录，并改为临时文件加原子重命名、写入失败会记入日志",
          en: "Fixed the watchdog respawning the app after a deliberate quit: its quit/stop flag files used to live next to the executable, which an unelevated process cannot write when installed under Program Files — and the failure was silently ignored, so every clean exit was undone. Flags now live in the user-writable %LOCALAPPDATA%\\Kestrel directory, are written atomically via a temp file plus rename, and log any failure",
          ja: "「サービスとしてインストール」で正常に終了しても自動再起動される問題を修正しました。終了／停止フラグは以前実行ファイルの隣に置かれており、Program Files にインストールすると通常権限では書き込めず、その失敗も黙って無視されていたため、終了するたびに再起動されていました。フラグは書き込み可能な %LOCALAPPDATA%\\Kestrel に移し、一時ファイル＋アトミックなリネームで書き込み、失敗時はログに記録するようになりました",
          es: "Corregido el reinicio automático tras una salida normal en «Instalar como servicio». Los archivos de bandera de salida/parada se escribían junto al ejecutable; con una instalación en Program Files un proceso sin privilegios no puede escribir allí y el error se ignoraba en silencio, por lo que cada salida se deshacía. Ahora están en %LOCALAPPDATA%\\Kestrel, que sí es escribible, se escriben de forma atómica con un archivo temporal y se registra cualquier fallo",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "配色方案精简为 15 套同色系加 1 套跟随主题：移除「港湾／极光／林间」三套多色系方案，新增赤陶、秋香、青柠、松石、天青、靛紫、玫紫、樱粉八套，各方案色相互相错开、不与原有方案重复。已选用旧多色系方案的配置会按强调色自动迁移到最接近的同色系方案，且三个旧方案分别落到不同新方案，不会都变成同一个颜色",
          en: "Colour schemes streamlined to 15 single-hue presets plus one theme-following default: the three multi-hue schemes (Harbour / Aurora / Grove) were removed and eight new single-hue ones added — Terracotta, Amber, Lime, Turquoise, Azure, Indigo, Magenta and Blush — each spaced so no two presets share a hue. Configs using a removed scheme migrate by accent colour to the nearest single-hue preset, and the three old schemes land on three different presets rather than collapsing into one",
          ja: "配色を同色系 15 件套と「テーマに従う」1 件套に整理しました。多色系の「ハーバー／オーロラ／森」の 3 件套を削除し、テラコッタ、アンバー、ライム、ターコイズ、アジュール、インディゴ、マゼンタ、ブラッシュの 8 件套を追加。各色相は重ならないよう配置されています。削除された方案を使っていた設定は強調色から最も近い同色系へ自動移動し、3 つの旧方案は別々の新方案へ移動するため同じ色に集中しません",
          es: "Las paletas se reducen a 15 esquemas monocromos más uno que sigue el tema: se eliminaron los tres esquemas multicolor (Puerto / Aurora / Bosque) y se añadieron ocho monocromos — Terracota, Ámbar, Lima, Turquesa, Azur, Índigo, Magenta y Rosado — con tonos separados para que ningún par comparta color. Las configuraciones que usaban un esquema eliminado migran por color de acento al monocromo más cercano, y los tres esquemas antiguos recaen en tres presets distintos en lugar de unificarse",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "「安装为服务」开启后的描述明确为「关闭开关即卸载服务」",
          en: "Clarified the enabled-state hint to say turning the switch off uninstalls the service",
          ja: "有効時の説明を「オフにするとサービスを削除します」に明確化",
          es: "Aclarada la descripción al activar: «desactívalo para desinstalar el servicio»",
        },
      },
    ],
  },
  {
    version: "1.2.19",
    date: "2026-11-17",
    title: {
      zh: "收藏收成一条，结果按类型分组，界面统一标尺",
      en: "Favorites on one row, grouped results, one type scale everywhere",
      ja: "お気に入りを 1 行に、結果は種類ごとに、UI はひとつの基準へ",
      es: "Favoritos en una fila, resultados agrupados y una sola escala",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "空搜索时的收藏从纵向列表改成一行可横向滚动的图标卡片，高度从约 368px 压到约 80px，最近与历史不用再往下滑就能看到；键盘 ↑↓ 与 Enter 只作用于下方列表，收藏卡片单击即打开，右键菜单照旧",
          en: "With an empty search, favorites are now a single horizontally scrolling row of icon cards instead of a vertical list — roughly 368 px of height becomes about 80 px, so recents and history are visible without scrolling. The arrow keys and Enter act on the list below only; a favorite card opens with a single click and keeps its context menu",
          ja: "空の検索時、お気に入りは縦のリストから横スクロールする 1 行のカードになり、高さは約 368px から約 80px に縮みました。最近使った項目と履歴までスクロールせずに見えます。↑↓ と Enter は下のリストだけを操作し、カードはクリックで開き、右クリックメニューも従来どおりです",
          es: "Con la búsqueda vacía, los favoritos pasan de una lista vertical a una sola fila de tarjetas con desplazamiento horizontal: de unos 368 px a unos 80 px de alto, así que los recientes y el historial se ven sin bajar. Las flechas y Enter actúan solo sobre la lista inferior; una tarjeta se abre con un clic y conserva su menú contextual",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "结果顺序改为稳定分组：已安装应用 → 收藏 → 打开过的 → 其它，组内仍按匹配度排序。以前是混合打分，一个用过很多次、与查询同名的文件可以压过已安装应用；现在应用始终在最前，同组里匹配更强的仍排在前面",
          en: "Results are now grouped in a stable order: installed apps, then favorites, then anything opened before, then everything else — strongest match first inside each group. The old scoring was one blended number, so a frequently used file with the same name could outrank an installed app; apps now always lead, and better matches still lead within a group",
          ja: "検索結果は「インストール済みアプリ → お気に入り → 過去に開いたもの → その他」という安定したグループ順になり、グループ内は一致度の順です。以前は単一の合成スコアだったため、同じ名前でよく使うファイルがインストール済みアプリより上に来ることがありました。現在はアプリが常に先頭で、グループ内では一致度の高いものが先に並びます",
          es: "Los resultados se agrupan en un orden estable: aplicaciones instaladas, luego favoritos, luego lo abierto antes y por último el resto; dentro de cada grupo manda la coincidencia. Antes había una sola puntuación combinada, así que un archivo con el mismo nombre y mucho uso podía superar a una aplicación instalada; ahora las aplicaciones van siempre primero y, dentro del grupo, gana la mejor coincidencia",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复搜索栏配色不协调：启动器的卡片底、页脚、徽标、键帽与边框以前各用一种色层，多色系方案下同屏出现四五种色相；现在结构件统一走中性色（都由文字色派生），色相只留在强调色和选中态，11 套配色方案下都不再花",
          en: "Fixed the mismatched launcher colours: card backgrounds, the footer band, badges, keycaps and borders each used a different tinted layer, so colour-scheme presets put four or five hues on one 780×540 screen. Structural surfaces are now neutral and derived from the text colour, with hue reserved for the accent and selected states, so all eleven presets stay coherent",
          ja: "ランチャーの配色の不整合を修正しました。カード背景、フッター、バッジ、キーキャップ、枠線がそれぞれ別の色層を使っていたため、多色系のプリセットでは 780×540 の画面に 4〜5 色の色相が同居していました。構造面は文字色から派生した中性色に統一し、色相はアクセントと選択状態だけに残したので、11 種のプリセットすべてでまとまります",
          es: "Corregida la incoherencia de color del lanzador: los fondos de tarjeta, la banda inferior, las insignias, las teclas y los bordes usaban cada uno una capa tintada distinta, así que los esquemas multicolor ponían cuatro o cinco tonos en una pantalla de 780×540. Las superficies estructurales ahora son neutras y derivan del color de texto, con el tono reservado al acento y a los estados seleccionados, de modo que los once esquemas quedan coherentes",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "新增界面规范并据此收敛：字号从 15 种归到 7 种（11 / 12 / 13 / 15 / 17 / 22，计算器 30 为唯一数值例外），字重 7 种归到 3 种，行高 7 种归到 3 种，间距统一到 4px 栅格。启动器、设置页、更新面板共 95 处字号改为引用同一套 token，规范写在仓库 docs/ui-spec.md",
          en: "Added a UI spec and converged on it: fifteen text sizes became seven (11 / 12 / 13 / 15 / 17 / 22, with 30 as the single numeric exception in the calculator), seven weights became three, seven line heights became three, and spacing snapped to a 4 px grid. Ninety-five font-size declarations across the launcher, settings and update panel now reference the same tokens; the spec lives in docs/ui-spec.md",
          ja: "UI 仕様を追加し、それに合わせて整理しました。文字サイズは 15 種から 7 種（11 / 12 / 13 / 15 / 17 / 22、計算機の 30 のみ例外）、ウェイトは 7 種から 3 種、行高は 7 種から 3 種、余白は 4px グリッドに統一。ランチャー、設定、更新パネルの 95 か所のフォントサイズが同じトークンを参照します。仕様は docs/ui-spec.md にあります",
          es: "Nueva especificación de interfaz y convergencia: quince tamaños de texto pasan a siete (11 / 12 / 13 / 15 / 17 / 22, con 30 como única excepción numérica en la calculadora), siete pesos pasan a tres, siete alturas de línea a tres y los espaciados se ajustan a una rejilla de 4 px. Noventa y cinco declaraciones de tamaño en el lanzador, los ajustes y el panel de actualización usan ahora los mismos tokens; la especificación está en docs/ui-spec.md",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "设置窗口固定为 1040×720，不再允许拖动改尺寸或最大化。界面是按这个画布设计的，拉大只会出现半空的栏位；小屏幕上仍按可用工作区自动钳制，不会超出屏幕",
          en: "The settings window is fixed at 1040×720 and can no longer be resized or maximised. The layout is designed for that canvas, and stretching it only produced half-empty panes; on smaller screens it still clamps to the available work area so it never spills off-screen",
          ja: "設定ウィンドウは 1040×720 に固定され、サイズ変更と最大化ができなくなりました。レイアウトはこのキャンバス前提で、広げると空いた欄が目立つだけでした。小さい画面では引き続き作業領域に合わせて自動的に縮み、画面外には出ません",
          es: "La ventana de ajustes queda fija en 1040×720 y ya no se puede redimensionar ni maximizar. El diseño está pensado para ese lienzo y estirarlo solo dejaba paneles medio vacíos; en pantallas pequeñas sigue ajustándose al área de trabajo disponible para no salirse",
        },
      },
    ],
  },
  {
    version: "1.2.18",
    date: "2026-11-10",
    title: {
      zh: "搜索结果不再继承管理员权限",
      en: "Results no longer inherit administrator rights",
      ja: "検索結果が管理者権限を引き継がなくなりました",
      es: "Los resultados ya no heredan los permisos de administrador",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "修复：以管理员身份运行 Kestrel 时，从搜索结果打开的程序会继承管理员权限，记事本、浏览器、编辑器全都莫名其妙地以管理员身份启动。默认档改为由一个短命的非提权副本执行打开动作，结果回到普通权限；副本创建失败时回退到原来的调用，最差也只是回到旧行为，不会出现点不开",
          en: "Fixed: when Kestrel ran as administrator, every program opened from the result list inherited the admin token, so Notepad, browsers and editors all started elevated without being asked to. The default mode now hands the open action to a short-lived non-elevated copy, returning results to ordinary rights; if that copy cannot be created the call falls back to the previous path — at worst the old behaviour comes back, never a result that refuses to open",
          ja: "修正：Kestrel を管理者として実行していると、検索結果から開いたプログラムが管理者トークンを引き継ぎ、メモ帳やブラウザ、エディターが意図せず管理者として起動していました。既定のモードでは、開く処理を短命の非昇格コピーに任せて通常権限に戻します。コピーを作成できない場合は従来の呼び出しに戻るため、最悪でも以前の動作に戻るだけで、開けない結果にはなりません",
          es: "Corregido: al ejecutar Kestrel como administrador, todos los programas abiertos desde los resultados heredaban el token de administrador, así que el Bloc de notas, los navegadores y los editores se iniciaban elevados sin pedirlo. El modo predeterminado delega la apertura en una copia efímera sin privilegios y devuelve los resultados a los permisos normales; si esa copia no se puede crear, se vuelve a la llamada anterior: en el peor caso reaparece el comportamiento antiguo, nunca un resultado que no se abre",
        },
      },
      {
        kind: "new",
        text: {
          zh: "新增「常规 › 权限 › 搜索结果的打开权限」三档设置：「普通权限」不继承提权（默认）、「跟随应用」保留原有行为、「始终管理员」让可执行结果每次请求 UAC；文档、链接与商店应用在「始终管理员」下仍按常规方式打开。需要临时提权时，Ctrl+Shift+Enter 或右键菜单的「以管理员身份运行」随时可用",
          en: "A three-way setting now lives in General › Privileges › Privilege for opened results: Normal never inherits elevation (default), Follow Kestrel keeps the previous behaviour, and Always elevated asks for UAC on every executable result — documents, links and Store apps still open normally under it. To elevate a single result, Ctrl+Shift+Enter or Run as administrator in the context menu are always there",
          ja: "「一般 › 権限 › 検索結果を開くときの権限」に 3 段階の設定を追加しました。「通常権限」は昇格を引き継がず（既定）、「Kestrel に従う」は従来の動作を保ち、「常に管理者」は実行ファイルの結果ごとに UAC を要求します。「常に管理者」でもドキュメント、リンク、ストアアプリは通常どおり開きます。1 件だけ昇格したいときは Ctrl+Shift+Enter または右クリックメニューの「管理者として実行」を使えます",
          es: "Nueva opción de tres niveles en General › Permisos › Permisos al abrir resultados: Normal no hereda la elevación (predeterminado), Seguir a Kestrel mantiene el comportamiento anterior y Siempre administrador pide UAC por cada resultado ejecutable; los documentos, enlaces y aplicaciones de la Store siguen abriéndose con normalidad. Para elevar algo puntualmente, Ctrl+Mayús+Intro o Ejecutar como administrador en el menú contextual siguen disponibles",
        },
      },
    ],
  },
  {
    version: "1.2.17",
    date: "2026-11-10",
    title: {
      zh: "暗色程序图标，图案更饱满",
      en: "A dark app icon with a bolder artwork",
      ja: "ダークなアプリアイコン、図柄を大きく",
      es: "Icono oscuro con el dibujo más grande",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "程序图标改用暗色版：可执行文件、窗口、托盘、安装包与快捷方式统一使用深蓝底配亮紫与青色的那一版。全套尺寸（ico 六帧、icns、Windows 磁贴、Android / iOS 图标）都从暗色 SVG 重新生成，README 预览图与图标生成脚本也一并指向暗色源。界面里的品牌图标仍然随明暗主题切换，不跟着固定成暗色",
          en: "The app icon now uses the dark artwork: the executable, windows, tray, installers and shortcuts all show the deep-blue square with the bright violet magnifier and cyan crosshair. Every size (six ico frames, icns, Windows tiles, Android / iOS icons) was regenerated from the dark SVG, and the README preview plus the icon script now point at that source. The in-app logo still follows the light / dark theme rather than being locked to dark",
          ja: "アプリアイコンをダーク版に変更しました。実行ファイル、ウィンドウ、トレイ、インストーラー、ショートカットは、深い青の背景に明るい紫の虫眼鏡とシアンの照準を配した版で統一されます。すべてのサイズ（ico 6 フレーム、icns、Windows タイル、Android / iOS アイコン）をダーク SVG から再生成し、README のプレビューとアイコン生成スクリプトもこのソースを参照します。アプリ内のロゴは従来どおりテーマに追従します",
          es: "El icono de la aplicación pasa a la versión oscura: el ejecutable, las ventanas, la bandeja, los instaladores y los accesos directos muestran el cuadrado azul profundo con la lupa violeta y la mira cian. Todos los tamaños (seis fotogramas ico, icns, mosaicos de Windows, iconos de Android / iOS) se regeneraron desde el SVG oscuro, y la vista previa del README y el script de iconos apuntan a esa fuente. El logotipo interno sigue el tema claro / oscuro en lugar de fijarse en oscuro",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "放大镜图案整体放大到画布的 1.35 倍：圆环、手柄、准星与中心点围绕圆心等比缩放，描边随比例加粗，图案在圆角底板里的占比从约一半提升到约三分之二，小尺寸下更容易认出来。暗色版还把外描边从 3px / 11% 提到 4px / 20%，放在深色任务栏上轮廓不再糊掉。两版 SVG 同步调整，界面品牌图标也使用新比例",
          en: "The magnifier artwork is scaled up 1.35× on the canvas: the ring, handle, crosshair ticks and centre dot grow around the artwork's centre with the stroke widening in proportion, so the drawing fills about two-thirds of the rounded square instead of roughly half and reads more clearly at small sizes. The dark variant also lifts its outer rim from 3px / 11% to 4px / 20%, so it no longer melts into a dark taskbar. Both SVGs changed together, so the in-app logo uses the new proportions too",
          ja: "虫眼鏡の図柄をキャンバス上で 1.35 倍に拡大しました。リング、持ち手、照準の刻み、中心点が図柄の中心を基準に拡大し、線幅も比例して太くなります。角丸の正方形に対して図柄の占有率は約半分から約三分の二に上がり、小さいサイズでも判別しやすくなりました。ダーク版は外周の縁取りも 3px / 11% から 4px / 20% に強め、暗いタスクバー上でも輪郭が残ります。2 つの SVG を同時に調整したため、アプリ内ロゴも新しい比率になります",
          es: "El dibujo de la lupa se amplía 1,35× en el lienzo: el anillo, el mango, las marcas de la mira y el punto central crecen alrededor del centro del dibujo y el trazo se engrosa en proporción, de modo que ocupa unos dos tercios del cuadrado redondeado en lugar de la mitad y se reconoce mejor en tamaños pequeños. La variante oscura sube además el borde exterior de 3px / 11% a 4px / 20%, así que ya no se funde con una barra de tareas oscura. Ambos SVG cambian a la vez, por lo que el logotipo interno también usa las nuevas proporciones",
        },
      },
    ],
  },
  {
    version: "1.2.16",
    date: "2026-11-10",
    title: {
      zh: "配色方案重做：11 套预设，可读性全部达标",
      en: "Colour schemes rebuilt: 11 presets, all readability-checked",
      ja: "配色スキームを作り直し：11 種類のプリセット、すべて読みやすさを検証",
      es: "Esquemas de color rehechos: 11 presets, todos con legibilidad verificada",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "外观不再只改几个按钮颜色：现在提供 11 套完整配色方案，分「跟随主题」「同色系」与「不同色系」三组。同色系 7 套（雾蓝、青瓷、薄荷、暖砂、玫瑰、藤紫、石墨）让背景、文字、描边与强调色共用同一色相；不同色系 3 套（港湾、极光、林间）把窗口底、卡片、交互控件、点缀分别映射到不同色相的界面层。每套都有浅色与深色两版",
          en: "Appearance is no longer just a few recoloured buttons: there are now 11 complete colour schemes in three groups — Follow theme, Monochrome and Multi-hue. The seven monochrome presets (Mist, Celadon, Mint, Sand, Rose, Violet, Graphite) share one hue across backgrounds, text, borders and accents; the three multi-hue presets (Harbour, Aurora, Woodland) map distinct hues onto the window, cards, interactive controls and spot details. Every scheme ships in both light and dark",
          ja: "外観はボタンの色を少し変えるだけではなくなりました。「テーマに従う」「同色系」「異色系」の 3 グループに、計 11 種類の完全な配色スキームを用意しています。同色系 7 種類（ミスト、青磁、ミント、暖砂、ローズ、藤紫、石墨）は背景・文字・枠線・強調色が同じ色相を共有し、異色系 3 種類（港、オーロラ、林間）はウィンドウ背景・カード・操作要素・アクセントにそれぞれ異なる色相を割り当てます。すべてにライト／ダークの 2 版があります",
          es: "La apariencia ya no se limita a cambiar el color de unos botones: ahora hay 11 esquemas completos en tres grupos — Seguir tema, Monocromo y Multitono. Los siete monocromos (Niebla, Celadón, Menta, Arena, Rosa, Violeta, Grafito) comparten un único tono en fondos, texto, bordes y acentos; los tres multitono (Puerto, Aurora, Bosque) asignan tonos distintos a la ventana, las tarjetas, los controles y los detalles. Cada esquema tiene versión clara y oscura",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "旧预设用固定 HSL 明度阶梯把界面全染成同一色相，辅助文字最低只有 2.78:1、白字压在薄荷绿按钮上只有 2.69:1。新版改用感知均匀的 OKLCH 派生整套颜色，并对窗口底、卡片、面板三个背景逐一校验文字对比度后自动钳制；强调色上的文字也按对比度自动选黑或白。「跟随主题」默认态同样拉到这条下限：强调色改为更深的蓝，浅色/深色辅助文字分别调深/调亮",
          en: "The old presets dyed the whole UI with a fixed HSL lightness ramp: faint text fell as low as 2.78:1 and white on the mint accent only 2.69:1. Schemes are now derived in perceptually uniform OKLCH, and text contrast is checked and clamped against the window, card and panel backgrounds; the ink on the accent colour is chosen automatically by contrast. The default Follow-theme palette was raised to the same floor with a deeper accent blue and adjusted faint-text greys in both light and dark",
          ja: "旧プリセットは固定の HSL 明度階調で画面全体を染色していたため、補助文字は最低 2.78:1、ミントの強調色上の白文字は 2.69:1 まで低下していました。新版は知覚的に均一な OKLCH で配色を生成し、ウィンドウ背景・カード・パネルの 3 種を対象に文字コントラストを検証して自動補正します。強調色上の文字色もコントラストで自動選択されます。既定の「テーマに従う」も同じ下限まで引き上げ、強調色を濃い青に、補助文字のグレーを明暗それぞれ調整しました",
          es: "Los presets anteriores teñían toda la interfaz con una escala HSL fija: el texto tenue bajaba a 2,78:1 y el blanco sobre el acento menta a solo 2,69:1. Ahora los esquemas se derivan en OKLCH perceptualmente uniforme, y el contraste del texto se comprueba y ajusta frente a los fondos de ventana, tarjeta y panel; el color del texto sobre el acento se elige automáticamente. El esquema predeterminado también se elevó al mismo mínimo, con un azul más profundo y grises ajustados en claro y oscuro",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "按反馈移除了全部自定义颜色功能：主色色板、染色强度滑块、12 项逐槽调色与「自动修正对比度」按钮都不再出现，设置里只挑方案，不会出现难读的组合。旧的 auto / custom / 未知值会按原强调色的色相自动落到最接近的同色系预设，已有配置不会丢失",
          en: "All custom-colour controls were removed as requested: the seed swatches, tint-strength slider, twelve per-slot colour pickers and the auto-fix button are gone. Picking a scheme is the only colour decision, so an unreadable combination cannot be built. Legacy auto / custom / unknown values are migrated to the nearest monochrome preset by the hue of the previous accent, keeping existing settings intact",
          ja: "ご要望に合わせてカスタムカラー機能をすべて削除しました。原色スウォッチ、染色強度スライダー、12 項目の個別カラーピッカー、「コントラスト自動修正」ボタンは表示されません。スキームを選ぶだけなので読みにくい組み合わせは作れません。旧 auto / custom / 未知の値は元の強調色の色相から最も近い同色系プリセットへ自動移行し、既存設定は失われません",
          es: "Se eliminaron todas las opciones de color personalizado: las muestras de color, el deslizador de intensidad, los doce selectores por elemento y el botón de corrección automática. Solo se elige un esquema, así que no se puede crear una combinación ilegible. Los valores antiguos auto / custom / desconocidos se migran al preset monocromo más cercano según el tono del acento anterior, sin perder la configuración",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "设置里的配色方案改成分组缩略图卡片：每张卡用该方案的真实颜色画出迷你窗口（侧栏、卡片、文字层级、按钮）和关键色色带，选中态有描边与勾选；下方是当前方案的大预览和 5 项实测对比度。点选后设置窗口本身立刻变色。另外新增配色对比度自动化测试（642 条断言 / 22 个方案变体）并接入 CI，词典校验器现在也会检查动态传入的方案名",
          en: "The settings page now shows grouped preview cards: each card draws a miniature window (sidebar, card, text levels, button) and a swatch strip using that scheme's real tokens, with a bordered selected state, plus a large preview of the current scheme and five measured contrast ratios. The settings window recolours itself instantly on selection. A new automated contrast test (642 assertions across 22 scheme variants) is wired into CI, and the dictionary checker now covers dynamically supplied scheme names",
          ja: "設定画面の配色スキームはグループ別のプレビューカードになりました。各カードはそのスキームの実際の色でミニウィンドウ（サイドバー、カード、文字階層、ボタン）とキーカラーの帯を描き、選択中は枠とチェックが付きます。下には現在のスキームの大きなプレビューと 5 項目の実測コントラストを表示します。選択すると設定ウィンドウ自身が即座に配色を変えます。さらにコントラスト自動テスト（22 バリエーション・642 アサーション）を CI に追加し、辞書チェッカーは動的に渡されるスキーム名も検証するようになりました",
          es: "En ajustes, los esquemas se muestran como tarjetas agrupadas: cada una dibuja una ventana en miniatura (barra lateral, tarjeta, niveles de texto, botón) y una franja de colores con los tokens reales, con borde y marca de selección; debajo aparecen una vista amplia del esquema activo y cinco ratios de contraste medidos. La ventana de ajustes se recolorea al instante. Se añadió a CI un test automático de contraste (642 aserciones en 22 variantes) y el verificador de diccionario ahora cubre los nombres de esquema pasados dinámicamente",
        },
      },
    ],
  },
  {
    version: "1.2.15",
    date: "2026-11-09",
    title: {
      zh: "全新品牌图标，明暗主题各一版",
      en: "A new brand icon, with light and dark variants",
      ja: "新しいブランドアイコン、ライト／ダークの 2 種類",
      es: "Nuevo icono de marca, con variantes clara y oscura",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "换上了新的品牌图标：圆角方块里是靛紫渐变的放大镜和青色准星。可执行文件、窗口、托盘、安装程序与快捷方式固定使用浅色版；两版 SVG 现作为唯一图标源，全套尺寸（含 ico、icns、Windows 磁贴和各平台图标）都由浅色版一键生成",
          en: "A new brand icon: an indigo magnifier with a cyan crosshair on a rounded square. The executable, windows, tray, installers and shortcuts always use the light artwork. Both SVGs are now the single source of truth, and every size (ico, icns, Windows tiles and mobile icons) is generated from the light one in a single step",
          ja: "新しいブランドアイコンになりました。角丸の正方形に藍紫のグラデーションの虫眼鏡とシアンの照準を配置しています。実行ファイル、ウィンドウ、トレイ、インストーラー、ショートカットは常にライト版を使用します。2 つの SVG が唯一のソースとなり、すべてのサイズ（ico、icns、Windows タイル、モバイル用アイコン）はライト版から一括生成されます",
          es: "Nuevo icono de marca: una lupa índigo con una mira cian sobre un cuadrado redondeado. El ejecutable, las ventanas, la bandeja, los instaladores y los accesos directos usan siempre la versión clara. Los dos SVG son ahora la única fuente y todos los tamaños (ico, icns, iconos de Windows y móviles) se generan a la vez desde la versión clara",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "设置窗口标题栏和「关于」页不再是渐变方块加放大镜字形，而是随当前明暗主题自动选用浅色或深色图标；切换主题时立刻跟随，深色界面上不会再出现一块发亮的白底",
          en: "The settings title bar and the About page no longer show a gradient badge with a magnifier glyph. They now pick the light or dark artwork to match the current theme, switching instantly with it — no bright white square on a dark surface",
          ja: "設定画面のタイトルバーと「バージョン情報」は、グラデーションの四角に虫眼鏡の字形を重ねたものではなくなりました。現在のテーマに合わせてライト／ダークのアイコンを自動で切り替え、深色テーマで白い四角が浮かぶこともありません",
          es: "La barra de título de ajustes y la página Acerca de ya no muestran una insignia degradada con una lupa. Ahora eligen la versión clara u oscura según el tema activo y cambian al instante con él, sin un cuadrado blanco brillando sobre una superficie oscura",
        },
      },
    ],
  },
  {
    version: "1.2.14",
    date: "2026-11-05",
    title: {
      zh: "QS 栏只在真正要选文件夹时出现",
      en: "Quick Switch only appears where a folder is actually being chosen",
      ja: "フォルダーを選ぶ場面だけに QS バーを表示",
      es: "Quick Switch solo aparece cuando se está eligiendo una carpeta",
    },
    items: [
      {
        kind: "fix",
        text: {
          zh: "重写了判定逻辑：以前只要对话框里有一个输入框看起来像文件夹路径，就会吸附 QS 栏，于是软件卸载界面、设置页等地方频繁误弹。现在必须有明确的「选择文件夹」证据（窗口标题、浏览按钮或已知压缩/文件管理器），安装、卸载、向导类窗口一律排除",
          en: "Rewrote the detection rule. Previously any dialog with a field that looked like a folder path got a Quick Switch, so uninstallers and settings pages kept showing one by mistake. An app dialog now needs positive evidence of a folder choice (title, browse button, or a known archive/file manager), and installers, uninstallers and wizards are excluded outright",
          ja: "判定ロジックを作り直しました。以前はフォルダーのパスに見える入力欄が 1 つあるだけで QS バーが付き、アンインストール画面や設定画面に頻繁に誤表示されていました。今は「フォルダー選択」の明確な根拠（タイトル・参照ボタン・既知のアーカイブ/ファイルマネージャー）が必要で、インストーラー、アンインストーラー、ウィザードは除外されます",
          es: "Se reescribió la regla de detección. Antes, cualquier diálogo con un campo parecido a una ruta de carpeta mostraba Quick Switch, por lo que aparecía por error en desinstaladores y páginas de ajustes. Ahora un diálogo propio de la app necesita una evidencia clara de elección de carpeta (título, botón Examinar o un gestor conocido), y los instaladores, desinstaladores y asistentes quedan excluidos",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "只读输入框（只显示安装位置、保存位置）不再被当成可写入的路径框；下载链接、多字段表单仍然排除。系统打开/保存对话框、7-Zip / WinRAR 的「解压到 / 复制到」照常可用",
          en: "Read-only fields that only display an install or save location are no longer treated as writable path boxes; download links and multi-field forms stay excluded. System Open/Save dialogs and 7-Zip / WinRAR extract and copy dialogs still work as before",
          ja: "インストール先や保存先を表示するだけの読み取り専用欄は、書き込み可能なパス欄として扱われません。ダウンロードリンクや入力欄の多いフォームも引き続き除外します。システムの開く/保存ダイアログ、7-Zip / WinRAR の「展開先 / コピー先」は従来どおり使えます",
          es: "Los campos de solo lectura que muestran una ubicación de instalación o guardado ya no se confunden con campos de ruta editables; los enlaces de descarga y los formularios con muchos campos siguen excluidos. Los diálogos del sistema y los de extraer/copiar de 7-Zip y WinRAR siguen funcionando igual",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "自动吸附变严格后留了手动通道：万一某个软件的文件夹对话框没被自动识别，在它里面按 Ctrl+G 仍可主动唤起 QS 栏（卸载向导、下载表单、只读输入框依然拒绝）",
          en: "The stricter auto-attach keeps a manual escape hatch: if an app's folder dialog is not recognised automatically, press Ctrl+G inside it to summon Quick Switch explicitly. Uninstall wizards, download forms and read-only fields are still refused",
          ja: "自動表示を厳しくした代わりに手動操作を残しました。フォルダーダイアログが自動認識されない場合でも、その中で Ctrl+G を押せば QS バーを呼び出せます（アンインストール画面、ダウンロードフォーム、読み取り専用欄は引き続き除外）",
          es: "El pegado automático más estricto conserva una vía manual: si el diálogo de carpetas de una app no se reconoce, pulsa Ctrl+G dentro de él para abrir Quick Switch explícitamente. Los asistentes de desinstalación, formularios de descarga y campos de solo lectura siguen excluidos",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "同时打开多个文件对话框时，QS 栏现在服务你正在使用的那一个：切换对话框只是平滑移动，不重置输入、不抢焦点；其中某个对话框关闭或隐藏后，会自动交给仍打开的那个，而不是跟着一起消失。每个对话框各自记住你是否手动关过 QS 栏；从资源管理器带回来的自动跳转也不会被顺手点开的另一个对话框消耗掉",
          en: "With several file dialogs open, Quick Switch now serves the one you are actually using: switching dialogs just moves the box smoothly, without clearing your input or stealing focus. When one dialog closes or hides, the box is handed to another one that is still open instead of disappearing. Each dialog remembers its own manual dismissal, and an Explorer round trip is no longer consumed by a dialog you merely clicked past",
          ja: "複数のファイルダイアログを開いている場合、QS バーは操作中のダイアログに追従します。切り替えても位置を滑らかに動かすだけで、入力内容は消えずフォーカスも奪いません。1 つ閉じたり隠したりしても、開いている別のダイアログに引き継がれ、消えてしまいません。手動で閉じた状態はダイアログごとに記憶され、エクスプローラーで訪れたフォルダーへの自動ジャンプも、通りすがりにクリックした別のダイアログに消費されません",
          es: "Con varios diálogos de archivo abiertos, Quick Switch ahora atiende al que estás usando: al cambiar de diálogo la caja solo se desplaza, sin borrar lo escrito ni robar el foco. Si un diálogo se cierra u oculta, la caja pasa a otro que siga abierto en vez de desaparecer. Cada diálogo recuerda su propio cierre manual, y la visita al Explorador ya no la consume un diálogo en el que solo hiciste clic de pasada",
        },
      },
    ],
  },
  {
    version: "1.2.13",
    date: "2026-11-04",
    title: {
      zh: "更新可以直接挑镜像，仅下载回到「下载」文件夹",
      en: "Pick a mirror in the update panel; downloads land in Downloads",
      ja: "更新画面でミラーを選択、ダウンロードは「ダウンロード」フォルダーへ",
      es: "Elige el espejo al actualizar; las descargas van a Descargas",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "发现新版本时，更新面板多了一个镜像下拉框：官方直连、三个内置加速镜像，以及自定义前缀。「更新」和「仅下载」都用当前选中的镜像，选择会写回设置；设置页的镜像选择也从一排按钮改成同款下拉，测速结果仍然显示",
          en: "The update panel now has a mirror drop-down: direct GitHub, three built-in accelerators, and a custom prefix. Both Update and Download only use the selected one, and the choice is saved back to settings. The settings page uses the same drop-down and still shows the speed-test results",
          ja: "更新パネルにミラーのドロップダウンを追加しました（直接接続・3 つの加速ミラー・カスタム接頭辞）。「更新」と「ダウンロードのみ」は選択中のミラーを使い、選択は設定に保存されます。設定画面も同じドロップダウンになり、速度測定の結果もそのまま表示されます",
          es: "El panel de actualización tiene ahora un desplegable de espejos: acceso directo, tres aceleradores integrados y un prefijo personalizado. Actualizar y Solo descargar usan el elegido, y la elección se guarda en los ajustes. La página de ajustes usa el mismo desplegable y sigue mostrando los resultados de la prueba de velocidad",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "「仅下载」现在把安装包存到系统的「下载」文件夹（跟随你在资源管理器里改过的下载位置），不再是藏在 AppData 里的临时目录。完成后显示保存路径，并给出「立即安装」和「打开所在文件夹」；重复下载同名文件会自动加 (1)，不会覆盖之前那份",
          en: "Download only now saves the installer to your real Downloads folder (including a location you redirected in Explorer), not a hidden AppData folder. When it finishes you get the path, an Install now button and Show in folder. Downloading the same release twice appends (1) instead of overwriting the earlier file",
          ja: "「ダウンロードのみ」は、AppData の隠しフォルダーではなくシステムの「ダウンロード」フォルダー（エクスプローラーで移動した場合もその場所）に保存するようになりました。完了後は保存先、「今すぐインストール」「フォルダーを開く」を表示します。同じファイル名は上書きせず (1) を付けます",
          es: "Solo descargar guarda el instalador en tu carpeta Descargas real (incluida una ubicación redirigida en el Explorador), no en una carpeta oculta de AppData. Al terminar muestra la ruta, Instalar ahora y Mostrar en la carpeta. Descargar dos veces lo mismo añade (1) en lugar de sobrescribir",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "修复 QS 栏出现在不该出现的地方：IDM 的「下载文件信息」这类对话框里有一个「保存到」文件夹路径，被误当成「请选择文件夹」。现在输入框很多的表单、含下载链接的对话框、以及下载管理器自己的窗口都不再触发；它们打开的系统文件对话框仍然照常吸附",
          en: "Quick Switch no longer appears where it does not belong. IDM's download-information dialog shows the folder a file will be saved to, which the folder-field rule mistook for a folder picker. Forms with many input fields, dialogs containing a download link, and download-manager windows are now skipped — while the real system file dialogs they open still attach as before",
          ja: "不要な場所に QS バーが出る問題を修正しました。IDM の「ダウンロードファイル情報」にある保存先フォルダーを、フォルダー選択欄と誤認していました。入力欄の多いフォーム、ダウンロードリンクを含むダイアログ、ダウンロードマネージャー自身のウィンドウは除外されますが、そこから開くシステムのファイルダイアログには今までどおり表示されます",
          es: "Quick Switch ya no aparece donde no debe. El diálogo de información de descarga de IDM muestra la carpeta de destino y la regla del campo de carpeta lo confundía con un selector. Ahora se descartan los formularios con muchos campos, los diálogos con un enlace de descarga y las ventanas propias del gestor; los diálogos de archivo del sistema que abren siguen funcionando igual",
        },
      },
    ],
  },
  {
    version: "1.2.12",
    date: "2026-11-04",
    title: {
      zh: "设置里的应用图标几乎是立刻就出来了",
      en: "App icons in settings show up almost immediately",
      ja: "設定のアプリアイコンがほぼすぐに表示されます",
      es: "Los iconos de las aplicaciones en los ajustes aparecen casi al instante",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "图标现在并行提取。系统每要一个图标就做一次应答，本机实测单个约 20 毫秒（应用商店的应用约 40 毫秒），236 个应用排队做下来要 5.3 秒；分到多个核心上只要约 0.6 秒",
          en: "Icons are extracted in parallel now. The shell answers one icon per call at ~20 ms each (~40 ms for a Store app), so 236 apps took 5.3 s in a queue; spread over the cores it takes about 0.6 s",
          ja: "アイコンの取得を並列化しました。シェルは 1 回の呼び出しにつき 1 つしか返さず、1 個あたり約 20 ミリ秒（ストアアプリは約 40 ミリ秒）かかるため、236 個を順番に処理すると 5.3 秒かかっていました。コアに分散すると約 0.6 秒です",
          es: "Los iconos se extraen en paralelo. El shell responde de uno en uno a ~20 ms cada uno (~40 ms para una aplicación de la Store), así que 236 aplicaciones tardaban 5,3 s en cola; repartido entre los núcleos tarda unos 0,6 s",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "图标分批返回，先到先显示。以前是整个列表一次性要，最后一个图标没出来之前，所有格子都停在占位符上——看起来像卡住了",
          en: "Icons come back in small batches and appear as they arrive. The whole list used to be requested in one go, so every tile sat on its placeholder until the very last icon had been extracted — it looked stuck",
          ja: "アイコンは小さなまとまりで返り、届いた順に表示されます。以前は一覧をまとめて要求していたため、最後の 1 個が取れるまで全部のタイルがプレースホルダーのままで、止まっているように見えていました",
          es: "Los iconos llegan en pequeños lotes y aparecen según llegan. Antes se pedía la lista completa de una vez, así que todas las casillas se quedaban en el marcador de posición hasta el último icono: parecía colgado",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "顺带修掉两个会留下空白格的问题：一是并发首次提取时有几个图标会失败（重试都能成功），失败结果却会被缓存，那个格子就整场都是空白；二是列表只给前 150 个取图标，应用多于 150 个时后面的永远是灰图标",
          en: "Two bugs that left tiles blank are gone: a concurrent first pass could lose a few icons (all of which succeed on retry) and the failure was cached, so that tile stayed blank for the rest of the session; and only the first 150 apps were asked for icons, leaving everything past 150 on the grey glyph forever",
          ja: "空白のタイルが残る 2 つの不具合も直しました。1 つは、並列の初回取得で数個のアイコンが失敗することがあり（再試行すれば成功します）、その失敗がキャッシュされて以降ずっと空白のままになっていたこと。もう 1 つは、最初の 150 個分しかアイコンを要求しておらず、151 個目以降が永久に灰色のままであったことです",
          es: "También se corrigieron dos fallos que dejaban casillas en blanco: una primera pasada concurrente podía perder algunos iconos (todos se obtienen al reintentar) y el fallo se guardaba en caché, dejando esa casilla vacía el resto de la sesión; y solo se pedían iconos para las primeras 150 aplicaciones, así que a partir de esa cifra se quedaban con el glifo gris para siempre",
        },
      },
    ],
  },
  {
    version: "1.2.11",
    date: "2026-11-04",
    title: {
      zh: "点开搜索结果不再空等一秒半",
      en: "Opening a result no longer leaves you waiting second and a half",
      ja: "検索結果を開くときに 1.5 秒待たされなくなりました",
      es: "Abrir un resultado ya no te hace esperar segundo y medio",
    },
    items: [
      {
        kind: "improve",
        text: {
          zh: "点击搜索结果后，搜索栏先立刻收起，再把路径交给系统。以前顺序是反的——先等系统把程序拉起来、再收窗口，而系统并不快：本机实测，启动 exe 约 70 毫秒，打开网址约 120 毫秒，「在资源管理器中显示」约 460 毫秒，打开一个文件夹要 1.6 秒（Explorer 通过 DDE 应答）。点文件夹时，搜索栏就是在屏幕上干等了一秒半",
          en: "A click now takes the search bar off the screen first and hands the path to the shell afterwards. It used to be the other way round — wait for the shell to bring the program up, then hide — and the shell is not quick: measured here, an .exe takes ~70 ms, a URL ~120 ms, \"show in Explorer\" ~460 ms and opening a folder a full 1.6 s (Explorer answers over DDE). Clicking a folder meant the bar sat there for a second and a half doing nothing",
          ja: "検索結果をクリックすると、まず検索バーが即座に引っ込み、そのあとでパスをシェルに渡すようになりました。以前は逆で、シェルがプログラムを起動するのを待ってからウィンドウを閉じていました。シェルは速くありません。実測では exe の起動に約 70 ミリ秒、URL で約 120 ミリ秒、「エクスプローラーで表示」で約 460 ミリ秒、フォルダーを開くには 1.6 秒かかります（Explorer が DDE で応答するため）。フォルダーをクリックすると、検索バーは 1.5 秒間そのまま画面に残っていました",
          es: "Ahora, al hacer clic, la barra de búsqueda desaparece primero y después se entrega la ruta al shell. Antes era al revés: se esperaba a que el shell abriera el programa y luego se ocultaba, y el shell no es rápido. Medido en este equipo, un .exe tarda unos 70 ms, una URL unos 120 ms, «mostrar en el Explorador» unos 460 ms y abrir una carpeta 1,6 s completos (el Explorador responde por DDE). Al hacer clic en una carpeta, la barra se quedaba en pantalla segundo y medio sin hacer nada",
        },
      },
      {
        kind: "improve",
        text: {
          zh: "打开动作也挪出了命令线程。以前它占着那个线程，那一秒半里连再次按热键唤出、打开设置都要排队",
          en: "The launch itself also moved off the command thread. It used to hold that thread for the whole wait, so summoning the launcher again or opening settings queued up behind it",
          ja: "起動処理もコマンドスレッドの外に出しました。以前はその待ち時間ずっとスレッドを占有していたため、その間にホットキーで呼び出したり設定を開いたりしても順番待ちになっていました",
          es: "La apertura también salió del hilo de comandos. Antes lo ocupaba durante toda la espera, así que volver a invocar el buscador o abrir los ajustes se quedaba en cola detrás",
        },
      },
      {
        kind: "new",
        text: {
          zh: "「以管理员身份运行」不再只对应用列表里的程序生效：任何能被提权的文件都有这一项了（exe、com、bat、cmd、msi、cpl、msc、ps1、vbs/jse/wsf、reg、lnk），位置也从第三项挪到「打开」下面的第二项",
          en: "\"Run as administrator\" is no longer limited to programs from the app list: any file the shell can elevate offers it now (.exe, .com, .bat, .cmd, .msi, .cpl, .msc, .ps1, .vbs/.jse/.wsf, .reg, .lnk). It also moved up from third place to right below \"Open\"",
          ja: "「管理者として実行」がアプリ一覧のプログラムだけではなくなりました。昇格できるファイルなら何でも表示されます（exe、com、bat、cmd、msi、cpl、msc、ps1、vbs/jse/wsf、reg、lnk）。位置も 3 番目から「開く」のすぐ下の 2 番目に移しました",
          es: "«Ejecutar como administrador» ya no se limita a los programas de la lista de aplicaciones: ahora aparece en cualquier archivo que el shell pueda elevar (.exe, .com, .bat, .cmd, .msi, .cpl, .msc, .ps1, .vbs/.jse/.wsf, .reg, .lnk). Además pasó del tercer puesto a justo debajo de «Abrir»",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "Ctrl+Shift+Enter 现在真的以管理员身份运行了。菜单上一直标着这个快捷键，但代码里从没实现过，按下去执行的其实是「打开方式」",
          en: "Ctrl+Shift+Enter really runs as administrator now. The menu has always advertised that shortcut, but it was never wired up — pressing it opened the \"Open with\" picker instead",
          ja: "Ctrl+Shift+Enter が実際に管理者として実行するようになりました。メニューには以前からこのショートカットが書かれていましたが、実装されておらず、押すと「プログラムから開く」が動いていました",
          es: "Ctrl+Mayús+Intro ahora sí ejecuta como administrador. El menú siempre mostró ese atajo, pero nunca estuvo implementado: al pulsarlo se abría «Abrir con»",
        },
      },
      {
        kind: "fix",
        text: {
          zh: "先收窗口再启动程序，会有新程序开在后台的风险（不是前台进程就没有置顶权）。现在启动前显式把前台权限转交给马上要运行的程序",
          en: "Hiding before launching risks the new program opening behind everything (a process that is not in the foreground cannot raise its window). The launcher now explicitly passes the foreground right to the program it is about to start",
          ja: "先にウィンドウを隠してから起動すると、新しいプログラムが背面に開いてしまう恐れがあります（前面のプロセスでないとウィンドウを最前面に出せません）。起動直前に、前面に出る権利をこれから動くプログラムへ明示的に渡すようにしました",
          es: "Ocultar la ventana antes de lanzar el programa puede hacer que la nueva ventana aparezca detrás de todo (un proceso que no está en primer plano no puede elevar su ventana). Ahora el buscador cede explícitamente ese derecho al programa que va a iniciar",
        },
      },
    ],
  },
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
