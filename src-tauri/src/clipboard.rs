//! Clipboard history monitor.

use crate::model::ClipboardItem;
use crate::state::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub static PAUSED: AtomicBool = AtomicBool::new(false);

fn png_data_url(img: &arboard::ImageData) -> Option<String> {
    let w = img.width as u32;
    let h = img.height as u32;
    if w == 0 || h == 0 || w * h > 16_000_000 {
        return None;
    }
    let buf = image::RgbaImage::from_raw(w, h, img.bytes.to_vec())?;
    let mut out = std::io::Cursor::new(Vec::new());
    buf.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(out.into_inner())))
}

pub fn start(state: Arc<AppState>, app: AppHandle) {
    std::thread::Builder::new()
        .name("kestrel-clipboard".into())
        .spawn(move || {
            let mut clipboard = loop {
                match arboard::Clipboard::new() {
                    Ok(c) => break c,
                    Err(_) => std::thread::sleep(Duration::from_secs(2)),
                }
            };
            let mut last_text = String::new();
            let mut last_image_len = 0usize;

            loop {
                std::thread::sleep(Duration::from_millis(650));
                if PAUSED.load(Ordering::Relaxed) {
                    continue;
                }
                // Read only the two flags we need. Cloning the whole settings
                // (favorites, engines, commands, notes) every 650 ms burned
                // CPU and produced a steady stream of garbage for nothing.
                let (enabled, images) = {
                    let s = state.settings.read();
                    (s.clipboard_enabled, s.clipboard_images)
                };
                if !enabled {
                    continue;
                }

                let mut changed = false;

                if let Ok(text) = clipboard.get_text() {
                    let t = text.trim().to_string();
                    if !t.is_empty() && t != last_text && t.len() < 200_000 {
                        last_text = t.clone();
                        changed = true;
                        push_text(&state, &app, t);
                    }
                }

                if images {
                    if let Ok(img) = clipboard.get_image() {
                        if img.bytes.len() != last_image_len && img.bytes.len() > 64 {
                            last_image_len = img.bytes.len();
                            if let Some(url) = png_data_url(&img) {
                                changed = true;
                                push_image(&state, &app, url);
                            }
                        }
                    }
                }

                if changed {
                    let _ = app.emit("clipboard://changed", ());
                }
            }
        })
        .ok();
}

fn push_text(state: &Arc<AppState>, app: &AppHandle, text: String) {
    let mut store = state.clipboard.write();
    store.retain(|i| !(i.kind == "text" && i.text == text));
    store.insert(
        0,
        ClipboardItem {
            id: format!("clip-{}", chrono::Local::now().timestamp_millis()),
            kind: "text".into(),
            text,
            image: None,
            at: chrono::Local::now().timestamp(),
            pinned: false,
            source: "clipboard".into(),
        },
    );
    trim(&mut store, state.settings.read().clipboard_limit as usize);
    drop(store);
    state.save_clipboard();
    let _ = app.emit("clipboard://item", ());
}

fn push_image(state: &Arc<AppState>, app: &AppHandle, url: String) {
    let mut store = state.clipboard.write();
    store.retain(|i| i.kind != "image");
    store.insert(
        0,
        ClipboardItem {
            id: format!("clip-{}", chrono::Local::now().timestamp_millis()),
            kind: "image".into(),
            text: "图片".into(),
            image: Some(url),
            at: chrono::Local::now().timestamp(),
            pinned: false,
            source: "clipboard".into(),
        },
    );
    trim(&mut store, state.settings.read().clipboard_limit as usize);
    drop(store);
    state.save_clipboard();
    let _ = app.emit("clipboard://item", ());
}

fn trim(store: &mut Vec<ClipboardItem>, limit: usize) {
    let limit = limit.max(20);
    if store.len() <= limit {
        return;
    }
    // Keep pinned entries, drop the oldest unpinned ones.
    let mut excess = store.len() - limit;
    let mut i = store.len();
    while i > 0 && excess > 0 {
        i -= 1;
        if !store[i].pinned {
            store.remove(i);
            excess -= 1;
        }
    }
}

/// Put an item back on the system clipboard.
pub fn restore(item: &ClipboardItem) -> bool {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    if item.kind == "text" {
        return clipboard.set_text(item.text.clone()).is_ok();
    }
    if let Some(url) = &item.image {
        let Some(b64) = url.split(',').nth(1) else {
            return false;
        };
        let Ok(bytes) = STANDARD.decode(b64) else {
            return false;
        };
        let Ok(img) = image::load_from_memory(&bytes) else {
            return false;
        };
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        return clipboard
            .set_image(arboard::ImageData {
                width: w as usize,
                height: h as usize,
                bytes: rgba.into_raw().into(),
            })
            .is_ok();
    }
    false
}
