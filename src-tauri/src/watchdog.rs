//! Optional watchdog ("安装为服务"): keeps Kestrel alive when it is closed
//! unexpectedly. A scheduled task (`KestrelWatchdog`) starts this module with
//! `--watchdog` at logon; it respawns the app unless the user quit on purpose
//! (quit flag) or requested uninstall (stop flag).

use crate::win::{wide, CloseHandle, CreateMutexW, GetLastError};
use std::path::PathBuf;

const ALIVE_MUTEX: &str = "Local\\KestrelAliveMutex";
const STOP_FLAG: &str = "kestrel-watchdog-stop";
const QUIT_FLAG: &str = "kestrel-quit";

/// Flag files live under `%LOCALAPPDATA%\Kestrel`, never next to the exe.
/// An installed copy sits in `C:\Program Files\Kestrel`, which a normal
/// (unelevated) process cannot write — so quit/stop flags silently failed and
/// the watchdog kept respawning the app after a deliberate quit. The log and
/// restart handshake already use this same user-writable directory.
pub fn flag_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join("Kestrel");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Atomically write a flag file via a temp file + rename. A reader (the
/// watchdog) only checks existence, so a half-written flag must never be
/// visible. Windows `rename` fails when the target exists, so remove it first.
/// Any failure is logged — silently swallowing it was how the original
/// "flags next to the exe" bug stayed invisible for so long.
fn write_flag(path: &std::path::Path, name: &str) {
    let tmp = path.with_extension("tmp");
    if let Err(e) = std::fs::write(&tmp, now().to_string()) {
        crate::debug_log::qs_log(&format!("watchdog: write {name}.tmp failed: {e}"));
        return;
    }
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        crate::debug_log::qs_log(&format!("watchdog: rename {name} failed: {e}"));
    }
}

/// Written right before a deliberate exit so the watchdog won't respawn us.
pub fn mark_graceful_quit() {
    if let Some(p) = flag_dir().map(|d| d.join(QUIT_FLAG)) {
        write_flag(&p, QUIT_FLAG);
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
        write_flag(&p, STOP_FLAG);
    }
}

pub fn clear_stop_flag() {
    if let Some(p) = flag_dir().map(|d| d.join(STOP_FLAG)) {
        let _ = std::fs::remove_file(&p);
    }
}

/// Older builds kept the flags next to the executable. Nothing reads them any
/// more, but they sit in `C:\Program Files\Kestrel` and look live to anyone
/// debugging a "关掉又自己起来" report. Removing them needs elevation, so this
/// is best-effort and logs only when it actually succeeds.
pub fn clean_legacy_flags() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    for name in [QUIT_FLAG, STOP_FLAG] {
        if remove_flag_in(dir, name) {
            crate::debug_log::qs_log(&format!(
                "watchdog: removed legacy flag {name} next to the exe"
            ));
        }
    }
}

/// Split out so the cleanup is testable without controlling the test binary's
/// own location. Returns whether a file was actually removed.
fn remove_flag_in(dir: &std::path::Path, name: &str) -> bool {
    let p = dir.join(name);
    p.exists() && std::fs::remove_file(&p).is_ok()
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

#[cfg(test)]
mod tests {
    use super::{remove_flag_in, QUIT_FLAG, STOP_FLAG};

    fn scratch(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("kestrel-watchdog-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("temp dir");
        p
    }

    #[test]
    fn legacy_flags_are_removed_and_absent_ones_report_false() {
        let dir = scratch("flags");
        std::fs::write(dir.join(QUIT_FLAG), b"1794900000").expect("quit flag");
        std::fs::write(dir.join(STOP_FLAG), b"1794900000").expect("stop flag");

        assert!(remove_flag_in(&dir, QUIT_FLAG));
        assert!(!dir.join(QUIT_FLAG).exists());
        assert!(remove_flag_in(&dir, STOP_FLAG));
        // A second pass must be a no-op, not an error report.
        assert!(!remove_flag_in(&dir, STOP_FLAG));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
