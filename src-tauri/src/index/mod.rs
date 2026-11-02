//! File index: in-memory store, binary persistence, incremental maintenance.

pub mod scan;
pub mod watch;

use parking_lot::RwLock;
use pinyin::ToPinyin;
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// One indexed file or folder.
///
/// The layout is memory-driven: at ~700k entries every byte per entry is worth
/// 700 KB, and the first version of this struct cost 569 MB once the heap
/// allocations were counted. Three things dominated it:
///
/// * `dir` was a full copy of `path`'s prefix — 76 MB of bytes for 61k distinct
///   directories, i.e. every sibling repeating its parent's name. It is now
///   sliced out of `path` on demand.
/// * `path` was stored twice, once here and once as the `by_path` key. The key
///   now shares this same `Arc<str>` allocation, so a path costs its bytes once.
/// * `lower` was a copy of `name` for the 48% of files whose name is already
///   lowercase. Those now store `None`.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Full path. Shared with the `by_path` key — never store it twice.
    pub path: Arc<str>,
    pub name: Box<str>,
    /// Lowercased name, kept for fast matching without re-allocating. `None`
    /// when `name` is already lowercase, which is the common case.
    pub lower: Option<Box<str>>,
    pub ext: Box<str>,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
    /// Pinyin initials ("wj" for 文件).
    pub py: Box<str>,
    /// Full pinyin without separators.
    pub pyf: Box<str>,
    /// Bitmap of ASCII letters (bit 0 = 'a') present in name/pinyin — lets the
    /// searcher reject entries with a single AND before any string work.
    pub mask: u32,
}

impl Entry {
    /// Lowercased name to match against.
    #[inline]
    pub fn lower(&self) -> &str {
        match &self.lower {
            Some(l) => l,
            None => &self.name,
        }
    }

    /// Parent directory, sliced out of `path` instead of stored separately.
    #[inline]
    pub fn dir(&self) -> &str {
        dir_of(&self.path)
    }
}

/// `C:\a\b\c.txt` -> `C:\a\b`. Empty when the path has no separator.
#[inline]
pub fn dir_of(path: &str) -> &str {
    match path.rfind('\\') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// The on-disk shape: a flat 10-element array per entry.
///
/// Written by hand rather than derived so the persisted form stays independent
/// of the in-memory types — `Arc<str>`/`Box<str>` are not serialisable on their
/// own, and deriving would force a full copy of the index into owned `String`s
/// before it could be written (or after it was read).
const ENTRY_FIELDS: usize = 10;

impl Serialize for Entry {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(ENTRY_FIELDS)?;
        t.serialize_element(&*self.path)?;
        t.serialize_element(&*self.name)?;
        t.serialize_element(&self.lower.as_deref())?;
        t.serialize_element(&*self.ext)?;
        t.serialize_element(&self.is_dir)?;
        t.serialize_element(&self.size)?;
        t.serialize_element(&self.mtime)?;
        t.serialize_element(&*self.py)?;
        t.serialize_element(&*self.pyf)?;
        t.serialize_element(&self.mask)?;
        t.end()
    }
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = Entry;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an index entry")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Entry, A::Error> {
                fn field<'de, A: SeqAccess<'de>, T: Deserialize<'de>>(
                    seq: &mut A,
                    at: usize,
                ) -> Result<T, A::Error> {
                    seq.next_element()?
                        .ok_or_else(|| de::Error::invalid_length(at, &"an index entry"))
                }

                // Each field is decoded straight into its final owned form, so
                // loading never materialises a second copy of the index.
                let path: String = field(&mut seq, 0)?;
                let name: String = field(&mut seq, 1)?;
                let lower: Option<String> = field(&mut seq, 2)?;
                let ext: String = field(&mut seq, 3)?;
                let is_dir: bool = field(&mut seq, 4)?;
                let size: u64 = field(&mut seq, 5)?;
                let mtime: i64 = field(&mut seq, 6)?;
                let py: String = field(&mut seq, 7)?;
                let pyf: String = field(&mut seq, 8)?;
                let mask: u32 = field(&mut seq, 9)?;
                Ok(Entry {
                    path: Arc::from(path),
                    name: name.into_boxed_str(),
                    lower: lower.map(String::into_boxed_str),
                    ext: ext.into_boxed_str(),
                    is_dir,
                    size,
                    mtime,
                    py: py.into_boxed_str(),
                    pyf: pyf.into_boxed_str(),
                    mask,
                })
            }
        }

        d.deserialize_tuple(ENTRY_FIELDS, EntryVisitor)
    }
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
    /// Path -> entry index. The keys are clones of `Entry::path`, i.e. the same
    /// allocation with a bumped refcount — a separate `String` key here used to
    /// duplicate all 92 MB of path bytes.
    pub by_path: HashMap<Arc<str>, usize>,
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
        if let Some(&idx) = store.by_path.get(&*entry.path) {
            store.entries[idx] = entry;
        } else {
            let path = entry.path.clone();
            store.entries.push(entry);
            let idx = store.entries.len() - 1;
            store.by_path.insert(path, idx);
        }
    }

    pub fn remove_path(&self, path: &str) {
        let prefix = format!("{}\\", path.trim_end_matches('\\'));
        let mut store = self.store.write();
        let before = store.entries.len();
        store
            .entries
            .retain(|e| &*e.path != path && !e.path.starts_with(&prefix));
        if store.entries.len() != before {
            // Field-level split so the rebuild can read `entries` while
            // inserting into `by_path`.
            let Store { entries, by_path } = &mut *store;
            by_path.clear();
            by_path.reserve(entries.len());
            for (i, e) in entries.iter().enumerate() {
                by_path.insert(e.path.clone(), i);
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
const INDEX_MAGIC: &[u8; 4] = b"KST6";
const INDEX_VERSION: u32 = 6;

/// Version of the index format this build writes. An index persisted with a
/// different version is stale (different exclusions, different entry layout)
/// and must be rebuilt.
pub const CURRENT_VERSION: u32 = INDEX_VERSION;

/// Borrowed view of what gets written, so saving never clones the index.
#[derive(Serialize)]
struct Persist<'a> {
    version: u32,
    roots: &'a [String],
    built_at: i64,
    entries: &'a [Entry],
}

#[derive(Deserialize)]
struct PersistOwned {
    version: u32,
    roots: Vec<String>,
    built_at: i64,
    entries: Vec<Entry>,
}

pub fn save(index: &Index, file: &Path) -> std::io::Result<()> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;

    // Serialise straight into the compressor while holding the read lock.
    // Both intermediate buffers this used to allocate are gone: the old code
    // cloned every entry (a 430 MB spike) and then built the whole uncompressed
    // payload (220 MB) before compressing it.
    let compressed = {
        let store = index.store.read();
        let roots = index.roots.read();
        let payload = Persist {
            version: INDEX_VERSION,
            roots: &roots,
            built_at: *index.last_built.read(),
            entries: &store.entries,
        };
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::new(6));
        rmp_serde::encode::write(&mut enc, &payload).map_err(std::io::Error::other)?;
        enc.finish()?
    };

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

/// Read just the format version out of an index file.
///
/// `load` builds the whole entry vector before the caller can find out the
/// version is stale, so an update from an older format used to hold both the
/// old and the new index in memory at the same time. The version is the first
/// field of the payload, so a handful of decompressed bytes is enough.
pub fn peek_version(file: &Path) -> Option<u32> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let bytes = std::fs::read(file).ok()?;
    if bytes.len() <= 12 || &bytes[..4] != INDEX_MAGIC {
        return None; // Legacy uncompressed file: let `load` deal with it.
    }
    let mut head = [0u8; 16];
    let mut dec = DeflateDecoder::new(&bytes[12..]);
    let n = dec.read(&mut head).ok()?;
    let head = &head[..n];

    // fixarray(4) followed by the version as a MessagePack uint.
    let mut i = 0;
    if *head.get(i)? != 0x94 {
        return None;
    }
    i += 1;
    let (tag, rest) = (head.get(i)?, &head[i + 1..]);
    let (len, rest) = match *tag {
        0xcc => (1, rest),
        0xcd => (2, rest),
        0xce => (4, rest),
        0xcf => (8, rest),
        t if t <= 0x7f => return Some(t as u32),
        _ => return None,
    };
    let mut v: u64 = 0;
    for b in rest.get(..len)? {
        v = (v << 8) | *b as u64;
    }
    u32::try_from(v).ok()
}

/// Load a previously persisted index.
///
/// Returns the format version of the loaded index so the caller can tell a
/// fresh index from one that has to be rebuilt.
pub fn load(index: &Index, file: &Path) -> Option<u32> {
    use flate2::read::DeflateDecoder;

    let bytes = std::fs::read(file).ok()?;
    let payload = if bytes.len() > 12 && &bytes[..4] == INDEX_MAGIC {
        // Decode while decompressing, so the 220 MB uncompressed payload never
        // exists as a buffer of its own.
        let mut dec = DeflateDecoder::new(&bytes[12..]);
        rmp_serde::from_read(&mut dec).ok()?
    } else {
        // Legacy index written before compression was introduced.
        rmp_serde::from_slice::<PersistOwned>(&bytes).ok()?
    };
    if payload.version > INDEX_VERSION {
        return None;
    }
    let version = payload.version;
    let roots = payload.roots;
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
    // Only keep a lowercased copy when it actually differs: for the ~48% of
    // names that are already lowercase this would be a duplicate string.
    let lower = name.to_lowercase();
    let lower = if lower == name {
        None
    } else {
        Some(lower.into_boxed_str())
    };
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
    Some(Entry {
        mask: letter_mask(&[lower.as_deref().unwrap_or(&name), &pyf]),
        name: name.into_boxed_str(),
        lower,
        path: Arc::from(path_str),
        ext: ext.into_boxed_str(),
        is_dir,
        size: if is_dir { 0 } else { meta.len() },
        mtime,
        py: py.into_boxed_str(),
        pyf: pyf.into_boxed_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kestrel-index-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn entry(path: &str) -> Entry {
        make_entry_with(Path::new(path), None).expect("entry")
    }

    /// `dir` is no longer stored; it has to be the same slice the old field
    /// held, or every result subtitle in the UI changes.
    #[test]
    fn slices_the_parent_directory_out_of_the_path() {
        assert_eq!(dir_of(r"C:\Users\x\a\b.txt"), r"C:\Users\x\a");
        assert_eq!(dir_of(r"C:\a.txt"), r"C:");
        assert_eq!(dir_of("a.txt"), "");
        assert_eq!(dir_of(r"\\server\share\f.txt"), r"\\server\share");
    }

    /// The lowercase copy is only kept when it differs from the name, but the
    /// accessor must always hand back something matchable.
    #[test]
    fn keeps_a_lowercase_copy_only_when_it_differs() {
        let dir = temp("lower");
        let plain = dir.join("readme.txt");
        let shouty = dir.join("README.TXT");
        std::fs::write(&plain, b"x").expect("file");
        std::fs::write(&shouty, b"x").expect("file");

        let plain = entry(&plain.to_string_lossy());
        let shouty = entry(&shouty.to_string_lossy());

        assert!(plain.lower.is_none(), "already lowercase, no copy needed");
        assert_eq!(plain.lower(), "readme.txt");
        assert_eq!(shouty.lower.as_deref(), Some("readme.txt"));
        assert_eq!(shouty.lower(), "readme.txt");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Saving used to clone the whole index; loading used to materialise the
    /// uncompressed payload first. Both now stream, so the only thing that can
    /// go wrong is the hand-written wire format — round-trip it in full.
    #[test]
    fn round_trips_through_the_index_file() {
        let dir = temp("roundtrip");
        std::fs::create_dir_all(dir.join("sub")).expect("sub dir");
        std::fs::write(dir.join("Mixed Case.TXT"), b"hello").expect("file");
        std::fs::write(dir.join("plain.md"), b"hi").expect("file");
        std::fs::write(dir.join("中文文件.txt"), b"hi").expect("file");

        let mut entries = vec![
            entry(&dir.join("Mixed Case.TXT").to_string_lossy()),
            entry(&dir.join("plain.md").to_string_lossy()),
            entry(&dir.join("中文文件.txt").to_string_lossy()),
            entry(&dir.join("sub").to_string_lossy()),
        ];
        entries[0].size = 12345;
        entries[0].mtime = 1_700_000_000;

        let index = Index::new();
        index.replace(entries.clone(), vec![dir.to_string_lossy().to_string()]);
        let file = dir.join("index.bin");
        save(&index, &file).expect("save");

        let loaded = Index::new();
        // The version has to be readable without decoding the entries, or a
        // stale index would be built in memory just to be thrown away.
        assert_eq!(peek_version(&file), Some(INDEX_VERSION));
        assert_eq!(load(&loaded, &file), Some(INDEX_VERSION));
        let store = loaded.store.read();
        assert_eq!(store.entries.len(), entries.len());
        assert_eq!(store.by_path.len(), entries.len());

        for (i, (a, b)) in entries.iter().zip(store.entries.iter()).enumerate() {
            assert_eq!(a.path, b.path);
            assert_eq!(a.name, b.name);
            assert_eq!(a.lower, b.lower);
            assert_eq!(a.ext, b.ext);
            assert_eq!(a.is_dir, b.is_dir);
            assert_eq!(a.size, b.size);
            assert_eq!(a.mtime, b.mtime);
            assert_eq!(a.py, b.py);
            assert_eq!(a.pyf, b.pyf);
            assert_eq!(a.mask, b.mask);
            assert_eq!(a.dir(), b.dir());
            // The map key must be the same allocation as the entry's path.
            assert_eq!(store.by_path.get(&*b.path), Some(&i));
            let key = store
                .by_path
                .iter()
                .find(|(_, &v)| v == i)
                .map(|(k, _)| k)
                .expect("key");
            assert!(Arc::ptr_eq(key, &b.path), "path was copied, not shared");
        }
        assert_eq!(store.entries[0].size, 12345);
        assert_eq!(store.entries[0].mtime, 1_700_000_000);
        // Non-ASCII names must survive with their pinyin intact.
        let cjk = store.entries.iter().find(|e| e.name.ends_with("中文文件.txt")).expect("cjk");
        assert!(!cjk.pyf.is_empty(), "pinyin was lost");
        drop(store);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Upsert must replace in place, and the shared path key has to keep
    /// pointing at the new entry.
    #[test]
    fn upsert_replaces_without_growing_the_store() {
        let dir = temp("upsert");
        let p = dir.join("a.txt");
        std::fs::write(&p, b"1").expect("file");

        let index = Index::new();
        index.replace(vec![entry(&p.to_string_lossy())], vec![]);
        let mut changed = entry(&p.to_string_lossy());
        changed.size = 99;
        index.upsert(changed);

        let store = index.store.read();
        assert_eq!(store.entries.len(), 1);
        assert_eq!(store.entries[0].size, 99);
        assert_eq!(store.by_path.len(), 1);
        drop(store);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
