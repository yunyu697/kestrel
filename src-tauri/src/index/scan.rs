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

pub fn scan<F>(
    opts: &ScanOptions,
    cancel: &AtomicBool,
    counter: &AtomicUsize,
    progress: F,
) -> Vec<Entry>
where
    F: Fn(usize, &str) + Sync,
{
    let excludes = build_excludes(opts.excludes);
    // Entries are built on the parallel walker threads and handed over here.
    // Arc because jwalk's callback requires 'static.
    let sink: Arc<Mutex<Vec<Entry>>> = Arc::new(Mutex::new(Vec::with_capacity(200_000)));
    let mut all: Vec<Entry> = Vec::with_capacity(200_000);

    'roots: for root in opts.roots {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if !std::path::Path::new(root).exists() {
            continue;
        }
        progress(counter.load(Ordering::Relaxed), root);

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
        let mut seen: usize = 0;
        for item in walker {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            if item.is_err() {
                continue;
            }
            seen += 1;
            if seen % 8192 == 0 {
                let n = counter.load(Ordering::Relaxed) + sink.lock().len();
                progress(n, root);
                if n >= 2_500_000 {
                    break 'roots;
                }
            }
        }

        let batch: Vec<Entry> = std::mem::take(&mut *sink.lock());
        counter.fetch_add(batch.len(), Ordering::Relaxed);
        all.extend(batch);
        progress(counter.load(Ordering::Relaxed), root);
    }

    progress(counter.load(Ordering::Relaxed), "");
    all
}
