//! In-app update: fetch the release installer and run it.
//!
//! The download happens on the Rust side (WinHTTP) for the same reasons the
//! update check does: the webview's content security policy can block
//! cross-origin requests and Chromium ignores the proxy environment
//! variables a lot of users rely on.
//!
//! Progress is pushed to the UI through the `update://progress` event so the
//! frontend can show a live bar instead of an indefinite spinner.

use crate::commands::net;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

/// Cancellation flags of the running downloads, keyed by job id.
fn jobs() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static JOBS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub id: String,
    pub received: u64,
    pub total: u64,
    /// "downloading" | "done" | "error" | "canceled"
    pub status: String,
    /// Final file path — only set when the download succeeded.
    pub path: String,
    pub error: String,
    pub ms: u64,
}

/// Where installers that are about to be installed right away are kept.
///
/// This is a private folder: those files are a means to an end, not something
/// the user asked to keep.
pub fn update_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Kestrel")
        .join("updates")
}

/// The user's real Downloads folder, honouring a relocation made through
/// Explorer's folder properties (known-folder redirection), not a hard-coded
/// `%USERPROFILE%\Downloads`.
pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
}

/// Pick the folder for a download. `mode` is "download" (the user pressed
/// "仅下载" and expects the installer to land in Downloads) or "install"
/// (the file is consumed immediately by the install helper).
fn dest_dir(mode: &str) -> PathBuf {
    if mode == "download" {
        if let Some(dir) = downloads_dir() {
            return dir;
        }
    }
    update_dir()
}

/// Never overwrite a file the user already has in Downloads. A second
/// "仅下载" of the same release becomes `Kestrel_1.2.13_x64 (1).msi`.
fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "kestrel-update".into());
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    for n in 1..100 {
        let name = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        let candidate = path.with_file_name(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

/// Keep only the file name: a caller must never be able to write outside the
/// download folder.
fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .filter(|c| !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'))
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        "kestrel-update.bin".to_string()
    } else {
        cleaned
    }
}

/// Drop the installers of older versions from our private update folder.
/// Only one update is ever relevant, and the folder would otherwise grow with
/// every release.
///
/// Two things keep this harmless: it is never called on the user's Downloads
/// folder, and it only removes files we produced ourselves (the `Kestrel_`
/// prefix). Deleting every .exe/.msi/.zip in a shared folder would destroy
/// somebody else's downloads.
fn prune_old_installers(dir: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep {
            continue;
        }
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if (lower.starts_with("kestrel_") || lower.starts_with("kestrel-"))
            && (lower.ends_with(".msi") || lower.ends_with(".exe") || lower.ends_with(".zip"))
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Start downloading an installer in the background.
///
/// `mode` is "install" (the file is handed to the installer as soon as it is
/// ready) or "download" (the user wants to keep it — save to the real
/// Downloads folder). Returns a job id; progress arrives through the
/// `update://progress` event.
#[tauri::command]
pub fn update_download(
    app: AppHandle,
    url: String,
    file_name: String,
    mode: Option<String>,
) -> String {
    let id = format!("u{}", crate::commands::now_ms());
    let cancel = Arc::new(AtomicBool::new(false));
    jobs().lock().unwrap().insert(id.clone(), cancel.clone());

    let mode = mode.unwrap_or_else(|| "install".into());
    let keep_file = mode == "download";
    let dir = dest_dir(&mode);
    if std::fs::create_dir_all(&dir).is_err() {
        jobs().lock().unwrap().remove(&id);
        let _ = app.emit(
            "update://progress",
            UpdateProgress {
                id: id.clone(),
                received: 0,
                total: 0,
                status: "error".into(),
                path: String::new(),
                error: "无法创建下载目录".into(),
                ms: 0,
            },
        );
        return id;
    }
    let dest = unique_path(dir.join(safe_name(&file_name)));

    let thread_id = id.clone();
    let thread_cancel = cancel.clone();
    let thread_dest = dest.clone();
    let thread_keep = keep_file;
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let emit = |app: &AppHandle, status: &str, received: u64, total: u64, path: &str, error: &str| {
            let _ = app.emit(
                "update://progress",
                UpdateProgress {
                    id: thread_id.clone(),
                    received,
                    total,
                    status: status.to_string(),
                    path: path.to_string(),
                    error: error.to_string(),
                    ms: started.elapsed().as_millis() as u64,
                },
            );
        };

        let result = net::download_to_file(&url, &thread_dest, 180_000, |received, total| {
            if thread_cancel.load(Ordering::Relaxed) {
                return false;
            }
            emit(&app, "downloading", received, total, "", "");
            true
        });

        match result {
            Ok(n) => {
                // Only the private update folder is ours to clean. The user's
                // Downloads folder may contain anything — never prune it.
                if !thread_keep {
                    prune_old_installers(&dir, &thread_dest);
                }
                emit(&app, "done", n, n, &thread_dest.to_string_lossy(), "");
            }
            Err(e) => {
                let canceled = thread_cancel.load(Ordering::Relaxed);
                let _ = std::fs::remove_file(&thread_dest);
                if canceled {
                    emit(&app, "canceled", 0, 0, "", "");
                } else {
                    emit(&app, "error", 0, 0, "", &e);
                }
            }
        }
        jobs().lock().unwrap().remove(&thread_id);
    });

    id
}

/// Stop a running download.
#[tauri::command]
pub fn update_cancel(id: String) -> bool {
    if let Some(flag) = jobs().lock().unwrap().get(&id) {
        flag.store(true, Ordering::Relaxed);
        return true;
    }
    false
}

/// Hand the downloaded installer over to a detached helper that waits for
/// this process to exit and then runs it.
#[tauri::command]
pub fn update_install(path: String) -> bool {
    let path = path.trim();
    if path.is_empty() {
        return false;
    }
    crate::install::spawn_helper(path)
}

/// Show a downloaded installer in Explorer.
#[tauri::command]
pub fn update_reveal(path: String) -> bool {
    if path.trim().is_empty() {
        return false;
    }
    crate::win::shell::reveal(path.trim())
}

/// How this copy was installed: "perMachine" (MSI, Program Files) or
/// "currentUser" (NSIS, AppData). The updater uses it to pick the installer
/// that replaces *this* installation instead of adding a second one.
#[tauri::command]
pub fn install_mode() -> String {
    let Ok(exe) = std::env::current_exe() else {
        return "perMachine".into();
    };
    let path = exe.to_string_lossy().to_ascii_lowercase();
    let local = dirs::data_local_dir()
        .map(|d| d.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !local.is_empty() && path.starts_with(&local) {
        "currentUser".into()
    } else {
        "perMachine".into()
    }
}

#[cfg(test)]
mod tests {
    use super::{dest_dir, downloads_dir, safe_name, unique_path, update_dir};
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("kestrel-update-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn keeps_a_plain_file_name() {
        assert_eq!(
            safe_name("Kestrel_1.2.4_x64_zh-CN.msi"),
            "Kestrel_1.2.4_x64_zh-CN.msi"
        );
    }

    #[test]
    fn strips_directories() {
        assert_eq!(safe_name("C:\\evil\\payload.msi"), "payload.msi");
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
    }

    #[test]
    fn strips_reserved_characters() {
        assert_eq!(safe_name("a:b*c?d\"e<f>g|h.msi"), "abcdefgh.msi");
    }

    #[test]
    fn falls_back_when_nothing_is_left() {
        assert_eq!(safe_name(""), "kestrel-update.bin");
        assert_eq!(safe_name("..."), "kestrel-update.bin");
    }

    #[test]
    fn download_only_uses_the_users_downloads_folder() {
        let expected = downloads_dir().expect("Windows always has a Downloads folder");
        assert_eq!(dest_dir("download"), expected);
        assert!(expected.to_string_lossy().to_ascii_lowercase().contains("download"));
        assert_ne!(dest_dir("download"), update_dir());
    }

    #[test]
    fn install_mode_uses_the_private_folder() {
        assert_eq!(dest_dir("install"), update_dir());
        assert_eq!(dest_dir(""), update_dir());
    }

    #[test]
    fn repeated_downloads_do_not_overwrite_each_other() {
        let dir = scratch("repeat");
        let first = dir.join("Kestrel_1.2.13_x64.msi");
        std::fs::write(&first, b"first").unwrap();
        let second = unique_path(first.clone());
        assert_eq!(second, dir.join("Kestrel_1.2.13_x64 (1).msi"));
        assert_eq!(std::fs::read(&first).unwrap(), b"first", "original kept");
        std::fs::write(&second, b"second").unwrap();
        let third = unique_path(first.clone());
        assert_eq!(third, dir.join("Kestrel_1.2.13_x64 (2).msi"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_fresh_path_is_used_unchanged() {
        let dir = scratch("fresh");
        let p = dir.join("new.msi");
        assert_eq!(unique_path(p.clone()), p);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
