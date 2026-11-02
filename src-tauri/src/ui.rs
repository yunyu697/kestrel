//! Window lifecycle, tray icon and launcher placement.

use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

/// Current always-on-top state of the shared launcher/QS window.
///
/// Starts as `true` because the window is *built* with
/// `.always_on_top(true)` (see `lib.rs`). Starting at `false` made the first
/// `set_topmost(app, false)` a no-op — it "changed" nothing according to the
/// flag — so the Quick Switch box kept the topmost bit it was created with and
/// sat in a different z-order band than its dialog. That is exactly the bug the
/// user kept reporting.
static TOPMOST: AtomicBool = AtomicBool::new(true);

/// Set the box's always-on-top state, only touching the window when it really
/// changes. Every call is a `SetWindowPos`, and the foreground monitor calls
/// this on each activation — unconditional calls made the box flicker.
pub fn set_topmost(app: &AppHandle, on: bool) {
    if TOPMOST.swap(on, Ordering::Relaxed) == on {
        return;
    }
    if let Some(win) = launcher(app) {
        let _ = win.set_always_on_top(on);
    }
}

/// Whether the launcher/QS window is currently topmost.
#[allow(dead_code)]
pub fn is_topmost() -> bool {
    TOPMOST.load(Ordering::Relaxed)
}

pub fn launcher(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("launcher")
}

/// Whether the launcher is on screen, from the cached flag.
///
/// Avoids `WebviewWindow::is_visible()`, which is a synchronous round trip to
/// the UI thread and was called from the 60 Hz follower loop.
pub fn launcher_visible(state: &Arc<AppState>) -> bool {
    state
        .launcher_visible
        .load(std::sync::atomic::Ordering::Relaxed)
}

/// Cached device pixel ratio of the launcher window.
///
/// `WebviewWindow::scale_factor()` is a round trip to the UI thread as well;
/// the value only changes on a DPI change, so it is measured when the window
/// is shown and reused afterwards.
pub fn launcher_scale(state: &Arc<AppState>) -> f64 {
    let bits = state.scale_bits.load(std::sync::atomic::Ordering::Relaxed);
    if bits == 0 {
        return 1.0;
    }
    let scale = f32::from_bits(bits) as f64;
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// Refresh the cached device pixel ratio of the launcher window.
fn refresh_scale(app: &AppHandle, state: &Arc<AppState>) {
    if let Some(win) = launcher(app) {
        if let Ok(s) = win.scale_factor() {
            state
                .scale_bits
                .store((s as f32).to_bits(), std::sync::atomic::Ordering::Relaxed);
        }
    }
}

pub fn settings_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("settings")
}

pub fn show_launcher(app: &AppHandle, state: &Arc<AppState>) {
    // Suspend any attached Quick Switch session instead of ending it.
    //
    // The launcher and the box share one window, so only one of them can be
    // on screen at a time — but they are *sessions*, not a single mode. The
    // dialog handle stays bound and `launcher_override` tells the follower to
    // leave the window alone; closing the launcher then puts the box straight
    // back under the dialog the user was working in. Clearing `dialog` here
    // (which is what this used to do) threw the session away, so after a
    // detour through the global search bar the box never came back.
    let bound = state.dialog.lock().is_some();
    crate::debug_log::qs_log(&format!(
        "show_launcher (suspends QS, bound={bound})"
    ));
    state
        .qs_active
        .store(false, std::sync::atomic::Ordering::Relaxed);
    state
        .launcher_override
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let Some(win) = launcher(app) else { return };
    if !state.launcher_visible.load(std::sync::atomic::Ordering::Relaxed) {
        // Restore the normal launcher footprint (Quick Switch shrinks it).
        let _ = win.set_min_size(Some(tauri::LogicalSize::new(520.0, 320.0)));
        let _ = win.set_size(tauri::LogicalSize::new(780.0, 540.0));
        let _ = win.center();
    }
    crate::debug_log::qs_log_verbose("show: sized");
    let _ = win.show();
    crate::debug_log::qs_log_verbose("show: shown");
    set_topmost(app, true);
    crate::debug_log::qs_log_verbose("show: topmost");
    let _ = win.set_focus();
    crate::debug_log::qs_log_verbose("show: focused");
    state
        .launcher_visible
        .store(true, std::sync::atomic::Ordering::Relaxed);
    refresh_scale(app, state);
    crate::debug_log::qs_log_verbose("show: scale");
    let _ = app.emit("launcher://shown", ());
    crate::debug_log::qs_log_verbose("show: emitted");
    // A program installed a moment ago has to be searchable right away; this
    // costs one directory walk on a worker thread and nothing on the UI.
    crate::apps::refresh_if_stale(app, state);
}

/// Show the launcher glued below the captured file dialog in Quick Switch mode.
/// The window only becomes visible after the geometry is applied and the
/// frontend has switched to Quick Switch mode — otherwise it would briefly
/// display the normal launcher UI.
pub fn show_quick_switch(app: &AppHandle, state: &Arc<AppState>, hwnd: isize) {
    show_quick_switch_at(app, state, hwnd, true);
}

/// Attach the box to `hwnd`. `take_focus` is false when the box is merely
/// coming back after the launcher was closed: the user is returning to the
/// *dialog*, so stealing the caret into our own box would be wrong.
fn show_quick_switch_at(app: &AppHandle, state: &Arc<AppState>, hwnd: isize, take_focus: bool) {
    let Some(win) = launcher(app) else { return };
    // Bind the box to this dialog FIRST: the follower thread keys off
    // `state.dialog` for following and for hiding when the dialog dies.
    // (The Ctrl+G path may reach here without the foreground monitor having
    // captured the dialog — forgetting this left the box glued to nothing.)
    *state.dialog.lock() = Some(hwnd);
    // An explicit attach (Ctrl+G, a launcher round trip) overrides an earlier
    // dismissal of this same dialog.
    *state.qs_dismissed.lock() = None;
    state
        .qs_active
        .store(true, std::sync::atomic::Ordering::Relaxed);
    // The window is no longer the global launcher's.
    state
        .launcher_override
        .store(false, std::sync::atomic::Ordering::Relaxed);
    // Bail out instead of showing a stale/global launcher window.
    refresh_scale(app, state);
    let scale = launcher_scale(state);
    let Some((x, y, w, h)) = crate::win::monitor::compute_qs_geometry(hwnd, state.settings.read().qs_visible_rows, scale) else {
        crate::debug_log::qs_log(&format!("show_qs hwnd={hwnd:#x} geometry FAILED"));
        // Nothing was shown, so the session is not running: leaving the flag
        // set would make every later foreground event think the box is up.
        state
            .qs_active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        return;
    };
    crate::debug_log::qs_log(&format!("show_qs hwnd={hwnd:#x} geom=({x},{y},{w},{h})"));
    let _ = win.set_min_size(Some(tauri::LogicalSize::new(260.0, 140.0)));
    let _ = win.set_size(tauri::PhysicalSize::new(w, h));
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    let dirs = state.qs_history.lock().clone();
    let _ = app.emit("quickswitch://show", dirs);
    // Give the frontend a moment to apply Quick Switch mode while the window
    // is still invisible, so the first painted frame is already the QS box.
    std::thread::sleep(std::time::Duration::from_millis(40));
    let _ = win.show();
    // Z-order: the box must look like part of the dialog — directly above it,
    // in the *same* band.
    //
    // Topmost was the wrong tool. It put the box in its own band, above the
    // taskbar and above every other application, while the dialog stayed a
    // normal window: the two were visibly on different levels, which is what
    // the user kept reporting. Dropping topmost and raising inside the normal
    // band instead puts the box exactly one slot above the dialog, and it
    // still sinks together with the dialog when another app is activated.
    set_topmost(app, false);
    if let Ok(h) = win.hwnd() {
        crate::win::keep_above(h.0 as isize, hwnd);
    }
    if take_focus {
        let _ = win.set_focus();
    }
    state
        .launcher_visible
        .store(true, std::sync::atomic::Ordering::Relaxed);
    refresh_scale(app, state);
}

pub fn hide_launcher(app: &AppHandle) {
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        state
            .qs_active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        state
            .launcher_override
            .store(false, std::sync::atomic::Ordering::Relaxed);
        state
            .launcher_visible
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
    crate::debug_log::qs_log("hide_launcher");
    set_topmost(app, false);
    if let Some(win) = launcher(app) {
        let _ = app.emit("launcher://hiding", ());
        let _ = win.hide();
    }
}

/// Close the launcher the way the user asked for — and, if a Quick Switch
/// session was suspended by it, hand the window back to that session.
///
/// This is the "two sessions, one window" half of the design: the box is not
/// destroyed when the global search bar appears, it steps aside and comes
/// back. Anything that hides the launcher for its own reasons (`open_settings`,
/// a dialog that went away) keeps using [`hide_launcher`] and does *not*
/// resurrect the box.
pub fn close_launcher(app: &AppHandle, state: &Arc<AppState>) {
    let resume = state.dialog.lock().is_some();
    hide_launcher(app);
    if resume {
        resume_quick_switch(app, state);
    }
}

/// Put the box back under the dialog that is still bound, if any.
fn resume_quick_switch(app: &AppHandle, state: &Arc<AppState>) {
    let Some(dialog) = *state.dialog.lock() else {
        return;
    };
    if crate::win::window::dialog_closing(dialog) {
        // The dialog was closed while the launcher was up; there is nothing
        // left to attach to.
        *state.dialog.lock() = None;
        *state.qs_dismissed.lock() = None;
        crate::debug_log::qs_log("resume QS: dialog is gone, session dropped");
        return;
    }
    if !state.settings.read().quick_switch_enabled {
        return;
    }
    crate::debug_log::qs_log(&format!("resume QS on dialog {dialog:#x}"));
    // No focus grab: the user is going back to the dialog, not to our box.
    show_quick_switch_at(app, state, dialog, false);
}

pub fn toggle_launcher(app: &AppHandle, state: &Arc<AppState>) {
    // Debounce rapid repeated toggles (key bounce, two shortcuts firing
    // together) — they swallow each other and look like "nothing happened".
    // The window is deliberately short: it must stay well below the double
    // Ctrl interval, otherwise hiding the launcher with a second double Ctrl
    // right after showing it would be swallowed.
    // parking_lot Mutex: cannot be poisoned by a panicked thread.
    static LAST_TOGGLE: parking_lot::Mutex<Option<std::time::Instant>> = parking_lot::Mutex::new(None);
    {
        let mut t = LAST_TOGGLE.lock();
        let now = std::time::Instant::now();
        if t.map(|l| now.duration_since(l) < std::time::Duration::from_millis(150)) == Some(true) {
            return;
        }
        *t = Some(now);
    }
    // A visible launcher is always hidden again — this is what makes a second
    // double Ctrl close the window instead of waking it up.
    // The real state is queried here (this path is user driven and rare) and
    // the cache is refreshed for the hot follower loop.
    let visible = launcher(app)
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    state
        .launcher_visible
        .store(visible, std::sync::atomic::Ordering::Relaxed);
    if !visible {
        show_launcher(app, state);
        return;
    }
    // Which of the two sessions is on screen decides what "close" means.
    // The attached box is simply dismissed (its binding survives, so returning
    // to the dialog brings it back); the global launcher hands the window back
    // to the box if one is still bound.
    if state.qs_active.load(std::sync::atomic::Ordering::Relaxed) {
        // Remember the dismissal. Hiding the box gives the foreground back to
        // the dialog, whose foreground event would otherwise re-attach the box
        // immediately — making double Ctrl look like a no-op.
        *state.qs_dismissed.lock() = *state.dialog.lock();
        hide_launcher(app);
    } else {
        close_launcher(app, state);
    }
}

pub fn open_settings(app: &AppHandle) {
    if let Some(win) = settings_window(app) {
        reveal_settings(app, &win);
        hide_launcher(app);
        return;
    }
    hide_launcher(app);
    // Creation happens on a WORKER thread, never inside a main-thread callback.
    // WebView2 builds its controller asynchronously and delivers the completion
    // through the window message queue: doing this from `run_on_main_thread`
    // nests the pump and the completion never arrives — the whole app froze on
    // the first open. A worker thread leaves the main pump running, so the
    // callback lands normally. DPI is unaffected because awareness is now
    // process wide (`win::become_dpi_aware`), not per creating thread.
    static OPENING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if OPENING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return; // already being created by another caller
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        let res = crate::ensure_settings_window(&handle);
        OPENING.store(false, std::sync::atomic::Ordering::SeqCst);
        match res {
            Ok(()) => {
                if let Some(win) = handle.get_webview_window("settings") {
                    reveal_settings(&handle, &win);
                }
            }
            Err(e) => crate::debug_log::qs_log(&format!("settings window unavailable: {e}")),
        }
    });
}

fn reveal_settings(app: &AppHandle, win: &WebviewWindow) {
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
    let _ = app.emit_to("settings", "settings://shown", ());
}

fn build_tray_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(
        app,
        "show",
        crate::i18n::tr(lang, "tray.show"),
        true,
        Some("Alt+Space"),
    )?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        crate::i18n::tr(lang, "tray.settings"),
        true,
        None::<&str>,
    )?;
    let reindex = MenuItem::with_id(
        app,
        "reindex",
        crate::i18n::tr(lang, "tray.reindex"),
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        crate::i18n::tr(lang, "tray.quit"),
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&show, &settings, &reindex, &sep, &quit])
}

/// Re-label the tray icon after a language switch (no restart needed).
pub fn refresh_tray_language(app: &AppHandle, lang: &str) {
    let Some(tray) = app.tray_by_id("main") else { return };
    if let Ok(menu) = build_tray_menu(app, lang) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(crate::i18n::tr(lang, "tray.tip")));
}

/// The user's own double-click interval, in milliseconds.
///
/// The tray handler used a fixed 240 ms window. That is shorter than the
/// Windows default (500 ms), so any double click slower than 240 ms fell
/// through to the single-click action and woke the launcher — which is
/// exactly "double clicking the tray randomly opens the search bar".
/// Reading the system setting makes our window agree with what Windows
/// itself considers a double click.
fn system_double_click_ms() -> u64 {
    #[link(name = "user32")]
    extern "system" {
        fn GetDoubleClickTime() -> u32;
    }
    let t = unsafe { GetDoubleClickTime() } as u64;
    // Returns 0 if it cannot be read; also guard against silly values.
    // +30 ms of margin so a click right on the boundary still counts.
    t.clamp(200, 1000) + 30
}

/// Monotonic millisecond clock (since boot). Used to correlate tray events.
fn ticks_ms() -> u64 {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount64() -> u64;
    }
    unsafe { GetTickCount64() }
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let lang = app
        .try_state::<Arc<AppState>>()
        .map(|s| s.settings.read().language.clone())
        .unwrap_or_else(|| "en".to_string());
    let menu = build_tray_menu(app, &lang)?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip(crate::i18n::tr(&lang, "tray.tip"))
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    show_launcher(app, &state);
                }
            }
            "settings" => open_settings(app),
            "reindex" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    crate::commands::index::spawn_rebuild(app.clone(), state.inner().clone());
                }
            }
            "quit" => {
                crate::watchdog::mark_graceful_quit();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            use std::sync::atomic::{AtomicU64, Ordering};
            // Token of the click whose "single click" action is still pending.
            // 0 means "nothing pending".
            static PENDING: AtomicU64 = AtomicU64::new(0);
            // Monotonic token generator (never repeats, even within a ms).
            static SEQ: AtomicU64 = AtomicU64::new(0);
            // When the last double click was handled, in ticks. Windows sends
            // WM_LBUTTONDOWN/UP, WM_LBUTTONDBLCLK, WM_LBUTTONUP — i.e. a
            // trailing Click *after* the DoubleClick. Clearing PENDING in the
            // DoubleClick arm let that trailing Click re-arm the debounce, so
            // 530 ms later the launcher popped up over the settings window:
            // "double click opens settings but still wakes the search bar".
            static LAST_DBL: AtomicU64 = AtomicU64::new(0);
            let app = tray.app_handle().clone();
            match event {
                // Left button release. The action is decided by *whether a
                // second click follows*, never by the DoubleClick event: on
                // Windows the tray icon is not always a real HWND, so the
                // double-click notification is unreliable and the launcher got
                // woken up even though the user asked for the settings.
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let window = system_double_click_ms();
                    // Swallow the release that Windows sends right after
                    // WM_LBUTTONDBLCLK. It belongs to the double click we just
                    // handled; treating it as a fresh single click is exactly
                    // what woke the launcher.
                    let last = LAST_DBL.load(Ordering::SeqCst);
                    if last != 0 && ticks_ms().saturating_sub(last) < 250 {
                        return;
                    }
                    if PENDING.load(Ordering::SeqCst) != 0 {
                        // A single click was already pending -> this is the
                        // second click of a double click: cancel the pending
                        // wake-up and open the settings instead.
                        PENDING.store(0, Ordering::SeqCst);
                        open_settings(&app);
                        return;
                    }
                    // First click of a (possible) double click: arm it and
                    // wake the launcher only if no second click shows up.
                    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
                    PENDING.store(seq, Ordering::SeqCst);
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(window));
                        if PENDING.load(Ordering::SeqCst) != seq {
                            return; // cancelled by a second click
                        }
                        PENDING.store(0, Ordering::SeqCst);
                        if let Some(state) = app.try_state::<Arc<AppState>>() {
                            show_launcher(&app, &state);
                        }
                    });
                }
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    // Some platforms report the double click explicitly.
                    // Make sure the launcher is not woken up and that the
                    // settings window opens exactly once.
                    LAST_DBL.store(ticks_ms(), Ordering::SeqCst);
                    PENDING.store(0, Ordering::SeqCst);
                    // Always act: when Windows reports Click+DoubleClick (not
                    // two Clicks) the second Click never arrives, and guarding
                    // on `pending == 0` meant a double click did nothing at
                    // all. open_settings is idempotent (it reuses the window
                    // and hides the launcher), so a duplicate call is harmless.
                    open_settings(&app);
                }
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}
