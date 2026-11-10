//! Serializable data model shared with the frontend (camelCase on the wire).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    /// file | folder | app | command | calc | web | clipboard | window | favorite
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub took_ms: f64,
    pub total_matched: usize,
    pub query: String,
    pub hint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub is_dir: bool,
    /// A disabled favorite is hidden from the launcher results but stays in
    /// the list so it can be switched back on.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// One of the built-in folders: it can be disabled and re-pointed, but
    /// never removed.
    #[serde(default)]
    pub pinned: bool,
}

/// The built-in folders that are always offered as favorites.
pub const FIXED_FAVORITE_IDS: [&str; 6] = [
    "fav-desktop",
    "fav-downloads",
    "fav-documents",
    "fav-pictures",
    "fav-music",
    "fav-videos",
];

/// Canonical path of every built-in folder, paired with its stable id.
fn fixed_folder_paths() -> Vec<(String, &'static str)> {
    let candidates = [
        (FIXED_FAVORITE_IDS[0], dirs::desktop_dir()),
        (FIXED_FAVORITE_IDS[1], dirs::download_dir()),
        (FIXED_FAVORITE_IDS[2], dirs::document_dir()),
        (FIXED_FAVORITE_IDS[3], dirs::picture_dir()),
        (FIXED_FAVORITE_IDS[4], dirs::audio_dir()),
        (FIXED_FAVORITE_IDS[5], dirs::video_dir()),
    ];
    candidates
        .into_iter()
        .filter_map(|(id, d)| d.map(|d| (d.to_string_lossy().to_string(), id)))
        .collect()
}

/// Mark the built-in folders as pinned.
///
/// Runs on every load: installs created before the pinned flag existed still
/// get it (recognised by their path), and a folder the user re-pointed stays
/// pinned because it already carries a built-in id.
pub fn normalize_favorites(favs: &mut Vec<Favorite>) {
    let fixed_paths = fixed_folder_paths();
    let mut taken: Vec<String> = favs
        .iter()
        .filter(|f| FIXED_FAVORITE_IDS.contains(&f.id.as_str()))
        .map(|f| f.id.clone())
        .collect();

    for f in favs.iter_mut() {
        if FIXED_FAVORITE_IDS.contains(&f.id.as_str()) {
            f.pinned = true;
            continue;
        }
        // Legacy installs used positional ids (fav-0 … fav-5) for the
        // defaults — recognise them by their path and give them the stable id.
        let Some((_, canonical)) = fixed_paths
            .iter()
            .find(|(p, _)| p.eq_ignore_ascii_case(&f.path))
        else {
            continue;
        };
        f.pinned = true;
        if !taken.iter().any(|t| t == canonical) {
            f.id = canonical.to_string();
            taken.push(canonical.to_string());
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WebEngine {
    pub id: String,
    pub name: String,
    /// Keyword typed before the query, e.g. "g" -> `g rust`.
    pub keyword: String,
    /// `{query}` is replaced with the URL-encoded query.
    pub url: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CustomCommand {
    pub id: String,
    pub name: String,
    pub keyword: String,
    /// url | path | shell | program
    pub action: String,
    pub target: String,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

fn three() -> u32 {
    3
}

fn daily() -> String {
    "daily".into()
}

fn open_privilege_default() -> String {
    "normal".into()
}

/// Presets seeded into an empty custom-command list, in `lang`.
pub fn preset_commands(lang: &str) -> Vec<CustomCommand> {
    let i = crate::i18n::index_of(lang);
    crate::i18n::PRESETS
        .iter()
        .map(|p| CustomCommand {
            id: p.id.to_string(),
            name: p.names[i].to_string(),
            keyword: p.keyword.to_string(),
            action: p.action.to_string(),
            target: p.target.to_string(),
            args: String::new(),
            description: p.descs[i].to_string(),
            enabled: true,
        })
        .collect()
}

/// Re-translate seeded presets after a language switch.
///
/// A preset is only rewritten when its current name still matches the shipped
/// name in *some* language — i.e. the user never renamed it. Renamed presets
/// and user-created commands are left untouched.
pub fn relocalize_presets(commands: &mut [CustomCommand], lang: &str) -> bool {
    let i = crate::i18n::index_of(lang);
    let mut changed = false;
    for c in commands.iter_mut() {
        let Some(p) = crate::i18n::PRESETS.iter().find(|p| p.id == c.id) else {
            continue;
        };
        let untouched = p.names.iter().any(|n| !n.is_empty() && *n == c.name);
        if !untouched {
            continue;
        }
        let name = p.names[i].to_string();
        let desc = p.descs[i].to_string();
        if c.name != name {
            c.name = name;
            changed = true;
        }
        if c.description != desc {
            c.description = desc;
            changed = true;
        }
    }
    changed
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // ---- general ----
    pub launch_at_startup: bool,
    pub start_minimized: bool,
    pub close_after_open: bool,
    pub clear_query_on_hide: bool,
    pub remember_last_query: bool,
    pub single_click_open: bool,
    /// Relaunch elevated (UAC) so privileged actions work without a prompt.
    pub run_as_admin: bool,
    /// Privilege used for programs opened from the result list:
    /// "normal" (never inherit our elevation) | "inherit" | "admin" (always UAC).
    #[serde(default = "open_privilege_default")]
    pub open_privilege: String,
    pub language: String,

    // ---- updates ----
    /// How often to look for a new release: "off" | "daily" | "weekly".
    #[serde(default = "daily")]
    pub update_check: String,
    /// Also check right away on every launch (only when update_check != off).
    #[serde(default = "yes")]
    pub startup_check: bool,
    /// Download mirror prefix (e.g. "https://ghfast.top/"); empty = direct.
    #[serde(default)]
    pub update_mirror: String,

    // ---- appearance ----
    pub theme: String,
    pub accent: String,
    /// Colour scheme id from palettes.ts; empty = follow the theme. Legacy
    /// values ("auto", "custom", unknown ids) are migrated on the frontend.
    pub color_scheme: String,
    pub opacity: f64,
    pub blur: bool,
    pub corner_radius: f64,
    pub font_scale: f64,
    pub animations: bool,
    pub compact_mode: bool,
    pub show_preview: bool,
    pub preview_position: String,

    // ---- hotkeys ----
    pub hotkey_toggle: String,
    pub double_ctrl: bool,
    pub double_ctrl_ms: u32,
    pub hotkey_quick_switch: String,
    pub quick_switch_enabled: bool,
    /// Automatically attach the search box under a foreground file dialog.
    #[serde(default = "yes")]
    pub qs_auto_show: bool,
    /// Jump the dialog to the last Explorer folder when returning to it.
    #[serde(default = "yes")]
    pub qs_auto_jump: bool,
    /// How many history records the attached box shows at once.
    #[serde(default = "three")]
    pub qs_visible_rows: u32,
    /// Intercept Win+S / Win+Q (application-level keyboard hook) and open
    /// Kestrel instead of the Windows search panel.
    #[serde(default)]
    pub replace_win_search: bool,

    // ---- search & index ----
    pub max_results: u32,
    pub indexed_drives: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub index_hidden_files: bool,
    pub match_path: bool,
    pub pinyin_search: bool,
    pub fuzzy_search: bool,
    pub search_apps: bool,
    pub search_commands: bool,
    pub search_file_content: bool,
    pub history_enabled: bool,
    pub history_limit: u32,

    // ---- extras ----
    pub clipboard_enabled: bool,
    pub clipboard_limit: u32,
    pub clipboard_images: bool,
    pub calculator_enabled: bool,
    pub window_switcher_enabled: bool,
    pub quick_notes: String,

    // ---- collections ----
    pub favorites: Vec<Favorite>,
    pub web_engines: Vec<WebEngine>,
    pub custom_commands: Vec<CustomCommand>,
}

impl Default for Settings {
    fn default() -> Self {
        let drives = detect_drives();
        Self {
            launch_at_startup: false,
            start_minimized: false,
            close_after_open: true,
            clear_query_on_hide: true,
            remember_last_query: false,
            single_click_open: false,
            run_as_admin: false,
            open_privilege: open_privilege_default(),
            language: crate::i18n::detect_system().into(),

            update_check: "daily".into(),
            startup_check: true,
            update_mirror: String::new(),

            theme: "system".into(),
            accent: "#4C7DFF".into(),
            color_scheme: String::new(),
            opacity: 0.98,
            blur: true,
            corner_radius: 18.0,
            font_scale: 1.0,
            animations: true,
            compact_mode: false,
            show_preview: true,
            preview_position: "right".into(),

            hotkey_toggle: "Alt+Space".into(),
            double_ctrl: true,
            double_ctrl_ms: 350,
            hotkey_quick_switch: "Ctrl+G".into(),
            quick_switch_enabled: true,
            qs_auto_show: true,
            qs_auto_jump: true,
            qs_visible_rows: 3,
            replace_win_search: false,

            max_results: 60,
            indexed_drives: drives,
            exclude_dirs: default_excludes(),
            index_hidden_files: false,
            match_path: true,
            pinyin_search: true,
            fuzzy_search: true,
            search_apps: true,
            search_commands: true,
            search_file_content: false,
            history_enabled: true,
            history_limit: 200,

            clipboard_enabled: true,
            clipboard_limit: 200,
            clipboard_images: false,
            calculator_enabled: true,
            window_switcher_enabled: true,
            quick_notes: String::new(),

            favorites: default_favorites(crate::i18n::detect_system()),
            web_engines: default_engines(),
            custom_commands: preset_commands(crate::i18n::detect_system()),
        }
    }
}

/// Folders that are excluded from the index by default.
///
/// The old defaults only skipped `C:\Windows` and the recycle bin, so the
/// index walked every `node_modules`, every `.git` directory and the whole
/// Program Files tree — that alone produced a 500 MB index on a typical
/// machine. Entries without a drive letter match a path component anywhere
/// (see `index::scan`).
pub fn default_excludes() -> Vec<String> {
    vec![
        "C:\\Windows".into(),
        "C:\\Program Files".into(),
        "C:\\Program Files (x86)".into(),
        "C:\\ProgramData".into(),
        "C:\\$Recycle.Bin".into(),
        "C:\\System Volume Information".into(),
        "node_modules".into(),
        ".git".into(),
        ".svn".into(),
        ".hg".into(),
        ".venv".into(),
        "__pycache__".into(),
        ".gradle".into(),
        ".nuget".into(),
        ".cargo".into(),
        ".rustup".into(),
    ]
}

/// The exclusion list shipped before the smarter defaults above.
pub const LEGACY_DEFAULT_EXCLUDES: [&str; 3] = [
    "C:\\Windows",
    "C:\\$Recycle.Bin",
    "C:\\System Volume Information",
];

/// True when the stored exclusions are exactly the legacy defaults, i.e. the
/// user never touched them and the smarter defaults may be adopted.
pub fn uses_legacy_default_excludes(stored: &[String]) -> bool {
    if stored.len() != LEGACY_DEFAULT_EXCLUDES.len() {
        return false;
    }
    stored.iter().all(|s| {
        LEGACY_DEFAULT_EXCLUDES
            .iter()
            .any(|l| s.trim_end_matches('\\').eq_ignore_ascii_case(l))
    })
}

pub fn detect_drives() -> Vec<String> {
    let mut out = Vec::new();
    for letter in b'C'..=b'Z' {
        let root = format!("{}:\\", letter as char);
        if std::path::Path::new(&root).exists() {
            out.push(root);
        }
    }
    if out.is_empty() {
        out.push("C:\\".into());
    }
    out
}

/// Make sure every built-in folder is present.
///
/// Self-healing: an install that removed a default folder in an older version
/// gets it back, and the six fixed entries can never disappear from the list.
pub fn ensure_fixed_favorites(favs: &mut Vec<Favorite>, lang: &str) {
    let names = crate::i18n::favorite_names(lang);
    let candidates = [
        (FIXED_FAVORITE_IDS[0], names[0], dirs::desktop_dir()),
        (FIXED_FAVORITE_IDS[1], names[1], dirs::download_dir()),
        (FIXED_FAVORITE_IDS[2], names[2], dirs::document_dir()),
        (FIXED_FAVORITE_IDS[3], names[3], dirs::picture_dir()),
        (FIXED_FAVORITE_IDS[4], names[4], dirs::audio_dir()),
        (FIXED_FAVORITE_IDS[5], names[5], dirs::video_dir()),
    ];
    for (id, name, dir) in candidates {
        if favs.iter().any(|f| f.id == id) {
            continue;
        }
        let Some(d) = dir else { continue };
        favs.push(Favorite {
            id: id.to_string(),
            name: name.to_string(),
            path: d.to_string_lossy().to_string(),
            is_dir: true,
            enabled: true,
            pinned: true,
        });
    }
}

fn default_favorites(lang: &str) -> Vec<Favorite> {
    let mut out = Vec::new();
    ensure_fixed_favorites(&mut out, lang);
    out
}

fn default_engines() -> Vec<WebEngine> {
    vec![
        WebEngine {
            id: "bing".into(),
            name: "Bing".into(),
            keyword: "b".into(),
            url: "https://www.bing.com/search?q={query}".into(),
            enabled: true,
        },
        WebEngine {
            id: "google".into(),
            name: "Google".into(),
            keyword: "g".into(),
            url: "https://www.google.com/search?q={query}".into(),
            enabled: true,
        },
        WebEngine {
            id: "baidu".into(),
            name: "百度".into(),
            keyword: "bd".into(),
            url: "https://www.baidu.com/s?wd={query}".into(),
            enabled: true,
        },
        WebEngine {
            id: "github".into(),
            name: "GitHub".into(),
            keyword: "gh".into(),
            url: "https://github.com/search?q={query}".into(),
            enabled: true,
        },
        WebEngine {
            id: "wiki".into(),
            name: "维基百科".into(),
            keyword: "w".into(),
            url: "https://zh.wikipedia.org/wiki/Special:Search?search={query}".into(),
            enabled: true,
        },
        WebEngine {
            id: "bili".into(),
            name: "哔哩哔哩".into(),
            keyword: "bi".into(),
            url: "https://search.bilibili.com/all?keyword={query}".into(),
            enabled: true,
        },
    ]
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub name: String,
    pub version: String,
    pub build: u32,
    pub built_at: String,
    pub arch: String,
    pub os: String,
    pub index_count: usize,
    pub index_roots: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub state: String, // idle | scanning | ready
    pub count: usize,
    pub roots: Vec<String>,
    pub scanned: usize,
    pub current: String,
    pub last_built: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    pub kind: String, // text | image
    pub text: String,
    pub image: Option<String>,
    pub at: i64,
    pub pinned: bool,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub query: String,
    pub at: i64,
    pub count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub target: String,
    pub lnk: String,
    pub icon: Option<String>,
    pub source: String,
}
