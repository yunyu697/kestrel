//! File index: in-memory store, binary persistence, incremental maintenance.

pub mod scan;
pub mod watch;

use parking_lot::RwLock;
use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub name: String,
    /// Lowercased name, kept for fast matching without re-allocating.
    pub lower: String,
    pub path: String,
    pub dir: String,
    pub ext: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
    /// Pinyin initials ("wj" for 文件).
    pub py: String,
    /// Full pinyin without separators.
    pub pyf: String,
    /// Bitmap of ASCII letters (bit 0 = 'a') present in name/pinyin — lets the
    /// searcher reject entries with a single AND before any string work.
    pub mask: u32,
}

fn letter_mask(parts: &[&str]) -> u32 {
    let mut m = 0u32;
    for part in parts {
        for &b in part.as_bytes() {
            if b.is_ascii_lowercase() {
                m |= 1 << (b - b'a');
            }
        }
    }
    m
}

#[derive(Default)]
pub struct Store {
    pub entries: Vec<Entry>,
    pub by_path: HashMap<String, usize>,
}

#[derive(Serialize, Deserialize)]
struct Persist {
    version: u32,
    roots: Vec<String>,
    built_at: i64,
    entries: Vec<Entry>,
}

pub struct Index {
    pub store: RwLock<Store>,
    pub roots: RwLock<Vec<String>>,
    pub last_built: RwLock<i64>,
    pub scanning: AtomicBool,
    pub cancel: AtomicBool,
    pub scanned: AtomicUsize,
    pub current: RwLock<String>,
    pub ready: AtomicBool,
}

impl Index {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: RwLock::new(Store::default()),
            roots: RwLock::new(Vec::new()),
            last_built: RwLock::new(0),
            scanning: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            scanned: AtomicUsize::new(0),
            current: RwLock::new(String::new()),
            ready: AtomicBool::new(false),
        })
    }

    pub fn count(&self) -> usize {
        self.store.read().entries.len()
    }

    pub fn replace(&self, entries: Vec<Entry>, roots: Vec<String>) {
        let mut map = HashMap::with_capacity(entries.len());
        for (i, e) in entries.iter().enumerate() {
            map.insert(e.path.clone(), i);
        }
        let mut store = self.store.write();
        store.entries = entries;
        store.by_path = map;
        drop(store);
        *self.roots.write() = roots;
        *self.last_built.write() = chrono::Local::now().timestamp();
        self.ready.store(true, Ordering::Relaxed);
    }

    pub fn upsert(&self, entry: Entry) {
        let mut store = self.store.write();
        if let Some(&idx) = store.by_path.get(&entry.path) {
            store.entries[idx] = entry;
        } else {
            store.entries.push(entry);
            let idx = store.entries.len() - 1;
            let path = store.entries[idx].path.clone();
            store.by_path.insert(path, idx);
        }
    }

    pub fn remove_path(&self, path: &str) {
        let prefix = format!("{}\\", path.trim_end_matches('\\'));
        let mut store = self.store.write();
        let before = store.entries.len();
        store
            .entries
            .retain(|e| e.path != path && !e.path.starts_with(&prefix));
    if store.entries.len() != before {
        let pairs: Vec<(String, usize)> = store
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.path.clone(), i))
            .collect();
        store.by_path.clear();
        for (p, i) in pairs {
            store.by_path.insert(p, i);
        }
    }
    }

    pub fn clear(&self) {
        let mut store = self.store.write();
        store.entries.clear();
        store.by_path.clear();
        self.ready.store(false, Ordering::Relaxed);
    }
}

/// The index file is compressed: the raw MessagePack payload was over 500 MB
/// for a typical machine, mostly because every entry repeated the same path
/// fragments. Deflate brings that down by roughly 5×.
const INDEX_MAGIC: &[u8; 4] = b"KST5";
const INDEX_VERSION: u32 = 5;

/// Version of the index format this build writes. An index persisted with a
/// different version is stale (different exclusions, different entry layout)
/// and must be rebuilt.
pub const CURRENT_VERSION: u32 = INDEX_VERSION;

pub fn save(index: &Index, file: &Path) -> std::io::Result<()> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let (entries, roots, built) = {
        let store = index.store.read();
        (
            store.entries.clone(),
            index.roots.read().clone(),
            *index.last_built.read(),
        )
    };
    let payload = Persist {
        version: INDEX_VERSION,
        roots,
        built_at: built,
        entries,
    };
    let raw = rmp_serde::to_vec(&payload).map_err(std::io::Error::other)?;

    let mut enc = DeflateEncoder::new(Vec::with_capacity(raw.len() / 3), Compression::new(6));
    enc.write_all(&raw)?;
    let compressed = enc.finish()?;

    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = Vec::with_capacity(compressed.len() + 12);
    out.extend_from_slice(INDEX_MAGIC);
    out.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
    out.extend_from_slice(&compressed);

    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, &out)?;
    std::fs::rename(tmp, file)?;
    Ok(())
}

/// Load a previously persisted index.
///
/// Returns the format version of the loaded index so the caller can tell a
/// fresh index from one that has to be rebuilt.
pub fn load(index: &Index, file: &Path) -> Option<u32> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let Ok(bytes) = std::fs::read(file) else {
        return None;
    };
    let payload = if bytes.len() > 12 && &bytes[..4] == INDEX_MAGIC {
        let mut dec = DeflateDecoder::new(&bytes[12..]);
        let mut raw = Vec::new();
        if dec.read_to_end(&mut raw).is_err() {
            return None;
        }
        match rmp_serde::from_slice::<Persist>(&raw) {
            Ok(p) => p,
            Err(_) => return None,
        }
    } else {
        // Legacy index written before compression was introduced.
        match rmp_serde::from_slice::<Persist>(&bytes) {
            Ok(p) => p,
            Err(_) => return None,
        }
    };
    if payload.version > INDEX_VERSION {
        return None;
    }
    let version = payload.version;
    let roots = payload.roots.clone();
    let built = payload.built_at;
    index.replace(payload.entries, roots);
    *index.last_built.write() = built;
    Some(version)
}

/// Build a pinyin index for a file name (initials + full syllables).
pub fn pinyin_fields(name: &str) -> (String, String) {
    let mut initials = String::with_capacity(name.len());
    let mut full = String::with_capacity(name.len() * 2);
    for ch in name.chars() {
        if ch.is_ascii() {
            let l = ch.to_ascii_lowercase();
            initials.push(l);
            full.push(l);
        } else if let Some(p) = ch.to_pinyin() {
            let plain = p.plain();
            if let Some(c) = plain.chars().next() {
                initials.push(c);
            }
            full.push_str(&plain);
        }
    }
    (initials, full)
}

pub fn make_entry(path: &Path) -> Option<Entry> {
    make_entry_with(path, None)
}

/// Build an index entry. `meta` comes from the walker when available to avoid
/// an extra filesystem round-trip per file.
pub fn make_entry_with(path: &Path, meta: Option<std::fs::Metadata>) -> Option<Entry> {
    let meta = match meta {
        Some(m) => m,
        None => std::fs::symlink_metadata(path).ok()?,
    };
    let is_dir = meta.is_dir();
    let name = path.file_name()?.to_string_lossy().to_string();
    let lower = name.to_lowercase();
    let ext = if is_dir {
        String::new()
    } else {
        path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    // Fast path: pure-ASCII names have no pinyin, so the pinyin fields would
    // be byte-identical to `lower`. Storing them anyway tripled the size of
    // the index for the vast majority of files; leaving them empty is
    // equivalent, because a pinyin match on an ASCII name is the very same
    // match as the lowercase one (and the searcher only consults the pinyin
    // fields when the lowercase match failed).
    let (py, pyf) = if name.is_ascii() {
        (String::new(), String::new())
    } else {
        pinyin_fields(&name)
    };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let path_str = path.to_string_lossy().to_string();
    // Derive the parent directory from the path string instead of a second
    // to_string_lossy on Path::parent().
    let dir = path_str
        .rsplit_once('\\')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();
    Some(Entry {
        mask: letter_mask(&[&lower, &pyf]),
        name,
        lower,
        path: path_str,
        dir,
        ext,
        is_dir,
        size: if is_dir { 0 } else { meta.len() },
        mtime,
        py,
        pyf,
    })
}
