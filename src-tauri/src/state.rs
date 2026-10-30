//! Global application state.

use crate::index::{watch::WatcherHandle, Index};
use crate::model::{AppEntry, ClipboardItem, HistoryEntry, Settings};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct Paths {
    pub base: PathBuf,
}

impl Paths {
    pub fn new(base: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&base);
        Self { base }
    }
    pub fn settings(&self) -> PathBuf {
        self.base.join("settings.json")
    }
    pub fn index(&self) -> PathBuf {
        self.base.join("index.bin")
    }
    pub fn apps(&self) -> PathBuf {
        self.base.join("apps.json")
    }
    pub fn usage(&self) -> PathBuf {
        self.base.join("usage.json")
    }
    pub fn history(&self) -> PathBuf {
        self.base.join("history.json")
    }
    pub fn clipboard(&self) -> PathBuf {
        self.base.join("clipboard.json")
    }
    pub fn qs_dirs(&self) -> PathBuf {
        self.base.join("qsdirs.json")
    }
}

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub index: Arc<Index>,
    pub apps: RwLock<Vec<AppEntry>>,
    pub usage: RwLock<HashMap<String, u32>>,
    pub history: RwLock<Vec<HistoryEntry>>,
    pub clipboard: RwLock<Vec<ClipboardItem>>,
    pub watcher: Mutex<Option<WatcherHandle>>,
    pub paths: Paths,
    pub self_pid: u32,
    /// HWND of the file dialog we are currently serving (Quick Switch).
    pub dialog: Mutex<Option<isize>>,
    /// True while an attached Quick Switch session is running: the launcher
    /// window must NOT hide on blur (clicking the dialog steals focus).
    pub qs_active: std::sync::atomic::AtomicBool,
    /// Recent folders visited in file dialogs / Explorer windows (Quick Switch history).
    pub qs_history: Mutex<Vec<String>>,
    /// Path of the last focused Explorer window with a timestamp (ms, mono).
    pub explorer_path: Mutex<Option<(String, u64)>>,
    /// Whether the launcher window is on screen.
    ///
    /// Mirrors the real window state so background threads do not have to ask
    /// the UI thread: `WebviewWindow::is_visible()` is a synchronous round
    /// trip to the main thread, and the Quick Switch follower used it 60
    /// times a second — enough to compete with the WebView rendering and
    /// show up as stutter.
    pub launcher_visible: std::sync::atomic::AtomicBool,
    /// Cached device pixel ratio of the launcher window (f32 bits).
    /// `0` means "not measured yet".
    pub scale_bits: std::sync::atomic::AtomicU32,
}

impl AppState {
    pub fn new(base: PathBuf) -> Arc<Self> {
        let paths = Paths::new(base);
        let usage = load_json::<HashMap<String, u32>>(&paths.usage()).unwrap_or_default();
        let history = load_json::<Vec<HistoryEntry>>(&paths.history()).unwrap_or_default();
        let clipboard = load_json::<Vec<ClipboardItem>>(&paths.clipboard()).unwrap_or_default();
        let apps = load_json::<Vec<AppEntry>>(&paths.apps()).unwrap_or_default();
        let qs_history = load_json::<Vec<String>>(&paths.qs_dirs()).unwrap_or_default();
        let mut settings = load_json::<Settings>(&paths.settings()).unwrap_or_default();
        let mut settings_changed = false;
        if settings.custom_commands.is_empty() {
            // Seed the presets for existing installs too.
            settings.custom_commands = crate::model::preset_commands(&settings.language);
            settings_changed = true;
        }
        if !settings.custom_commands.iter().any(|c| c.id == "preset-settings") {
            // Migration: add the new "打开设置" preset.
            if let Some(p) = crate::model::preset_commands(&settings.language)
                .into_iter()
                .find(|c| c.id == "preset-settings")
            {
                settings.custom_commands.insert(0, p);
                settings_changed = true;
            }
        }
        // Migration: adopt the smarter default exclusions when the user
        // never changed them. The old defaults indexed node_modules, .git and
        // the whole Program Files tree, which produced a 500 MB index.
        if crate::model::uses_legacy_default_excludes(&settings.exclude_dirs) {
            settings.exclude_dirs = crate::model::default_excludes();
            settings_changed = true;
        }
        // Migration: the built-in folders are pinned and can never disappear.
        crate::model::normalize_favorites(&mut settings.favorites);
        let before = settings.favorites.len();
        crate::model::ensure_fixed_favorites(&mut settings.favorites, &settings.language);
        if settings.favorites.len() != before {
            settings_changed = true;
        }
        if settings_changed {
            save_json(&paths.settings(), &settings);
        }
        Arc::new(Self {
            settings: RwLock::new(settings),
            index: Index::new(),
            apps: RwLock::new(apps),
            usage: RwLock::new(usage),
            history: RwLock::new(history),
            clipboard: RwLock::new(clipboard),
            watcher: Mutex::new(None),
            paths,
            self_pid: std::process::id(),
            dialog: Mutex::new(None),
            qs_active: std::sync::atomic::AtomicBool::new(false),
            qs_history: Mutex::new(qs_history),
            explorer_path: Mutex::new(None),
            launcher_visible: std::sync::atomic::AtomicBool::new(false),
            scale_bits: std::sync::atomic::AtomicU32::new(0),
        })
    }

    pub fn save_settings(&self) {
        let s = self.settings.read().clone();
        save_json_pretty(&self.paths.settings(), &s);
    }

    pub fn save_usage(&self) {
        save_json(&self.paths.usage(), &*self.usage.read());
    }

    pub fn save_history(&self) {
        save_json(&self.paths.history(), &*self.history.read());
    }

    pub fn save_clipboard(&self) {
        save_json(&self.paths.clipboard(), &*self.clipboard.read());
    }

    pub fn save_apps(&self) {
        save_json(&self.paths.apps(), &*self.apps.read());
    }

    pub fn bump_usage(&self, key: &str) {
        {
            let mut u = self.usage.write();
            *u.entry(key.to_string()).or_insert(0) += 1;
        }
        self.save_usage();
    }

    /// Record a folder visit at the front of the Quick Switch history.
    pub fn push_qs_dir(&self, dir: String) {
        if dir.is_empty() || !dir.contains(':') {
            return;
        }
        let snapshot = {
            let mut h = self.qs_history.lock();
            h.retain(|d| !d.eq_ignore_ascii_case(&dir));
            h.insert(0, dir);
            h.truncate(12);
            h.clone()
        };
        save_json(&self.paths.qs_dirs(), &snapshot);
    }
}

pub fn load_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Write a JSON file.
///
/// Compact by default: the pretty printer inflates the app-data files (the
/// application list alone was 84 KB) for no benefit. Only the settings file
/// uses the pretty form, because people do read it.
pub fn save_json<T: serde::Serialize>(path: &PathBuf, value: &T) {
    write_json(path, value, false)
}

pub fn save_json_pretty<T: serde::Serialize>(path: &PathBuf, value: &T) {
    write_json(path, value, true)
}

fn write_json<T: serde::Serialize>(path: &PathBuf, value: &T, pretty: bool) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let raw = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    if let Ok(raw) = raw {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, raw).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}
