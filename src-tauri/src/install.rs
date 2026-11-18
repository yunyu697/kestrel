//! `kestrel.exe --install <file> --parent <pid> --exe <original exe>`
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
use crate::win::{CloseHandle, OpenProcess, WaitForSingleObject, HANDLE};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Spawn the detached helper that will run the installer.
///
/// The helper is staged in the temp directory rather than started in place: a
/// process holds an open image lock on its own executable, so a helper running
/// from `C:\Program Files\Kestrel` prevented the installer from replacing
/// `kestrel.exe` — silently, because MSI has no FilesInUse dialog in a quiet
/// install.
pub fn spawn_helper(installer: &str) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let pid = std::process::id();
    let args = format!(
        "--install \"{installer}\" --parent {pid} --exe \"{}\"",
        exe.display()
    );
    if let Some(staged) = stage_copy(&exe, pid) {
        let dir = stage_dir(pid);
        let staged_s = staged.to_string_lossy().to_string();
        let dir_s = dir.to_string_lossy().to_string();
        if shell::spawn(&staged_s, Some(&args), Some(&dir_s)) {
            crate::debug_log::qs_log(&format!("install helper: staged at {staged_s}"));
            return true;
        }
        crate::debug_log::qs_log(
            "install helper: staged copy would not start, using the exe in place",
        );
    }
    shell::spawn(&exe.to_string_lossy(), Some(&args), None)
}

/// Where the helper copy lives for a given parent process.
fn stage_dir(pid: u32) -> PathBuf {
    std::env::temp_dir().join(format!("kestrel-update-{pid}"))
}

/// Copy the executable and any sibling DLL into the staging directory.
///
/// The DLLs are not strictly needed today (the binary links the library
/// statically), but `crate-type` also emits a cdylib, and a copy that fails to
/// start would break updates in a way that is much harder to diagnose than an
/// extra 150 KB in temp.
fn stage_copy(exe: &Path, pid: u32) -> Option<PathBuf> {
    let dir = stage_dir(pid);
    std::fs::create_dir_all(&dir).ok()?;
    let target = dir.join("kestrel.exe");
    std::fs::copy(exe, &target).ok()?;
    if let Some(parent) = exe.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let src = entry.path();
                let is_dll = src
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("dll"));
                if is_dll {
                    let _ = std::fs::copy(&src, dir.join(entry.file_name()));
                }
            }
        }
    }
    Some(target)
}

/// Remove staging directories left behind by earlier updates.
///
/// Best-effort by design: the running helper's own directory cannot be deleted,
/// and a failed attempt is not worth logging on every start.
pub fn cleanup_stale() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with("kestrel-update-") && std::fs::remove_dir_all(entry.path()).is_ok() {
            crate::debug_log::qs_log(&format!("install helper: cleaned stale {name}"));
        }
    }
}

/// Windows Installer only understands backslashes: a path with forward
/// slashes makes msiexec fail with 1619 ("package could not be opened").
fn native_path(path: &str) -> String {
    path.replace('/', "\\")
}

/// Entry point of the helper mode.
///
/// `target` is the executable that was running when the update started. It has
/// to be passed in because this process no longer lives there — asking
/// `current_exe()` after staging would relaunch the staged copy in temp
/// instead of the installed one.
pub fn run(installer: &str, parent_pid: u32, target: Option<&str>) {
    let installer = native_path(installer);
    // Hold the watchdog's alive mutex while the installer runs. Otherwise the
    // watchdog sees "no Kestrel running" in the gap between our exit and the
    // installer's start and respawns the old binary — which is exactly the
    // "file in use" failure the helper exists to avoid.
    crate::watchdog::hold_alive_mutex();
    wait_for_parent(parent_pid);
    let ok = launch(&installer);
    crate::debug_log::qs_log(&format!(
        "install helper: installer={installer} launched={ok}"
    ));
    // Start Kestrel again either way. A failed install used to leave the user
    // with nothing at all: the app had quit and was still on the old version,
    // which is exactly what "退出但没更新" looks like from the outside.
    std::thread::sleep(Duration::from_millis(if ok { 1200 } else { 600 }));
    relaunch(target);
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

/// Run the installer and wait for it to finish.
fn launch(installer: &str) -> bool {
    let path = PathBuf::from(installer);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "msi" => install_msi(installer),
        // Tauri's NSIS installer understands /S for a silent install, and a
        // per-user install needs no elevation.
        "exe" => run_waited(installer, "/S"),
        // A zip cannot be installed — show it in Explorer instead.
        _ => {
            shell::reveal(installer);
            false
        }
    }
}

/// Install an MSI, elevating when this process cannot write to the target.
///
/// This is where every update died before: the helper ran
/// `msiexec /i … /quiet` as the logged-on user. A per-machine package installed
/// under Program Files needs an administrator token, so Windows Installer gave
/// up with 1603 and its own log recorded 1730 — "you must be an administrator
/// to remove this application" — while the app restarted on the old version.
///
/// `/passive` rather than `/quiet` keeps a progress bar on screen. A fully
/// silent install made the only visible effect of pressing 更新 the app
/// vanishing.
fn install_msi(installer: &str) -> bool {
    let args = format!("/i \"{installer}\" /passive /norestart");
    if crate::win::is_elevated() {
        return run_waited("msiexec.exe", &args);
    }
    match shell::run_elevated_waited("msiexec.exe", &args, None) {
        shell::ElevateRun::Exited(code) => {
            crate::debug_log::qs_log(&format!(
                "install helper: msiexec.exe (elevated) exited with code {code}"
            ));
            // 3010 and 1641 are "succeeded, reboot required".
            matches!(code, 0 | 3010 | 1641)
        }
        shell::ElevateRun::Cancelled => {
            crate::debug_log::qs_log("install helper: elevation was cancelled by the user");
            false
        }
        shell::ElevateRun::Failed(err) => {
            crate::debug_log::qs_log(&format!(
                "install helper: could not start msiexec elevated (error {err})"
            ));
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
/// The executable path does not change during an in-place upgrade, so the file
/// sitting there now is the new one — and after a failed install it is still
/// the old one, which is better than no app at all.
fn relaunch(target: Option<&str>) {
    let exe = match target.map(PathBuf::from) {
        Some(p) if p.as_os_str().is_empty() => return,
        Some(p) => p,
        None => match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return,
        },
    };
    if !exe.exists() {
        return;
    }
    crate::debug_log::qs_log(&format!("install helper: relaunching {}", exe.display()));
    let _ = shell::spawn(&exe.to_string_lossy(), None, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_forward_slashes_into_backslashes() {
        assert_eq!(
            native_path("C:/Users/someone/app.msi"),
            "C:\\Users\\someone\\app.msi"
        );
        assert_eq!(
            native_path("C:\\already\\native.msi"),
            "C:\\already\\native.msi"
        );
    }

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

    #[test]
    fn the_stage_directory_is_per_process() {
        // Two concurrent updates must not overwrite each other's copy.
        assert_ne!(stage_dir(1), stage_dir(2));
        assert!(stage_dir(4242)
            .to_string_lossy()
            .ends_with("kestrel-update-4242"));
    }
}
