//! Application discovery (Start Menu / Desktop shortcuts) and built-in commands.

use crate::model::AppEntry;
use std::path::{Path, PathBuf};

fn start_menu_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(pd) = std::env::var("PROGRAMDATA") {
        out.push(Path::new(&pd).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Ok(ad) = std::env::var("APPDATA") {
        out.push(Path::new(&ad).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        out.push(Path::new(&up).join("Desktop"));
    }
    out.into_iter().filter(|p| p.exists()).collect()
}

fn walk(dir: &Path, out: &mut Vec<AppEntry>, depth: usize) {
    if depth > 4 || out.len() > 4000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            walk(&path, out, depth + 1);
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext != "lnk" && ext != "url" {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() || name.starts_with('~') {
            continue;
        }
        // Skip obvious uninstall / help shortcuts.
        let lower = name.to_lowercase();
        if lower.contains("卸载")
            || lower.contains("uninstall")
            || lower.contains("readme")
            || lower.contains("帮助")
            || lower.contains("help")
            || lower.contains("website")
            || lower.contains("官网")
        {
            continue;
        }
        let source = if dir.to_string_lossy().contains("Desktop") {
            "desktop"
        } else {
            "startmenu"
        };
        out.push(AppEntry {
            id: format!("app:{}", path.to_string_lossy()),
            name,
            target: if ext == "url" {
                read_url(&path).unwrap_or_default()
            } else {
                path.to_string_lossy().to_string()
            },
            lnk: path.to_string_lossy().to_string(),
            icon: None,
            source: source.to_string(),
        });
    }
}

fn read_url(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("URL=") {
            return Some(v.trim().to_string());
        }
    }
    None
}

pub fn scan_apps() -> Vec<AppEntry> {
    let mut out: Vec<AppEntry> = Vec::new();
    for dir in start_menu_dirs() {
        walk(&dir, &mut out, 0);
    }
    // De-duplicate by name, preferring the user profile copy.
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    out
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinCommand {
    pub id: String,
    pub name: String,
    pub keyword: String,
    pub group: String,
    pub icon: String,
    pub description: String,
}

/// Language-independent part of a built-in command: the search keywords
/// (latin + pinyin) and the icon name.
fn builtin_meta(id: &str) -> (&'static str, &'static str) {
    match id {
        "cmd.settings" => ("settings shezhi", "gear"),
        "cmd.control" => ("control kongzhi", "sliders"),
        "cmd.taskmgr" => ("taskmgr renwu", "activity"),
        "cmd.terminal" => ("terminal cmd powershell", "terminal"),
        "cmd.regedit" => ("regedit zhucebiao", "database"),
        "cmd.explorer" => ("explorer ziyuan", "folder"),
        "cmd.recycle" => ("recycle huishouzhan trash", "trash"),
        "cmd.emptyrecycle" => ("emptyrecycle qingkong", "trash"),
        "cmd.lock" => ("lock suoding", "lock"),
        "cmd.sleep" => ("sleep shuimian", "moon"),
        "cmd.shutdown" => ("shutdown guanji", "power"),
        "cmd.restart" => ("restart chongqi", "refresh"),
        "cmd.logout" => ("logout zhuxiao", "user"),
        "cmd.screenshot" => ("screenshot jietu snip", "camera"),
        "cmd.magnifier" => ("magnifier fangdajing", "search"),
        "cmd.onenote" => ("sketch caotu", "pen"),
        "cmd.network" => ("network wangluo", "wifi"),
        "cmd.printers" => ("printer dayinji", "printer"),
        "cmd.sound" => ("sound shengyin", "volume"),
        "cmd.display" => ("display xianshi", "monitor"),
        "cmd.bluetooth" => ("bluetooth lanya", "bluetooth"),
        "cmd.apps" => ("apps yingyong", "grid"),
        "cmd.datetime" => ("datetime shijian", "clock"),
        "cmd.env" => ("env huanjing", "code"),
        "cmd.device" => ("device shebei", "cpu"),
        "cmd.disk" => ("disk cipan", "hard-drive"),
        "cmd.services" => ("services fuwu", "server"),
        "cmd.about" => ("about guanyu", "info"),
        "cmd.settings.kestrel" => ("preferences settings shezhi", "gear"),
        "cmd.reindex" => ("reindex chongjian", "refresh"),
        "cmd.clipboard" => ("clipboard jiantieban", "clipboard"),
        "cmd.notes" => ("notes biji memo", "note"),
        _ => ("", "command"),
    }
}

/// Built-in commands, localised through `crate::i18n::BUILTINS`.
pub fn builtin_commands(lang: &str) -> Vec<BuiltinCommand> {
    let i = crate::i18n::index_of(lang);
    crate::i18n::BUILTINS
        .iter()
        .map(|b| {
            let (keyword, icon) = builtin_meta(b.id);
            BuiltinCommand {
                id: b.id.into(),
                name: b.names[i].into(),
                keyword: keyword.into(),
                group: crate::i18n::group_label(lang, b.group).into(),
                icon: icon.into(),
                description: b.descs[i].into(),
            }
        })
        .collect()
}
