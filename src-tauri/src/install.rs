//! `kestrel.exe --install <file> --parent <pid>`
//!
//! An installer cannot replace Kestrel's own files while Kestrel is running:
//! Windows Installer would either refuse or schedule the replacement for the
//! next reboot. So the running app hands the job to a detached copy of
//! itself. The copy waits until the parent process is really gone, runs the
//! installer, and finally starts the freshly installed version.
//!
//! The copy is the same executable, which is why this mode is dispatched
//! before anything else — in particular before the single-instance plugin,
//! whose whole purpose is to *not* start a second copy.

use crate::win::shell;
use crate::win::{CloseHandle, OpenProcess, HANDLE};
use std::path::PathBuf;
use std::time::Duration;

/// Spawn the detached helper that will run the installer.
///
/// The helper inherits nothing from the parent: it is a fresh process that
/// only shares the executable path.
pub fn spawn_helper(installer: &str) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let pid = std::process::id();
    let args = format!("--install \"{installer}\" --parent {pid}");
    shell::spawn(&exe.to_string_lossy(), Some(&args), None)
}

/// Entry point of the helper mode.
pub fn run(installer: &str, parent_pid: u32) {
    // Hold the watchdog's alive mutex while the installer runs. Otherwise the
    // watchdog sees "no Kestrel running" in the gap between our exit and the
    // installer's start and respawns the old binary — which is exactly the
    // "file in use" failure the helper exists to avoid.
    crate::watchdog::hold_alive_mutex();
    wait_for_parent(parent_pid);
    let ok = launch(installer);
    crate::debug_log::qs_log(&format!(
        "install helper: installer={installer} launched={ok}"
    ));
    if !ok {
        return;
    }
    // The installer may still be flushing files when its process exits.
    std::thread::sleep(Duration::from_millis(1200));
    relaunch();
}

/// Block until the given process is gone (or two minutes pass).
fn wait_for_parent(pid: u32) {
    if pid == 0 {
        return;
    }
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const TIMEOUT_MS: u32 = 120_000;
    unsafe {
        let h: HANDLE = OpenProcess(SYNCHRONIZE, 0, pid);
        if h == 0 {
            // The parent is already gone — nothing to wait for.
            return;
        }
        WaitForSingleObject(h, TIMEOUT_MS);
        CloseHandle(h);
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn WaitForSingleObject(h: HANDLE, ms: u32) -> u32;
}

/// Run the installer and wait for it to finish.
fn launch(installer: &str) -> bool {
    let path = PathBuf::from(installer);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        // Windows Installer: a quiet install of the new version over the old
        // one. A per-machine package still asks for elevation on its own.
        "msi" => run_waited("msiexec.exe", &format!("/i \"{installer}\" /quiet /norestart")),
        // Tauri's NSIS installer understands /S for a silent install.
        "exe" => run_waited(installer, "/S"),
        // A zip cannot be installed — show it in Explorer instead.
        _ => {
            shell::reveal(installer);
            false
        }
    }
}

/// Run a program and wait for it, without a console window flashing up.
fn run_waited(program: &str, args: &str) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let result = std::process::Command::new(program)
        // `raw_arg` keeps the quoted paths intact, which splitting on spaces
        // would not.
        .raw_arg(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    match result {
        Ok(status) => {
            let code = status.code().unwrap_or(-1);
            crate::debug_log::qs_log(&format!(
                "install helper: {program} exited with code {code}"
            ));
            status.success()
        }
        Err(e) => {
            crate::debug_log::qs_log(&format!(
                "install helper: {program} could not be started: {e}"
            ));
            false
        }
    }
}

/// Start the installed version again.
///
/// The executable path does not change during an in-place upgrade, so the
/// file sitting there now is the new one.
fn relaunch() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let target = dir.join("kestrel.exe");
    if !target.exists() {
        return;
    }
    crate::debug_log::qs_log(&format!("install helper: relaunching {}", target.display()));
    let _ = shell::spawn(&target.to_string_lossy(), None, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_at_once_without_a_parent() {
        let start = std::time::Instant::now();
        wait_for_parent(0);
        assert!(start.elapsed().as_millis() < 500);
    }

    #[test]
    fn returns_at_once_for_a_parent_that_is_already_gone() {
        let start = std::time::Instant::now();
        // A pid that cannot be running.
        wait_for_parent(0xFFFF_FFFE);
        assert!(start.elapsed().as_millis() < 500);
    }
}
