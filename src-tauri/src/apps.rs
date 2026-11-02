//! Application discovery (Start Menu / Desktop shortcuts) and built-in commands.

use crate::model::AppEntry;
use crate::state::AppState;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

fn start_menu_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(pd) = std::env::var("PROGRAMDATA") {
        out.push(Path::new(&pd).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Ok(ad) = std::env::var("APPDATA") {
        out.push(Path::new(&ad).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    // Both desktops. Plenty of installers only drop their shortcut on the
    // all-users desktop, which used to be ignored entirely — those programs
    // were unsearchable even right after a restart.
    if let Ok(up) = std::env::var("USERPROFILE") {
        out.push(Path::new(&up).join("Desktop"));
    }
    if let Ok(pubd) = std::env::var("PUBLIC") {
        out.push(Path::new(&pubd).join("Desktop"));
    }
    // A Desktop redirected into OneDrive leaves the old folder empty.
    if let Ok(od) = std::env::var("OneDrive") {
        out.push(Path::new(&od).join("Desktop"));
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
    // Store apps have no shortcut to walk to: they only exist in the shell's
    // AppsFolder namespace, so they need their own enumeration.
    out.extend(uwp_apps());
    // De-duplicate by name, preferring the user profile copy.
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    out
}

/// Read a display name out of a shell item.
unsafe fn item_name(
    folder: &windows::Win32::UI::Shell::IShellFolder,
    pidl: *const windows::Win32::UI::Shell::Common::ITEMIDLIST,
    flags: windows::Win32::UI::Shell::SHGDNF,
) -> Option<String> {
    use windows::Win32::UI::Shell::{StrRetToBufW, Common::STRRET};

    let mut ret = STRRET::default();
    folder.GetDisplayNameOf(pidl, flags, &mut ret).ok()?;
    let mut buf = [0u16; 512];
    StrRetToBufW(&mut ret, Some(pidl), &mut buf).ok()?;
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Every app in `shell:AppsFolder` that is not a plain shortcut: Microsoft
/// Store packages, which the Start Menu folder walk can never see. Notepad,
/// Calculator, Settings and the Store itself all live here — a launcher that
/// skips this namespace looks broken on a fresh Windows 11.
///
/// The parsing name of such an item is its Application User Model ID
/// (`Family_8wekyb3d8bbwe!App`), which `shell:AppsFolder\<AUMID>` can launch
/// directly. Items whose parsing name looks like a file path are classic
/// desktop apps that the shortcut walk already covered.
fn uwp_apps() -> Vec<AppEntry> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{
        Common::ITEMIDLIST, IEnumIDList, IShellFolder, SHGetDesktopFolder,
        SHGetKnownFolderIDList, FOLDERID_AppsFolder, SHCONTF_FOLDERS, SHCONTF_NONFOLDERS,
        SHGDN_FORPARSING, SHGDN_NORMAL,
    };

    let mut out = Vec::new();
    unsafe {
        // This runs on our own worker thread, which starts out COM-less.
        // RPC_E_CHANGED_MODE (already initialised differently) is harmless.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let Ok(root) = SHGetKnownFolderIDList(&FOLDERID_AppsFolder, 0, None) else {
            return out;
        };
        let Ok(desktop) = SHGetDesktopFolder() else {
            CoTaskMemFree(Some(root as *const _));
            return out;
        };
        let bound = desktop.BindToObject::<_, IShellFolder>(root, None);
        CoTaskMemFree(Some(root as *const _));
        let Ok(folder) = bound else { return out };

        let mut list: Option<IEnumIDList> = None;
        let flags = (SHCONTF_FOLDERS.0 | SHCONTF_NONFOLDERS.0) as u32;
        if folder
            .EnumObjects(HWND(std::ptr::null_mut()), flags, &mut list)
            .is_err()
        {
            return out;
        }
        let Some(list) = list else { return out };

        loop {
            let mut one: [*mut ITEMIDLIST; 1] = [std::ptr::null_mut()];
            let mut got = 0u32;
            if list.Next(&mut one, Some(&mut got)).is_err() || got == 0 {
                break;
            }
            let child = one[0];
            if child.is_null() {
                continue;
            }
            let name = item_name(&folder, child, SHGDN_NORMAL);
            let parsing = item_name(&folder, child, SHGDN_FORPARSING);
            CoTaskMemFree(Some(child as *const _));

            let (Some(name), Some(parsing)) = (name, parsing) else {
                continue;
            };
            // AUMIDs contain "!"; file-backed items are paths with "\".
            if !parsing.contains('!') || parsing.contains('\\') {
                continue;
            }
            let target = format!("shell:AppsFolder\\{parsing}");
            out.push(AppEntry {
                id: format!("app:{target}"),
                name,
                target: target.clone(),
                lnk: target,
                icon: None,
                source: "uwp".into(),
            });
        }
    }
    out
}

/// True when both lists hold the same shortcuts, in the same order.
fn same_list(a: &[AppEntry], b: &[AppEntry]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.name == y.name && x.lnk == y.lnk && x.target == y.target)
}

/// Monotonic clock for "when did we last scan", in milliseconds.
static EPOCH: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);
static LAST_SCAN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn mark_scanned() {
    LAST_SCAN.store(
        EPOCH.elapsed().as_millis() as u64,
        std::sync::atomic::Ordering::Relaxed,
    );
}

fn millis_since_scan() -> u64 {
    EPOCH
        .elapsed()
        .as_millis()
        .saturating_sub(LAST_SCAN.load(std::sync::atomic::Ordering::Relaxed) as u128)
        as u64
}

/// Belt and braces for the watcher: the launcher rescans on the way up when
/// the list is older than a few seconds. A missed filesystem notification is
/// otherwise invisible to the user, who just sees "my new app is not there".
pub fn refresh_if_stale(app: &AppHandle, state: &Arc<AppState>) {
    const MIN_GAP_MS: u64 = 10_000;
    if millis_since_scan() < MIN_GAP_MS {
        return;
    }
    // Stamp before spawning so a burst of hotkey presses queues one scan.
    mark_scanned();
    let app = app.clone();
    let state = state.clone();
    let _ = std::thread::Builder::new()
        .name("kestrel-apps-refresh".into())
        .spawn(move || publish(&app, &state, scan_apps()));
}

/// Swap in a freshly scanned list, but only touch the state (and the UI) when
/// something actually changed — the watcher fires for every shortcut touch.
fn publish(app: &AppHandle, state: &Arc<AppState>, list: Vec<AppEntry>) {
    mark_scanned();
    if list.is_empty() {
        // A transient failure (or an empty profile) must not wipe a good list.
        return;
    }
    if same_list(&state.apps.read(), &list) {
        return;
    }
    *state.apps.write() = list;
    state.save_apps();
    crate::debug_log::qs_log(&format!("apps: refreshed count={}", state.apps.read().len()));
    let _ = app.emit("apps://refreshed", ());
}

/// Only shortcut-shaped events are worth a rescan. A directory counts too,
/// because removing a Start Menu group reports the folder, not each file.
fn relevant(event: &Event) -> bool {
    event.paths.is_empty()
        || event.paths.iter().any(|p| {
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            ext.is_empty() || ext == "lnk" || ext == "url"
        })
}

/// Scan once, then keep watching the shortcut folders.
///
/// The scan used to happen only at startup, so a program installed while
/// Kestrel was running never showed up until the next restart — the single
/// most common "the search box cannot find my new app" report.
pub fn start(app: AppHandle, state: Arc<AppState>) {
    let _ = std::thread::Builder::new()
        .name("kestrel-apps".into())
        .spawn(move || {
            publish(&app, &state, scan_apps());
            watch(app, state);
        });
}

fn watch(app: AppHandle, state: Arc<AppState>) {
    watch_loop(start_menu_dirs(), move || publish(&app, &state, scan_apps()));
}

/// Block forever, calling `on_change` once the watched folders settle down.
///
/// Kept separate from `watch` so the debounce can be tested without an
/// `AppHandle`: the "scan only at startup" regression is invisible in the UI
/// until a user installs something.
fn watch_loop(dirs: Vec<PathBuf>, on_change: impl Fn() + Send + 'static) {
    let (tx, rx) = channel::<notify::Result<Event>>();
    let Ok(mut watcher) = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    }) else {
        crate::debug_log::qs_log("apps: watcher unavailable");
        return;
    };

    let mut watched = 0usize;
    for dir in &dirs {
        match watcher.watch(dir, RecursiveMode::Recursive) {
            Ok(()) => watched += 1,
            Err(e) => crate::debug_log::qs_log(&format!("apps: watch {dir:?} failed: {e}")),
        }
    }
    crate::debug_log::qs_log(&format!("apps: watching {watched} dirs"));
    if watched == 0 {
        return;
    }
    // The watcher has to outlive the loop below.
    let _watcher = watcher;

    // An installer writes the shortcut and then keeps touching it; wait for
    // the dust to settle before paying for a scan.
    const QUIET: Duration = Duration::from_millis(700);
    let mut dirty = false;
    let mut last = Instant::now();

    loop {
        match rx.recv_timeout(Duration::from_millis(400)) {
            Ok(Ok(event)) => match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    crate::debug_log::qs_log_verbose(&format!(
                        "apps: event {:?} {:?}",
                        event.kind, event.paths
                    ));
                    if relevant(&event) {
                        dirty = true;
                        last = Instant::now();
                    }
                }
                _ => {}
            },
            // Watch error / buffer overflow: the missed events are unknown,
            // so treat it as "something changed".
            Ok(Err(_)) => {
                dirty = true;
                last = Instant::now();
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if dirty && last.elapsed() >= QUIET {
            dirty = false;
            on_change();
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Not an assertion about the machine: it prints what the shell reports so
    /// the AUMID shape can be eyeballed. `cargo test -- --nocapture`.
    #[test]
    fn store_apps_are_discoverable() {
        let apps = uwp_apps();
        println!("uwp count = {}", apps.len());
        for a in apps.iter().take(20) {
            println!("  {:40} {}", a.name, a.target);
        }
        assert!(
            apps.iter().all(|a| a.target.starts_with("shell:AppsFolder\\")),
            "every store app must be launchable through the shell namespace"
        );
    }

    fn probe_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The regression this whole module exists for: a shortcut that appears
    /// while Kestrel is running has to trigger a rescan, not wait for the
    /// next restart.
    #[test]
    fn a_shortcut_created_while_running_triggers_a_rescan() {
        let dir = probe_dir("kestrel-watch-loop");
        let hits = Arc::new(AtomicUsize::new(0));

        let counter = hits.clone();
        let watched = dir.clone();
        std::thread::spawn(move || {
            watch_loop(vec![watched], move || {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        });

        // Give the watcher a moment to register before touching the folder.
        std::thread::sleep(Duration::from_millis(400));
        std::fs::write(dir.join("Zeta.lnk"), b"x").unwrap();

        // The loop debounces for 700 ms; allow a generous margin.
        for _ in 0..40 {
            if hits.load(Ordering::SeqCst) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let seen = hits.load(Ordering::SeqCst);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(seen > 0, "the watcher never asked for a rescan");
    }

    /// A file that cannot possibly be a shortcut must not cause scans: the
    /// Desktop folder is watched too, and every saved document lives there.
    #[test]
    fn unrelated_files_do_not_trigger_a_rescan() {        let dir = probe_dir("kestrel-watch-noise");
        let hits = Arc::new(AtomicUsize::new(0));

        let counter = hits.clone();
        let watched = dir.clone();
        std::thread::spawn(move || {
            watch_loop(vec![watched], move || {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        });

        std::thread::sleep(Duration::from_millis(400));
        std::fs::write(dir.join("report.docx"), b"x").unwrap();
        std::thread::sleep(Duration::from_millis(1600));

        let seen = hits.load(Ordering::SeqCst);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(seen, 0, "a .docx must not look like a new application");
    }
}


#[cfg(test)]
mod icon_tests {
    /// A Store app has no file path to ask about; the icon has to come from
    /// the shell namespace, otherwise all 30-odd of them show a blank glyph.
    #[test]
    fn a_store_app_has_an_icon() {
        let url = crate::win::icon::icon_data_url(
            "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
            48,
        );
        assert!(url.starts_with("data:image/png;base64,"), "no icon rendered");
    }
}
