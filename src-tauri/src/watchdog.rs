//! Optional watchdog ("安装为服务"): keeps Kestrel alive when it is closed
//! unexpectedly. A scheduled task (`KestrelWatchdog`) starts this module with
//! `--watchdog` at logon; it respawns the app unless the user quit on purpose
//! (quit flag) or requested uninstall (stop flag).

use crate::win::{wide, CloseHandle, CreateMutexW, GetLastError};
use std::path::PathBuf;

const ALIVE_MUTEX: &str = "Local\\KestrelAliveMutex";
const STOP_FLAG: &str = "kestrel-watchdog-stop";
const QUIT_FLAG: &str = "kestrel-quit";

pub fn flag_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Written right before a deliberate exit so the watchdog won't respawn us.
pub fn mark_graceful_quit() {
    if let Some(p) = flag_dir().map(|d| d.join(QUIT_FLAG)) {
        let _ = std::fs::write(&p, now().to_string());
    }
}

/// Cleared on startup so a later crash is guarded again.
pub fn clear_quit_flag() {
    if let Some(p) = flag_dir().map(|d| d.join(QUIT_FLAG)) {
        let _ = std::fs::remove_file(&p);
    }
}

pub fn mark_stop() {
    if let Some(p) = flag_dir().map(|d| d.join(STOP_FLAG)) {
        let _ = std::fs::write(&p, now().to_string());
    }
}

pub fn clear_stop_flag() {
    if let Some(p) = flag_dir().map(|d| d.join(STOP_FLAG)) {
        let _ = std::fs::remove_file(&p);
    }
}

/// Hold a named mutex so the watchdog can tell the app is alive. The handle
/// is intentionally never closed: the OS releases it when the process dies.
pub fn hold_alive_mutex() {
    let w = wide(ALIVE_MUTEX);
    unsafe {
        // A raw Win32 handle has no destructor, so simply not closing it is
        // enough to keep it alive for the lifetime of the process.
        let _h = CreateMutexW(std::ptr::null_mut(), 1, w.as_ptr());
    }
}

fn app_alive() -> bool {
    let w = wide(ALIVE_MUTEX);
    unsafe {
        let h = CreateMutexW(std::ptr::null_mut(), 0, w.as_ptr());
        let exists = GetLastError() == 183; // ERROR_ALREADY_EXISTS
        if h != 0 {
            CloseHandle(h);
        }
        exists
    }
}

/// Watchdog loop (`kestrel.exe --watchdog`). Never shows any UI.
pub fn run() {
    let Some(dir) = flag_dir() else { return };
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return,
    };
    let stop = dir.join(STOP_FLAG);
    let quit = dir.join(QUIT_FLAG);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3));
        if stop.exists() {
            let _ = std::fs::remove_file(&stop);
            return;
        }
        if quit.exists() {
            // The user closed Kestrel on purpose: leave it closed.
            continue;
        }
        if app_alive() {
            continue;
        }
        // Give a shutting-down instance a moment before respawning.
        std::thread::sleep(std::time::Duration::from_secs(4));
        if !app_alive() && !quit.exists() && !stop.exists() {
            crate::win::shell::spawn(&exe.to_string_lossy(), None, None);
        }
    }
}
