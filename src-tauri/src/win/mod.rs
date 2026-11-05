//! Minimal hand-written Win32 bindings.
//!
//! We deliberately avoid the `windows` crate: its wrapper API changes between
//! releases, while these flat C-style declarations are stable and predictable.

#![allow(non_snake_case, non_camel_case_types, dead_code, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HICON = isize;
pub type HBITMAP = isize;
pub type HDC = isize;
pub type HINSTANCE = isize;
pub type HMODULE = isize;
pub type HHOOK = isize;
pub type HANDLE = isize;
pub type HBRUSH = isize;
pub type HGDIOBJ = isize;
pub type HKEY = isize;
pub type BOOL = i32;
pub type DWORD = u32;
pub type UINT = u32;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type ATOM = u16;

pub const MAX_PATH: usize = 260;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
    pub lPrivate: u32,
}

#[repr(C)]
pub struct SHFILEINFOW {
    pub hIcon: HICON,
    pub iIcon: i32,
    pub dwAttributes: DWORD,
    pub szDisplayName: [u16; MAX_PATH],
    pub szTypeName: [u16; 80],
}

/// `TVITEMW` from commctrl.h — the payload of `TVM_GETITEMW`.
///
/// A tree view keeps its item text in its own process, so reading it means
/// handing the control a struct that lives in *its* address space; the field
/// offsets here have to match the C layout exactly.
#[repr(C)]
pub struct TVITEMW {
    pub mask: u32,
    pub h_item: isize,
    pub state: u32,
    pub state_mask: u32,
    pub psz_text: *mut u16,
    pub cch_text_max: i32,
    pub i_image: i32,
    pub i_selected_image: i32,
    pub c_children: i32,
    pub l_param: isize,
}

#[repr(C)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}

#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [u32; 3],
}

#[repr(C)]
pub struct KBDLLHOOKSTRUCT {
    pub vkCode: u32,
    pub scanCode: u32,
    pub flags: u32,
    pub time: u32,
    pub dwExtraInfo: usize,
}

#[repr(C)]
pub struct SHELLEXECUTEINFOW {
    pub cbSize: u32,
    pub fMask: u32,
    pub hwnd: HWND,
    pub lpVerb: *const u16,
    pub lpFile: *const u16,
    pub lpParameters: *const u16,
    pub lpDirectory: *const u16,
    pub nShow: i32,
    pub hInstApp: HINSTANCE,
    pub lpIDList: *mut c_void,
    pub lpClass: *const u16,
    pub hkeyClass: HKEY,
    pub dwHotKey: u32,
    pub hIcon: HICON,
    pub hProcess: HANDLE,
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: UINT,
    pub style: UINT,
    pub lpfnWndProc: Option<extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: isize,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}

// ---------- constants ----------
/// `GetLastError` after `CreateMutexW`/`CreateEventW` when the object already
/// exists — the only reliable "somebody else is already running" signal.
pub const ERROR_ALREADY_EXISTS: u32 = 183;
/// First message id applications may use for themselves.
pub const WM_APP: u32 = 0x8000;
pub const WS_POPUP: isize = 0x8000_0000u32 as isize;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_SHOW: i32 = 5;
pub const SW_RESTORE: i32 = 9;
pub const SW_HIDE: i32 = 0;

pub const GWL_EXSTYLE: i32 = -20;
pub const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
pub const WS_EX_NOACTIVATE: isize = 0x0800_0000;

pub const SHGFI_ICON: u32 = 0x0000_0100;
pub const SHGFI_LARGEICON: u32 = 0x0000_0000;
pub const SHGFI_SMALLICON: u32 = 0x0000_0001;
/// `pszName` is a PIDL, not a path (shell namespace items).
pub const SHGFI_PIDL: u32 = 0x0000_0008;
pub const SHGFI_USEFILEATTRIBUTES: u32 = 0x0000_0010;
pub const SHGFI_TYPENAME: u32 = 0x0000_0400;
pub const SHGFI_DISPLAYNAME: u32 = 0x0000_0200;

pub const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;

pub const DI_NORMAL: u32 = 0x0003;
pub const DIB_RGB_COLORS: u32 = 0;

pub const WH_KEYBOARD_LL: i32 = 13;
pub const HC_ACTION: i32 = 0;
pub const WM_KEYDOWN: usize = 0x0100;
pub const WM_KEYUP: usize = 0x0101;
pub const WM_SYSKEYDOWN: usize = 0x0104;
pub const WM_SYSKEYUP: usize = 0x0105;

pub const WM_SETTEXT: u32 = 0x000C;
pub const WM_GETTEXT: u32 = 0x000D;
pub const WM_GETTEXTLENGTH: u32 = 0x000E;

pub const VK_CONTROL: u32 = 0x11;
pub const VK_LCONTROL: u32 = 0xA2;
pub const VK_RCONTROL: u32 = 0xA3;
pub const VK_SHIFT: u32 = 0x10;
pub const VK_MENU: u32 = 0x12;

pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
pub const PROCESS_VM_OPERATION: u32 = 0x0008;
pub const PROCESS_VM_READ: u32 = 0x0010;
pub const PROCESS_VM_WRITE: u32 = 0x0020;

pub const MEM_COMMIT: u32 = 0x1000;
pub const MEM_RESERVE: u32 = 0x2000;
pub const MEM_RELEASE: u32 = 0x8000;
pub const PAGE_READWRITE: u32 = 0x04;

pub const SMTO_BLOCK: u32 = 0x0001;
pub const SMTO_ABORTIFHUNG: u32 = 0x0002;

pub const SEE_MASK_INVOKEIDLIST: u32 = 0x0000_000C;
pub const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;

pub const IMAGE_ICON: u32 = 1;
pub const LR_LOADFROMFILE: u32 = 0x0000_0010;
pub const LR_DEFAULTSIZE: u32 = 0x0000_0040;

#[link(name = "user32")]
extern "system" {
    pub fn GetForegroundWindow() -> HWND;
    pub fn SetForegroundWindow(hwnd: HWND) -> BOOL;
    pub fn BringWindowToTop(hwnd: HWND) -> BOOL;
    pub fn EnumWindows(cb: extern "system" fn(HWND, LPARAM) -> BOOL, lparam: LPARAM) -> BOOL;
    pub fn GetWindowTextW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn GetClassNameW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn GetWindowThreadProcessId(hwnd: HWND, pid: *mut u32) -> u32;
    pub fn IsWindowVisible(hwnd: HWND) -> BOOL;
    pub fn IsWindowEnabled(hwnd: HWND) -> BOOL;
    pub fn IsIconic(hwnd: HWND) -> BOOL;
    pub fn IsWindow(hwnd: HWND) -> BOOL;
    pub fn ShowWindow(hwnd: HWND, cmd: i32) -> BOOL;
    pub fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: BOOL) -> BOOL;
    pub fn GetCurrentThreadId() -> u32;
    pub fn SendMessageW(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    pub fn SendMessageTimeoutW(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> LRESULT;
    pub fn PostMessageW(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL;
    pub fn FindWindowW(class: *const u16, title: *const u16) -> HWND;
    pub fn RegisterClassExW(class: *const WNDCLASSEXW) -> ATOM;
    pub fn CreateWindowExW(
        exstyle: DWORD,
        class: *const u16,
        name: *const u16,
        style: DWORD,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: isize,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn DestroyWindow(hwnd: HWND) -> BOOL;
    pub fn DefWindowProcW(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    pub fn FindWindowExW(parent: HWND, after: HWND, class: *const u16, title: *const u16) -> HWND;
    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn GetParent(hwnd: HWND) -> HWND;
    pub fn GetWindow(hwnd: HWND, cmd: u32) -> HWND;
    pub fn GetTopWindow(hwnd: HWND) -> HWND;
    pub fn GetWindowLongW(hwnd: HWND, index: i32) -> isize;
    pub fn SetWindowLongW(hwnd: HWND, index: i32, value: isize) -> isize;
    pub fn SetWindowsHookExW(id_hook: i32, proc: HOOKPROC, hmod: HMODULE, thread: u32) -> HHOOK;
    pub fn UnhookWindowsHookEx(hook: HHOOK) -> BOOL;
    pub fn CallNextHookEx(hook: HHOOK, code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> BOOL;
    pub fn PostThreadMessageW(thread: u32, msg: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL;
    pub fn TranslateMessage(msg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn GetCursorPos(pt: *mut POINT) -> BOOL;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    pub fn GetPixel(hdc: HDC, x: i32, y: i32) -> u32;
    pub fn DrawIconEx(
        hdc: HDC,
        x: i32,
        y: i32,
        icon: HICON,
        cx: i32,
        cy: i32,
        istep: u32,
        brush: HBRUSH,
        flags: u32,
    ) -> BOOL;
    pub fn DestroyIcon(icon: HICON) -> BOOL;
    pub fn SetWindowPos(hwnd: HWND, after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> BOOL;
    pub fn GetAncestor(hwnd: HWND, flags: u32) -> HWND;
    pub fn GetSystemMetrics(index: i32) -> i32;
    pub fn LoadImageW(hinst: HINSTANCE, name: *const u16, kind: u32, cx: i32, cy: i32, flags: u32) -> HANDLE;
    pub fn RegisterWindowMessageW(name: *const u16) -> u32;
    pub fn keybd_event(vk: u8, scan: u8, flags: u32, extra: usize);
    pub fn SystemParametersInfoW(action: u32, param: u32, data: *mut c_void, winini: u32) -> BOOL;
    pub fn GetDpiForSystem() -> u32;
    pub fn SetProcessDPIAware() -> BOOL;
    pub fn SetProcessDpiAwarenessContext(ctx: isize) -> BOOL;
}

pub type HOOKPROC = extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

#[link(name = "shell32")]
extern "system" {
    pub fn SHGetFileInfoW(
        path: *const u16,
        attrs: u32,
        info: *mut SHFILEINFOW,
        cb: u32,
        flags: u32,
    ) -> usize;
    pub fn ShellExecuteW(
        hwnd: HWND,
        verb: *const u16,
        file: *const u16,
        params: *const u16,
        dir: *const u16,
        show: i32,
    ) -> HINSTANCE;
    pub fn ShellExecuteExW(info: *mut SHELLEXECUTEINFOW) -> BOOL;
    pub fn ILCreateFromPathW(path: *const u16) -> *mut c_void;
    pub fn SHOpenFolderAndSelectItems(pidl: *const c_void, count: u32, items: *const *const c_void, flags: u32) -> i32;
    pub fn ILFree(pidl: *mut c_void);
    pub fn ExtractIconExW(file: *const u16, index: i32, large: *mut HICON, small: *mut HICON, count: u32) -> u32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn CreateDIBSection(
        hdc: HDC,
        bmi: *const BITMAPINFO,
        usage: u32,
        bits: *mut *mut c_void,
        section: HANDLE,
        offset: u32,
    ) -> HBITMAP;
    pub fn SelectObject(hdc: HDC, obj: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(obj: HGDIOBJ) -> BOOL;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn GetObjectW(obj: HGDIOBJ, size: i32, out: *mut c_void) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> HMODULE;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn OpenProcess(access: u32, inherit: BOOL, pid: u32) -> HANDLE;
    pub fn QueryFullProcessImageNameW(h: HANDLE, flags: u32, buf: *mut u16, size: *mut u32) -> BOOL;
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn GetLastError() -> u32;
    pub fn CreateMutexW(attrs: *mut c_void, initial_owner: BOOL, name: *const u16) -> HANDLE;
    pub fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    pub fn GlobalLock(mem: *mut c_void) -> *mut c_void;
    pub fn GlobalUnlock(mem: *mut c_void) -> BOOL;
    pub fn GlobalFree(mem: *mut c_void) -> *mut c_void;
    pub fn VirtualAllocEx(
        process: HANDLE,
        addr: *mut c_void,
        size: usize,
        kind: u32,
        protect: u32,
    ) -> *mut c_void;
    pub fn VirtualFreeEx(process: HANDLE, addr: *mut c_void, size: usize, kind: u32) -> BOOL;
    pub fn ReadProcessMemory(
        process: HANDLE,
        addr: *const c_void,
        buf: *mut c_void,
        size: usize,
        read: *mut usize,
    ) -> BOOL;
    pub fn WriteProcessMemory(
        process: HANDLE,
        addr: *mut c_void,
        buf: *const c_void,
        size: usize,
        written: *mut usize,
    ) -> BOOL;
}

#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd: HWND, attr: u32, value: *const c_void, size: u32) -> i32;
}

pub mod icon;
pub mod monitor;
pub mod shell;
pub mod tree;
pub mod window;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub fn hwnd_string(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buf[..len as usize])
}

pub fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    from_wide(&buf[..len as usize])
}

/// Primary monitor work area in **physical** pixels (taskbar excluded).
pub fn work_area_physical() -> (f64, f64) {
    const SPI_GETWORKAREA: u32 = 0x0030;
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        let ok = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut r as *mut RECT as *mut c_void,
            0,
        );
        if ok == 0 {
            return (1280.0, 800.0);
        }
        ((r.right - r.left) as f64, (r.bottom - r.top) as f64)
    }
}

/// Primary monitor work area in **logical** pixels (taskbar excluded).
///
/// Windows silently shrinks a window that does not fit the work area, but the
/// webview keeps the size it was created with — the content then overflows and
/// its right edge (buttons, toggles) is clipped. Callers must clamp their
/// requested logical size to this box *before* creating the window.
pub fn work_area_logical() -> (f64, f64) {
    let (w, h) = work_area_physical();
    let dpi = unsafe { GetDpiForSystem() }.max(96) as f64 / 96.0;
    (w / dpi, h / dpi)
}

/// Opt the process into per-monitor-v2 DPI awareness.
///
/// The generated executable carries no `dpiAware` entry in its manifest, so
/// Windows starts it as DPI **unaware**: every window coordinate is virtualised
/// and the whole window is bitmap-stretched by the monitor scale. On a 175%
/// display a 1040x720 request became a 1820x1260 *physical* window that the
/// compositor then blew up to 3185x2205 — larger than the screen, blurry, and
/// with the right edge of every settings row pushed off the display.
///
/// This must run before the first window is created; `main` calls it before
/// Tauri starts. Returns the level that took effect, for the log.
pub fn become_dpi_aware() -> &'static str {
    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 is the pseudo-handle -4.
    const PER_MONITOR_AWARE_V2: isize = -4;
    unsafe {
        if SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2) != 0 {
            return "per-monitor-v2";
        }
        // Older builds (pre-1703) do not know the context API.
        if SetProcessDPIAware() != 0 {
            return "system";
        }
    }
    // Already aware: a manifest or an earlier call won the race.
    "unchanged"
}

// ---------------- elevation ----------------

#[repr(C)]
struct TOKEN_ELEVATION {
    token_is_elevated: u32,
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: HANDLE, access: u32, token: *mut HANDLE) -> BOOL;
    fn GetTokenInformation(
        token: HANDLE,
        class: u32,
        info: *mut c_void,
        len: u32,
        ret_len: *mut u32,
    ) -> BOOL;
}

/// Whether this process is running with an elevated (administrator) token.
///
/// `IsUserAnAdmin` is deprecated and answers a different question (is the
/// account an administrator), so the token itself is asked instead: with UAC
/// on, an administrator's normal process carries a *filtered* token and only
/// the elevated one reports `TokenIsElevated`.
pub fn is_elevated() -> bool {
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION_CLASS: u32 = 20; // TOKEN_INFORMATION_CLASS::TokenElevation
    unsafe {
        let mut token: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut info = TOKEN_ELEVATION {
            token_is_elevated: 0,
        };
        let mut ret: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut info as *mut TOKEN_ELEVATION as *mut c_void,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret,
        );
        CloseHandle(token);
        ok != 0 && info.token_is_elevated != 0
    }
}

/// Raise a window to the top of the *normal* z-order band without giving it
/// focus and without making it topmost.
///
/// The Quick Switch box must live on the same level as the file dialog it is
/// attached to: visible right above the dialog, but still covered by any other
/// window the user switches to. `always_on_top` gave it its own band and made
/// it float over everything, which is exactly what the user complained about.
pub fn raise_no_activate(hwnd: HWND) {
    const HWND_TOP: HWND = 0;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOACTIVATE: u32 = 0x0010;
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE,
        );
    }
}

/// Drop the topmost band. `SetWindowPos(HWND_NOTOPMOST)` is used instead of
/// Tauri's `set_always_on_top(false)` where a synchronous, no-round-trip call
/// is needed (the follower runs at 60 Hz).
pub fn clear_topmost(hwnd: HWND) {
    const HWND_NOTOPMOST: HWND = -2;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOACTIVATE: u32 = 0x0010;
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE,
        );
    }
}

/// Whether `child` currently sits above `anchor` in the z-order.
///
/// Walks the top-level list from the front, so the answer reflects what the
/// user actually sees. Used to avoid calling `SetWindowPos` on every follower
/// tick: unconditional raises made the box flicker.
pub fn is_above(child: HWND, anchor: HWND) -> bool {
    const GW_HWNDNEXT: u32 = 2;
    unsafe {
        let mut h = GetTopWindow(0);
        while h != 0 {
            if h == child {
                return true;
            }
            if h == anchor {
                return false;
            }
            h = GetWindow(h, GW_HWNDNEXT);
        }
        false
    }
}

/// Keep `child` one slot above `anchor`, in the same (normal) z-order band.
///
/// This is the whole "the box is part of the dialog" rule, and it is fiddly
/// because of a Windows guarantee: the foreground window always sits at the
/// top of its band, and `HWND_TOP` on another window of that band is silently
/// ignored (measured on Win11: `SetWindowPos(child, HWND_TOP, ...)` returned
/// TRUE and changed nothing). A plain raise therefore could never put the box
/// back above the dialog it belongs to — which is why every previous attempt
/// either did nothing or fell back to making the box topmost, and topmost put
/// it in a *different band* than the dialog (the user's actual complaint).
///
/// The way around the rule is to bounce through the topmost band and come
/// straight back out: `HWND_NOTOPMOST` inserts the window at the top of the
/// normal band, i.e. exactly one slot above the foreground dialog. Both calls
/// are back-to-back with no message pump in between, so the box is never
/// painted while topmost — no flash.
///
/// Only does anything while `anchor` really is the foreground window; lifting
/// the box above whatever the user switched to would be the opposite of what
/// we want. Returns whether anything was changed.
pub fn keep_above(child: HWND, anchor: HWND) -> bool {
    const HWND_TOPMOST: HWND = -1;
    const HWND_NOTOPMOST: HWND = -2;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOPMOST: isize = 0x8;
    const FLAGS: u32 = SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE;

    if unsafe { GetForegroundWindow() } != anchor {
        return false;
    }
    let topmost = unsafe { GetWindowLongW(child, GWL_EXSTYLE) } & WS_EX_TOPMOST != 0;
    if !topmost && is_above(child, anchor) {
        return false;
    }
    unsafe {
        SetWindowPos(child, HWND_TOPMOST, 0, 0, 0, 0, FLAGS);
        SetWindowPos(child, HWND_NOTOPMOST, 0, 0, 0, 0, FLAGS);
    }
    true
}
