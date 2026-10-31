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

/// Where downloaded installers are kept.
pub fn update_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Kestrel")
        .join("updates")
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

/// Drop the installers of older versions. Only one update is ever relevant,
/// and the folder would otherwise grow with every release.
fn prune_old_installers(dir: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep {
            continue;
        }
        let name = path.to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".msi") || name.ends_with(".exe") || name.ends_with(".zip") {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Start downloading an installer in the background.
///
/// Returns a job id; the progress arrives through the `update://progress`
/// event.
#[tauri::command]
pub fn update_download(app: AppHandle, url: String, file_name: String) -> String {
    let id = format!("u{}", crate::commands::now_ms());
    let cancel = Arc::new(AtomicBool::new(false));
    jobs().lock().unwrap().insert(id.clone(), cancel.clone());

    let dir = update_dir();
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
    let dest = dir.join(safe_name(&file_name));

    let thread_id = id.clone();
    let thread_cancel = cancel.clone();
    let thread_dest = dest.clone();
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
                prune_old_installers(&dir, &thread_dest);
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
    if path.trim().is_empty() {
        return false;
    }
    crate::install::spawn_helper(path.trim())
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
    use super::safe_name;

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
}
