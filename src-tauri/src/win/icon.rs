//! Shell icon extraction -> PNG bytes (base64 encoded for the frontend).

use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::OnceLock;

struct Cache {
    map: HashMap<String, String>,
    order: Vec<String>,
}

const CACHE_CAP: usize = 1200;

fn cache() -> &'static Mutex<Cache> {
    static C: OnceLock<Mutex<Cache>> = OnceLock::new();
    C.get_or_init(|| {
        Mutex::new(Cache {
            map: HashMap::new(),
            order: Vec::new(),
        })
    })
}

/// Returns a data URL (`data:image/png;base64,...`) for the given path.
pub fn icon_data_url(path: &str, size: u32) -> String {
    let key = format!("{size}|{path}");
    if let Some(hit) = cache().lock().map.get(&key) {
        return hit.clone();
    }
    let value = match render_icon(path, size) {
        Some(bytes) => format!("data:image/png;base64,{}", STANDARD.encode(bytes)),
        None => String::new(),
    };
    let mut c = cache().lock();
    if c.map.len() >= CACHE_CAP {
        if let Some(oldest) = c.order.first().cloned() {
            c.map.remove(&oldest);
            c.order.remove(0);
        }
    }
    c.map.insert(key.clone(), value.clone());
    c.order.push(key);
    value
}

pub fn icon_for_extension(ext: &str, size: u32) -> String {
    let probe = format!("kestrel_placeholder.{ext}");
    icon_data_url(&probe, size)
}

fn render_icon(path: &str, size: u32) -> Option<Vec<u8>> {
    let wide_path = wide(path);
    let attrs = if std::path::Path::new(path).exists() {
        0
    } else {
        FILE_ATTRIBUTE_NORMAL
    };
    let flags = SHGFI_ICON | SHGFI_LARGEICON | if attrs != 0 { SHGFI_USEFILEATTRIBUTES } else { 0 };

    let mut info = SHFILEINFOW {
        hIcon: 0,
        iIcon: 0,
        dwAttributes: 0,
        szDisplayName: [0; MAX_PATH],
        szTypeName: [0; 80],
    };

    let ok = unsafe {
        SHGetFileInfoW(
            wide_path.as_ptr(),
            attrs,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };
    if ok == 0 || info.hIcon == 0 {
        return None;
    }

    let png = unsafe { hicon_to_png(info.hIcon, size as i32) };
    unsafe {
        DestroyIcon(info.hIcon);
    }
    png
}

unsafe fn hicon_to_png(hicon: HICON, size: i32) -> Option<Vec<u8>> {
    let hdc_screen = GetDC(0);
    if hdc_screen == 0 {
        return None;
    }
    let hdc = CreateCompatibleDC(hdc_screen);
    ReleaseDC(0, hdc_screen);
    if hdc == 0 {
        return None;
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [0; 3],
    };

    let mut bits: *mut c_void = std::ptr::null_mut();
    let hbmp = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, 0, 0);
    if hbmp == 0 || bits.is_null() {
        DeleteDC(hdc);
        return None;
    }

    let old = SelectObject(hdc, hbmp);
    DrawIconEx(hdc, 0, 0, hicon, size, size, 0, 0, DI_NORMAL);

    let count = (size * size) as usize;
    let src = std::slice::from_raw_parts(bits as *const u8, count * 4);
    let mut rgba = vec![0u8; count * 4];
    let mut any_alpha = false;
    for i in 0..count {
        let b = src[i * 4];
        let g = src[i * 4 + 1];
        let r = src[i * 4 + 2];
        let a = src[i * 4 + 3];
        if a > 8 {
            any_alpha = true;
        }
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = a;
    }

    SelectObject(hdc, old);
    DeleteObject(hbmp);
    DeleteDC(hdc);

    if !any_alpha {
        // Legacy icons carry no alpha; treat near-black as transparent mask edge.
        for i in 0..count {
            if rgba[i * 4] == 0 && rgba[i * 4 + 1] == 0 && rgba[i * 4 + 2] == 0 {
                rgba[i * 4 + 3] = 0;
            }
        }
    }

    let img = image::RgbaImage::from_raw(size as u32, size as u32, rgba)?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

/// Windows "type description" for a path, e.g. "文本文档" / "文件夹".
pub fn type_name(path: &str, is_dir: bool) -> String {
    let wide_path = wide(path);
    let attrs = if is_dir { FILE_ATTRIBUTE_DIRECTORY } else { FILE_ATTRIBUTE_NORMAL };
    let mut info = SHFILEINFOW {
        hIcon: 0,
        iIcon: 0,
        dwAttributes: 0,
        szDisplayName: [0; MAX_PATH],
        szTypeName: [0; 80],
    };
    let ok = unsafe {
        SHGetFileInfoW(
            wide_path.as_ptr(),
            attrs,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
        )
    };
    if ok == 0 {
        return String::new();
    }
    from_wide(&info.szTypeName)
}
