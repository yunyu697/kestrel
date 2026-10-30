//! Settings, build info, applications and clipboard commands.

use crate::clipboard as clip;
use crate::model::{AppEntry, BuildInfo, ClipboardItem, Settings};
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Settings {
    state.settings.read().clone()
}

pub fn apply_side_effects(app: &AppHandle, state: &Arc<AppState>) {
    let s = state.settings.read().clone();

    crate::hotkeys::set_double_ctrl(s.double_ctrl, s.double_ctrl_ms as u64);
    crate::hotkeys::set_quick_switch(s.quick_switch_enabled);
    crate::hotkeys::set_replace_win_search(s.replace_win_search);
    crate::hotkeys::register_toggle_shortcut(app, &s.hotkey_toggle);

    let autolaunch = app.autolaunch();
    let enabled = autolaunch.is_enabled().unwrap_or(false);
    if s.launch_at_startup && !enabled {
        let _ = autolaunch.enable();
    } else if !s.launch_at_startup && enabled {
        let _ = autolaunch.disable();
    }
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, Arc<AppState>>, settings: Settings) -> Result<(), String> {
    let old_roots = state.settings.read().indexed_drives.clone();
    let old_excludes = state.settings.read().exclude_dirs.clone();
    let old_lang = state.settings.read().language.clone();
    let mut settings = settings;
    // The built-in folders are pinned: a full-settings save must never be
    // able to drop them or clear their enabled flag.
    crate::model::normalize_favorites(&mut settings.favorites);
    crate::model::ensure_fixed_favorites(&mut settings.favorites, &settings.language);
    *state.settings.write() = settings.clone();
    state.save_settings();
    apply_side_effects(&app, state.inner());

    // Language changed: re-label the tray menu and re-translate any seeded
    // preset the user never renamed, so switching is instant everywhere.
    if old_lang != settings.language {
        crate::ui::refresh_tray_language(&app, &settings.language);
        let mut cmds = settings.custom_commands.clone();
        if crate::model::relocalize_presets(&mut cmds, &settings.language) {
            let mut next = settings.clone();
            next.custom_commands = cmds;
            *state.settings.write() = next.clone();
            state.save_settings();
            let _ = app.emit("settings://changed", &next);
        }
    }

    if old_roots != settings.indexed_drives || old_excludes != settings.exclude_dirs {
        crate::commands::index::restart_watcher(&app, state.inner());
        crate::commands::index::spawn_rebuild(app.clone(), state.inner().clone());
    }
    let _ = app.emit("settings://changed", &settings);
    Ok(())
}

/// Register the main toggle shortcut and remember it.
///
/// Returns `false` when the system refuses the combination (another program
/// already owns it) so the UI can tell the user instead of silently doing
/// nothing.
#[tauri::command]
pub fn apply_hotkey(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    accelerator: String,
) -> bool {
    if accelerator.trim().is_empty() {
        return false;
    }
    let ok = crate::hotkeys::register_toggle_shortcut(&app, accelerator.trim());
    if ok {
        {
            let mut s = state.settings.write();
            s.hotkey_toggle = accelerator.trim().to_string();
        }
        state.save_settings();
    }
    ok
}

#[tauri::command]
pub fn reset_settings(app: AppHandle, state: State<'_, Arc<AppState>>) -> Settings {
    let fresh = Settings::default();
    *state.settings.write() = fresh.clone();
    state.save_settings();
    apply_side_effects(&app, state.inner());
    let _ = app.emit("settings://changed", &fresh);
    fresh
}

#[tauri::command]
pub fn build_info(state: State<'_, Arc<AppState>>) -> BuildInfo {
    BuildInfo {
        name: "Kestrel".into(),
        version: env!("KESTREL_VERSION").into(),
        build: env!("KESTREL_BUILD").parse().unwrap_or(0),
        built_at: env!("KESTREL_BUILT_AT").into(),
        arch: std::env::consts::ARCH.into(),
        os: format!("Windows {}", std::env::consts::OS),
        index_count: state.index.count(),
        index_roots: state.index.roots.read().clone(),
    }
}

#[tauri::command]
pub fn list_apps(state: State<'_, Arc<AppState>>) -> Vec<AppEntry> {
    state.apps.read().clone()
}

#[tauri::command]
pub async fn refresh_apps(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<Vec<AppEntry>, String> {
    let list = tauri::async_runtime::spawn_blocking(crate::apps::scan_apps)
        .await
        .map_err(|e| e.to_string())?;
    *state.apps.write() = list.clone();
    state.save_apps();
    let _ = app.emit("apps://refreshed", ());
    Ok(list)
}

// ---------------- clipboard ----------------

#[tauri::command]
pub fn clipboard_list(state: State<'_, Arc<AppState>>) -> Vec<ClipboardItem> {
    state.clipboard.read().clone()
}

#[tauri::command]
pub fn clipboard_copy(state: State<'_, Arc<AppState>>, id: String) -> bool {
    let item = state.clipboard.read().iter().find(|i| i.id == id).cloned();
    match item {
        Some(it) => {
            clip::PAUSED.store(true, std::sync::atomic::Ordering::Relaxed);
            let ok = clip::restore(&it);
            std::thread::sleep(std::time::Duration::from_millis(180));
            clip::PAUSED.store(false, std::sync::atomic::Ordering::Relaxed);
            ok
        }
        None => false,
    }
}

#[tauri::command]
pub fn clipboard_delete(state: State<'_, Arc<AppState>>, id: String) {
    state.clipboard.write().retain(|i| i.id != id);
    state.save_clipboard();
}

#[tauri::command]
pub fn clipboard_pin(state: State<'_, Arc<AppState>>, id: String, pinned: bool) {
    if let Some(item) = state.clipboard.write().iter_mut().find(|i| i.id == id) {
        item.pinned = pinned;
    }
    state.save_clipboard();
}

#[tauri::command]
pub fn clipboard_clear(state: State<'_, Arc<AppState>>) {
    state.clipboard.write().retain(|i| i.pinned);
    state.save_clipboard();
}

#[tauri::command]
pub fn clipboard_push_text(state: State<'_, Arc<AppState>>, text: String) {
    if let Ok(mut c) = arboard::Clipboard::new() {
        let _ = c.set_text(text);
    }
    let _ = state;
}

// ---------------- favorites ----------------

#[tauri::command]
pub fn favorites_add(state: State<'_, Arc<AppState>>, path: String, name: Option<String>) -> Vec<crate::model::Favorite> {
    let display = name.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone())
    });
    let mut s = state.settings.write();
    if !s.favorites.iter().any(|f| f.path == path) {
        let id = format!("fav-{}", chrono::Local::now().timestamp_millis());
        s.favorites.push(crate::model::Favorite {
            id,
            name: display,
            path: path.clone(),
            is_dir: std::path::Path::new(&path).is_dir(),
            enabled: true,
            pinned: false,
        });
    }
    let list = s.favorites.clone();
    drop(s);
    state.save_settings();
    list
}

/// Remove a favorite. Pinned (built-in) favorites are never removed — they can
/// only be disabled or re-pointed.
#[tauri::command]
pub fn favorites_remove(state: State<'_, Arc<AppState>>, id: String) -> Vec<crate::model::Favorite> {
    let mut s = state.settings.write();
    s.favorites.retain(|f| f.id != id || f.pinned);
    let list = s.favorites.clone();
    drop(s);
    state.save_settings();
    list
}

/// Switch a favorite on or off. Disabled favorites stay in the list but no
/// longer show up in the launcher results.
#[tauri::command]
pub fn favorites_set_enabled(
    state: State<'_, Arc<AppState>>,
    id: String,
    enabled: bool,
) -> Vec<crate::model::Favorite> {
    {
        let mut s = state.settings.write();
        if let Some(f) = s.favorites.iter_mut().find(|f| f.id == id) {
            f.enabled = enabled;
        }
    }
    state.save_settings();
    state.settings.read().favorites.clone()
}

/// Re-point a favorite at another file or folder. The display name is kept:
/// for a built-in folder it names the slot, not the target.
#[tauri::command]
pub fn favorites_set_path(
    state: State<'_, Arc<AppState>>,
    id: String,
    path: String,
) -> Vec<crate::model::Favorite> {
    {
        let mut s = state.settings.write();
        if let Some(f) = s.favorites.iter_mut().find(|f| f.id == id) {
            f.path = path.clone();
            f.is_dir = std::path::Path::new(&path).is_dir();
        }
    }
    state.save_settings();
    state.settings.read().favorites.clone()
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) {
    crate::ui::open_settings(&app);
}

/// Whether the current process holds an elevated (administrator) token.
#[tauri::command]
pub fn is_elevated() -> bool {
    crate::win::is_elevated()
}

/// Turn "run as administrator" on or off.
///
/// Elevation is a property of the process, so it cannot be toggled in place:
/// the setting is stored and the app restarts itself — elevated when turning
/// on, unelevated when turning off. Returns `false` when the user dismissed
/// the UAC prompt, in which case the setting is rolled back and the caller
/// keeps running exactly as before.
#[tauri::command]
pub fn set_run_as_admin(app: AppHandle, state: State<'_, Arc<AppState>>, enabled: bool) -> String {
    let already = crate::win::is_elevated();
    if enabled == already {
        // Nothing to restart: just remember the choice for the next start.
        state.settings.write().run_as_admin = enabled;
        state.save_settings();
        return "saved".into();
    }
    let launched = if enabled {
        crate::restart::begin();
        crate::win::shell::relaunch_self("--no-single-instance", true)
    } else {
        crate::restart::begin();
        crate::win::shell::relaunch_self_unelevated("--no-single-instance")
    };
    if !launched {
        // Dismissed UAC, or the shell token could not be duplicated. Nothing is
        // stored, so a later start never surprises the user with a prompt.
        crate::restart::arrived();
        crate::debug_log::qs_log(&format!(
            "run_as_admin: relaunch failed (enabled={enabled}, elevated={already})"
        ));
        return "failed".into();
    }
    state.settings.write().run_as_admin = enabled;
    state.save_settings();
    crate::debug_log::qs_log(&format!("run_as_admin: restarting (enabled={enabled})"));
    // Tell the watchdog this exit is deliberate, otherwise it would respawn an
    // UNELEVATED copy a few seconds later and the UAC prompt would loop.
    crate::watchdog::mark_graceful_quit();
    // Hold on until the new process has its windows: both instances share one
    // WebView2 browser process, and leaving first would tear it down under the
    // newcomer's feet.
    crate::restart::wait_arrival(8000);
    app.exit(0);
    "restarted".into()
}

#[tauri::command]
pub fn hide_launcher_window(app: AppHandle) {
    crate::ui::hide_launcher(&app);
}

#[tauri::command]
pub fn show_launcher_window(app: AppHandle) {
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        crate::ui::show_launcher(&app, &state);
    }
}

#[tauri::command]
pub fn set_always_on_top(app: AppHandle, on_top: bool) {
    if let Some(w) = app.get_webview_window("launcher") {
        let _ = w.set_always_on_top(on_top);
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::watchdog::mark_graceful_quit();
    app.exit(0);
}
