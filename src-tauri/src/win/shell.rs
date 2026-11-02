//! Shell integration: launching, revealing, properties, clipboard file drops.

use super::*;
use std::ffi::c_void;
use std::ptr;

#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(hwnd: HWND) -> BOOL;
    fn EmptyClipboard() -> BOOL;
    fn SetClipboardData(format: u32, mem: HANDLE) -> HANDLE;
    fn CloseClipboard() -> BOOL;
    fn IsClipboardFormatAvailable(format: u32) -> BOOL;
}

const CF_HDROP: u32 = 15;
const GMEM_MOVEABLE: u32 = 0x0002;

#[repr(C)]
struct DROPFILES {
    p_files: u32,
    pt: POINT,
    f_nc: i32,
    f_wide: i32,
}

/// Open a file/folder/url with the default handler.
pub fn open_path(path: &str) -> bool {
    // A Store app has no file behind it: it is addressed as a shell namespace
    // item (`shell:AppsFolder\<AUMID>`). `ShellExecuteW` on that string only
    // resolves on a thread with a shell-initialised COM apartment, and the
    // Tauri worker that runs commands has none — the call "succeeds" and
    // nothing appears. Explorer resolves it for us, which is what the Start
    // Menu itself does.
    if path.starts_with("shell:") {
        return open_shell_target(path);
    }
    let verb = wide("open");
    let file = wide(path);
    let h = unsafe { ShellExecuteW(0, verb.as_ptr(), file.as_ptr(), ptr::null(), ptr::null(), SW_SHOWNORMAL) };
    h as isize > 32
}

/// Hand a shell namespace path to the shell (used for Store apps).
///
/// The namespace only resolves inside a shell-initialised apartment, and the
/// thread that runs a Tauri command has none (WebView2 leaves it MTA, which
/// the shell refuses with RPC_E_CHANGED_MODE). A throwaway STA thread is the
/// cheapest way to guarantee the right apartment — it costs a few hundred
/// microseconds and only runs when the user opens a Store app.
pub fn open_shell_target(target: &str) -> bool {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

    let target = target.to_string();
    std::thread::Builder::new()
        .name("kestrel-shell-open".into())
        .spawn(move || {
            let ready = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
            if !ready {
                return false;
            }
            let verb = wide("open");
            let file = wide(&target);
            let h = unsafe {
                ShellExecuteW(
                    0,
                    verb.as_ptr(),
                    file.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    SW_SHOWNORMAL,
                )
            };
            unsafe { CoUninitialize() };
            h as isize > 32
        })
        .map(|t| t.join().unwrap_or(false))
        .unwrap_or(false)
}

/// Show the "Open with…" picker.
pub fn open_with(path: &str) -> bool {
    let verb = wide("openas");
    let file = wide(path);
    let h = unsafe { ShellExecuteW(0, verb.as_ptr(), file.as_ptr(), ptr::null(), ptr::null(), SW_SHOWNORMAL) };
    h as isize > 32
}

/// Launch elevated.
pub fn run_as_admin(path: &str) -> bool {
    let verb = wide("runas");
    let file = wide(path);
    let h = unsafe { ShellExecuteW(0, verb.as_ptr(), file.as_ptr(), ptr::null(), ptr::null(), SW_SHOWNORMAL) };
    h as isize > 32
}

/// Restart this executable, optionally asking Windows for elevation.
///
/// `extra_args` is appended to the command line (`--no-single-instance` is
/// passed so the new process does not lose the single-instance race against
/// the one that is still shutting down).
///
/// Returns false when the user dismisses the UAC prompt (ShellExecuteW then
/// reports ERROR_CANCELLED / 1223), so the caller can keep running as-is.
pub fn relaunch_self(extra_args: &str, elevate: bool) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let verb = wide(if elevate { "runas" } else { "open" });
    let file = wide(&exe.to_string_lossy());
    let params = wide(extra_args);
    let h = unsafe {
        ShellExecuteW(
            0,
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    h as isize > 32
}

/// Restart unelevated from an elevated process.
///
/// A process started by an elevated one inherits elevation, so it has to be
/// created with somebody else's token: the desktop shell (`explorer.exe`)
/// always runs with the user's normal token. `CreateProcessWithTokenW` with a
/// duplicate of that token is the only reliable route — asking explorer to
/// launch the exe for us (`ShellExecuteW("explorer.exe", path)`) is silently
/// dropped when the caller is elevated, because the DDE hand-off to a lower
/// integrity process is blocked.
pub fn relaunch_self_unelevated(extra_args: &str) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    spawn_unelevated(&exe, extra_args)
}

#[repr(C)]
struct STARTUPINFOW {
    cb: u32,
    lp_reserved: *mut u16,
    lp_desktop: *mut u16,
    lp_title: *mut u16,
    dw_x: u32,
    dw_y: u32,
    dw_x_size: u32,
    dw_y_size: u32,
    dw_x_count_chars: u32,
    dw_y_count_chars: u32,
    dw_fill_attribute: u32,
    dw_flags: u32,
    w_show_window: u16,
    cb_reserved2: u16,
    lp_reserved2: *mut u8,
    h_std_input: HANDLE,
    h_std_output: HANDLE,
    h_std_error: HANDLE,
}

#[repr(C)]
struct PROCESS_INFORMATION {
    h_process: HANDLE,
    h_thread: HANDLE,
    dw_process_id: u32,
    dw_thread_id: u32,
}

#[link(name = "advapi32")]
extern "system" {
    fn DuplicateTokenEx(
        existing: HANDLE,
        access: u32,
        attrs: *mut c_void,
        level: u32,
        ty: u32,
        out: *mut HANDLE,
    ) -> BOOL;
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateProcessWithTokenW(
        token: HANDLE,
        logon_flags: u32,
        app: *const u16,
        cmd: *mut u16,
        flags: u32,
        env: *mut c_void,
        cwd: *const u16,
        si: *mut STARTUPINFOW,
        pi: *mut PROCESS_INFORMATION,
    ) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    fn GetShellWindow() -> HWND;
}

/// Start `exe` with the desktop shell's token, i.e. without elevation.
pub fn spawn_unelevated(exe: &std::path::Path, extra_args: &str) -> bool {
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const TOKEN_ASSIGN_PRIMARY: u32 = 0x0001;
    const TOKEN_DUPLICATE: u32 = 0x0002;
    const TOKEN_QUERY: u32 = 0x0008;
    const MAXIMUM_ALLOWED: u32 = 0x0200_0000;
    const SECURITY_IMPERSONATION: u32 = 2;
    const TOKEN_PRIMARY: u32 = 1;
    const LOGON_WITH_PROFILE: u32 = 0x0000_0001;
    const CREATE_UNICODE_ENVIRONMENT: u32 = 0x0000_0400;

    unsafe {
        let shell = GetShellWindow();
        if shell == 0 {
            return false;
        }
        let mut pid = 0u32;
        super::GetWindowThreadProcessId(shell, &mut pid);
        if pid == 0 {
            return false;
        }
        let proc = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if proc == 0 {
            return false;
        }
        let mut token: HANDLE = 0;
        let ok = super::OpenProcessToken(
            proc,
            TOKEN_ASSIGN_PRIMARY | TOKEN_DUPLICATE | TOKEN_QUERY,
            &mut token,
        );
        CloseHandle(proc);
        if ok == 0 {
            return false;
        }
        let mut primary: HANDLE = 0;
        let ok = DuplicateTokenEx(
            token,
            MAXIMUM_ALLOWED,
            ptr::null_mut(),
            SECURITY_IMPERSONATION,
            TOKEN_PRIMARY,
            &mut primary,
        );
        CloseHandle(token);
        if ok == 0 {
            return false;
        }
        let app = wide(&exe.to_string_lossy());
        let mut cmd = wide(&format!("\"{}\" {}", exe.to_string_lossy(), extra_args));
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let ok = CreateProcessWithTokenW(
            primary,
            LOGON_WITH_PROFILE,
            app.as_ptr(),
            cmd.as_mut_ptr(),
            CREATE_UNICODE_ENVIRONMENT,
            ptr::null_mut(),
            ptr::null(),
            &mut si,
            &mut pi,
        );
        CloseHandle(primary);
        if ok == 0 {
            return false;
        }
        CloseHandle(pi.h_process);
        CloseHandle(pi.h_thread);
        true
    }
}

/// Open an arbitrary URL / shell target.
pub fn open_url(url: &str) -> bool {
    open_path(url)
}

/// Reveal a path inside Explorer, selecting it when it is a file.
pub fn reveal(path: &str) -> bool {
    let wide_path = wide(path);
    unsafe {
        let pidl = ILCreateFromPathW(wide_path.as_ptr());
        if !pidl.is_null() {
            let hr = SHOpenFolderAndSelectItems(pidl as *const c_void, 0, ptr::null(), 0);
            ILFree(pidl);
            if hr == 0 {
                return true;
            }
        }
    }
    let is_dir = std::path::Path::new(path).is_dir();
    if is_dir {
        open_path(path)
    } else {
        let params = wide(&format!("/select,\"{path}\""));
        let exe = wide("explorer.exe");
        unsafe { ShellExecuteW(0, ptr::null(), exe.as_ptr(), params.as_ptr(), ptr::null(), SW_SHOWNORMAL) as isize > 32 }
    }
}

/// Native properties sheet.
pub fn show_properties(path: &str) -> bool {
    let verb = wide("properties");
    let file = wide(path);
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_INVOKEIDLIST;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    unsafe { ShellExecuteExW(&mut info) != 0 }
}

/// Open a terminal rooted at `dir`.
pub fn open_terminal(dir: &str) -> bool {
    let params = wide(&format!("-d \"{dir}\""));
    let wt = wide("wt.exe");
    let ok = unsafe { ShellExecuteW(0, ptr::null(), wt.as_ptr(), params.as_ptr(), ptr::null(), SW_SHOWNORMAL) as isize > 32 };
    if ok {
        return true;
    }
    let cmd = wide("cmd.exe");
    let k = wide("/K");
    unsafe { ShellExecuteW(0, ptr::null(), cmd.as_ptr(), k.as_ptr(), wide(dir).as_ptr(), SW_SHOWNORMAL) as isize > 32 }
}

/// Run a command line through cmd.exe without showing a console flash.
pub fn run_shell(command: &str, cwd: Option<&str>) -> bool {
    let cmd = wide("cmd.exe");
    let args = wide(&format!("/C {command}"));
    let dir = cwd.map(wide);
    let dir_ptr = dir.as_ref().map(|d| d.as_ptr()).unwrap_or(ptr::null());
    unsafe {
        let h = ShellExecuteW(0, ptr::null(), cmd.as_ptr(), args.as_ptr(), dir_ptr, SW_HIDE);
        h as isize > 32
    }
}

/// Start a program with arguments.
pub fn spawn(program: &str, args: Option<&str>, cwd: Option<&str>) -> bool {
    let file = wide(program);
    let a = args.map(wide);
    let d = cwd.map(wide);
    unsafe {
        let h = ShellExecuteW(
            0,
            ptr::null(),
            file.as_ptr(),
            a.as_ref().map(|v| v.as_ptr()).unwrap_or(ptr::null()),
            d.as_ref().map(|v| v.as_ptr()).unwrap_or(ptr::null()),
            SW_SHOWNORMAL,
        );
        h as isize > 32
    }
}

/// Put real file references (CF_HDROP) on the clipboard so they can be pasted in Explorer.
pub fn copy_files_to_clipboard(paths: &[String]) -> bool {
    if paths.is_empty() {
        return false;
    }
    let mut payload: Vec<u16> = Vec::new();
    for p in paths {
        payload.extend(p.encode_utf16());
        payload.push(0);
    }
    payload.push(0); // double-null terminator

    let header = std::mem::size_of::<DROPFILES>();
    let bytes = header + payload.len() * 2;

    unsafe {
        if OpenClipboard(0) == 0 {
            return false;
        }
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if mem.is_null() {
            CloseClipboard();
            return false;
        }
        let dst = GlobalLock(mem) as *mut u8;
        if dst.is_null() {
            CloseClipboard();
            return false;
        }
        let drop = DROPFILES {
            p_files: header as u32,
            pt: POINT { x: 0, y: 0 },
            f_nc: 0,
            f_wide: 1,
        };
        ptr::copy_nonoverlapping(&drop as *const DROPFILES as *const u8, dst, header);
        ptr::copy_nonoverlapping(payload.as_ptr() as *const u8, dst.add(header), payload.len() * 2);
        GlobalUnlock(mem);
        let ok = SetClipboardData(CF_HDROP, mem as isize) != 0;
        CloseClipboard();
        ok
    }
}

pub fn clipboard_has_files() -> bool {
    unsafe { IsClipboardFormatAvailable(CF_HDROP) != 0 }
}
