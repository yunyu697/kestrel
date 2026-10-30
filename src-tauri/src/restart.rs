//! Handshake for a self-relaunch with different privileges.
//!
//! Both instances share one WebView2 user-data folder, so they share a single
//! browser process. The outgoing process must therefore not release its webview
//! before the incoming one has attached its own: otherwise the shared browser
//! process is torn down and the new instance dies halfway through creating its
//! windows (log stops at "window 'launcher' creating"). A fixed sleep is a
//! guess; this flag file makes the hand-off deterministic — the old process
//! waits until the new one reports that its windows exist.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const FLAG: &str = "kestrel-restart";

/// Set once at startup: this process is the incoming half of a relaunch.
static RELAUNCHING: AtomicBool = AtomicBool::new(false);

pub fn mark_relaunching(on: bool) {
    RELAUNCHING.store(on, Ordering::Relaxed);
}

pub fn is_relaunching() -> bool {
    RELAUNCHING.load(Ordering::Relaxed)
}

fn flag_path() -> Option<PathBuf> {
    let dir = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(dir).join("Kestrel").join(FLAG))
}

/// WebView2 profile folder used by the incoming process of a relaunch.
///
/// All instances of an app share one WebView2 browser process through the
/// user-data folder. When the outgoing instance releases its webview, that
/// shared browser process goes down and the newcomer — even one that has
/// finished creating its windows — loses its webviews with it. A separate
/// profile makes the two processes completely independent.
pub fn profile_dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(base)
            .join("Kestrel")
            .join("EBWebView-relaunch"),
    )
}

/// Whether a relaunch flag is waiting (i.e. somebody is handing over to us).
pub fn pending() -> bool {
    flag_path().map(|p| p.exists()).unwrap_or(false)
}

/// Announce a relaunch before the new process is started.
pub fn begin() {
    let Some(path) = flag_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, std::process::id().to_string());
}

/// Called by the incoming process once its windows are up. Returns true when
/// this start was a relaunch (the flag was there to clear).
pub fn arrived() -> bool {
    match flag_path() {
        Some(path) if path.exists() => {
            let _ = std::fs::remove_file(&path);
            true
        }
        _ => false,
    }
}

/// Block until the incoming process clears the flag, or the timeout expires.
pub fn wait_arrival(timeout_ms: u64) {
    let Some(path) = flag_path() else { return };
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        if !path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
