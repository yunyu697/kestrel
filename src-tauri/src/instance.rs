//! One Kestrel per desktop session.
//!
//! Tauri ships a `single-instance` plugin, and it was doing this job until it
//! showed two holes that matter for a launcher sitting on a hotkey:
//!
//! * When the named mutex already exists but the first instance has **not
//!   created its message window yet**, `FindWindowW` comes back empty and the
//!   plugin's `if !hwnd.is_null()` has no `else` — the second copy falls
//!   straight through and starts as a real second instance. That window is
//!   only a few microseconds wide, but it is exactly what a fast double-click
//!   and a watchdog respawn racing a manual launch both land in.
//! * Its callback ran before `AppState` was managed, so a second launch during
//!   the first one's startup found `try_state` empty and did *nothing at all*:
//!   no window, no error, nothing.
//!
//! This guard is taken before Tauri is built, so a loser leaves before a
//! single WebView2 exists — no profile-lock panic, no crash dialog. A launch
//! that arrives mid-startup is remembered and answered once the UI is ready.
//!
//! The mutex is `Local\`, i.e. per session: two logged-in users may each run
//! their own copy, which is the behaviour a desktop tool should have.

use crate::state::AppState;
use crate::win::*;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::Manager;

const MUTEX_NAME: &str = "Local\\Kestrel-SingleInstance";
const CLASS_NAME: &str = "KestrelSingleInstanceWnd";
const WINDOW_NAME: &str = "KestrelSingleInstance";

/// Posted by a second launch: "you are the instance, show yourself".
const WM_KESTREL_SHOW: u32 = WM_APP + 1;

/// How long a second launch waits for the first one's message window. The
/// owner creates it within microseconds of taking the mutex, but a cold start
/// under a virus scanner can stretch that, and leaving without forwarding
/// would look like the double-click did nothing at all.
const FIND_TIMEOUT_MS: u64 = 4_000;
const FIND_POLL_MS: u64 = 40;

static WINDOW: AtomicIsize = AtomicIsize::new(0);
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
static PENDING_SHOW: AtomicBool = AtomicBool::new(false);

/// Take the instance slot.
///
/// Returns `false` when another copy owns it; the caller must then leave
/// without touching any UI. `successor` marks the elevated (or de-elevated)
/// hand-off, whose predecessor is still holding the mutex while it starts.
pub fn acquire(successor: bool) -> bool {
    let name = wide(MUTEX_NAME);
    // bInitialOwner = TRUE: the winner owns it from the moment of creation, so
    // two launches cannot both see "not taken" and both proceed.
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr()) };
    if handle == 0 {
        // No mutex at all (only if the object namespace is unusable): refusing
        // to start would be worse than allowing a duplicate.
        crate::debug_log::qs_log("instance: CreateMutexW failed, starting unguarded");
        return true;
    }
    let taken = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if taken && !successor {
        request_show();
        return false;
    }
    // The handle is deliberately leaked: the OS drops it when the process
    // ends, and holding it is what keeps the name reserved.
    create_window();
    true
}

/// Publish the app handle so later "show yourself" messages reach the UI.
pub fn mark_ready(app: &tauri::AppHandle) {
    let _ = APP.set(app.clone());
}

/// Whether a second launch asked for the window before we were ready.
///
/// Drained once by the setup hook, which is also where the normal startup show
/// happens, so the two cannot fight over the launcher geometry.
pub fn take_pending_show() -> bool {
    PENDING_SHOW.swap(false, Ordering::SeqCst)
}

/// Ask the running instance to show its launcher.
///
/// A second launch used to be silent — the app was already in the tray and the
/// double-click appeared to do nothing. Sending the request makes it behave
/// like the icon does.
fn request_show() {
    let class = wide(CLASS_NAME);
    let title = wide(WINDOW_NAME);
    let deadline = Instant::now() + Duration::from_millis(FIND_TIMEOUT_MS);
    loop {
        let hwnd = unsafe { FindWindowW(class.as_ptr(), title.as_ptr()) };
        if hwnd != 0 {
            // Bounded: never hang behind an instance that stopped pumping
            // messages (a modal dialog in another process can block it).
            unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    WM_KESTREL_SHOW,
                    0,
                    0,
                    SMTO_ABORTIFHUNG,
                    4_000,
                    std::ptr::null_mut(),
                );
            }
            return;
        }
        if Instant::now() >= deadline {
            crate::debug_log::qs_log(
                "instance: the lock is held but no window answered; leaving silently",
            );
            return;
        }
        std::thread::sleep(Duration::from_millis(FIND_POLL_MS));
    }
}

/// Create the hidden message window.
///
/// It is never shown: no `WS_VISIBLE`, a tool window, no activation. It exists
/// only as a `SendMessage` target, so it never reaches the taskbar, Alt+Tab or
/// the screen — `FindWindowW` does not care about visibility.
fn create_window() {
    let class = wide(CLASS_NAME);
    let title = wide(WINDOW_NAME);
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class.as_ptr(),
        hIconSm: 0,
    };
    unsafe {
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) as DWORD,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP as DWORD,
            0,
            0,
            0,
            0,
            0,
            0,
            instance,
            std::ptr::null_mut(),
        );
        if hwnd == 0 {
            // Still the instance — just unreachable for "show yourself".
            crate::debug_log::qs_log("instance: message window could not be created");
            return;
        }
        WINDOW.store(hwnd, Ordering::SeqCst);
    }
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_KESTREL_SHOW {
        // Runs on the main thread inside Tauri's own message loop, which is
        // where every other UI event is handled — safe to touch the window.
        show_now();
        return 0;
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn show_now() {
    if let Some(app) = APP.get() {
        if let Some(state) = app.try_state::<Arc<AppState>>() {
            crate::ui::show_launcher(app, &state);
            return;
        }
    }
    // Still starting up: the setup hook shows it as soon as the state exists,
    // instead of dropping the request on the floor.
    PENDING_SHOW.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_class_has_the_c_layout() {
        // Getting cbSize wrong makes RegisterClassExW fail with
        // ERROR_INVALID_PARAMETER, which would silently leave the guard with
        // no message target.
        assert_eq!(std::mem::size_of::<WNDCLASSEXW>(), 80);
        assert_eq!(std::mem::offset_of!(WNDCLASSEXW, lpfnWndProc), 8);
        assert_eq!(std::mem::offset_of!(WNDCLASSEXW, hInstance), 24);
        assert_eq!(std::mem::offset_of!(WNDCLASSEXW, lpszClassName), 64);
        assert_eq!(std::mem::offset_of!(WNDCLASSEXW, hIconSm), 72);
    }

    #[test]
    fn a_pending_show_is_drained_once() {
        PENDING_SHOW.store(true, Ordering::SeqCst);
        assert!(take_pending_show());
        assert!(
            !take_pending_show(),
            "the flag must not survive the first read"
        );
    }

    #[test]
    fn the_show_message_stays_out_of_the_system_range() {
        // WM_APP + 1 .. 0xBFFF is the range reserved for applications; below
        // 0x8000 would collide with a real Windows message.
        assert!(WM_KESTREL_SHOW >= 0x8000 && WM_KESTREL_SHOW <= 0xBFFF);
    }
}
