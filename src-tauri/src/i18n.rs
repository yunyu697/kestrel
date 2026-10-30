//! Interface languages for the strings the Rust side produces: tray menu,
//! seeded presets, default favorites and search hints.
//!
//! The frontend has its own dictionary (`src/shared/i18n.ts`); both sides use
//! the same language ids: `zh`, `zh-TW`, `en`, `ja`, `es`.

pub const LANGS: [&str; 5] = ["zh", "zh-TW", "en", "ja", "es"];

/// Index of a language id inside `LANGS` (unknown → English).
///
/// `"system"` (the "follow Windows" option) resolves to whatever the OS
/// reports, so every backend string follows the system without callers having
/// to special-case it.
pub fn index_of(lang: &str) -> usize {
    if lang == "system" {
        return index_of(detect_system());
    }
    match LANGS.iter().position(|l| *l == lang) {
        Some(i) => i,
        None => 2,
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
}

/// Map a BCP-47 locale tag to one of our languages.
/// Anything unsupported falls back to English, as specified.
pub fn from_locale(tag: &str) -> &'static str {
    let low = tag.trim().to_ascii_lowercase().replace('_', "-");
    if low.is_empty() {
        return "en";
    }
    if low.starts_with("zh") {
        if low.contains("tw")
            || low.contains("hk")
            || low.contains("mo")
            || low.contains("hant")
            || low.contains("zh-cht")
        {
            return "zh-TW";
        }
        return "zh";
    }
    if low.starts_with("ja") {
        return "ja";
    }
    if low.starts_with("es") {
        return "es";
    }
    "en"
}

/// The OS interface language, so a fresh install matches the system.
pub fn detect_system() -> &'static str {
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return "en";
    }
    let tag = String::from_utf16_lossy(&buf[..(n as usize).saturating_sub(1)]);
    from_locale(&tag)
}

/// Small key/value table: index 0..=4 = zh, zh-TW, en, ja, es.
pub fn tr(lang: &str, key: &str) -> &'static str {
    let i = index_of(lang);
    let row: [&'static str; 5] = match key {
        "tray.show" => ["打开 Kestrel", "開啟 Kestrel", "Open Kestrel", "Kestrel を開く", "Abrir Kestrel"],
        "tray.settings" => ["设置…", "設定…", "Settings…", "設定…", "Ajustes…"],
        "tray.reindex" => ["重建索引", "重建索引", "Rebuild index", "インデックスを再構築", "Reconstruir índice"],
        "tray.quit" => ["退出", "結束", "Quit", "終了", "Salir"],
        "tray.tip" => [
            "Kestrel — 单击唤起，双击设置",
            "Kestrel — 單擊喚起，雙擊設定",
            "Kestrel — click to open, double-click for settings",
            "Kestrel — クリックで起動、ダブルクリックで設定",
            "Kestrel — clic para abrir, doble clic para ajustes",
        ],
        "hint.indexing" => [
            "正在建立索引，结果会逐步完整",
            "正在建立索引，結果會逐步完整",
            "Building the index — results fill in as it goes",
            "インデックス作成中 — 結果は順次追加されます",
            "Creando el índice: los resultados irán apareciendo",
        ],
        "hint.indexing_partial" => [
            "索引构建中，结果可能不完整",
            "索引建置中，結果可能不完整",
            "Index is still building, results may be incomplete",
            "インデックス作成中のため、結果が不完全な場合があります",
            "El índice se está creando; los resultados pueden estar incompletos",
        ],
        "badge.custom" => ["自定义", "自訂", "Custom", "カスタム", "Personalizado"],
        "badge.desktop" => [
            "桌面快捷方式",
            "桌面捷徑",
            "Desktop shortcut",
            "デスクトップのショートカット",
            "Acceso directo",
        ],
        "badge.app" => ["应用程序", "應用程式", "Application", "アプリ", "Aplicación"],
        "badge.favorite" => ["收藏", "收藏", "Favorite", "お気に入り", "Favorito"],
        "badge.frequent" => ["常用", "常用", "Frequent", "よく使う", "Frecuente"],
        "badge.calc" => ["计算", "計算", "Calc", "計算", "Cálculo"],
        "web.search_title" => [
            "用 {engine} 搜索 “{q}”",
            "用 {engine} 搜尋「{q}」",
            "Search {engine} for “{q}”",
            "{engine} で「{q}」を検索",
            "Buscar “{q}” en {engine}",
        ],
        _ => ["", "", "", "", ""],
    };
    row[i]
}

/// A seeded custom command, localised into all supported languages.
pub struct Preset {
    pub id: &'static str,
    pub keyword: &'static str,
    pub action: &'static str,
    pub target: &'static str,
    /// zh, zh-TW, en, ja, es
    pub names: [&'static str; 5],
    pub descs: [&'static str; 5],
}

pub const PRESETS: [Preset; 13] = [
    Preset {
        id: "preset-settings",
        keyword: "settings",
        action: "kestrel",
        target: "",
        names: ["打开设置", "開啟設定", "Open settings", "設定を開く", "Abrir ajustes"],
        descs: [
            "打开 Kestrel 设置界面",
            "開啟 Kestrel 設定介面",
            "Open the Kestrel settings window",
            "Kestrel の設定画面を開く",
            "Abrir la ventana de ajustes de Kestrel",
        ],
    },
    Preset {
        id: "preset-hosts",
        keyword: "hosts",
        action: "path",
        target: "C:\\Windows\\System32\\drivers\\etc\\hosts",
        names: ["hosts 文件", "hosts 檔案", "Hosts file", "hosts ファイル", "Archivo hosts"],
        descs: [
            "用默认编辑器打开 hosts",
            "用預設編輯器開啟 hosts",
            "Open hosts with the default editor",
            "既定のエディターで hosts を開く",
            "Abrir hosts con el editor predeterminado",
        ],
    },
    Preset {
        id: "preset-ip",
        keyword: "ip",
        action: "shell",
        target: "ipconfig /all",
        names: ["IP 配置", "IP 設定", "IP configuration", "IP 構成", "Configuración IP"],
        descs: [
            "查看完整网络配置",
            "檢視完整網路設定",
            "Show the full network configuration",
            "ネットワーク構成をすべて表示",
            "Ver la configuración de red completa",
        ],
    },
    Preset {
        id: "preset-temp",
        keyword: "temp",
        action: "shell",
        target: "explorer %TEMP%",
        names: ["临时目录", "暫存目錄", "Temp folder", "一時フォルダー", "Carpeta temporal"],
        descs: [
            "打开当前用户的临时文件夹",
            "開啟目前使用者的暫存資料夾",
            "Open the current user's temp folder",
            "現在のユーザーの一時フォルダーを開く",
            "Abrir la carpeta temporal del usuario",
        ],
    },
    Preset {
        id: "preset-fonts",
        keyword: "fonts",
        action: "url",
        target: "shell:Fonts",
        names: ["字体", "字型", "Fonts", "フォント", "Fuentes"],
        descs: [
            "打开系统字体文件夹",
            "開啟系統字型資料夾",
            "Open the system fonts folder",
            "システムのフォントフォルダーを開く",
            "Abrir la carpeta de fuentes del sistema",
        ],
    },
    Preset {
        id: "preset-calc",
        keyword: "calc",
        action: "program",
        target: "calc.exe",
        names: ["计算器", "計算機", "Calculator", "電卓", "Calculadora"],
        descs: ["", "", "", "", ""],
    },
    Preset {
        id: "preset-notepad",
        keyword: "notepad",
        action: "program",
        target: "notepad.exe",
        names: ["记事本", "記事本", "Notepad", "メモ帳", "Bloc de notas"],
        descs: ["", "", "", "", ""],
    },
    Preset {
        id: "preset-paint",
        keyword: "paint",
        action: "program",
        target: "mspaint.exe",
        names: ["画图", "小畫家", "Paint", "ペイント", "Paint"],
        descs: ["", "", "", "", ""],
    },
    Preset {
        id: "preset-cleanmgr",
        keyword: "cleanmgr",
        action: "program",
        target: "cleanmgr.exe",
        names: [
            "磁盘清理",
            "磁碟清理",
            "Disk Cleanup",
            "ディスク クリーンアップ",
            "Liberador de espacio",
        ],
        descs: [
            "清理磁盘垃圾",
            "清理磁碟垃圾",
            "Free up disk space",
            "ディスクの不要ファイルを削除",
            "Liberar espacio en disco",
        ],
    },
    Preset {
        id: "preset-msinfo",
        keyword: "msinfo",
        action: "program",
        target: "msinfo32.exe",
        names: [
            "系统信息",
            "系統資訊",
            "System Information",
            "システム情報",
            "Información del sistema",
        ],
        descs: [
            "查看系统详细信息",
            "檢視系統詳細資訊",
            "Show detailed system information",
            "システムの詳細情報を表示",
            "Ver información detallada del sistema",
        ],
    },
    Preset {
        id: "preset-rdp",
        keyword: "rdp",
        action: "program",
        target: "mstsc.exe",
        names: [
            "远程桌面",
            "遠端桌面",
            "Remote Desktop",
            "リモート デスクトップ",
            "Escritorio remoto",
        ],
        descs: [
            "打开远程桌面连接",
            "開啟遠端桌面連線",
            "Open Remote Desktop Connection",
            "リモート デスクトップ接続を開く",
            "Abrir Conexión a Escritorio remoto",
        ],
    },
    Preset {
        id: "preset-charmap",
        keyword: "charmap",
        action: "program",
        target: "charmap.exe",
        names: [
            "字符映射表",
            "字元對應表",
            "Character Map",
            "文字コード表",
            "Mapa de caracteres",
        ],
        descs: [
            "查找与复制特殊字符",
            "尋找與複製特殊字元",
            "Find and copy special characters",
            "特殊文字を検索してコピー",
            "Buscar y copiar caracteres especiales",
        ],
    },
    Preset {
        id: "preset-clipbrd",
        keyword: "clipbrd",
        action: "shell",
        target: "cmd /c echo | clip",
        names: ["剪贴板查看", "剪貼簿檢視", "Clipboard", "クリップボード", "Portapapeles"],
        descs: [
            "清空剪贴板",
            "清空剪貼簿",
            "Clear the clipboard",
            "クリップボードを空にする",
            "Vaciar el portapapeles",
        ],
    },
];

/// Localised names for the default favorites, in creation order.
pub fn favorite_names(lang: &str) -> [&'static str; 6] {
    match index_of(lang) {
        0 => ["桌面", "下载", "文档", "图片", "音乐", "视频"],
        1 => ["桌面", "下載", "文件", "圖片", "音樂", "影片"],
        3 => [
            "デスクトップ",
            "ダウンロード",
            "ドキュメント",
            "ピクチャ",
            "ミュージック",
            "ビデオ",
        ],
        4 => [
            "Escritorio",
            "Descargas",
            "Documentos",
            "Imágenes",
            "Música",
            "Vídeos",
        ],
        _ => [
            "Desktop",
            "Downloads",
            "Documents",
            "Pictures",
            "Music",
            "Videos",
        ],
    }
}

// ---------------- built-in system commands ----------------

/// Group labels for built-in commands.
pub fn group_label(lang: &str, group: &str) -> &'static str {
    let i = index_of(lang);
    let row: [&'static str; 5] = match group {
        "系统" => ["系统", "系統", "System", "システム", "Sistema"],
        "电源" => ["电源", "電源", "Power", "電源", "Energía"],
        "工具" => ["工具", "工具", "Tools", "ツール", "Herramientas"],
        _ => ["Kestrel", "Kestrel", "Kestrel", "Kestrel", "Kestrel"],
    };
    row[i]
}

/// id, group, [name ×5], [desc ×5]
pub struct Builtin {
    pub id: &'static str,
    pub group: &'static str,
    pub names: [&'static str; 5],
    pub descs: [&'static str; 5],
}

pub const BUILTINS: [Builtin; 32] = [
    Builtin { id: "cmd.settings", group: "系统",
        names: ["Windows 设置", "Windows 設定", "Windows Settings", "Windows の設定", "Configuración de Windows"],
        descs: ["打开系统设置", "開啟系統設定", "Open system settings", "システム設定を開く", "Abrir la configuración del sistema"] },
    Builtin { id: "cmd.control", group: "系统",
        names: ["控制面板", "控制台", "Control Panel", "コントロール パネル", "Panel de control"],
        descs: ["打开控制面板", "開啟控制台", "Open Control Panel", "コントロール パネルを開く", "Abrir el Panel de control"] },
    Builtin { id: "cmd.taskmgr", group: "系统",
        names: ["任务管理器", "工作管理員", "Task Manager", "タスク マネージャー", "Administrador de tareas"],
        descs: ["打开任务管理器", "開啟工作管理員", "Open Task Manager", "タスク マネージャーを開く", "Abrir el Administrador de tareas"] },
    Builtin { id: "cmd.terminal", group: "系统",
        names: ["终端", "終端機", "Terminal", "ターミナル", "Terminal"],
        descs: ["打开 Windows 终端", "開啟 Windows 終端機", "Open Windows Terminal", "Windows ターミナルを開く", "Abrir Windows Terminal"] },
    Builtin { id: "cmd.regedit", group: "系统",
        names: ["注册表编辑器", "登錄編輯程式", "Registry Editor", "レジストリ エディター", "Editor del Registro"],
        descs: ["打开注册表编辑器", "開啟登錄編輯程式", "Open Registry Editor", "レジストリ エディターを開く", "Abrir el Editor del Registro"] },
    Builtin { id: "cmd.explorer", group: "系统",
        names: ["文件资源管理器", "檔案總管", "File Explorer", "エクスプローラー", "Explorador de archivos"],
        descs: ["打开此电脑", "開啟本機", "Open This PC", "PC を開く", "Abrir Este equipo"] },
    Builtin { id: "cmd.recycle", group: "系统",
        names: ["回收站", "資源回收筒", "Recycle Bin", "ごみ箱", "Papelera de reciclaje"],
        descs: ["打开回收站", "開啟資源回收筒", "Open the Recycle Bin", "ごみ箱を開く", "Abrir la Papelera de reciclaje"] },
    Builtin { id: "cmd.emptyrecycle", group: "系统",
        names: ["清空回收站", "清空資源回收筒", "Empty Recycle Bin", "ごみ箱を空にする", "Vaciar la Papelera"],
        descs: ["永久删除回收站内容", "永久刪除資源回收筒內容", "Permanently delete the contents", "ごみ箱の中身を完全に削除", "Eliminar el contenido de forma permanente"] },
    Builtin { id: "cmd.lock", group: "电源",
        names: ["锁定屏幕", "鎖定畫面", "Lock screen", "画面をロック", "Bloquear pantalla"],
        descs: ["立即锁定此设备", "立即鎖定此裝置", "Lock this device now", "このデバイスを今すぐロック", "Bloquear este dispositivo ahora"] },
    Builtin { id: "cmd.sleep", group: "电源",
        names: ["睡眠", "睡眠", "Sleep", "スリープ", "Suspender"],
        descs: ["让电脑进入睡眠", "讓電腦進入睡眠", "Put the computer to sleep", "コンピューターをスリープ状態にする", "Poner el equipo en suspensión"] },
    Builtin { id: "cmd.shutdown", group: "电源",
        names: ["关机", "關機", "Shut down", "シャットダウン", "Apagar"],
        descs: ["关闭计算机", "關閉電腦", "Turn the computer off", "コンピューターを終了する", "Apagar el equipo"] },
    Builtin { id: "cmd.restart", group: "电源",
        names: ["重启", "重新啟動", "Restart", "再起動", "Reiniciar"],
        descs: ["重新启动计算机", "重新啟動電腦", "Restart the computer", "コンピューターを再起動する", "Reiniciar el equipo"] },
    Builtin { id: "cmd.logout", group: "电源",
        names: ["注销", "登出", "Sign out", "サインアウト", "Cerrar sesión"],
        descs: ["注销当前用户", "登出目前使用者", "Sign the current user out", "現在のユーザーをサインアウト", "Cerrar la sesión del usuario actual"] },
    Builtin { id: "cmd.screenshot", group: "工具",
        names: ["截图", "剪取工具", "Screenshot", "スクリーンショット", "Captura de pantalla"],
        descs: ["打开截图工具", "開啟剪取工具", "Open the snipping tool", "切り取りツールを開く", "Abrir la herramienta de recortes"] },
    Builtin { id: "cmd.magnifier", group: "工具",
        names: ["放大镜", "放大鏡", "Magnifier", "拡大鏡", "Lupa"],
        descs: ["打开放大镜", "開啟放大鏡", "Open Magnifier", "拡大鏡を開く", "Abrir la Lupa"] },
    Builtin { id: "cmd.onenote", group: "工具",
        names: ["屏幕草图", "螢幕繪圖", "Screen sketch", "画面スケッチ", "Boceto de pantalla"],
        descs: ["打开屏幕草图", "開啟螢幕繪圖", "Open screen sketch", "画面スケッチを開く", "Abrir el boceto de pantalla"] },
    Builtin { id: "cmd.network", group: "系统",
        names: ["网络连接", "網路連線", "Network connections", "ネットワーク接続", "Conexiones de red"],
        descs: ["打开网络连接面板", "開啟網路連線面板", "Open network connections", "ネットワーク接続を開く", "Abrir las conexiones de red"] },
    Builtin { id: "cmd.printers", group: "系统",
        names: ["打印机", "印表機", "Printers", "プリンター", "Impresoras"],
        descs: ["打开打印机设置", "開啟印表機設定", "Open printer settings", "プリンター設定を開く", "Abrir la configuración de impresoras"] },
    Builtin { id: "cmd.sound", group: "系统",
        names: ["声音设置", "音效設定", "Sound settings", "サウンド設定", "Configuración de sonido"],
        descs: ["打开声音设置", "開啟音效設定", "Open sound settings", "サウンド設定を開く", "Abrir la configuración de sonido"] },
    Builtin { id: "cmd.display", group: "系统",
        names: ["显示设置", "顯示設定", "Display settings", "ディスプレイ設定", "Configuración de pantalla"],
        descs: ["打开显示设置", "開啟顯示設定", "Open display settings", "ディスプレイ設定を開く", "Abrir la configuración de pantalla"] },
    Builtin { id: "cmd.bluetooth", group: "系统",
        names: ["蓝牙设置", "藍牙設定", "Bluetooth settings", "Bluetooth 設定", "Configuración de Bluetooth"],
        descs: ["打开蓝牙设置", "開啟藍牙設定", "Open Bluetooth settings", "Bluetooth 設定を開く", "Abrir la configuración de Bluetooth"] },
    Builtin { id: "cmd.apps", group: "系统",
        names: ["已安装应用", "已安裝應用程式", "Installed apps", "インストール済みアプリ", "Aplicaciones instaladas"],
        descs: ["管理已安装的应用", "管理已安裝的應用程式", "Manage installed apps", "インストール済みアプリを管理", "Administrar las aplicaciones instaladas"] },
    Builtin { id: "cmd.datetime", group: "系统",
        names: ["日期和时间", "日期與時間", "Date and time", "日付と時刻", "Fecha y hora"],
        descs: ["打开日期时间设置", "開啟日期時間設定", "Open date and time settings", "日付と時刻の設定を開く", "Abrir la configuración de fecha y hora"] },
    Builtin { id: "cmd.env", group: "系统",
        names: ["环境变量", "環境變數", "Environment variables", "環境変数", "Variables de entorno"],
        descs: ["编辑系统环境变量", "編輯系統環境變數", "Edit system environment variables", "システム環境変数を編集", "Editar las variables de entorno del sistema"] },
    Builtin { id: "cmd.device", group: "系统",
        names: ["设备管理器", "裝置管理員", "Device Manager", "デバイス マネージャー", "Administrador de dispositivos"],
        descs: ["打开设备管理器", "開啟裝置管理員", "Open Device Manager", "デバイス マネージャーを開く", "Abrir el Administrador de dispositivos"] },
    Builtin { id: "cmd.disk", group: "系统",
        names: ["磁盘管理", "磁碟管理", "Disk Management", "ディスクの管理", "Administración de discos"],
        descs: ["打开磁盘管理", "開啟磁碟管理", "Open Disk Management", "ディスクの管理を開く", "Abrir la Administración de discos"] },
    Builtin { id: "cmd.services", group: "系统",
        names: ["服务", "服務", "Services", "サービス", "Servicios"],
        descs: ["打开服务管理器", "開啟服務管理員", "Open Services", "サービスを開く", "Abrir Servicios"] },
    Builtin { id: "cmd.about", group: "Kestrel",
        names: ["关于 Kestrel", "關於 Kestrel", "About Kestrel", "Kestrel について", "Acerca de Kestrel"],
        descs: ["查看版本与构建信息", "檢視版本與組建資訊", "Show version and build info", "バージョンとビルド情報を表示", "Ver la versión y la información de compilación"] },
    Builtin { id: "cmd.settings.kestrel", group: "Kestrel",
        names: ["Kestrel 设置", "Kestrel 設定", "Kestrel Settings", "Kestrel の設定", "Ajustes de Kestrel"],
        descs: ["打开 Kestrel 设置", "開啟 Kestrel 設定", "Open Kestrel settings", "Kestrel の設定を開く", "Abrir los ajustes de Kestrel"] },
    Builtin { id: "cmd.reindex", group: "Kestrel",
        names: ["重建索引", "重建索引", "Rebuild index", "インデックスを再構築", "Reconstruir índice"],
        descs: ["重新扫描文件索引", "重新掃描檔案索引", "Rescan the file index", "ファイルインデックスを再スキャン", "Volver a escanear el índice de archivos"] },
    Builtin { id: "cmd.clipboard", group: "Kestrel",
        names: ["剪贴板历史", "剪貼簿歷史", "Clipboard history", "クリップボード履歴", "Historial del portapapeles"],
        descs: ["查看剪贴板历史", "檢視剪貼簿歷史", "Show clipboard history", "クリップボード履歴を表示", "Ver el historial del portapapeles"] },
    Builtin { id: "cmd.notes", group: "Kestrel",
        names: ["快捷笔记", "快捷筆記", "Quick notes", "クイックノート", "Notas rápidas"],
        descs: ["打开快捷笔记", "開啟快捷筆記", "Open quick notes", "クイックノートを開く", "Abrir las notas rápidas"] },
];
