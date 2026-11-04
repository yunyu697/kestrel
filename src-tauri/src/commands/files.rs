//! File operations, icon/thumbnail extraction and preview metadata.

use crate::state::AppState;
use crate::win::{icon as winicon, shell};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub dir: String,
    pub ext: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub type_name: String,
    pub readonly: bool,
    pub hidden: bool,
    pub exists: bool,
    pub item_count: Option<usize>,
    pub thumb: Option<String>,
}

fn ts(t: std::io::Result<std::time::SystemTime>) -> i64 {
    t.ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub async fn file_info(path: String) -> Result<FileInfo, String> {
    tauri::async_runtime::spawn_blocking(move || build_info(&path))
        .await
        .map_err(|e| e.to_string())?
}

fn build_info(path: &str) -> Result<FileInfo, String> {
    let p = Path::new(path);
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let is_dir = meta.is_dir();
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let item_count = if is_dir {
        std::fs::read_dir(p).ok().map(|it| it.take(5000).count())
    } else {
        None
    };

    let thumb = if !is_dir && is_thumbable(&ext) {
        make_thumb(p, 360)
    } else {
        None
    };

    Ok(FileInfo {
        name,
        path: path.to_string(),
        dir: p
            .parent()
            .map(|d| d.to_string_lossy().to_string())
            .unwrap_or_default(),
        ext: ext.clone(),
        is_dir,
        size: if is_dir { 0 } else { meta.len() },
        modified: ts(meta.modified()),
        created: ts(meta.created()),
        type_name: winicon::type_name(path, is_dir),
        readonly: meta.permissions().readonly(),
        hidden: false,
        exists: true,
        item_count,
        thumb,
    })
}

fn is_thumbable(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tif" | "tiff"
    )
}

/// Thumbnail cache.
///
/// Decoding a photo takes hundreds of milliseconds and can allocate hundreds
/// of megabytes, and the preview pane asks for a thumbnail again every time
/// the selection moves. Caching the few most recent ones keeps scrolling
/// through image results smooth.
const THUMB_CACHE_MAX: usize = 24;
static THUMB_CACHE: parking_lot::Mutex<Vec<(String, String)>> = parking_lot::Mutex::new(Vec::new());

fn thumb_cache_get(key: &str) -> Option<String> {
    THUMB_CACHE
        .lock()
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

fn thumb_cache_put(key: String, value: String) {
    let mut cache = THUMB_CACHE.lock();
    if let Some(pos) = cache.iter().position(|(k, _)| *k == key) {
        cache.remove(pos);
    }
    cache.insert(0, (key, value));
    cache.truncate(THUMB_CACHE_MAX);
}

/// Maximum pixel dimension of a decoded image. A 24 MP photo would decode to
/// ~96 MB of raw pixels; refusing the giants keeps the preview responsive.
const THUMB_MAX_DECODE: u32 = 4096;

fn make_thumb(path: &Path, max: u32) -> Option<String> {
    // Key on path + mtime + size so an edited file gets a fresh thumbnail.
    let key = match std::fs::metadata(path) {
        Ok(meta) => {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("{}|{}|{}", path.to_string_lossy(), mtime, meta.len())
        }
        Err(_) => path.to_string_lossy().to_string(),
    };
    if let Some(hit) = thumb_cache_get(&key) {
        return Some(hit);
    }

    let img = decode_capped(path)?;
    let thumb = img.thumbnail(max, max);
    let mut out = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut out, image::ImageFormat::Png).ok()?;
    let url = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(out.into_inner())
    );
    thumb_cache_put(key, url.clone());
    Some(url)
}

/// Decode an image, refusing anything that would blow up memory.
fn decode_capped(path: &Path) -> Option<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(THUMB_MAX_DECODE);
    limits.max_image_height = Some(THUMB_MAX_DECODE);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().ok()
}

/// Batch icon lookup — one round trip for a whole result list.
#[tauri::command]
pub async fn icons_for(paths: Vec<String>, size: Option<u32>) -> Result<HashMap<String, String>, String> {
    let px = size.unwrap_or(48).clamp(16, 256);
    tauri::async_runtime::spawn_blocking(move || {
        let mut map = HashMap::new();
        for p in paths {
            let url = winicon::icon_data_url(&p, px);
            map.insert(p, url);
        }
        map
    })
    .await
    .map_err(|e| e.to_string())
}

/// Tauri requires a `Result` from an async command that borrows its inputs
/// (`State` is a reference); the failure case never happens, so the frontend
/// always sees a plain `true`/`false`.
#[tauri::command]
pub async fn open_path(state: State<'_, Arc<AppState>>, path: String) -> Result<bool, String> {
    let st = state.inner().clone();
    let ok = run_detached(path.clone(), shell::open_path).await;
    if ok {
        st.bump_usage(&path);
    }
    Ok(ok)
}

/// Run a shell hand-off away from the thread that drives the UI.
///
/// These calls block until the shell has taken over, and the shell is not in a
/// hurry: an executable comes up in ~70ms, but opening a *folder* takes 1.6s
/// (Explorer receives the request through DDE). The launcher hides itself
/// before any of this runs, so the wait is invisible — but it must not sit in
/// the command thread, where it would stall every later call.
async fn run_detached<F>(path: String, f: F) -> bool
where
    F: FnOnce(&str) -> bool + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(&path))
        .await
        .unwrap_or(false)
}

#[tauri::command]
pub async fn open_url(url: String) -> bool {
    run_detached(url, shell::open_url).await
}

#[tauri::command]
pub async fn reveal_path(path: String) -> bool {
    run_detached(path, shell::reveal).await
}

#[tauri::command]
pub async fn open_with(path: String) -> bool {
    run_detached(path, shell::open_with).await
}

#[tauri::command]
pub async fn run_as_admin(path: String) -> bool {
    run_detached(path, shell::run_as_admin).await
}

#[tauri::command]
pub async fn show_properties(path: String) -> bool {
    run_detached(path, shell::show_properties).await
}

#[tauri::command]
pub async fn open_terminal(path: String) -> bool {
    let dir = if Path::new(&path).is_dir() {
        path
    } else {
        Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    run_detached(dir, shell::open_terminal).await
}

#[tauri::command]
pub fn copy_file_refs(paths: Vec<String>) -> bool {
    shell::copy_files_to_clipboard(&paths)
}

#[tauri::command]
pub fn delete_to_trash(state: State<'_, Arc<AppState>>, paths: Vec<String>) -> Result<usize, String> {
    let mut n = 0;
    for p in &paths {
        if trash::delete(p).is_ok() {
            state.index.remove_path(p);
            n += 1;
        }
    }
    Ok(n)
}

#[tauri::command]
pub fn rename_path(state: State<'_, Arc<AppState>>, path: String, new_name: String) -> Result<String, String> {
    let src = Path::new(&path);
    let parent = src.parent().ok_or("无法获取父目录")?;
    let clean = new_name.trim();
    if clean.is_empty() || clean.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("名称包含非法字符".into());
    }
    let dst = parent.join(clean);
    if dst.exists() {
        return Err("目标已存在".into());
    }
    std::fs::rename(src, &dst).map_err(|e| e.to_string())?;
    let dst_str = dst.to_string_lossy().to_string();
    state.index.remove_path(&path);
    if let Some(entry) = crate::index::make_entry(&dst) {
        state.index.upsert(entry);
    }
    Ok(dst_str)
}

#[tauri::command]
pub fn create_folder(state: State<'_, Arc<AppState>>, parent: String, name: String) -> Result<String, String> {
    let p = Path::new(&parent).join(name.trim());
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    let s = p.to_string_lossy().to_string();
    if let Some(entry) = crate::index::make_entry(&p) {
        state.index.upsert(entry);
    }
    Ok(s)
}

#[tauri::command]
pub fn bump_usage(state: State<'_, Arc<AppState>>, path: String) {
    state.bump_usage(&path);
}

/// Open a folder in Explorer (used by the Quick Switch panel).
#[tauri::command]
pub async fn browse_folder(path: String) -> bool {
    run_detached(path, shell::open_path).await
}

#[tauri::command]
pub fn run_shell_command(command: String, cwd: Option<String>) -> bool {
    shell::run_shell(&command, cwd.as_deref())
}

#[tauri::command]
pub fn spawn_program(program: String, args: Option<String>, cwd: Option<String>) -> bool {
    shell::spawn(&program, args.as_deref(), cwd.as_deref())
}

// ---------------- checksum ----------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHashes {
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub size: u64,
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Stream the file once, computing MD5 / SHA-1 / SHA-256 in a single pass.
#[tauri::command]
pub async fn file_hashes(path: String) -> Result<FileHashes, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use md5::Digest as _;
        let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        let meta = f.metadata().map_err(|e| e.to_string())?;
        let mut md5 = md5::Md5::new();
        let mut sha1 = sha1::Sha1::new();
        let mut sha256 = sha2::Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            use std::io::Read;
            let n = f.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            md5.update(&buf[..n]);
            sha1.update(&buf[..n]);
            sha256.update(&buf[..n]);
        }
        Ok(FileHashes {
            md5: to_hex(&md5.finalize()),
            sha1: to_hex(&sha1.finalize()),
            sha256: to_hex(&sha256.finalize()),
            size: meta.len(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
