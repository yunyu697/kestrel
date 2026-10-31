//! Kestrel — instant file search and launcher.

mod apps;
mod calc;
mod clipboard;
mod commands;
mod debug_log;
mod hotkeys;
mod i18n;
mod index;
mod install;
mod instance;
mod model;
mod restart;
mod search;
mod state;
mod ui;
mod watchdog;
mod win;

use state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

pub fn run() {
    // FIRST, before any window exists: without this the process is DPI
    // unaware, every window coordinate is virtualised by the monitor scale and
    // the compositor bitmap-stretches the whole UI (blurry, oversized,
    // clipped). See `win::become_dpi_aware`.
    let dpi_mode = win::become_dpi_aware();
    install_panic_hook();
    // KESTREL_DEBUG=1 turns on the noisy per-probe tracing (off by default so
    // it costs nothing).
    debug_log::init_verbose_from_env();
    debug_log::qs_log(&format!("dpi awareness: {dpi_mode}"));
    // Watchdog mode: keep-the-app-alive loop without any UI.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--watchdog") {
        watchdog::run();
        return;
    }
    // Installer helper mode: wait for the app to exit, then run the installer
    // that replaces it. It has to be handled before the single-instance
    // plugin, which would otherwise turn this copy into a second launcher.
    if let Some(i) = args.iter().position(|a| a == "--install") {
        let path = args.get(i + 1).cloned().unwrap_or_default();
        let parent = args
            .iter()
            .position(|a| a == "--parent")
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        install::run(&path, parent);
        return;
    }
    // A self-relaunch (elevation change) is the one copy that may start while
    // its predecessor still holds the instance lock, and only when that
    // predecessor actually announced the hand-off. Without the flag file the
    // switch is a bare bypass that would let any process start a second
    // instance by appending the argument.
    let handoff = restart::pending();
    let successor = handoff && args.iter().any(|a| a == "--no-single-instance");
    if !instance::acquire(successor) {
        debug_log::qs_log("instance: another copy owns the session, leaving");
        return;
    }
    // Remember whether we are the incoming half of a relaunch: those windows
    // need their own WebView2 profile (see `restart::profile_dir`).
    restart::mark_relaunching(handoff);

    let builder = tauri::Builder::default();
    builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            debug_log::qs_log("setup: begin");
            let handle = app.handle().clone();

            let data_dir = handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));

            // "Run as administrator" is a property of the process, not of a
            // window: when it is on but this process is not elevated, hand over
            // to an elevated copy and quit before anything is shown (no tray
            // icon flash, no half-built UI).
            if !win::is_elevated() {
                let wanted = state::load_json::<crate::model::Settings>(
                    &data_dir.join("settings.json"),
                )
                .map(|s| s.run_as_admin)
                .unwrap_or(false);
                if wanted {
                    // Announce the hand-off: the elevated copy is allowed past
                    // the instance guard only because of this flag.
                    restart::begin();
                    if win::shell::relaunch_self("--no-single-instance", true) {
                        debug_log::qs_log("setup: relaunching elevated (run_as_admin)");
                        // Deliberate exit: keep the watchdog from respawning an
                        // unelevated copy and looping the UAC prompt.
                        watchdog::mark_graceful_quit();
                        std::process::exit(0);
                    }
                    // UAC was dismissed: withdraw the announcement, otherwise a
                    // later normal start would mistake itself for the successor
                    // and build its windows against the relaunch profile.
                    restart::arrived();
                }
            }

            // Windows are created here (not from tauri.conf.json) so a failed
            // WebView2 creation can be retried: Windows occasionally rejects
            // the HWND during startup with 0x80070578 ("invalid window
            // handle"), which used to abort the whole app (闪退).
            create_windows(&handle)?;
            debug_log::qs_log("setup: windows ok");
            // Tell a predecessor (privilege change) that our webviews exist, so
            // it can let go of the shared WebView2 browser process.
            if restart::arrived() {
                debug_log::qs_log("setup: relaunch handshake cleared");
            }

            let first_run = !data_dir.join("settings.json").exists();
            let state = AppState::new(data_dir);
            app.manage(state.clone());
            // From here on a second launch can be answered directly; before
            // this it was recorded as pending (see `instance`).
            instance::mark_ready(&handle);
            debug_log::qs_log("setup: state ready");

            // Guard the process with a watchdog-visible alive mutex.
            watchdog::hold_alive_mutex();
            watchdog::clear_quit_flag();
            debug_log::qs_log("setup: watchdog ok");

            commands::settings::apply_side_effects(&handle, &state);
            debug_log::qs_log("setup: side effects ok");
            ui::setup_tray(&handle)?;
            debug_log::qs_log("setup: tray ok");
            hotkeys::install_hook(handle.clone(), state.clone());
            debug_log::qs_log("setup: hotkeys ok");
            win::monitor::start(handle.clone(), state.clone());
            debug_log::qs_log("setup: monitor ok");

            // Load the persisted index off the startup path: a slow disk must
            // never delay the window from appearing.
            {
                let h = handle.clone();
                let st = state.clone();
                std::thread::spawn(move || {
                    let loaded = commands::index::load_persisted(&st);
                    debug_log::qs_log(&format!(
                        "setup: index loaded={:?} count={}",
                        loaded,
                        st.index.count()
                    ));
                    if loaded.is_none() || st.index.count() == 0 {
                        commands::index::spawn_rebuild(h.clone(), st.clone());
                    } else {
                        commands::index::restart_watcher(&h, &st);
                        // The window was created before the index finished
                        // loading, so its first `index_status` call returned 0
                        // and the footer kept saying "0 个文件" until something
                        // else triggered a rebuild. Announce the loaded index.
                        let _ = h.emit(
                            "index://done",
                            serde_json::json!({ "count": st.index.count(), "ms": 0 }),
                        );
                    }
                });
            }
            debug_log::qs_log("setup: index ok");

            // Discover applications in the background.
            {
                let h = handle.clone();
                let st = state.clone();
                std::thread::spawn(move || {
                    let list = apps::scan_apps();
                    if !list.is_empty() {
                        *st.apps.write() = list;
                        st.save_apps();
                        let _ = tauri::Emitter::emit(&h, "apps://refreshed", ());
                    }
                });
            }

            clipboard::start(state.clone(), handle.clone());
            debug_log::qs_log("setup: clipboard ok");

            // Show the launcher on startup unless the user opted to start
            // minimized — and always when a second launch asked for it while
            // we were still starting.
            let asked = instance::take_pending_show();
            let start_minimized = state.settings.read().start_minimized;
            if asked || first_run || !start_minimized {
                ui::show_launcher(&handle, &state);
            }
            debug_log::qs_log("setup: done");

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "launcher" {
                    api.prevent_close();
                    crate::ui::hide_launcher(window.app_handle());
                } else if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            WindowEvent::Focused(false) => {
                if window.label() == "launcher" {
                    if let Some(state) = window.app_handle().try_state::<Arc<AppState>>() {
                        // An attached Quick Switch session survives focus loss:
                        // clicking the dialog must never dismiss the box.
                        if state.qs_active.load(std::sync::atomic::Ordering::Relaxed) {
                            return;
                        }
                        let keep = !state.settings.read().close_after_open;
                        if !keep {
                            crate::ui::hide_launcher(window.app_handle());
                        }
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            // search
            commands::search::search,
            commands::search::record_query,
            commands::search::history_list,
            commands::search::history_clear,
            // index
            commands::index::index_status,
            commands::index::rebuild_index,
            commands::index::cancel_index,
            commands::index::clear_index,
            // files
            commands::files::file_info,
            commands::files::icons_for,
            commands::files::open_path,
            commands::files::open_url,
            commands::files::reveal_path,
            commands::files::open_with,
            commands::files::run_as_admin,
            commands::files::show_properties,
            commands::files::open_terminal,
            commands::files::copy_file_refs,
            commands::files::delete_to_trash,
            commands::files::rename_path,
            commands::files::create_folder,
            commands::files::bump_usage,
            commands::files::browse_folder,
            commands::files::file_hashes,
            commands::files::run_shell_command,
            commands::files::spawn_program,
            // network
            commands::net::http_probe,
            // update
            commands::update::update_download,
            commands::update::update_cancel,
            commands::update::update_install,
            commands::update::update_reveal,
            commands::update::install_mode,
            // settings & apps
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::reset_settings,
            commands::settings::apply_hotkey,
            commands::settings::build_info,
            commands::settings::list_apps,
            commands::settings::refresh_apps,
            commands::settings::clipboard_list,
            commands::settings::clipboard_copy,
            commands::settings::clipboard_delete,
            commands::settings::clipboard_pin,
            commands::settings::clipboard_clear,
            commands::settings::clipboard_push_text,
            commands::settings::favorites_add,
            commands::settings::favorites_remove,
            commands::settings::favorites_set_enabled,
            commands::settings::favorites_set_path,
            commands::settings::open_settings_window,
            commands::settings::is_elevated,
            commands::settings::set_run_as_admin,
            commands::settings::hide_launcher_window,
            commands::settings::show_launcher_window,
            commands::settings::set_always_on_top,
            commands::settings::quit_app,
            // system
            commands::system::list_windows,
            commands::system::activate_window,
            commands::system::close_window,
            commands::system::minimize_window,
            commands::system::toggle_topmost_window,
            commands::system::snap_window,
            commands::system::pick_color,
            commands::system::run_builtin,
            commands::system::quick_switch_apply,
            commands::system::clear_dialog_target,
            commands::system::has_dialog_target,
            commands::system::qs_context,
            commands::system::service_status,
            commands::system::service_install,
            commands::system::service_uninstall,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Kestrel")
        .run(|_app, event| {
            // A tray app must survive "all windows closed": during startup a
            // failed window creation momentarily leaves zero windows, and the
            // implicit exit would kill the retry loop before it can recover.
            //
            // `code` is None only for that implicit exit. An explicit
            // `app.exit(n)` (tray Quit, elevation restart) carries a code and
            // must be allowed through — preventing it left the process alive
            // forever with no window.
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

/// Create the launcher and settings windows, retrying a few times: WebView2
/// creation can transiently fail with `0x80070578` (invalid window handle)
/// right after startup. Without a retry that error tears the whole app down.
fn create_windows(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};

    fn build_with_retry<F>(name: &str, mut make: F) -> tauri::Result<()>
    where
        F: FnMut() -> tauri::Result<WebviewWindow>,
    {
        let mut last: Option<String> = None;
        for attempt in 1..=4 {
            debug_log::qs_log(&format!("window '{name}' creating (attempt {attempt})"));
            match make() {
                Ok(_) => {
                    debug_log::qs_log(&format!("window '{name}' created (attempt {attempt})"));
                    return Ok(());
                }
                Err(e) => {
                    debug_log::qs_log(&format!(
                        "window '{name}' create failed (attempt {attempt}): {e}"
                    ));
                    last = Some(e.to_string());
                    std::thread::sleep(std::time::Duration::from_millis(450));
                }
            }
        }
        Err(tauri::Error::Anyhow(anyhow::anyhow!(
            "could not create window '{name}': {}",
            last.unwrap_or_default()
        )))
    }

    let a = app.clone();
    build_with_retry("launcher", move || {
        let mut b = WebviewWindowBuilder::new(&a, "launcher", WebviewUrl::App("index.html".into()))
            .title("Kestrel")
            .inner_size(780.0, 540.0)
            .min_inner_size(520.0, 320.0)
            .center()
            .resizable(true)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(true);
        if restart::is_relaunching() {
            if let Some(dir) = restart::profile_dir() {
                b = b.data_directory(dir);
            }
        }
        b.build()
    })?;

    Ok(())
}

/// Create the settings window on first use.
///
/// It is deliberately NOT built at startup: spinning up a second WebView2
/// controller during boot doubled the startup cost and could block the whole
/// process (observed: the creation call never returned, so no tray, no hotkeys
/// and no launcher). Creating it lazily keeps the launcher path short and puts
/// the risk on the one action that actually needs it.
pub fn ensure_settings_window(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    if app.get_webview_window("settings").is_some() {
        return Ok(());
    }
    let a = app.clone();
    let mut last: Option<String> = None;
    // Clamp to the work area in *logical* pixels: a window larger than the
    // screen is silently shrunk by Windows while the webview keeps the
    // original size, which clips the right edge of the UI (the favourite
    // rows lost their toggle/路径 buttons on 1366x768 @150%).
    let (avail_w, avail_h) = crate::win::work_area_logical();
    let w = 1040.0f64.min(avail_w - 24.0).max(560.0);
    let h = 720.0f64.min(avail_h - 24.0).max(420.0);
    let min_w = 840.0f64.min(w);
    let min_h = 520.0f64.min(h);
    for attempt in 1..=4 {
        debug_log::qs_log(&format!("window 'settings' creating (attempt {attempt})"));
        let mut b = WebviewWindowBuilder::new(&a, "settings", WebviewUrl::App("settings.html".into()))
            .title("Kestrel 设置")
            .inner_size(w, h)
            .min_inner_size(min_w, min_h)
            .center()
            .resizable(true)
            .decorations(false)
            .transparent(false)
            .visible(false)
            .skip_taskbar(false);
        if restart::is_relaunching() {
            if let Some(dir) = restart::profile_dir() {
                b = b.data_directory(dir);
            }
        }
        let built = b.build();
        match built {
            Ok(win) => {
                debug_log::qs_log(&format!("window 'settings' created (attempt {attempt})"));
                let _ = win;
                return Ok(());
            }
            Err(e) => {
                debug_log::qs_log(&format!(
                    "window 'settings' create failed (attempt {attempt}): {e}"
                ));
                last = Some(e.to_string());
                std::thread::sleep(std::time::Duration::from_millis(450));
            }
        }
    }
    Err(tauri::Error::Anyhow(anyhow::anyhow!(
        "could not create window 'settings': {}",
        last.unwrap_or_default()
    )))
}

/// Run `f` forever, restarting it after any panic so a crash in a background
/// helper (hook pumps, the Quick Switch follower, …) never permanently
/// disables a subsystem.
pub fn supervise_forever<F: Fn() + Send + 'static>(name: &str, f: F) {
    let name = name.to_string();
    std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || loop {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(&f));
            if res.is_err() {
                debug_log::qs_log(&format!("thread {name} panicked -> restarting"));
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        })
        .ok();
}

/// Record every panic (message, location, backtrace) to
/// `%LOCALAPPDATA%\Kestrel\panic.log` before the process goes down. With
/// `panic = "abort"` removed the app usually survives a panic in a worker
/// thread, but FFI callbacks still abort — this keeps the evidence.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string payload>".into()
        };
        let bt = std::backtrace::Backtrace::force_capture();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let text = format!(
            "==== panic @ unix {} ====\nlocation: {}\nmessage: {}\nbacktrace:\n{}\n",
            ts, loc, msg, bt
        );
        eprintln!("{}", text);
        if let Ok(dir) = std::env::var("LOCALAPPDATA") {
            let dir = std::path::PathBuf::from(dir).join("Kestrel");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("panic.log"))
                .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()));
        }
    }));
}
