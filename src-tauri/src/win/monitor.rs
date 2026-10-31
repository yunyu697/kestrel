//! Foreground monitor: detects file dialogs and Explorer windows, drives the
//! attached Quick Switch box (Listary-style).
//!
//! - When a file dialog becomes foreground, the launcher window is shown and
//!   glued right below it, showing the folder visit history.
//! - When the user visits an Explorer window and returns to the dialog, the
//!   dialog automatically jumps to that folder.
//! - A follower thread keeps the box aligned while the dialog moves and hides
//!   it when the dialog loses focus or closes.

use crate::state::AppState;
use crate::win::{class_name, EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, GetWindowRect, BOOL, HWND, LPARAM, MSG, RECT};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, WebviewWindow};

use super::{window, DispatchMessageW, GetMessageW, TranslateMessage};

static CTX: Mutex<Option<(AppHandle, Arc<AppState>)>> = Mutex::new(None);

/// Foreground events are handed to a single worker instead of one thread per
/// event. With a thread per event the "Explorer activated" and "dialog
/// activated" handlers raced: when the dialog's handler won, the Explorer
/// folder had not been recorded yet and the jump was silently dropped — the
/// user saw "the first switch does nothing, do it again". Serialising makes
/// the ordering the same as the event ordering.
static QUEUE: std::sync::OnceLock<std::sync::mpsc::Sender<HWND>> =
    std::sync::OnceLock::new();

/// Last Explorer window we resolved a folder for. Used as a fallback when the
/// pending record is missing (event coalesced / hook respawned mid-switch).
static LAST_EXPLORER: Mutex<Option<(HWND, u64)>> = Mutex::new(None);

/// Last jump we performed. Applying a path calls `activate(dialog)`, which
/// fires *another* foreground event for the same dialog; without this guard
/// that echo re-applied the jump (the dialog navigated twice and the filename
/// field was refilled right after we cleared it).
static LAST_APPLIED: Mutex<Option<(HWND, u64)>> = Mutex::new(None);

fn now_ms() -> u64 {
    static START: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);
    // +1 keeps 0 out of the timestamp space (0 would read as "unset").
    START.elapsed().as_millis() as u64 + 1
}

const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;

type HWINEVENTHOOK = isize;

pub type Wineventproc =
    extern "system" fn(hook: HWINEVENTHOOK, event: u32, hwnd: HWND, idobject: i32, idchild: i32, idthread: u32, dwmseventtime: u32);

#[link(name = "user32")]
extern "system" {
    fn SetWinEventHook(
        eventmin: u32,
        eventmax: u32,
        hmodwineventproc: isize,
        pfnwineventproc: Wineventproc,
        idprocess: u32,
        idthread: u32,
        dwflags: u32,
    ) -> HWINEVENTHOOK;
    fn UnhookWinEvent(hwineventhook: HWINEVENTHOOK) -> i32;
}

/// Install the foreground monitor and the Quick Switch follower thread.
pub fn start(app: AppHandle, state: Arc<AppState>) {
    *CTX.lock() = Some((app.clone(), state.clone()));

    // Seed the history with common user folders on first run.
    if state.qs_history.lock().is_empty() {
        for name in ["Desktop", "Downloads", "Documents", "Pictures"] {
            if let Some(base) = dirs::home_dir() {
                let p = base.join(name);
                if p.is_dir() {
                    state.push_qs_dir(p.to_string_lossy().to_string());
                }
            }
        }
    }

    let (tx, rx) = std::sync::mpsc::channel::<HWND>();
    let _ = QUEUE.set(tx);
    std::thread::spawn(move || {
        while let Ok(hwnd) = rx.recv() {
            let Some((app, state)) = CTX.lock().clone() else { continue };
            // A panic in one classification must not take the whole pipeline
            // down: every later foreground event would be lost.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handle_foreground(&app, &state, hwnd);
            }));
        }
    });

    crate::supervise_forever("kestrel-fg-monitor", move || unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            0,
            win_event_proc,
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        crate::debug_log::qs_log(&format!("fg-monitor thread: hook={hook:#x}"));
        if hook == 0 {
            // Nothing to pump for; park so the supervisor does not spin.
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWinEvent(hook);
    });

    spawn_follower(app, state);
}

extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _idobject: i32,
    _idchild: i32,
    _idthread: u32,
    _time: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND || hwnd == 0 {
        return;
    }
    if CTX.lock().is_none() {
        return;
    }
    // Hand off to the serial worker: the hook thread must stay responsive, but
    // ordering matters (see QUEUE).
    if let Some(tx) = QUEUE.get() {
        let _ = tx.send(hwnd);
    }
}

fn handle_foreground(app: &AppHandle, state: &Arc<AppState>, hwnd: HWND) {
    // Bumped for every foreground event. A late-attach poller (see
    // `spawn_late_attach`) remembers the generation it belongs to, so as soon
    // as the user activates anything else its work becomes stale.
    let gen = LATE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    // Our own windows (launcher/settings) never reset Quick Switch state.
    if pid == state.self_pid || pid == 0 {
        crate::debug_log::qs_log(&format!("fg hwnd={hwnd:#x} pid={pid} self -> skip"));
        return;
    }
    let class = class_name(hwnd);
    crate::debug_log::qs_log(&format!("fg hwnd={hwnd:#x} pid={pid} class={class:?}"));

    // Explorer window: remember its folder (powers history + auto jump).
    if is_explorer_class(&class) {
        // Win11 reports the XAML island that hosts the tab strip / navigation
        // pane as the foreground window whenever the user clicks that part of
        // an Explorer window. Attributing those activations to their
        // CabinetWClass sibling matters: ignoring them meant the *first*
        // Explorer visit produced no record at all.
        let target = if class == "CabinetWClass" || class == "ExploreWClass" {
            Some(hwnd)
        } else {
            topmost_explorer_window(pid)
        };
        let Some(target) = target else {
            crate::debug_log::qs_log(&format!(
                "explorer island {hwnd:#x}: no CabinetWClass sibling for pid {pid}"
            ));
            return;
        };
        match explorer_location(target) {
            Some((_, path)) => {
                // Libraries, search results and shell: folders resolve to a
                // .library-ms / .search-ms pseudo path. Writing one of those
                // into a dialog navigates nowhere, so the jump looked broken.
                if std::path::Path::new(&path).is_dir() {
                    crate::debug_log::qs_log(&format!("explorer folder recorded: {path}"));
                    *state.explorer_path.lock() = Some((path.clone(), now_ms()));
                    *LAST_EXPLORER.lock() = Some((target, now_ms()));
                    state.push_qs_dir(path);
                } else {
                    crate::debug_log::qs_log(&format!(
                        "explorer location is not a folder, ignored: {path}"
                    ));
                }
            }
            None => {
                // Explorer was activated before its WebBrowser2 had a document
                // (typical right after `explorer.exe <path>`: the window is up
                // and foreground, the shell view is still loading). The folder
                // is unreadable *now*, but it will be readable a moment later.
                // Keeping the handle means maybe_auto_jump can resolve it
                // lazily when the user returns to the dialog — that is a
                // second or two later, which is exactly the window Explorer
                // needs. Dropping the handle here was why the *first* switch
                // after opening Explorer silently did nothing and only the
                // second one worked.
                // Drop any older record: this visit is the newest intent, and
                // a stale path would otherwise win over the lazy resolution.
                *state.explorer_path.lock() = None;
                *LAST_EXPLORER.lock() = Some((target, now_ms()));
                crate::debug_log::qs_log("explorer folder pending (resolved on return)");
            }
        }
        return;
    }

    // File dialog: attach the Quick Switch box below it.
    if class == "#32770" {
        let mut ready = window::is_file_dialog(hwnd);
        if !ready {
            // Dialogs are often still building their child controls when the
            // foreground event fires, and on a loaded machine they can take
            // well over a second to reach ShowWindow (measured: 1.5-2.3s).
            // Giving up here was the actual cause of "the first switch does
            // nothing": the dialog was dismissed as "not a file dialog", and
            // because a window *becoming* visible does not raise another
            // foreground event, there was no second chance — the user had to
            // click the dialog again to get the box to appear.
            for i in 0..8 {
                std::thread::sleep(Duration::from_millis(200));
                unsafe {
                    if IsWindow(hwnd) == 0 {
                        crate::debug_log::qs_log("dialog died during retry");
                        return;
                    }
                }
                ready = window::is_file_dialog(hwnd);
                if ready {
                    break;
                }
                // On screen but still not a file dialog: almost certainly a
                // message box (also #32770). Only stop once the window is
                // really visible — bailing out on a hidden window is what
                // killed slow file dialogs.
                if i >= 2 && unsafe { IsWindowVisible(hwnd) } != 0 {
                    break;
                }
            }
            if !ready {
                if unsafe { IsWindowVisible(hwnd) } != 0 {
                    crate::debug_log::qs_log("dialog #32770 not a file dialog (visible)");
                    return;
                }
                // Still hidden after ~1.6s: merely a slow dialog. Hand the
                // wait to a poller instead of blocking the serial event
                // queue, and let it re-enter this function once the dialog is
                // genuinely ready.
                crate::debug_log::qs_log("dialog #32770 still hidden -> late attach");
                spawn_late_attach(hwnd, gen);
                return;
            }
            // Dialog became ready late; only proceed if the user has not
            // already moved on. Comparing against the exact hwnd was too
            // strict: while a common dialog is still coming up, the
            // foreground window is often its owner or one of its own child
            // windows, and the attach was abandoned even though the dialog was
            // right there. Same process is the meaningful test.
            let fg = window::foreground();
            if fg != hwnd {
                let mut fg_pid = 0u32;
                unsafe { GetWindowThreadProcessId(fg, &mut fg_pid) };
                if fg_pid != pid {
                    crate::debug_log::qs_log(&format!(
                        "dialog ready but foreground moved to {fg:#x} pid={fg_pid} class={:?}",
                        class_name(fg)
                    ));
                    return;
                }
            }
        }

        // Dialogs are created hidden and shown a moment later. Waiting briefly
        // gives us the real rect (a hidden window can still report a stale
        // one); if it never shows up we attach anyway and let the follower
        // place the box once the dialog is on screen.
        if !wait_dialog_visible(hwnd, Duration::from_millis(400)) {
            crate::debug_log::qs_log("dialog not visible yet -> attach anyway");
            if unsafe { IsWindow(hwnd) } == 0 {
                return;
            }
        }

        let already = *state.dialog.lock() == Some(hwnd);
        let visible = crate::ui::launcher_visible(state);
        *state.dialog.lock() = Some(hwnd);
        crate::debug_log::qs_log(&format!("dialog captured hwnd={hwnd:#x} already={already} visible={visible}"));
        if let Some(dir) = window::dialog_current_folder(hwnd) {
            state.push_qs_dir(dir);
        }
        // Read only the two flags we need: cloning the whole settings
        // (favorites, engines, commands) on every focus change was pure
        // waste.
        let (quick_switch_enabled, qs_auto_show) = {
            let s = state.settings.read();
            (s.quick_switch_enabled, s.qs_auto_show)
        };
        if !quick_switch_enabled {
            return;
        }
        if already && visible {
            // Same dialog already served: refresh position but NEVER steal
            // focus back — otherwise the dialog could never be closed.
            // Coming back to the dialog means Windows just raised it to the
            // top of the normal band, which pushes the box behind it: put the
            // box back one slot above, still in the same band.
            crate::ui::set_topmost(app, false);
            if let Some(win) = crate::ui::launcher(app) {
                let rows = state.settings.read().qs_visible_rows;
                let scale = crate::ui::launcher_scale(state);
                // Re-raise above the dialog without focus. Both windows are
                // plain normal-band windows, so this lands the box directly
                // above the dialog instead of in a separate topmost layer.
                if let Ok(h) = win.hwnd() {
                    crate::win::keep_above(h.0 as isize, hwnd);
                }
                position_for_dialog(win, hwnd, rows, scale);
            }
            // ...but do honour the "came back from Explorer" jump. Skipping it
            // here is what broke the feature: the box was already attached, so
            // the branch above returned before the jump was ever considered,
            // and returning to the dialog after visiting a folder did nothing.
            maybe_auto_jump(app, state, hwnd, true);
            return;
        }
        maybe_auto_jump(app, state, hwnd, already);
        if qs_auto_show {
            crate::ui::show_quick_switch(app, state, hwnd);
        }
        return;
    }

    // Other windows: leave the box alone. Its lifetime is bound to the
    // dialog's lifetime (managed by the follower thread), not to focus.
    //
    // Z-order needs no work here: the box is *not* topmost any more, so the
    // window the user just activated is raised above both it and the dialog —
    // they sink together, which is exactly "part of the dialog" behaviour.
    // (`set_topmost(false)` is kept as a cheap safety net for sessions that
    // started before this rule existed.)
    if state.qs_active.load(Ordering::Relaxed) {
        crate::ui::set_topmost(app, false);
    }
}

/// Generation counter bumped by every foreground event. A late-attach poller
/// remembers the generation it was started for and abandons its work as soon as
/// the user activates something else, so a stale poll can never pop the box on
/// top of an unrelated window.
static LATE_GEN: AtomicU64 = AtomicU64::new(0);

/// Retry attaching to a dialog that was still hidden when its foreground event
/// arrived.
///
/// Windows sends exactly one `EVENT_SYSTEM_FOREGROUND` per activation and
/// showing the window later produces none, so a dialog slower than our inline
/// wait would otherwise never get the box — the user had to click it again.
/// Polling here stays off the serial event queue, so other foreground events
/// are not delayed while we wait.
fn spawn_late_attach(hwnd: HWND, gen: u64) {
    std::thread::spawn(move || {
        // ~8s is far beyond any real dialog's cold start, but bounded so a
        // zombie window cannot keep a poller alive forever.
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(200));
            if LATE_GEN.load(Ordering::SeqCst) != gen {
                crate::debug_log::qs_log("late attach: superseded by newer focus");
                return;
            }
            unsafe {
                if IsWindow(hwnd) == 0 {
                    crate::debug_log::qs_log("late attach: dialog died");
                    return;
                }
                if IsWindowVisible(hwnd) == 0 {
                    continue;
                }
            }
            if !window::is_file_dialog(hwnd) {
                continue;
            }
            // Only attach while the dialog still owns the foreground: the user
            // may have switched away and back inside our polling window.
            let mut fg_pid = 0u32;
            let mut dlg_pid = 0u32;
            let fg = window::foreground();
            unsafe {
                GetWindowThreadProcessId(fg, &mut fg_pid);
                GetWindowThreadProcessId(hwnd, &mut dlg_pid);
            }
            if fg_pid != dlg_pid {
                crate::debug_log::qs_log("late attach: dialog no longer foreground");
                return;
            }
            crate::debug_log::qs_log(&format!("late attach: dialog {hwnd:#x} ready"));
            // Re-enter the normal path now that the dialog is ready: it skips
            // the wait branch and attaches/follows exactly like a fast dialog.
            if let Some((app, state)) = CTX.lock().clone() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handle_foreground(&app, &state, hwnd);
                }));
            }
            return;
        }
        crate::debug_log::qs_log("late attach: gave up waiting for dialog");
    });
}

/// Window classes that mean "the user just activated an Explorer window".
fn is_explorer_class(class: &str) -> bool {
    matches!(class, "CabinetWClass" | "ExploreWClass")
        || class.starts_with("XamlExplorerHostIslandWindow")
}

static SCAN_PID: AtomicU32 = AtomicU32::new(0);
static SCAN_HIT: AtomicIsize = AtomicIsize::new(0);

extern "system" fn scan_explorer_cb(hwnd: HWND, _l: LPARAM) -> BOOL {
    if SCAN_HIT.load(Ordering::Relaxed) != 0 {
        return 0; // already found: stop
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid != SCAN_PID.load(Ordering::Relaxed) {
        return 1;
    }
    if class_name(hwnd) != "CabinetWClass" {
        return 1;
    }
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    SCAN_HIT.store(hwnd, Ordering::Relaxed);
    0 // stop enumerating
}

/// Topmost visible Explorer window belonging to `pid`.
///
/// EnumWindows walks in z-order, so the first match is the one the user is
/// actually looking at.
fn topmost_explorer_window(pid: u32) -> Option<HWND> {
    if pid == 0 {
        return None;
    }
    SCAN_PID.store(pid, Ordering::Relaxed);
    SCAN_HIT.store(0, Ordering::Relaxed);
    unsafe { EnumWindows(scan_explorer_cb, 0) };
    let hit = SCAN_HIT.load(Ordering::Relaxed);
    if hit != 0 {
        Some(hit)
    } else {
        None
    }
}

/// Listary-style auto jump: returning to a dialog right after visiting an
/// Explorer window navigates the dialog to that Explorer folder.
///
/// `allow_fallback` is only true when the dialog was already attached, i.e.
/// this really is a "came back to the dialog" event. Using the last-seen
/// Explorer folder on a *fresh* dialog would teleport the user somewhere they
/// never asked for.
fn maybe_auto_jump(app: &AppHandle, state: &Arc<AppState>, dialog: HWND, allow_fallback: bool) {
    let (enabled, auto_jump) = {
        let s = state.settings.read();
        (s.quick_switch_enabled, s.qs_auto_jump)
    };
    if !enabled || !auto_jump {
        crate::debug_log::qs_log("auto-jump skipped: disabled");
        return;
    }
    // Our own activate() call echoes back as a foreground event for the same
    // dialog: ignore it instead of applying the jump a second time.
    if let Some((h, ts)) = *LAST_APPLIED.lock() {
        if h == dialog && now_ms().saturating_sub(ts) < 800 {
            crate::debug_log::qs_log("auto-jump skipped: echo of our own activate");
            return;
        }
    }
    // Take the record instead of cloning it: one Explorer visit may power
    // exactly one jump. Keeping it around meant every later focus change
    // re-applied the same folder (and made the feature feel random).
    let pending = state.explorer_path.lock().take();
    let (path, ts) = match pending {
        Some(v) => v,
        None => {
            if !allow_fallback {
                crate::debug_log::qs_log("auto-jump skipped: no explorer record");
                return;
            }
            // The record can be missing even though the user did visit an
            // Explorer window (event coalesced, hook respawned, or the visit
            // happened while the queue was busy). Asking that window directly
            // turns "the first switch does nothing" into a working jump.
            // One-shot: consumed here, so it can never power two jumps.
            let Some((h, ts)) = LAST_EXPLORER.lock().take() else {
                crate::debug_log::qs_log("auto-jump skipped: no explorer record, no fallback");
                return;
            };
            if unsafe { IsWindow(h) } == 0 {
                crate::debug_log::qs_log("auto-jump skipped: fallback window gone");
                return;
            }
            match explorer_location(h) {
                Some((_, p)) if std::path::Path::new(&p).is_dir() => {
                    crate::debug_log::qs_log(&format!("auto-jump fallback: {p}"));
                    (p, ts)
                }
                Some((_, p)) => {
                    crate::debug_log::qs_log(&format!("auto-jump fallback: not a folder, {p}"));
                    return;
                }
                None => {
                    crate::debug_log::qs_log("auto-jump skipped: fallback has no folder");
                    return;
                }
            }
        }
    };
    crate::debug_log::qs_log(&format!(
        "auto-jump considering: path={path} age={}ms",
        now_ms().saturating_sub(ts)
    ));
    // Generous, because the user may well browse around before coming back —
    // the record is consumed on first use, so a stale jump cannot pile up.
    if now_ms().saturating_sub(ts) > 30_000 {
        crate::debug_log::qs_log("auto-jump skipped: too old");
        return;
    }
    if let Some(cur) = window::dialog_current_folder(dialog) {
        if cur.eq_ignore_ascii_case(&path) {
            crate::debug_log::qs_log(&format!("auto-jump skipped: already at {path}"));
            return;
        }
    }
    crate::debug_log::qs_log(&format!("auto-jump: dialog {dialog:#x} -> {path}"));
    let applied = crate::commands::system::apply_path_to_dialog(state, dialog, &path);
    if applied {
        *LAST_APPLIED.lock() = Some((dialog, now_ms()));
        // This Explorer visit has been spent; the fallback must not replay it.
        *LAST_EXPLORER.lock() = None;
        // The dialog moved by itself. Without this the box keeps resolving
        // `..` and half-typed names against the folder it has already left.
        let _ = app.emit("quickswitch://moved", ());
    } else if window::dialog_closing(dialog) {
        // The dialog the jump was meant for is going away — the user accepted
        // or cancelled it in the very instant the jump ran. Retrying a closed
        // dialog is pointless, and keeping the record would spend it on the
        // next dialog that gets focus, which is a completely unrelated one.
        crate::debug_log::qs_log("auto-jump: dialog closed, record dropped");
    } else {
        // Put it back so the next focus change retries instead of the user
        // having to redo the whole Explorer round trip.
        crate::debug_log::qs_log("auto-jump: apply failed, record kept for retry");
        *state.explorer_path.lock() = Some((path, ts));
    }
}

// ---------------- Explorer location via Shell COM ----------------

/// Active folder of an Explorer window, via IShellWindows / IWebBrowser2.
fn explorer_location(target: HWND) -> Option<(isize, String)> {
    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};
    use windows::Win32::UI::Shell::{IWebBrowser2, IShellWindows, ShellWindows};

    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let inited = hr.is_ok();
        if !inited && hr.0 != 0x8001_0106u32 as i32 {
            // RPC_E_CHANGED_MODE means COM is already up in another mode: fine.
            return None;
        }
        let result = (|| {
            let sw: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER).ok()?;
            let n = sw.Count().unwrap_or(0);
            for i in 0..n.min(64) {
                let var = VARIANT {
                    Anonymous: VARIANT_0 {
                        Anonymous: core::mem::ManuallyDrop::new(VARIANT_0_0 {
                            vt: VT_I4,
                            wReserved1: 0,
                            wReserved2: 0,
                            wReserved3: 0,
                            Anonymous: VARIANT_0_0_0 { lVal: i },
                        }),
                    },
                };
                let Ok(disp) = sw.Item(&var) else { continue };
                let Ok(wb) = disp.cast::<IWebBrowser2>() else { continue };
                let Ok(h) = wb.HWND() else { continue };
                let hwnd = h.0 as isize;
                if hwnd != target {
                    continue;
                }
                let url = wb.LocationURL().map(|s| s.to_string()).unwrap_or_default();
                let p = file_url_to_path(&url);
                if !p.is_empty() {
                    return Some((hwnd, p));
                }
            }
            None
        })();
        if inited {
            CoUninitialize();
        }
        result
    }
}

/// "file:///C:/My%20Docs" -> "C:\My Docs"
fn file_url_to_path(url: &str) -> String {
    let rest = match url.strip_prefix("file:///") {
        Some(r) => r,
        None => return String::new(),
    };
    let b = rest.as_bytes();
    let mut bytes = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
            {
                bytes.push(v);
                i += 3;
                continue;
            }
        }
        bytes.push(b[i]);
        i += 1;
    }
    let s = String::from_utf8_lossy(&bytes).replace('/', "\\");
    if s.len() >= 2 && s.as_bytes()[1] == b':' {
        s
    } else {
        String::new()
    }
}

// ---------------- Follower + positioning ----------------

/// Compute the geometry (x, y, width, height) of the box glued to the
/// bottom-center of the dialog. Width is two thirds of the dialog width;
/// height shows `rows` history records. Clamped to the visible work area.
///
/// `scale` is the cached device pixel ratio: reading it from the window would
/// be a round trip to the UI thread, and this runs on every follower tick.
pub fn compute_qs_geometry(hwnd: HWND, rows: u32, scale: f64) -> Option<(i32, i32, u32, u32)> {
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return None;
    }
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let wa = window::monitor_work_area(hwnd);
    let wa_w = (wa.2 - wa.0).max(320);
    let dlg_w = rect.right - rect.left;
    let width = ((dlg_w * 2) / 3).clamp(320, wa_w);
    // Exact height from the stylesheet: bar(58) + hairline(1) + section
    // header(28) + rows*46 + footer(34) + 6px gap, in logical pixels.
    // Forgetting the section header is what sliced the last row in half.
    let chrome = 58.0 + 1.0 + 28.0 + 34.0 + 6.0;
    let h_full = ((chrome + 46.0 * rows.max(1) as f64) * scale) as i32;
    // Minimum height that still shows exactly one result row.
    let h_min = ((chrome + 46.0) * scale) as i32;
    // Never taller than the work area itself.
    let h_full = h_full.min((wa.3 - wa.1).max(h_min));
    let gap = (6.0 * scale) as i32;
    // Shrinking to the exact remaining space left the footer bar sliced in
    // half. Quantise to whole result rows instead: bar + hairline + n rows +
    // footer always fits, so the last row is never clipped.
    let row_px = 46.0 * scale;
    let chrome_px = chrome * scale;
    let snap_rows = |avail: i32| -> i32 {
        let n = ((avail as f64 - chrome_px) / row_px).floor().max(1.0);
        (chrome_px + row_px * n) as i32
    };
    // Bottom-center anchor.
    let mut x = rect.left + (dlg_w - width) / 2;
    x = x.clamp(wa.0, (wa.2 - width).max(wa.0));

    // Edge policy (in order of preference):
    //  1. below the dialog, full height
    //  2. below, shrunk to whole rows (down to one visible row)
    //  3. above the dialog, full height
    //  4. above, shrunk to whole rows (still glued to the dialog)
    //  5. only then overlap the dialog, aligned to the work-area top.
    let y_below = rect.bottom + gap;
    let space_below = wa.3 - y_below;
    if space_below >= h_full {
        return Some((x, y_below, width as u32, h_full as u32));
    }
    if space_below >= h_min {
        let h = snap_rows(space_below).min(space_below).max(h_min);
        return Some((x, y_below, width as u32, h as u32));
    }
    let space_above = rect.top - gap - wa.1;
    if space_above >= h_full {
        let y = rect.top - gap - h_full;
        return Some((x, y, width as u32, h_full as u32));
    }
    if space_above >= h_min {
        let h = snap_rows(space_above).min(space_above).max(h_min);
        return Some((x, rect.top - gap - h, width as u32, h as u32));
    }
    // Overlap the dialog, flush with the top of the work area.
    Some((x, wa.1, width as u32, h_full as u32))
}

/// Apply a computed geometry unconditionally (session start / re-attach).
pub fn position_for_dialog(win: WebviewWindow, hwnd: HWND, rows: u32, scale: f64) {
    let Some((x, y, w, h)) = compute_qs_geometry(hwnd, rows, scale) else {
        return;
    };
    // The attached box is smaller than the normal launcher: relax the min size
    // so it can shrink down to a single result row when an edge is hit.
    let _ = win.set_min_size(Some(tauri::LogicalSize::new(260.0, 140.0)));
    let _ = win.set_size(tauri::PhysicalSize::new(w, h));
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Wait until a dialog window is actually on screen. Dialogs are created
/// hidden and only shown a moment later, so attaching while hidden gives a
/// stale rect — and used to make the box disappear instantly.
fn wait_dialog_visible(hwnd: HWND, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        unsafe {
            if IsWindow(hwnd) == 0 {
                return false;
            }
            if IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0 {
                return true;
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Drop the attached session and get the box off screen.
fn end_session(app: &AppHandle, state: &Arc<AppState>) {
    state
        .qs_active
        .store(false, std::sync::atomic::Ordering::Relaxed);
    *state.dialog.lock() = None;
    if crate::ui::launcher_visible(state) {
        crate::ui::hide_launcher(app);
    }
}

fn spawn_follower(app: AppHandle, state: Arc<AppState>) {
    crate::supervise_forever("kestrel-qs-follow", move || {
        // Last applied geometry: (x, y, width, height).
        let mut last: (i32, i32, u32, u32) = (0, 0, 0, 0);
        // Last seen dialog rectangle. As long as it does not change there is
        // nothing to follow: recomputing the geometry every tick used to ask
        // the UI thread for the scale factor 60 times a second, which starved
        // the WebView and showed up as stutter.
        let mut last_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut last_rows = u32::MAX;
        // Z-order watchdog throttle (see the loop body).
        let mut last_z = Instant::now() - Duration::from_secs(1);
        // A dialog that exists but is not on screen is usually a freshly
        // created one (shown a few hundred ms later) or one that is closing
        // (hidden just before it is destroyed). Both look identical, so wait a
        // short grace period instead of killing the session — killing it
        // immediately made the box flash and vanish on every dialog open.
        let mut hidden_since: Option<Instant> = None;
        // Set when the box is pulled off screen because the dialog was
        // minimised, so it can be restored together with the dialog.
        let mut hidden_for_iconic = false;
        loop {
                std::thread::sleep(Duration::from_millis(16));
                let Some(win) = crate::ui::launcher(&app) else { continue };
                let Some(h) = *state.dialog.lock() else {
                    hidden_since = None;
                    hidden_for_iconic = false;
                    last = (0, 0, 0, 0);
                    last_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    last_rows = u32::MAX;
                    continue;
                };

                let exists = unsafe { IsWindow(h) != 0 };
                let on_screen = exists && unsafe { IsWindowVisible(h) != 0 };
                let iconic = exists && unsafe { IsIconic(h) != 0 };

                // Really destroyed: the dialog is gone, so is the box.
                if !exists {
                    crate::debug_log::qs_log(&format!("follower: dialog {h:#x} destroyed -> hide"));
                    end_session(&app, &state);
                    hidden_since = None;
                    hidden_for_iconic = false;
                    last = (0, 0, 0, 0);
                    last_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    last_rows = u32::MAX;
                    continue;
                }

                // Minimised: get out of the way, but keep the binding so the
                // box returns when the dialog is restored.
                if iconic {
                    hidden_since = None;
                    if crate::ui::launcher_visible(&state) {
                        hidden_for_iconic = true;
                        crate::ui::hide_launcher(&app);
                    }
                    continue;
                }

                // Exists but not on screen yet (or already fading out).
                if !on_screen {
                    let since = *hidden_since.get_or_insert_with(Instant::now);
                    if since.elapsed() > Duration::from_millis(1500) {
                        crate::debug_log::qs_log(&format!(
                            "follower: dialog {h:#x} hidden for {}ms -> hide",
                            since.elapsed().as_millis()
                        ));
                        end_session(&app, &state);
                        hidden_since = None;
                        hidden_for_iconic = false;
                        last = (0, 0, 0, 0);
                        last_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                        last_rows = u32::MAX;
                    }
                    continue;
                }
                hidden_since = None;

                // Restored from the taskbar: bring the box back.
                if hidden_for_iconic {
                    hidden_for_iconic = false;
                    last = (0, 0, 0, 0);
                    last_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    last_rows = u32::MAX;
                    crate::ui::show_quick_switch(&app, &state, h);
                    continue;
                }

                if !crate::ui::launcher_visible(&state) {
                    continue;
                }

                // Z-order watchdog.
                //
                // Every activation of the dialog (clicking its file list is
                // enough) raises it to the top of the normal band, which
                // silently pushes the box behind it — the box and the dialog
                // then look like two different layers. While the dialog is the
                // foreground window the box belongs one slot above it; when
                // anything else is in front, nothing has to happen: both sink
                // together.
                //
                // Throttled and conditional: an unconditional SetWindowPos on
                // every 16 ms tick made the box flicker.
                if last_z.elapsed() > Duration::from_millis(150) {
                    last_z = Instant::now();
                    if let Ok(qs) = win.hwnd() {
                        let qs = qs.0 as isize;
                        if unsafe { GetForegroundWindow() } == h
                            && crate::win::keep_above(qs, h)
                        {
                            crate::debug_log::qs_log("follower: z-order re-asserted above dialog");
                        }
                    }
                }

                // Follow the dialog: only touch the window when something
                // actually changed (repeated set_size / set_position cause
                // visible flicker).
                let rows = state.settings.read().qs_visible_rows;
                let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                let got_rect = unsafe { GetWindowRect(h, &mut rect) } != 0;
                let moved = !got_rect
                    || rect.left != last_rect.left
                    || rect.top != last_rect.top
                    || rect.right != last_rect.right
                    || rect.bottom != last_rect.bottom;
                if !moved && rows == last_rows && last != (0, 0, 0, 0) {
                    // The dialog did not move: nothing to recompute.
                    continue;
                }

                let scale = crate::ui::launcher_scale(&state);
                let Some((x, y, w, hh)) = compute_qs_geometry(h, rows, scale) else {
                    continue;
                };
                if (x, y, w, hh) != last {
                    crate::debug_log::qs_log(&format!("follower: apply geom ({x},{y},{w},{hh})"));
                    if w != last.2 || hh != last.3 {
                        let _ = win.set_size(tauri::PhysicalSize::new(w, hh));
                    }
                    if x != last.0 || y != last.1 {
                        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                    last = (x, y, w, hh);
                }
                last_rect = rect;
                last_rows = rows;
            }
        });
}
