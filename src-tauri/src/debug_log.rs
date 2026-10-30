//! Lightweight tracing for the Quick Switch lifecycle.
//!
//! Appends to `%LOCALAPPDATA%\Kestrel\qs.log`. The file is capped (truncated
//! when it exceeds ~256 KB) so it can never grow unbounded. This is the
//! primary diagnostic source when Quick Switch misbehaves.
//!
//! The handle is opened **once** and kept open: the logger used to open and
//! close the file for every single line, which turned a hot code path (the
//! per-tick follower loop and the file-dialog probe) into a stream of file
//! create/close syscalls and showed up as UI stutter.
//!
//! `qs_log_verbose` writes only when `KESTREL_DEBUG=1` is set — it is meant
//! for the really noisy probes and must never cost anything by default.

use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::OnceLock;

// parking_lot: a poisoned lock must never silence the diagnostics.
use parking_lot::Mutex;

static STATE: AtomicU8 = AtomicU8::new(0); // 0 unknown, 1 ok, 2 disabled
static WRITES: AtomicU32 = AtomicU32::new(0);
static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Cap the log at ~256 KB, checked every 128 writes.
const CAP_BYTES: u64 = 256 * 1024;
const CAP_CHECK_EVERY: u32 = 128;

fn log_path() -> Option<std::path::PathBuf> {
    let dir = std::env::var("LOCALAPPDATA").ok()?;
    Some(std::path::PathBuf::from(dir).join("Kestrel").join("qs.log"))
}

fn handle() -> &'static Mutex<Option<std::fs::File>> {
    static HANDLE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    HANDLE.get_or_init(|| Mutex::new(None))
}

/// Open the log file once and remember the result.
fn init() -> bool {
    if STATE.load(Ordering::Relaxed) != 0 {
        return STATE.load(Ordering::Relaxed) == 1;
    }
    let ok = match log_path() {
        Some(path) => {
            if let Some(dir) = path.parent() {
                if std::fs::create_dir_all(dir).is_err() {
                    false
                } else {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .is_ok()
                }
            } else {
                false
            }
        }
        None => false,
    };
    STATE.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
    if ok {
        let path = log_path().unwrap();
        if let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            *handle().lock() = Some(f);
        }
    }
    ok
}

/// Append one line to the QS debug log (silently skipped if impossible).
pub fn qs_log(msg: &str) {
    if !init() {
        return;
    }
    let mut guard = handle().lock();
    let Some(f) = guard.as_mut() else { return };

    // Keep the file bounded without stat-ing on every single line.
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    if n % CAP_CHECK_EVERY == 0 {
        if let Ok(meta) = f.metadata() {
            if meta.len() > CAP_BYTES {
                let _ = f.set_len(0);
                let _ = f.seek(SeekFrom::Start(0));
            }
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let _ = writeln!(f, "{} {}", ts, msg);
}

/// Noisy per-probe logging — only when `KESTREL_DEBUG=1`.
pub fn qs_log_verbose(msg: &str) {
    if !VERBOSE.load(Ordering::Relaxed) {
        return;
    }
    qs_log(msg);
}

/// Turn the verbose channel on/off.
#[allow(dead_code)]
pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
}

pub fn init_verbose_from_env() {
    let on = std::env::var("KESTREL_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    VERBOSE.store(on, Ordering::Relaxed);
}
