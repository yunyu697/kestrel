//! Parallel filesystem walker that feeds the index.

use super::{make_entry_with, Entry};
use jwalk::WalkDir;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct ScanOptions<'a> {
    pub roots: &'a [String],
    pub excludes: &'a [String],
    pub include_hidden: bool,
}

/// An exclude without a drive prefix (e.g. "node_modules") matches any path
/// component; one with a drive prefix (e.g. "C:\Windows") matches by prefix.
#[derive(Clone, Debug)]
enum Exclude {
    Prefix(String),
    Component(String),
}

fn build_excludes(excludes: &[String]) -> Vec<Exclude> {
    excludes
        .iter()
        .map(|e| e.trim().trim_end_matches('\\').to_lowercase())
        .filter(|e| !e.is_empty())
        .map(|e| {
            if e.contains(':') {
                Exclude::Prefix(e)
            } else {
                Exclude::Component(e)
            }
        })
        .collect()
}

fn is_excluded(path: &str, excludes: &[Exclude]) -> bool {
    if excludes.is_empty() {
        return false;
    }
    let lower = path.to_lowercase();
    excludes.iter().any(|ex| match ex {
        Exclude::Prefix(p) => lower == *p || lower.starts_with(&format!("{p}\\")),
        Exclude::Component(c) => lower.split('\\').any(|part| part == c.as_str()),
    })
}

/// How many walked items to skip between two progress reports.
///
/// Every report takes the sink lock and writes the shared counter, so it must
/// not happen per item. Tests pass a tiny stride to exercise the reporting path
/// with a handful of files instead of ten thousand.
const PROGRESS_STRIDE: usize = 8192;

pub fn scan<F>(
    opts: &ScanOptions,
    cancel: &AtomicBool,
    counter: &AtomicUsize,
    progress: F,
) -> Vec<Entry>
where
    F: Fn(usize, &str) + Sync,
{
    scan_with_stride(opts, cancel, counter, progress, PROGRESS_STRIDE)
}

fn scan_with_stride<F>(
    opts: &ScanOptions,
    cancel: &AtomicBool,
    counter: &AtomicUsize,
    progress: F,
    stride: usize,
) -> Vec<Entry>
where
    F: Fn(usize, &str) + Sync,
{
    let excludes = build_excludes(opts.excludes);
    // Entries are built on the parallel walker threads and handed over here.
    // Arc because jwalk's callback requires 'static.
    let sink: Arc<Mutex<Vec<Entry>>> = Arc::new(Mutex::new(Vec::with_capacity(200_000)));
    let mut all: Vec<Entry> = Vec::with_capacity(200_000);

    for root in opts.roots {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if !std::path::Path::new(root).exists() {
            continue;
        }
        let base = counter.load(Ordering::Relaxed);
        progress(base, root);

        let ex = excludes.clone();
        let sink_ref = Arc::clone(&sink);
        let walker = WalkDir::new(root)
            .follow_links(false)
            // Honour the "index hidden files" setting. It used to be ignored
            // entirely, which pulled every dot-folder (and every hidden system
            // directory) into the index — pure bloat.
            .skip_hidden(!opts.include_hidden)
            // Dedicated pool; rayon treats 0 threads as "use all cores".
            .parallelism(jwalk::Parallelism::RayonNewPool(0))
            // Build index entries on the parallel worker threads — metadata,
            // pinyin and string allocation are the expensive part and must
            // not run serially in the consumer loop.
            .process_read_dir(move |_depth, _path, _state, children| {
                let mut batch: Vec<Entry> = Vec::with_capacity(children.len());
                children.retain_mut(|c| match c {
                    Ok(entry) => {
                        if entry.file_type().is_symlink() {
                            return false;
                        }
                        let p = entry.path();
                        let s = p.to_string_lossy();
                        if is_excluded(&s, &ex) {
                            return false;
                        }
                        let meta = entry.metadata().ok();
                        if let Some(e) = make_entry_with(&p, meta) {
                            batch.push(e);
                        }
                        true
                    }
                    Err(_) => false,
                });
                if !batch.is_empty() {
                    sink_ref.lock().extend(batch);
                }
            });

        // Drain the iterator: entries were already built in parallel. Report
        // progress occasionally without locking the sink on every item.
        //
        // `counter` is also what `index_status` hands to the UI as `scanned`,
        // so it must be *stored* while walking. It used to be accumulated only
        // once per finished root, which made the settings page show "0 项" and
        // a dead progress bar for the whole first drive.
        let mut seen: usize = 0;
        let mut capped = false;
        for item in walker {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            if item.is_err() {
                continue;
            }
            seen += 1;
            if seen % stride == 0 {
                let n = base + sink.lock().len();
                counter.store(n, Ordering::Relaxed);
                progress(n, root);
                if n >= 2_500_000 {
                    capped = true;
                    break;
                }
            }
        }

        let batch: Vec<Entry> = std::mem::take(&mut *sink.lock());
        let total = base + batch.len();
        counter.store(total, Ordering::Relaxed);
        all.extend(batch);
        progress(total, root);
        // Hitting the cap ends the whole walk, but only after the entries
        // already built have been handed over — breaking straight out of the
        // outer loop used to drop the last sink batch on the floor.
        if capped {
            break;
        }
    }

    progress(counter.load(Ordering::Relaxed), "");
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn temp_dir(tag: &str, files: usize) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kestrel-scan-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        for i in 0..files {
            std::fs::File::create(dir.join(format!("f{i}.txt"))).expect("temp file");
        }
        dir
    }

    /// Regression: the shared `scanned` counter used to be accumulated once per
    /// finished root, so `index_status` reported 0 for the whole first drive and
    /// the settings page showed "0 项" with a frozen progress bar. The counter
    /// must already hold the reported value when the progress callback fires.
    #[test]
    fn publishes_the_live_count_while_walking() {
        let dir = temp_dir("live", 60);
        let root = dir.to_string_lossy().to_string();
        let counter = AtomicUsize::new(0);
        let cancel = AtomicBool::new(false);
        let stale = AtomicUsize::new(0);
        let reported = AtomicUsize::new(0);
        let opts = ScanOptions {
            roots: std::slice::from_ref(&root),
            excludes: &[],
            include_hidden: false,
        };

        // A small stride keeps the test to a few dozen files while hitting the
        // exact same reporting branch production uses.
        let entries = scan_with_stride(&opts, &cancel, &counter, |n, _root| {
            if n > 0 {
                reported.fetch_add(1, Ordering::Relaxed);
            }
            // The UI reads exactly this atomic, so it has to be current.
            if counter.load(Ordering::Relaxed) != n {
                stale.fetch_add(1, Ordering::Relaxed);
            }
        }, 8);

        let _ = std::fs::remove_dir_all(&dir);
        assert!(reported.load(Ordering::Relaxed) > 0, "no progress reported");
        assert_eq!(
            stale.load(Ordering::Relaxed),
            0,
            "counter was stale when progress fired"
        );
        // 60 files, plus the directory itself when jwalk yields it.
        assert!(
            entries.len() == 60 || entries.len() == 61,
            "unexpected entry count: {}",
            entries.len()
        );
        assert_eq!(counter.load(Ordering::Relaxed), entries.len());
    }

    /// A cancel must not leave a half-drained sink behind.
    #[test]
    fn cancelled_walk_still_returns_what_it_found() {
        let dir = temp_dir("cancel", 16);
        let root = dir.to_string_lossy().to_string();
        let counter = AtomicUsize::new(0);
        let cancel = AtomicBool::new(true); // already cancelled
        let opts = ScanOptions {
            roots: std::slice::from_ref(&root),
            excludes: &[],
            include_hidden: false,
        };
        let entries = scan(&opts, &cancel, &counter, |_, _| {});
        let _ = std::fs::remove_dir_all(&dir);
        assert!(entries.is_empty());
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
