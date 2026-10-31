//! Global hotkeys: low-level keyboard hook (double-Ctrl, Quick Switch) plus
//! a registered system shortcut for the main launcher toggle.

use crate::state::AppState;
use crate::win::window;
use crate::win::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

static TRIGGER: AtomicU8 = AtomicU8::new(0); // 0 none, 1 toggle, 2 quick-switch, 3 show
static CTRL_DIRTY: AtomicBool = AtomicBool::new(false);
/// Hook diagnostics, read by the worker thread. **The hook itself must never
/// touch the file system**: it runs on the system input path and a slow hook
/// callback gets silently removed by Windows.
static CTRL_DOWNS: AtomicU64 = AtomicU64::new(0);
static HOOK_DIAG: AtomicU64 = AtomicU64::new(0); // dirty | enabled<<1 | gap<<2
static LAST_SEEN: AtomicU64 = AtomicU64::new(0);
static DOUBLE_CTRL_ENABLED: AtomicBool = AtomicBool::new(true);
static DOUBLE_CTRL_MS: AtomicU64 = AtomicU64::new(350);
static QUICK_SWITCH_ENABLED: AtomicBool = AtomicBool::new(true);
static REPLACE_WIN_SEARCH: AtomicBool = AtomicBool::new(false);
// Key currently held down inside the Win+<key> interception (repeat guard).
static WIN_SEARCH_HELD: AtomicBool = AtomicBool::new(false);
static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);

// ---- system idle time (watchdog input) -----------------------------------
#[repr(C)]
struct LastInputInfo {
    cb_size: u32,
    dw_time: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetLastInputInfo(plii: *mut LastInputInfo) -> i32;
    fn GetTickCount() -> u32;
}

/// Milliseconds since the last keyboard/mouse input anywhere on the desktop.
fn user_idle_ms() -> u64 {
    let mut lii = LastInputInfo {
        cb_size: std::mem::size_of::<LastInputInfo>() as u32,
        dw_time: 0,
    };
    let ok = unsafe { GetLastInputInfo(&mut lii) };
    if ok == 0 {
        return 0; // unknown: assume active, the caller's other checks still apply
    }
    let now = unsafe { GetTickCount() };
    now.wrapping_sub(lii.dw_time) as u64
}
// Heartbeats for the independent watchdog below: the hook bumps EVENTS on
// every keystroke it sees (atomic add only — safe on the input path), the
// worker bumps WORKER_TICK once per poll. If EVENTS stops advancing while
// keys are being pressed the hook died; if WORKER_TICK stops the worker is
// stuck. The watchdog itself never touches either data path.
static HOOK_EVENTS: AtomicU64 = AtomicU64::new(0);
static WORKER_TICK: AtomicU64 = AtomicU64::new(0);
// Hook-thread takeover bookkeeping: ID is the pump thread's Win32 thread id
// (so the watchdog can post it WM_QUIT), GEN is bumped on every respawn so a
// superseded pump exits instead of reinstalling itself.
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static HOOK_GEN: AtomicU64 = AtomicU64::new(0);
// Watchdog-side bookkeeping (only the watchdog touches these).
static WATCH_LAST_EVENTS: AtomicU64 = AtomicU64::new(0);
static WATCH_LAST_CHANGE: AtomicU64 = AtomicU64::new(0);
// The watchdog pings the pump thread with WM_NULL; the pump counts them.
// Rising pings + frozen events = the hook was removed while the pump lives
// (i.e. Windows silently dropped it). Frozen pings = the pump thread itself
// is stuck.
static PUMP_PINGS: AtomicU64 = AtomicU64::new(0);
const WM_NULL: u32 = 0x0000;

const WM_QUIT: u32 = 0x0012;

/// Install the LL keyboard hook and pump messages on the calling thread.
/// Runs forever; if the message pump dies the hook is reinstalled on the
/// same thread, unless a newer generation took over (watchdog respawn).
fn hook_thread_body() {
    unsafe {
        let my_gen = HOOK_GEN.load(Ordering::SeqCst);
        HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);
        let mut hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            hook_proc,
            GetModuleHandleW(std::ptr::null()),
            0,
        );
        crate::debug_log::qs_log_verbose(&format!("hook: installed={} gen={my_gen}", hook != 0));
        if hook == 0 {
            // Hook install failed (rare): park so the supervisor does not spin.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let rc = GetMessageW(&mut msg, 0, 0, 0);
            if rc > 0 {
                if msg.message == WM_NULL {
                    // Watchdog liveness probe — no dispatch needed.
                    PUMP_PINGS.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
                continue;
            }
            // rc == 0 (WM_QUIT) or -1 (error). Logging is safe here — this is
            // the hook *thread*, never the input path itself.
            if HOOK_GEN.load(Ordering::SeqCst) != my_gen {
                // A newer hook thread took over. Park forever: returning here
                // would let the supervisor re-run this body, and by then the
                // generation has already moved on — the re-run would believe
                // it is current and install a *duplicate* hook.
                let _ = UnhookWindowsHookEx(hook);
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
            crate::debug_log::qs_log(&format!("hook: pump exited rc={rc} -> reinstall"));
            let _ = UnhookWindowsHookEx(hook);
            hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                hook_proc,
                GetModuleHandleW(std::ptr::null()),
                0,
            );
            crate::debug_log::qs_log(&format!("hook: reinstalled={}", hook != 0));
            if hook == 0 {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
        }
    }
}

fn spawn_hook_thread() {
    crate::supervise_forever("kestrel-hotkey-hook", hook_thread_body);
}

fn now_ms() -> u64 {
    static START: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);
    // +1 so a timestamp is never 0: 0 doubles as the "no previous press"
    // sentinel, and the very first key event after startup would otherwise
    // store 0 and silently swallow the first double Ctrl gesture.
    START.elapsed().as_millis() as u64 + 1
}

fn is_ctrl(vk: u32) -> bool {
    vk == VK_CONTROL || vk == VK_LCONTROL || vk == VK_RCONTROL
}

fn is_modifier(vk: u32) -> bool {
    is_ctrl(vk) || vk == VK_SHIFT || vk == VK_MENU || (0x5B..=0x5C).contains(&vk)
}

extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code == HC_ACTION {
            HOOK_EVENTS.fetch_add(1, Ordering::Relaxed);
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
            let down = wparam == WM_KEYDOWN || wparam == WM_SYSKEYDOWN;
            let up = wparam == WM_KEYUP || wparam == WM_SYSKEYUP;
            let vk = kb.vkCode;

            // "Replace Windows search": swallow Win+S / Win+Q before the shell
            // sees it and open Kestrel instead. Purely application level — the
            // toggle restores the original behaviour instantly.
            let win_held = (GetAsyncKeyState(0x5B) as u16 & 0x8000) != 0
                || (GetAsyncKeyState(0x5C) as u16 & 0x8000) != 0;
            let is_search_key = vk == 0x53 || vk == 0x51; // S or Q
            if REPLACE_WIN_SEARCH.load(Ordering::Relaxed)
                && is_search_key
                && win_held
                && (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) == 0
                && (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) == 0
                && (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) == 0
            {
                if down && !WIN_SEARCH_HELD.swap(true, Ordering::Relaxed) {
                    TRIGGER.store(3, Ordering::Relaxed);
                }
                if up {
                    WIN_SEARCH_HELD.store(false, Ordering::Relaxed);
                }
                return 1; // swallow: Windows search never sees this key
            }

            if down {
                if is_ctrl(vk) {
                    // Double Ctrl itself is detected by polling in the worker
                    // (`poll_double_ctrl`), not here: Windows may silently drop
                    // a low-level hook at any time (observed: it stops
                    // delivering right after the launcher window is first
                    // shown), and a gesture detector that depends on the hook
                    // would then be dead for the rest of the process lifetime.
                    // The hook only publishes the chord flag + diagnostics.
                    CTRL_DOWNS.fetch_add(1, Ordering::Relaxed);
                    HOOK_DIAG.store(
                        (CTRL_DIRTY.load(Ordering::Relaxed) as u64)
                            | ((DOUBLE_CTRL_ENABLED.load(Ordering::Relaxed) as u64) << 1),
                        Ordering::Relaxed,
                    );
                } else if !is_modifier(vk) {
                    // Only mark the chord dirty when Ctrl is really held:
                    // otherwise a plain keystroke ("a") would permanently
                    // disable the double Ctrl detector.
                    if (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 {
                        CTRL_DIRTY.store(true, Ordering::Relaxed);
                    }
                    // Ctrl+G is polled by the worker (see `poll_double_ctrl`),
                    // not handled here: the Quick Switch box is focused while
                    // it is on screen, and the hook never sees those keys.
                }
            }
            // `CTRL_DIRTY` is deliberately *not* cleared here: the poller owns
            // the flag's lifetime. It must still read `true` on the release
            // edge to know the gesture was a chord and to drop the armed
            // timestamp — clearing it here made "Ctrl+C, then Ctrl" fire.
        }
        CallNextHookEx(0, code, wparam, lparam)
    }
}

pub fn set_double_ctrl(enabled: bool, interval_ms: u64) {
    DOUBLE_CTRL_ENABLED.store(enabled, Ordering::Relaxed);
    DOUBLE_CTRL_MS.store(interval_ms.clamp(120, 900), Ordering::Relaxed);
}

pub fn set_quick_switch(enabled: bool) {
    QUICK_SWITCH_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn set_replace_win_search(enabled: bool) {
    REPLACE_WIN_SEARCH.store(enabled, Ordering::Relaxed);
    if !enabled {
        WIN_SEARCH_HELD.store(false, Ordering::Relaxed);
    }
}

/// Double Ctrl detection by polling instead of hooking.
///
/// A low-level keyboard hook only sees keys destined for *other* processes:
/// once Kestrel's own window holds the focus the hook goes silent. That is
/// exactly when "double Ctrl hides the visible launcher" must still work, so
/// the gesture is detected by polling the global key state instead. The
/// worker calls this every ~18 ms, far below the shortest meaningful Ctrl
/// tap, so no gesture is missed and nothing depends on hook health.
/// Chord rejection still comes from the hook (`CTRL_DIRTY`); if the hook is
/// gone the detector degrades to "every Ctrl tap counts", which is still far
/// better than a dead hotkey.
fn poll_double_ctrl() {
    static POLL_DOWN: AtomicBool = AtomicBool::new(false);
    static POLL_LAST: AtomicU64 = AtomicU64::new(0);

    let ctrl = unsafe { (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 };

    // Ctrl+G -> Quick Switch, same reason: it is pressed while the Quick
    // Switch box itself is focused, where the hook never fires.
    {
        static G_DOWN: AtomicBool = AtomicBool::new(false);
        let g = unsafe { (GetAsyncKeyState(0x47) as u16 & 0x8000) != 0 };
        let g_was = G_DOWN.swap(g, Ordering::Relaxed);
        if g && !g_was && ctrl && QUICK_SWITCH_ENABLED.load(Ordering::Relaxed) {
            TRIGGER.store(2, Ordering::Relaxed);
            // The Ctrl of Ctrl+G must not arm (or pair with) the double Ctrl
            // detector. The hook cannot set CTRL_DIRTY here: while the Quick
            // Switch box is focused the keys go to our own webview and the
            // hook stays silent, so two Ctrl+G presses in a row used to look
            // like "double Ctrl" and toggled the main launcher *on top of*
            // the Quick Switch bar.
            POLL_LAST.store(0, Ordering::Relaxed);
        }
    }

    let was = POLL_DOWN.swap(ctrl, Ordering::Relaxed);
    if ctrl && !was {
        // Rising edge: one Ctrl press.
        let enabled = DOUBLE_CTRL_ENABLED.load(Ordering::Relaxed);
        let dirty = CTRL_DIRTY.load(Ordering::Relaxed);
        let ms = DOUBLE_CTRL_MS.load(Ordering::Relaxed);
        let now = now_ms();
        let last = POLL_LAST.load(Ordering::Relaxed);
        if !enabled || dirty {
            // Chords never arm the detector, and an in-progress chord must
            // also break the chain so the press after it cannot pair with
            // the press before it.
            POLL_LAST.store(0, Ordering::Relaxed);
            return;
        }
        let gap = now.saturating_sub(last);
        // A Ctrl held together with G is the Quick Switch chord, not a bare
        // Ctrl tap — also when both keys went down inside one poll tick and
        // the G-edge branch above never ran.
        let chord = unsafe { (GetAsyncKeyState(0x47) as u16 & 0x8000) != 0 };
        if chord {
            // Break the chain: this press must not pair with anything.
            POLL_LAST.store(0, Ordering::Relaxed);
        } else if last > 0 && gap < ms {
            POLL_LAST.store(0, Ordering::Relaxed); // disarm: one trigger per gesture
            TRIGGER.store(1, Ordering::Relaxed);
        } else {
            POLL_LAST.store(now, Ordering::Relaxed);
        }
    } else if !ctrl && was {
        // Release edge: a chord is over, so the next press starts fresh.
        if CTRL_DIRTY.load(Ordering::Relaxed) {
            POLL_LAST.store(0, Ordering::Relaxed);
        }
        CTRL_DIRTY.store(false, Ordering::Relaxed);
    }
}

/// Installs the low-level hook on its own thread with a message pump.
pub fn install_hook(app: AppHandle, state: Arc<AppState>) {
    if HOOK_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }

    // Worker that reacts to hook triggers without blocking input.
    {
        let app = app.clone();
        let state = state.clone();
        crate::supervise_forever("kestrel-hotkey-worker", move || loop {
            std::thread::sleep(std::time::Duration::from_millis(18));
            WORKER_TICK.fetch_add(1, Ordering::Relaxed);
            // Double Ctrl is polled here, not hooked: the hook can be
            // dropped by Windows at any moment, the poll cannot.
            poll_double_ctrl();
            // Report hook activity from here — never from the hook itself.
            let seen = CTRL_DOWNS.load(Ordering::Relaxed);
            if seen != LAST_SEEN.swap(seen, Ordering::Relaxed) {
                let diag = HOOK_DIAG.load(Ordering::Relaxed);
                crate::debug_log::qs_log_verbose(&format!(
                    "hook: ctrl downs={} dirty={} enabled={}",
                    seen,
                    diag & 1,
                    (diag >> 1) & 1,
                ));
            }
            match TRIGGER.swap(0, Ordering::Relaxed) {
                0 => {}
                1 => {
                    crate::debug_log::qs_log_verbose("hook: double ctrl -> toggle");
                    crate::ui::toggle_launcher(&app, &state);
                }
                3 => crate::ui::show_launcher(&app, &state),
                2 => {
                    // Ctrl+G inside a file dialog: show the attached box,
                    // or if it is already attached, jump to the record the
                    // user selected (frontend knows the selection).
                    // When the box is on screen *it* owns the focus, so the
                    // foreground check below would never match — handle the
                    // attached case first.
                    let attached = state.dialog.lock().is_some();
                    let visible = state.qs_active.load(Ordering::Relaxed)
                        && crate::ui::launcher(&app)
                            .map(|w| w.is_visible().unwrap_or(false))
                            .unwrap_or(false);
                    if attached && visible {
                        let _ = app.emit("quickswitch://jump", ());
                    } else {
                        let fg = window::foreground();
                        if window::is_quick_switch_target(fg) {
                            *state.dialog.lock() = Some(fg);
                            if let Some(dir) = window::dialog_current_folder(fg) {
                                state.push_qs_dir(dir);
                            }
                            crate::ui::show_quick_switch(&app, &state, fg);
                        }
                    }
                }
                _ => {}
            }
        })
    }

    // Independent watchdog: reports hook/worker liveness without going
    // through either of them, and self-heals a silently removed hook. A
    // low-level hook that stops delivering (Windows removes timed-out hooks)
    // is indistinguishable from "no keys pressed" by design, so the respawn
    // threshold has to be generous. It is also the only signal available:
    // `user_idle_ms` comes from GetLastInputInfo, which counts *mouse* motion
    // as activity, while the hook only sees keystrokes — so "reading a long
    // document while scrolling" looked exactly like a dead hook and respawned
    // it every five minutes, swallowing a Ctrl tap each time. Half an hour of
    // mouse-only use is a far better trade: a genuinely removed hook still
    // self-heals, and normal reading never trips it.
    {
        crate::supervise_forever("kestrel-hotkey-watchdog", move || loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let hook_id = HOOK_THREAD_ID.load(Ordering::SeqCst);
            if hook_id != 0 {
                unsafe { PostThreadMessageW(hook_id, WM_NULL, 0, 0) };
            }
            crate::debug_log::qs_log_verbose(&format!(
                "hookwatch: events={} downs={} worker_tick={} pump_pings={}",
                HOOK_EVENTS.load(Ordering::Relaxed),
                CTRL_DOWNS.load(Ordering::Relaxed),
                WORKER_TICK.load(Ordering::Relaxed),
                PUMP_PINGS.load(Ordering::Relaxed),
            ));
            let events = HOOK_EVENTS.load(Ordering::Relaxed);
            if events != WATCH_LAST_EVENTS.load(Ordering::Relaxed) {
                WATCH_LAST_EVENTS.store(events, Ordering::Relaxed);
                WATCH_LAST_CHANGE.store(now_ms(), Ordering::Relaxed);
                continue;
            }
            let app_alive = WORKER_TICK.load(Ordering::Relaxed) != 0;
            let stall_secs = std::env::var("KESTREL_HOOK_STALL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1800)
                .clamp(15, 7200);
            let frozen = now_ms().saturating_sub(WATCH_LAST_CHANGE.load(Ordering::Relaxed));
            // A silent hook and an idle user look identical from the event
            // counter alone. Respawning on idleness tore down a perfectly
            // healthy hook every five minutes and could swallow the Ctrl tap
            // in flight, so only act while the user is demonstrably typing.
            if user_idle_ms() > 5_000 {
                continue;
            }
            if app_alive && frozen > stall_secs * 1000 {
                crate::debug_log::qs_log(&format!(
                    "hookwatch: hook stalled {stall_secs}s -> respawning"
                ));
                WATCH_LAST_CHANGE.store(now_ms(), Ordering::Relaxed); // don't respawn in a loop
                let old_id = HOOK_THREAD_ID.load(Ordering::SeqCst);
                HOOK_GEN.fetch_add(1, Ordering::SeqCst);
                if old_id != 0 {
                    unsafe { PostThreadMessageW(old_id, WM_QUIT, 0, 0) };
                }
                spawn_hook_thread();
            }
        });
    }

    spawn_hook_thread();
    WATCH_LAST_CHANGE.store(now_ms(), Ordering::Relaxed);

    let _ = app.emit("hotkeys://ready", ());
}

/// Register the global shortcut that toggles the launcher.
///
/// The new combination is registered *before* the previous one is released,
/// so a combination the system refuses (already owned by another program)
/// leaves the old hotkey working instead of leaving the app without one.
/// Returns `false` when the combination could not be registered.
pub fn register_toggle_shortcut(app: &AppHandle, accelerator: &str) -> bool {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

    static CURRENT: once_cell::sync::Lazy<parking_lot::Mutex<Option<String>>> =
        once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(None));

    {
        let cur = CURRENT.lock();
        if cur.as_deref() == Some(accelerator) {
            // Already active: nothing to do (and re-registering the same
            // combination would fail on Windows).
            return true;
        }
    }

    let gs = app.global_shortcut();
    let Ok(shortcut) = accelerator.parse::<Shortcut>() else {
        return false;
    };
    let handle = app.clone();
    if gs
        .on_shortcut(shortcut, move |_app, _sc, event| {
            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                if let Some(state) = handle.try_state::<Arc<AppState>>() {
                    crate::ui::toggle_launcher(&handle, &state);
                }
            }
        })
        .is_err()
    {
        return false;
    }
    // The new combination works — release the old one.
    let mut cur = CURRENT.lock();
    if let Some(old) = cur.as_ref() {
        if old != accelerator {
            if let Ok(old_sc) = old.parse::<Shortcut>() {
                let _ = gs.unregister(old_sc);
            }
        }
    }
    *cur = Some(accelerator.to_string());
    true
}
