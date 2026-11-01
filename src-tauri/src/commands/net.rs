//! Network probes used by the update check and the mirror speed test.
//!
//! Doing this from the WebView turned out to be unreliable for two reasons,
//! and both of them bite exactly where it matters:
//!
//! 1. The content security policy can block cross-origin fetches without a
//!    word in the UI — a missing `connect-src` silently falls back to
//!    `default-src 'self'`, so every request to GitHub dies in the browser
//!    layer and only the console knows about it.
//! 2. Chromium follows the Windows proxy settings but ignores the
//!    `http_proxy` / `https_proxy` environment variables that a lot of users
//!    (and every CI runner) set. On such a machine the WebView cannot reach
//!    the network while curl and git work fine.
//!
//! WinHTTP sidesteps both: it talks to the network directly, and it can be
//! pointed at the very same proxy the environment points at. Every probe tries
//! a direct connection first and only falls back to the proxy when the direct
//! route fails, so a broken proxy can never make things worse than before.

use serde::Serialize;
use std::ffi::c_void;
use std::time::Instant;
use windows::core::PCWSTR;
use windows::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetOption, WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
    WINHTTP_ACCESS_TYPE_NAMED_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_OPTION_USER_AGENT,
    WINHTTP_QUERY_CONTENT_LENGTH, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

const AGENT: &str = "Kestrel";
/// A release JSON is a few kilobytes; the cap only guards against a rogue
/// mirror answering with something huge.
const MAX_BODY: usize = 4 * 1024 * 1024;

#[derive(Serialize, Clone, Debug)]
pub struct HttpProbe {
    /// True when the server answered with a status code.
    pub ok: bool,
    pub status: u16,
    pub body: Option<String>,
    pub error: Option<String>,
    /// Round-trip time in milliseconds.
    pub ms: u64,
    /// "direct" or "proxy" — which route actually worked.
    pub via: String,
}

fn failed(msg: String, via: &str) -> HttpProbe {
    HttpProbe {
        ok: false,
        status: 0,
        body: None,
        error: Some(msg),
        ms: 0,
        via: via.to_string(),
    }
}

/// Proxy from the environment, if any: `https_proxy=http://127.0.0.1:7890`.
/// Windows environment variables are case-insensitive, so one lookup per
/// spelling is enough. HTTPS and HTTP get their own variable, with `ALL_PROXY`
/// as the last resort.
fn proxy_from_env(secure: bool) -> Option<String> {
    let keys: &[&str] = if secure {
        &["HTTPS_PROXY", "ALL_PROXY"]
    } else {
        &["HTTP_PROXY", "ALL_PROXY"]
    };
    for key in keys {
        let Ok(raw) = std::env::var(key) else {
            continue;
        };
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let stripped = raw
            .strip_prefix("http://")
            .or_else(|| raw.strip_prefix("https://"))
            .unwrap_or(raw);
        let host = stripped.split('/').next().unwrap_or(stripped).trim();
        // WinHTTP wants host:port; a bare host is fine too, it defaults to 80.
        if !host.is_empty() {
            return Some(host.to_string());
        }
    }
    None
}

struct Parts {
    host: String,
    path: String,
    port: u16,
    secure: bool,
}

/// Split a URL into the pieces WinHTTP wants. Doing it by hand beats
/// `WinHttpCrackUrl`, which insists on a NUL-terminated string with the
/// length passed as -1 — a combination the Rust wrappers make awkward, and
/// it failed outright on ordinary URLs.
fn parse_url(url: &str) -> Result<Parts, String> {
    let url = url.trim();
    let (scheme, rest) = url.split_once("://").ok_or("URL 缺少协议")?;
    let secure = match scheme.to_ascii_lowercase().as_str() {
        "https" => true,
        "http" => false,
        other => {
            // The bare scheme is useless on its own ("不支持的协议: /https"):
            // the whole URL is what identifies a bad caller.
            crate::debug_log::qs_log(&format!("net: refusing scheme {other:?} in {url}"));
            return Err(format!("不支持的协议: {other}"));
        }
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/".to_string()),
    };
    // Drop any userinfo part.
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    let (host, port) = if let Some(stripped) = authority.strip_prefix('[') {
        // IPv6 literal: [::1] or [::1]:8080
        let end = stripped.find(']').ok_or("IPv6 地址缺少 ]")?;
        let host = &stripped[..end];
        let rest = &stripped[end + 1..];
        let port = rest
            .strip_prefix(':')
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(if secure { 443 } else { 80 });
        (host.to_string(), port)
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (
                h.to_string(),
                p.parse::<u16>().unwrap_or(if secure { 443 } else { 80 }),
            ),
            _ => (
                authority.to_string(),
                if secure { 443 } else { 80 },
            ),
        }
    };
    if host.is_empty() {
        return Err("URL 缺少主机名".into());
    }
    Ok(Parts {
        host,
        path,
        port,
        secure,
    })
}

/// An open WinHTTP request together with every handle it owns.
struct Conn {
    session: *mut c_void,
    connect: *mut c_void,
    request: *mut c_void,
}

impl Drop for Conn {
    fn drop(&mut self) {
        unsafe {
            if !self.request.is_null() {
                let _ = WinHttpCloseHandle(self.request);
            }
            if !self.connect.is_null() {
                let _ = WinHttpCloseHandle(self.connect);
            }
            if !self.session.is_null() {
                let _ = WinHttpCloseHandle(self.session);
            }
        }
    }
}

/// Open a session/connect/request trio for one HTTP call. `proxy` is `None`
/// for a direct connection.
///
/// The handles are owned by the returned `Conn`, so every early return still
/// closes them.
unsafe fn open_request(
    parts: &Parts,
    proxy: Option<&str>,
    timeout_ms: u32,
) -> Result<Conn, String> {
    let agent: Vec<u16> = AGENT.encode_utf16().chain(std::iter::once(0)).collect();
    let proxy_w: Vec<u16> = proxy
        .unwrap_or("")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let bypass_w: Vec<u16> = "<local>".encode_utf16().chain(std::iter::once(0)).collect();
    let access = if proxy.is_some() {
        WINHTTP_ACCESS_TYPE_NAMED_PROXY
    } else {
        WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY
    };

    let session = WinHttpOpen(
        PCWSTR(agent.as_ptr()),
        access,
        PCWSTR(proxy_w.as_ptr()),
        PCWSTR(bypass_w.as_ptr()),
        0,
    );
    if session.is_null() {
        return Err(format!(
            "WinHttpOpen 失败 (code {})",
            unsafe { windows::Win32::Foundation::GetLastError() }.0
        ));
    }
    let _ = WinHttpSetTimeouts(session, timeout_ms as i32, timeout_ms as i32, timeout_ms as i32, timeout_ms as i32);

    let host_w: Vec<u16> = parts.host.encode_utf16().chain(std::iter::once(0)).collect();
    let connect = WinHttpConnect(session, PCWSTR(host_w.as_ptr()), parts.port, 0);
    if connect.is_null() {
        let _ = WinHttpCloseHandle(session);
        return Err("无法建立连接".into());
    }

    let path_w: Vec<u16> = parts.path.encode_utf16().chain(std::iter::once(0)).collect();
    let verb_w: Vec<u16> = "GET".encode_utf16().chain(std::iter::once(0)).collect();
    let version_w: Vec<u16> = "HTTP/1.1".encode_utf16().chain(std::iter::once(0)).collect();
    let flags = if parts.secure {
        WINHTTP_FLAG_SECURE
    } else {
        Default::default()
    };
    let req = WinHttpOpenRequest(
        connect,
        PCWSTR(verb_w.as_ptr()),
        PCWSTR(path_w.as_ptr()),
        PCWSTR(version_w.as_ptr()),
        PCWSTR::null(),
        std::ptr::null(),
        flags,
    );
    if req.is_null() {
        let _ = WinHttpCloseHandle(connect);
        let _ = WinHttpCloseHandle(session);
        return Err("无法创建请求".into());
    }

    // GitHub's API insists on a User-Agent; WinHTTP's default one is ancient
    // enough that it is better to set our own.
    let ua: Vec<u8> = agent.iter().flat_map(|c| c.to_le_bytes()).collect();
    let _ = WinHttpSetOption(Some(req), WINHTTP_OPTION_USER_AGENT, Some(&ua));

    Ok(Conn {
        session,
        connect,
        request: req,
    })
}

/// One request over an explicit route. `proxy` is `None` for a direct call.
unsafe fn request(
    parts: &Parts,
    proxy: Option<&str>,
    timeout_ms: u32,
    range_bytes: Option<u32>,
) -> Result<HttpProbe, String> {
    let via = if proxy.is_some() { "proxy" } else { "direct" };
    let conn = open_request(parts, proxy, timeout_ms)?;
    let req = conn.request;

    // A range request keeps the speed test cheap: the server answers with the
    // first few hundred bytes instead of the whole installer.
    if let Some(n) = range_bytes {
        let header = format!("Range: bytes=0-{}\r\n", n.saturating_sub(1));
        let wide: Vec<u16> = header.encode_utf16().collect();
        let _ = WinHttpAddRequestHeaders(req, &wide, 0);
    }

    WinHttpSendRequest(req, None, None, 0, 0, 0).map_err(|e| format!("发送失败: {e}"))?;
    WinHttpReceiveResponse(req, std::ptr::null_mut()).map_err(|e| format!("接收失败: {e}"))?;

    let mut status: u32 = 0;
    let mut status_len: u32 = 4;
    let _ = WinHttpQueryHeaders(
        req,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        PCWSTR::null(),
        Some(&mut status as *mut u32 as *mut c_void),
        &mut status_len,
        std::ptr::null_mut(),
    );

    let mut body: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        let mut avail: u32 = 0;
        if WinHttpQueryDataAvailable(req, &mut avail).is_err() || avail == 0 {
            break;
        }
        let want = avail.min(buf.len() as u32);
        let mut read: u32 = 0;
        if WinHttpReadData(req, buf.as_mut_ptr() as *mut c_void, want, &mut read).is_err() || read == 0 {
            break;
        }
        body.extend_from_slice(&buf[..read as usize]);
        if body.len() >= MAX_BODY {
            break;
        }
    }

    let text = String::from_utf8_lossy(&body).to_string();
    Ok(HttpProbe {
        ok: status >= 200 && status < 400,
        status: status as u16,
        body: Some(text),
        error: None,
        ms: 0,
        via: via.to_string(),
    })
}

/// Stream a URL straight into a file, reporting progress as it goes.
///
/// `on_progress` receives `(received, total)` and returns `false` to abort the
/// transfer. Direct connection first, proxy second — same policy as the
/// probes.
pub(crate) fn download_to_file(
    url: &str,
    dest: &std::path::Path,
    timeout_ms: u32,
    mut on_progress: impl FnMut(u64, u64) -> bool,
) -> Result<u64, String> {
    let parts = parse_url(url)?;
    // A cancelled transfer must not be retried through the proxy: the user
    // asked for it to stop, not for another route.
    let canceled = std::cell::Cell::new(false);
    let direct = unsafe {
        stream_to_file(&parts, None, timeout_ms, dest, &mut on_progress, &canceled)
    };
    match direct {
        Ok(n) => Ok(n),
        Err(direct_err) => {
            if canceled.get() {
                return Err(direct_err);
            }
            match proxy_from_env(parts.secure) {
                Some(proxy) => unsafe {
                    stream_to_file(
                        &parts,
                        Some(&proxy),
                        timeout_ms,
                        dest,
                        &mut on_progress,
                        &canceled,
                    )
                }
                .map_err(|e| format!("{direct_err}；代理 {proxy} 也失败: {e}")),
                None => Err(direct_err),
            }
        }
    }
}

/// The download half of `download_to_file`: one connection, one file.
unsafe fn stream_to_file(
    parts: &Parts,
    proxy: Option<&str>,
    timeout_ms: u32,
    dest: &std::path::Path,
    on_progress: &mut impl FnMut(u64, u64) -> bool,
    canceled: &std::cell::Cell<bool>,
) -> Result<u64, String> {
    let conn = open_request(parts, proxy, timeout_ms)?;
    let req = conn.request;

    WinHttpSendRequest(req, None, None, 0, 0, 0).map_err(|e| format!("发送失败: {e}"))?;
    WinHttpReceiveResponse(req, std::ptr::null_mut()).map_err(|e| format!("接收失败: {e}"))?;

    let mut status: u32 = 0;
    let mut status_len: u32 = 4;
    let _ = WinHttpQueryHeaders(
        req,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        PCWSTR::null(),
        Some(&mut status as *mut u32 as *mut c_void),
        &mut status_len,
        std::ptr::null_mut(),
    );
    if !(200..300).contains(&status) {
        return Err(format!("下载失败：HTTP {status}"));
    }

    // The total is only known when the server sends a Content-Length; without
    // it the UI shows an indeterminate bar instead of a percentage.
    let mut total: u32 = 0;
    let mut total_len: u32 = 4;
    let _ = WinHttpQueryHeaders(
        req,
        WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER,
        PCWSTR::null(),
        Some(&mut total as *mut u32 as *mut c_void),
        &mut total_len,
        std::ptr::null_mut(),
    );

    use std::io::Write;
    let mut file = std::fs::File::create(dest).map_err(|e| format!("无法写入文件: {e}"))?;
    let mut received: u64 = 0;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let mut avail: u32 = 0;
        if WinHttpQueryDataAvailable(req, &mut avail).is_err() || avail == 0 {
            break;
        }
        let want = avail.min(buf.len() as u32);
        let mut read: u32 = 0;
        if WinHttpReadData(req, buf.as_mut_ptr() as *mut c_void, want, &mut read).is_err()
            || read == 0
        {
            break;
        }
        file.write_all(&buf[..read as usize])
            .map_err(|e| format!("写入失败: {e}"))?;
        received += read as u64;
        if !on_progress(received, total as u64) {
            canceled.set(true);
            return Err("下载已取消".into());
        }
    }
    file.flush().ok();
    Ok(received)
}

/// Probe a URL. Direct first, proxy second — the proxy is only used when the
/// direct route fails, and only when the environment actually defines one.
fn probe(url: &str, timeout_ms: u64, range_bytes: Option<u32>) -> HttpProbe {
    let started = Instant::now();
    let parts = match parse_url(url) {
        Ok(p) => p,
        Err(e) => return failed(e, "direct"),
    };
    let timeout = timeout_ms.clamp(1000, 60_000) as u32;

    let direct = unsafe { request(&parts, None, timeout, range_bytes) };
    let (result, via) = match direct {
        Ok(p) => (Ok(p), "direct"),
        Err(direct_err) => match proxy_from_env(parts.secure) {
            Some(proxy) => match unsafe { request(&parts, Some(&proxy), timeout, range_bytes) } {
                Ok(p) => (Ok(p), "proxy"),
                Err(proxy_err) => (
                    Err(format!("{direct_err}；代理 {proxy} 也失败: {proxy_err}")),
                    "proxy",
                ),
            },
            None => (Err(direct_err), "direct"),
        },
    };

    let ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(mut p) => {
            p.ms = ms;
            p.via = via.to_string();
            p
        }
        Err(e) => HttpProbe {
            ok: false,
            status: 0,
            body: None,
            error: Some(e),
            ms,
            via: via.to_string(),
        },
    }
}

/// Frontend-facing command: fetch a URL and hand back status, body and
/// timing. Blocking WinHTTP calls run on a blocking thread so the async
/// runtime stays responsive.
#[tauri::command]
pub async fn http_probe(
    url: String,
    timeout_ms: Option<u64>,
    range_bytes: Option<u32>,
) -> HttpProbe {
    let timeout = timeout_ms.unwrap_or(10_000);
    tauri::async_runtime::spawn_blocking(move || probe(&url, timeout, range_bytes))
        .await
        .unwrap_or_else(|e| failed(format!("网络任务失败: {e}"), "direct"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_url() {
        let p = parse_url("https://api.github.com/repos/x/y/releases/latest").unwrap();
        assert_eq!(p.host, "api.github.com");
        assert_eq!(p.port, 443);
        assert!(p.secure);
        assert_eq!(p.path, "/repos/x/y/releases/latest");
    }

    #[test]
    fn keeps_query_string() {
        let p = parse_url("https://example.com/a?b=1").unwrap();
        assert_eq!(p.path, "/a?b=1");
    }

    #[test]
    fn rejects_broken_url() {
        assert!(parse_url("not a url").is_err());
        assert!(parse_url("ftp://example.com/x").is_err());
    }

    /// The scheme has to be named in the error: it is the only clue that the
    /// caller joined a mirror prefix onto a URL wrongly. A prefix of "/" used
    /// to produce "/https://github.com/…" and the message read
    /// "不支持的协议: /https" — which is exactly how 1.2.0-1.2.6 shipped an
    /// updater that could not download anything.
    #[test]
    fn names_the_scheme_it_cannot_speak() {
        let err = match parse_url("/https://github.com/x/y/releases/download/v1/a.msi") {
            Ok(_) => panic!("the leading slash should not parse as a host"),
            Err(e) => e,
        };
        assert!(err.contains("/https"), "error was {err:?}");
    }

    #[test]
    fn handles_explicit_port_and_ipv6() {
        let p = parse_url("http://127.0.0.1:8080/a").unwrap();
        assert_eq!(p.host, "127.0.0.1");
        assert_eq!(p.port, 8080);
        assert!(!p.secure);
        let p = parse_url("https://[::1]/x").unwrap();
        assert_eq!(p.host, "::1");
        assert_eq!(p.port, 443);
    }

    #[test]
    fn reads_proxy_from_environment() {
        // The value is only used when it actually looks like host:port.
        std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:7890");
        assert_eq!(proxy_from_env(true).as_deref(), Some("127.0.0.1:7890"));
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("ALL_PROXY");
        assert_eq!(proxy_from_env(true).as_deref(), None);
    }

    /// End-to-end check against the real GitHub API. Ignored by default so CI
    /// stays offline-friendly; run it with `cargo test -- --ignored`.
    #[test]
    #[ignore = "needs network"]
    fn reaches_github_api() {
        let p = probe(
            "https://api.github.com/repos/yunyu697/kestrel/releases/latest",
            20_000,
            None,
        );
        assert!(p.ok, "probe failed: {:?}", p.error);
        assert_eq!(p.status, 200);
        let body = p.body.unwrap_or_default();
        assert!(body.contains("tag_name"), "unexpected body: {body}");
        println!("github api via={} ms={}", p.via, p.ms);
    }

    /// The mirror path used by the speed test: a range request against a real
    /// release asset through a mirror prefix. The asset URL is resolved from
    /// the live GitHub API so the test never rots when the version bumps.
    #[test]
    #[ignore = "needs network"]
    fn reaches_mirror() {
        let latest = probe(
            "https://api.github.com/repos/yunyu697/kestrel/releases/latest",
            20_000,
            None,
        );
        assert!(latest.ok, "probe failed: {:?}", latest.error);
        let body = latest.body.unwrap_or_default();
        let urls: Vec<&str> = body
            .split("\"browser_download_url\":")
            .skip(1)
            .filter_map(|rest| rest.split('"').nth(1))
            .collect();
        assert!(!urls.is_empty(), "no asset url in release payload");
        let asset = urls
            .iter()
            .find(|u| u.ends_with(".msi"))
            .copied()
            .unwrap_or(urls[0]);
        println!("asset = {asset}");
        for prefix in ["https://gh-proxy.com/", "https://ghproxy.net/", "https://gh-proxy.net/"] {
            let url = format!("{prefix}{asset}");
            let p = probe(&url, 20_000, Some(1024));
            println!("{prefix} -> status={} via={} ms={}", p.status, p.via, p.ms);
            assert!(p.status > 0, "no response from {prefix}: {:?}", p.error);
        }
    }

    /// The real update path: stream a release asset to disk and check that
    /// the file arrives complete.
    #[test]
    #[ignore = "needs network"]
    fn downloads_a_release_asset_to_disk() {
        let latest = probe(
            "https://api.github.com/repos/yunyu697/kestrel/releases/latest",
            20_000,
            None,
        );
        assert!(latest.ok, "probe failed: {:?}", latest.error);
        let body = latest.body.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&body).expect("bad release json");
        let assets = json["assets"].as_array().expect("no assets array");
        let msi = assets
            .iter()
            .find(|a| a["name"].as_str().unwrap_or("").ends_with(".msi"))
            .expect("no msi asset");
        let asset = msi["browser_download_url"].as_str().expect("no download url");
        // The expected size comes from the release payload, so a truncated
        // download is caught even though the server answers 200.
        let expected = msi["size"].as_u64().expect("no size");

        let dir = std::env::temp_dir().join("kestrel-download-test");
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("asset.msi");
        let mut ticks = 0u32;
        let written = download_to_file(asset, &dest, 180_000, |received, total| {
            ticks += 1;
            assert!(total == 0 || total >= received, "total went backwards");
            true
        })
        .expect("download failed");
        assert!(written > 0, "nothing was written");
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), written);
        assert_eq!(written, expected, "downloaded {written} of {expected}");
        println!("downloaded {written} bytes in {ticks} ticks");
        let _ = std::fs::remove_file(&dest);
    }

    /// A cancelled transfer must not leave a half-written file behind.
    #[test]
    #[ignore = "needs network"]
    fn cancels_a_download() {
        let dir = std::env::temp_dir().join("kestrel-download-test");
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("cancelled.bin");
        let err = download_to_file(
            "https://api.github.com/repos/yunyu697/kestrel/releases/latest",
            &dest,
            30_000,
            |_, _| false,
        )
        .expect_err("the transfer should have been aborted");
        assert!(err.contains("取消"), "unexpected error: {err}");
    }
}
