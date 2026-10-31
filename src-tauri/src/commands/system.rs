//! Window management, system actions, colour picking and Quick Switch.

use crate::state::AppState;
use crate::win::shell;
use crate::win::window::{self, WindowInfo};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_windows(state: State<'_, Arc<AppState>>) -> Vec<WindowInfo> {
    window::list_windows(state.self_pid)
}

#[tauri::command]
pub fn activate_window(hwnd: isize) -> bool {
    window::activate(hwnd)
}

#[tauri::command]
pub fn close_window(hwnd: isize) {
    window::close(hwnd);
}

#[tauri::command]
pub fn minimize_window(hwnd: isize) {
    window::minimize(hwnd);
}

#[tauri::command]
pub fn toggle_topmost_window(hwnd: isize) -> bool {
    window::toggle_topmost(hwnd)
}

#[tauri::command]
pub fn snap_window(hwnd: isize, side: String) -> bool {
    window::snap(hwnd, &side)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorSample {
    pub hex: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub x: i32,
    pub y: i32,
}

#[tauri::command]
pub fn pick_color() -> ColorSample {
    let (x, y) = window::cursor_pos();
    let (r, g, b) = window::pixel_color(x, y);
    ColorSample {
        hex: format!("#{r:02X}{g:02X}{b:02X}"),
        r,
        g,
        b,
        x,
        y,
    }
}

#[link(name = "shell32")]
extern "system" {
    fn SHEmptyRecycleBinW(hwnd: isize, path: *const u16, flags: u32) -> i32;
}

fn run(id: &str) -> bool {
    match id {
        "cmd.settings" => shell::open_url("ms-settings:"),
        "cmd.control" => shell::spawn("control.exe", None, None),
        "cmd.taskmgr" => shell::spawn("taskmgr.exe", None, None),
        "cmd.terminal" => shell::spawn("wt.exe", None, None),
        "cmd.regedit" => shell::spawn("regedit.exe", None, None),
        "cmd.explorer" => shell::spawn("explorer.exe", None, None),
        "cmd.recycle" => shell::open_url("shell:RecycleBinFolder"),
        "cmd.emptyrecycle" => unsafe { SHEmptyRecycleBinW(0, std::ptr::null(), 1 | 2 | 4) == 0 },
        "cmd.lock" => shell::spawn("rundll32.exe", Some("user32.dll,LockWorkStation"), None),
        "cmd.sleep" => shell::spawn(
            "rundll32.exe",
            Some("powrprof.dll,SetSuspendState 0,1,0"),
            None,
        ),
        "cmd.shutdown" => shell::spawn("shutdown.exe", Some("/s /t 0"), None),
        "cmd.restart" => shell::spawn("shutdown.exe", Some("/r /t 0"), None),
        "cmd.logout" => shell::spawn("shutdown.exe", Some("/l"), None),
        "cmd.screenshot" => shell::open_url("ms-screenclip:"),
        "cmd.magnifier" => shell::spawn("magnify.exe", None, None),
        "cmd.network" => shell::spawn("control.exe", Some("ncpa.cpl"), None),
        "cmd.printers" => shell::spawn("control.exe", Some("printers"), None),
        "cmd.sound" => shell::open_url("ms-settings:sound"),
        "cmd.display" => shell::open_url("ms-settings:display"),
        "cmd.bluetooth" => shell::open_url("ms-settings:bluetooth"),
        "cmd.apps" => shell::open_url("ms-settings:appsfeatures"),
        "cmd.datetime" => shell::open_url("ms-settings:dateandtime"),
        "cmd.env" => shell::spawn(
            "rundll32.exe",
            Some("sysdm.cpl,EditEnvironmentVariables"),
            None,
        ),
        "cmd.device" => shell::spawn("mmc.exe", Some("devmgmt.msc"), None),
        "cmd.disk" => shell::spawn("mmc.exe", Some("diskmgmt.msc"), None),
        "cmd.services" => shell::spawn("mmc.exe", Some("services.msc"), None),
        _ => false,
    }
}

#[tauri::command]
pub fn run_builtin(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> bool {
    match id.as_str() {
        "cmd.settings.kestrel" => {
            crate::ui::open_settings(&app);
            true
        }
        "cmd.reindex" => {
            crate::commands::index::spawn_rebuild(app.clone(), state.inner().clone());
            true
        }
        other => {
            if let Some(cid) = other.strip_prefix("custom:") {
                let cmd = state
                    .settings
                    .read()
                    .custom_commands
                    .iter()
                    .find(|c| c.id == cid)
                    .cloned();
                let Some(c) = cmd else { return false };
                return match c.action.as_str() {
                    "kestrel" => {
                        crate::ui::open_settings(&app);
                        true
                    }
                    "url" => shell::open_url(&c.target),
                    "path" => shell::open_path(&c.target),
                    "shell" => shell::run_shell(&c.target, None),
                    "program" => shell::spawn(
                        &c.target,
                        if c.args.is_empty() {
                            None
                        } else {
                            Some(c.args.as_str())
                        },
                        None,
                    ),
                    _ => false,
                };
            }
            run(other)
        }
    }
}

/// Navigate the dialog to `path` (a folder) **without confirming it**.
/// Shared by the Quick Switch command and the automatic Explorer jump.
///
/// This used to write the path into the filename box and post `WM_COMMAND`
/// /IDOK. On many dialogs that dismissed them: Quick Switch was silently
/// answering "use this folder" on the user's behalf, when its whole job is
/// only to move the dialog somewhere else and let the user decide.
/// NOTE: keeps the Quick Switch session alive — the box stays attached until
/// the dialog itself is closed (the follower thread handles that).
pub fn apply_path_to_dialog(_state: &Arc<AppState>, dialog: isize, path: &str) -> bool {
    window::apply_folder(dialog, path)
}

/// Apply the Quick Switch: move the focused file dialog to the chosen folder,
/// or drop a chosen file's name into the filename box. Never presses OK.
#[tauri::command]
pub fn quick_switch_apply(state: State<'_, Arc<AppState>>, path: String) -> bool {
    let target = *state.dialog.lock();
    let Some(dialog) = target else {
        // No dialog captured: fall back to just opening the folder.
        return shell::open_path(&path);
    };
    if std::path::Path::new(&path).is_dir() {
        if apply_path_to_dialog(&state, dialog, &path) {
            return true;
        }
    } else if window::fill_filename(dialog, &path) {
        return true;
    }
    shell::open_path(&path)
}

#[tauri::command]
pub fn clear_dialog_target(state: State<'_, Arc<AppState>>) {
    *state.dialog.lock() = None;
}

#[tauri::command]
pub fn has_dialog_target(state: State<'_, Arc<AppState>>) -> bool {
    state
        .dialog
        .lock()
        .map(|h| window::is_quick_switch_target(h))
        .unwrap_or(false)
}

/// Where the Quick Switch box is standing: the folder the captured dialog is
/// showing, the user's home (for `~`), and the subfolders of the current one.
///
/// The box needs these to resolve typed paths (`..`, `\`, `D:`, `~`) and to
/// let arrow keys walk the tree. Reading them from the dialog every time would
/// mean a tree enumeration per keystroke, so the listing is cached until the
/// folder actually changes.
#[derive(Clone, Serialize)]
pub struct QsContext {
    pub folder: Option<String>,
    pub home: String,
    pub children: Vec<String>,
}

static QS_CHILDREN: std::sync::OnceLock<std::sync::Mutex<(String, Vec<String>)>> =
    std::sync::OnceLock::new();

fn child_dirs(folder: &str) -> Vec<String> {
    if let Some(lock) = QS_CHILDREN.get() {
        if let Ok(g) = lock.lock() {
            if g.0 == folder {
                return g.1.clone();
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(folder) {
        for e in rd.flatten() {
            // Symlinks to folders count; a broken one just yields nothing.
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                out.push(e.path().to_string_lossy().to_string());
            }
        }
    }
    // Case-insensitive: Windows folders are, and a list that puts "zoo" before
    // "Alpha" reads as broken.
    out.sort_by_key(|a| a.to_lowercase());
    // A folder with thousands of entries would make the box enumerate forever;
    // the user is navigating, not browsing a whole drive.
    if out.len() > 300 {
        out.truncate(300);
    }
    if let Some(lock) = QS_CHILDREN.get() {
        if let Ok(mut g) = lock.lock() {
            *g = (folder.to_string(), out.clone());
        }
    }
    out
}

#[tauri::command]
pub fn qs_context(state: State<'_, Arc<AppState>>) -> QsContext {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| std::env::var("USERPROFILE").unwrap_or_default());
    let folder = state
        .dialog
        .lock()
        .and_then(|h| window::dialog_current_folder(h));
    let children = folder.as_deref().map(child_dirs).unwrap_or_default();
    QsContext {
        folder,
        home,
        children,
    }
}

// ---------------- Watchdog service (安装为服务) ----------------

const WATCHDOG_TASK: &str = "KestrelWatchdog";

fn schtasks(args: &[&str]) -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("schtasks")
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether the watchdog task is registered.
#[tauri::command]
pub fn service_status() -> bool {
    schtasks(&["/Query", "/TN", WATCHDOG_TASK])
}

/// Register a logon task that keeps Kestrel alive (restarts after crashes
/// or forced kills), and start the watchdog immediately.
///
/// Returns `"ok"`, `"needsAdmin"` or `"failed"`. Creating a logon task is a
/// machine-level change, so `schtasks` answers "Access is denied" when the app
/// runs unelevated — that case is reported separately instead of being
/// collapsed into a bare `false`, because the UI has to *explain* it.
#[tauri::command]
pub fn service_install() -> String {
    if !crate::win::is_elevated() {
        return "needsAdmin".into();
    }
    let Ok(exe) = std::env::current_exe() else {
        return "failed".into();
    };
    let tr = format!("\"{}\" --watchdog", exe.display());
    if !schtasks(&[
        "/Create",
        "/F",
        "/SC",
        "ONLOGON",
        "/RL",
        "LIMITED",
        "/TN",
        WATCHDOG_TASK,
        "/TR",
        &tr,
    ]) {
        crate::debug_log::qs_log("service: schtasks /Create failed while elevated");
        return "failed".into();
    }
    crate::watchdog::clear_stop_flag();
    crate::win::shell::spawn(&exe.to_string_lossy(), Some("--watchdog"), None);
    "ok".into()
}

/// Remove the watchdog task and tell the running watchdog to stop.
#[tauri::command]
pub fn service_uninstall() -> String {
    if !crate::win::is_elevated() {
        return "needsAdmin".into();
    }
    let deleted = schtasks(&["/Delete", "/F", "/TN", WATCHDOG_TASK]);
    crate::watchdog::mark_stop();
    // "Nothing was registered" also counts as success, otherwise the toggle
    // would stick on after cleaning up a task that was already gone.
    if deleted || !service_status() {
        "ok".into()
    } else {
        "failed".into()
    }
}
