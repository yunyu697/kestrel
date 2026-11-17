//! Index lifecycle commands.

use crate::index::{self as idx, scan::ScanOptions, watch};
use crate::model::IndexStatus;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub fn status_of(state: &Arc<AppState>) -> IndexStatus {
    let scanning = state.index.scanning.load(Ordering::Relaxed);
    IndexStatus {
        state: if scanning {
            "scanning".into()
        } else if state.index.ready.load(Ordering::Relaxed) {
            "ready".into()
        } else {
            "idle".into()
        },
        count: state.index.count(),
        roots: state.index.roots.read().clone(),
        scanned: state.index.scanned.load(Ordering::Relaxed),
        current: state.index.current.read().clone(),
        last_built: *state.index.last_built.read(),
    }
}

#[tauri::command]
pub fn index_status(state: State<'_, Arc<AppState>>) -> IndexStatus {
    status_of(state.inner())
}

pub fn restart_watcher(app: &AppHandle, state: &Arc<AppState>) {
    let settings = state.settings.read().clone();
    if let Some(old) = state.watcher.lock().take() {
        drop(old);
    }
    let app_handle = app.clone();
    let st = state.clone();
    let handle = watch::start(
        state.index.clone(),
        settings.indexed_drives.clone(),
        settings.exclude_dirs.clone(),
        move || {
            let _ = app_handle.emit("index://overflow", ());
            let _ = st;
        },
    );
    *state.watcher.lock() = handle;
}

pub fn spawn_rebuild(app: AppHandle, state: Arc<AppState>) {
    if state.index.scanning.swap(true, Ordering::SeqCst) {
        return;
    }
    state.index.cancel.store(false, Ordering::Relaxed);
    state.index.scanned.store(0, Ordering::Relaxed);
    if let Some(w) = state.watcher.lock().as_ref() {
        w.paused.store(true, Ordering::Relaxed);
    }

    std::thread::Builder::new()
        .name("kestrel-scan".into())
        .spawn(move || {
            let settings = state.settings.read().clone();
            let started = std::time::Instant::now();
            let app_progress = app.clone();
            let last_emit = std::sync::atomic::AtomicU64::new(0);

            let opts = ScanOptions {
                roots: &settings.indexed_drives,
                excludes: &settings.exclude_dirs,
                include_hidden: settings.index_hidden_files,
            };

            let st = state.clone();
            let entries = idx::scan::scan(
                &opts,
                &state.index.cancel,
                &state.index.scanned,
                |n, cur| {
                    *st.index.current.write() = cur.to_string();
                    let now = crate::commands::now_ms() as u64;
                    let prev = last_emit.load(Ordering::Relaxed);
                    if now.saturating_sub(prev) > 220 {
                        last_emit.store(now, Ordering::Relaxed);
                        let _ = app_progress.emit(
                            "index://progress",
                            serde_json::json!({ "scanned": n, "current": cur }),
                        );
                    }
                },
            );

            let count = entries.len();
            // A cancelled walk only produced a *partial* entry set. Swapping it
            // in would silently shrink the index until the next rebuild, so the
            // previous index is kept as-is and the partial scan is dropped.
            if state.index.cancel.swap(false, Ordering::Relaxed) {
                crate::debug_log::qs_log(&format!(
                    "scan cancelled after {count} entries: keeping the previous index"
                ));
                drop(entries);
                state.index.scanning.store(false, Ordering::SeqCst);
                *state.index.current.write() = String::new();
                if let Some(w) = state.watcher.lock().as_ref() {
                    w.paused.store(false, Ordering::Relaxed);
                }
                let _ = app.emit(
                    "index://done",
                    serde_json::json!({
                        "count": state.index.count(),
                        "ms": started.elapsed().as_millis() as u64,
                        "cancelled": true,
                    }),
                );
                return;
            }
            state
                .index
                .replace(entries, settings.indexed_drives.clone());
            // A silent failure here is invisible until the next cold start,
            // which then re-scans the whole disk for no visible reason.
            if let Err(e) = idx::save(&state.index, &state.paths.index()) {
                crate::debug_log::qs_log(&format!(
                    "index: saving {count} entries failed: {e} (next start will re-scan)"
                ));
            }
            state.index.scanning.store(false, Ordering::SeqCst);
            *state.index.current.write() = String::new();

            if let Some(w) = state.watcher.lock().as_ref() {
                w.paused.store(false, Ordering::Relaxed);
            }
            let _ = app.emit(
                "index://done",
                serde_json::json!({
                    "count": count,
                    "ms": started.elapsed().as_millis() as u64,
                }),
            );
        })
        .ok();
}

#[tauri::command]
pub fn rebuild_index(app: AppHandle, state: State<'_, Arc<AppState>>) {
    spawn_rebuild(app, state.inner().clone());
}

#[tauri::command]
pub fn cancel_index(state: State<'_, Arc<AppState>>) {
    state.index.cancel.store(true, Ordering::Relaxed);
}

/// Load a previously persisted index on startup.
///
/// Returns the format version of the loaded index, or `None` when there is
/// none. An index written by an older build cannot be decoded (different entry
/// layout), so the caller rebuilds from scratch — but the stale file is *kept*
/// until that rebuild succeeds. Two reasons:
///
/// * `idx::save` writes a temp file and renames it over the old one, so a
///   successful rebuild replaces it atomically and never leaves a half-written
///   index behind. Deleting up front buys nothing there.
/// * Deleting up front means a cancelled or failed rebuild leaves the user with
///   no index at all, and every later start pays the full scan again. Keeping
///   the stale file makes a failed rebuild cost one wasted scan instead of a
///   permanent regression to cold starts.
///
/// The historical reason for deleting eagerly was disk space: the old
/// uncompressed format ran to hundreds of megabytes. The current format is
/// deflate-compressed (about 20 MB for a full disk), so holding it for the
/// length of one rebuild is cheap.
pub fn load_persisted(state: &Arc<AppState>) -> Option<u32> {
    // Check the version before decoding: a stale file is about to be replaced,
    // and decoding it first would keep the old index in memory alongside the
    // rebuild that replaces it.
    if let Some(v) = idx::peek_version(&state.paths.index()) {
        if v != idx::CURRENT_VERSION {
            crate::debug_log::qs_log(&format!(
                "index: on-disk format v{v} != v{}, rebuilding (old file kept until the new one lands)",
                idx::CURRENT_VERSION
            ));
            return None;
        }
    }
    let version = idx::load(&state.index, &state.paths.index());
    if let Some(v) = version {
        if v != idx::CURRENT_VERSION {
            return None;
        }
    }
    version
}

#[tauri::command]
pub fn clear_index(state: State<'_, Arc<AppState>>) {
    state.index.clear();
    let _ = std::fs::remove_file(state.paths.index());
}
