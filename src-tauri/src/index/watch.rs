//! Filesystem watcher -> incremental index maintenance.

use super::{make_entry, Index};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

pub struct WatcherHandle {
    /// Held so the watcher stays alive for as long as the handle exists.
    /// Dropping it stops the filesystem notifications.
    #[allow(dead_code)]
    pub watcher: Mutex<notify::RecommendedWatcher>,
    pub paused: Arc<std::sync::atomic::AtomicBool>,
}

fn excluded(path: &str, excludes: &[String]) -> bool {
    let lower = path.to_lowercase();
    excludes.iter().any(|e| {
        let e = e.trim_end_matches('\\').to_lowercase();
        !e.is_empty() && (lower == e || lower.starts_with(&format!("{e}\\")))
    })
}

pub fn start(
    index: Arc<Index>,
    roots: Vec<String>,
    excludes: Vec<String>,
    on_overflow: impl Fn() + Send + 'static,
) -> Option<WatcherHandle> {
    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .ok()?;

    for root in &roots {
        let p = Path::new(root);
        if p.exists() {
            let _ = watcher.watch(p, RecursiveMode::Recursive);
        }
    }

    let paused = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let paused_thread = paused.clone();

    std::thread::Builder::new()
        .name("kestrel-watch".into())
        .spawn(move || {
            let mut pending: HashSet<PathBuf> = HashSet::new();
            loop {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(Ok(event)) => {
                        if paused_thread.load(std::sync::atomic::Ordering::Relaxed) {
                            continue;
                        }
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                                for p in event.paths {
                                    let s = p.to_string_lossy().to_string();
                                    if !excluded(&s, &excludes) {
                                        pending.insert(p);
                                    }
                                }
                                if pending.len() > 4000 {
                                    pending.clear();
                                    on_overflow();
                                }
                            }
                            EventKind::Other => {}
                            _ => {}
                        }
                    }
                    Ok(Err(_)) => {
                        // Buffer overflow or watch error: ask for a full rebuild.
                        on_overflow();
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if pending.is_empty() {
                            continue;
                        }
                        let batch: Vec<PathBuf> = pending.drain().collect();
                        for p in batch {
                            let s = p.to_string_lossy().to_string();
                            if p.exists() {
                                if let Some(e) = make_entry(&p) {
                                    index.upsert(e);
                                }
                            } else {
                                index.remove_path(&s);
                            }
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .ok()?;

    Some(WatcherHandle {
        watcher: Mutex::new(watcher),
        paused,
    })
}
