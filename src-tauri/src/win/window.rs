//! Window enumeration, activation, file-dialog detection, window management.

use super::*;
use serde::Serialize;
use std::ffi::c_void;

#[link(name = "user32")]
extern "system" {
    fn EnumChildWindows(hwnd: HWND, cb: extern "system" fn(HWND, LPARAM) -> BOOL, lparam: LPARAM) -> BOOL;
    fn MonitorFromWindow(hwnd: HWND, flags: u32) -> isize;
    fn GetMonitorInfoW(monitor: isize, info: *mut MONITORINFO) -> BOOL;
    fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
}

#[repr(C)]
struct MONITORINFO {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    dw_flags: u32,
}

const GW_CHILD: u32 = 5;
const GW_HWNDNEXT: u32 = 2;
const MONITOR_DEFAULTTONEAREST: u32 = 2;

const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;
const HWND_TOPMOST: isize = -1;
const HWND_NOTOPMOST: isize = -2;

#[derive(Serialize, Clone)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub class: String,
    pub pid: u32,
    pub process: String,
    pub process_path: String,
}

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn process_path(pid: u32) -> String {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h == 0 {
            return String::new();
        }
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(h);
        if ok == 0 {
            String::new()
        } else {
            from_wide(&buf[..size as usize])
        }
    }
}

fn is_tool_window(hwnd: HWND) -> bool {
    let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    (ex & WS_EX_TOOLWINDOW) != 0
}

pub fn list_windows(include_self_pid: u32) -> Vec<WindowInfo> {
    let mut out: Vec<WindowInfo> = Vec::new();
    extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let list = &mut *(lparam as *mut Vec<WindowInfo>);
            if IsWindowVisible(hwnd) == 0 || is_tool_window(hwnd) {
                return 1;
            }
            let title = hwnd_string(hwnd);
            if title.trim().is_empty() {
                return 1;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            let path = process_path(pid);
            let process = std::path::Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            list.push(WindowInfo {
                hwnd,
                title,
                class: class_name(hwnd),
                pid,
                process,
                process_path: path,
            });
            1
        }
    }
    unsafe {
        EnumWindows(cb, &mut out as *mut Vec<WindowInfo> as LPARAM);
    }
    out.retain(|w| w.pid != include_self_pid && !w.class.is_empty());
    out
}

pub fn activate(hwnd: HWND) -> bool {
    unsafe {
        if IsWindow(hwnd) == 0 {
            return false;
        }
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let my_thread = GetCurrentThreadId();
        let attached = if fg_thread != my_thread {
            AttachThreadInput(my_thread, fg_thread, 1)
        } else {
            0
        };
        SetForegroundWindow(hwnd);
        BringWindowToTop(hwnd);
        if attached != 0 {
            AttachThreadInput(my_thread, fg_thread, 0);
        }
        GetForegroundWindow() == hwnd
    }
}

pub fn minimize(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, 6); // SW_MINIMIZE
    }
}

pub fn close(hwnd: HWND) {
    unsafe {
        PostMessageW(hwnd, 0x0010, 0, 0); // WM_CLOSE
    }
}

pub fn toggle_topmost(hwnd: HWND) -> bool {
    let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let is_top = (ex & 0x0000_0008) != 0; // WS_EX_TOPMOST
    unsafe {
        SetWindowPos(
            hwnd,
            if is_top { HWND_NOTOPMOST } else { HWND_TOPMOST },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    !is_top
}

fn work_area(hwnd: HWND) -> RECT {
    unsafe {
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cb_size = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(mon, &mut info) != 0 {
            info.rc_work
        } else {
            RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
        }
    }
}

/// Work area (left, top, right, bottom) of the monitor containing `hwnd`.
pub fn monitor_work_area(hwnd: HWND) -> (i32, i32, i32, i32) {
    let wa = work_area(hwnd);
    (wa.left, wa.top, wa.right, wa.bottom)
}

/// side: "left" | "right" | "max" | "center"
pub fn snap(hwnd: HWND, side: &str) -> bool {
    let wa = work_area(hwnd);
    let (w, h) = (wa.right - wa.left, wa.bottom - wa.top);
    let (x, y, cw, ch) = match side {
        "left" => (wa.left, wa.top, w / 2, h),
        "right" => (wa.left + w / 2, wa.top, w / 2, h),
        "max" => (wa.left, wa.top, w, h),
        "center" => (wa.left + w / 4, wa.top + h / 4, w / 2, h / 2),
        _ => return false,
    };
    unsafe { SetWindowPos(hwnd, 0, x, y, cw, ch, 0x0040 | SWP_NOACTIVATE) != 0 }
}

// ---------------- file dialog helpers ----------------

/// Walks descendants looking for a control with the given class.
fn find_child_deep(parent: HWND, class: &str) -> Option<HWND> {
    unsafe {
        let mut child = GetWindow(parent, GW_CHILD);
        while child != 0 {
            let mut buf = [0u16; 256];
            let len = GetClassNameW(child, buf.as_mut_ptr(), buf.len() as i32);
            if len > 0 {
                let name = from_wide(&buf[..len as usize]);
                if name.eq_ignore_ascii_case(class) {
                    return Some(child);
                }
            }
            if let Some(found) = find_child_deep(child, class) {
                return Some(found);
            }
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }
    None
}

fn child_count(parent: HWND) -> usize {
    let mut n = 0;
    unsafe {
        let mut child = GetWindow(parent, GW_CHILD);
        while child != 0 {
            n += 1;
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }
    n
}

/// One single pass over the child tree of a dialog.
///
/// The old implementation called `find_child_deep` up to seven times, and
/// each call walked the whole tree (a modern file dialog has hundreds of
/// child windows). One walk with the class names compared on the way is
/// dramatically cheaper and the result is identical.
///
/// Returns `(has_edit, has_list, child_count)`.
fn scan_dialog_children(parent: HWND) -> (bool, bool, usize) {
    let mut has_edit = false;
    let mut has_list = false;
    let mut count = 0usize;
    let mut stack: Vec<HWND> = Vec::new();

    unsafe {
        let mut child = GetWindow(parent, GW_CHILD);
        while child != 0 {
            stack.push(child);
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }

    while let Some(h) = stack.pop() {
        count += 1;
        let mut buf = [0u16; 256];
        let len = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        if len > 0 {
            let name = from_wide(&buf[..len as usize]);
            if name.eq_ignore_ascii_case("Edit") || name.eq_ignore_ascii_case("ComboBoxEx32") {
                has_edit = true;
            } else if name.eq_ignore_ascii_case("DirectUIHWND")
                || name.eq_ignore_ascii_case("DUIViewWndClassName")
                || name.eq_ignore_ascii_case("SHELLDLL_DefView")
                || name.eq_ignore_ascii_case("SysListView32")
                || name.eq_ignore_ascii_case("NamespaceTreeControl")
            {
                has_list = true;
            }
        }
        unsafe {
            let mut c = GetWindow(h, GW_CHILD);
            while c != 0 {
                stack.push(c);
                c = GetWindow(c, GW_HWNDNEXT);
            }
        }
    }
    (has_edit, has_list, count)
}

/// A file dialog is a #32770 window with an edit box and a list view / shell view.
pub fn is_file_dialog(hwnd: HWND) -> bool {
    if class_name(hwnd) != "#32770" {
        return false;
    }
    let (has_edit, has_list, n) = scan_dialog_children(hwnd);
    crate::debug_log::qs_log_verbose(&format!(
        "is_file_dialog {hwnd:#x}: edit={has_edit} list={has_list} children={n}"
    ));
    has_edit && has_list && n > 3
}

/// The "File name" edit of a dialog, if any.
pub fn dialog_edit(hwnd: HWND) -> Option<HWND> {
    if !is_file_dialog(hwnd) {
        return None;
    }
    // Prefer an Edit nested in a ComboBox (that is the filename field).
    if let Some(combo) = find_child_deep(hwnd, "ComboBox") {
        if let Some(edit) = find_child_deep(combo, "Edit") {
            return Some(edit);
        }
    }
    find_child_deep(hwnd, "Edit")
}

pub fn get_text(hwnd: HWND) -> String {
    unsafe {
        let len = SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0) as usize;
        if len == 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len + 1];
        SendMessageW(hwnd, WM_GETTEXT, buf.len(), buf.as_mut_ptr() as LPARAM);
        from_wide(&buf)
    }
}

pub fn set_text(hwnd: HWND, text: &str) {
    let w = wide(text);
    unsafe {
        SendMessageW(hwnd, WM_SETTEXT, 0, w.as_ptr() as LPARAM);
    }
}

/// Best-effort guess of the folder currently shown in a file dialog.
pub fn dialog_current_folder(hwnd: HWND) -> Option<String> {    // The address bar of the modern dialog exposes the path through a toolbar's text.
    if let Some(toolbar) = find_child_deep(hwnd, "ToolbarWindow32") {
        let t = get_text(toolbar);
        if t.contains(":\\") {
            if let Some(idx) = t.find(":\\") {
                let start = t[..idx].rfind(|c| c == ' ').map(|i| i + 1).unwrap_or(0);
                let candidate = &t[start..];
                if std::path::Path::new(candidate).is_dir() {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    // Fallback: if the filename box already holds a full path.
    if let Some(edit) = dialog_edit(hwnd) {
        let t = get_text(edit);
        if t.contains(":\\") {
            let p = std::path::PathBuf::from(&t);
            if let Some(parent) = p.parent() {
                return Some(parent.to_string_lossy().to_string());
            }
        }
    }
    None
}

struct EditBand {
    dlg_top: i32,
    /// Keep the edit closest to the top (address bar) or closest to the bottom
    /// (filename field).
    want_top: bool,
    hit: Option<HWND>,
    hit_pos: i32,
}

extern "system" fn scan_edit_band(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = unsafe { &mut *(lparam as *mut EditBand) };
    if class_name(hwnd) != "Edit" {
        return 1;
    }
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    let mut r: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut r) } == 0 {
        return 1;
    }
    // Only consider edits that live inside the dialog's vertical span; the
    // dialog is the parent we started from, so stray popups are excluded by
    // visibility plus this range check.
    if r.top < ctx.dlg_top - 20 {
        return 1;
    }
    if ctx.want_top {
        if ctx.hit.is_none() || r.top < ctx.hit_pos {
            ctx.hit = Some(hwnd);
            ctx.hit_pos = r.top;
        }
    } else if ctx.hit.is_none() || r.bottom > ctx.hit_pos {
        ctx.hit = Some(hwnd);
        ctx.hit_pos = r.bottom;
    }
    1
}

fn pick_edit(dialog: HWND, want_top: bool) -> Option<HWND> {
    let mut r: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(dialog, &mut r) } == 0 {
        return None;
    }
    let mut ctx = EditBand { dlg_top: r.top, want_top, hit: None, hit_pos: 0 };
    unsafe { EnumChildWindows(dialog, scan_edit_band, &mut ctx as *mut EditBand as LPARAM) };
    ctx.hit
}

/// The filename field (the Edit at the bottom of the dialog).
pub fn filename_edit(dialog: HWND) -> Option<HWND> {
    if !is_file_dialog(dialog) {
        return None;
    }
    pick_edit(dialog, false).or_else(|| dialog_edit(dialog))
}

const VK_CONTROL: u8 = 0x11;
const VK_RETURN: u8 = 0x0D;
const KEYEVENTF_KEYUP: u32 = 0x0002;

/// Navigate an open file dialog to `path` **without confirming it**.
///
/// Typing a folder into the filename box and pressing OK is what used to
/// happen, and on a save dialog (or with a path the shell resolves to a file)
/// that dismisses the dialog — the user never got to choose. Ctrl+L focuses the
/// address bar (Alt+D only makes it visible — measured), and Enter *there* only
/// ever navigates. No fallback to the filename box on purpose: Enter in that
/// field means "open", which is exactly what must not happen.
pub fn navigate_to_folder(dialog: HWND, path: &str) -> bool {
    if !is_file_dialog(dialog) {
        return false;
    }
    // The box itself has the focus when the user clicks a folder in it, so the
    // dialog has to be activated first — and we must *verify* that happened
    // before sending keys. Alt+D fired at the wrong window would open a menu
    // somewhere else entirely.
    let mut focused = false;
    for _ in 0..6 {
        activate(dialog);
        let fg = foreground();
        if fg == dialog {
            focused = true;
            break;
        }
        let mut fg_pid = 0u32;
        let mut dlg_pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(fg, &mut fg_pid);
            GetWindowThreadProcessId(dialog, &mut dlg_pid);
        }
        // While the dialog is coming up, the foreground is often one of its own
        // child windows — same process is good enough.
        if fg_pid == dlg_pid && fg_pid != 0 {
            focused = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    if !focused {
        crate::debug_log::qs_log("navigate: dialog did not take focus, keys not sent");
        return false;
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    // Ctrl+L: turn the breadcrumb into an editable address bar AND give it the
    // caret. Plain Alt+D only made the edit visible while the filename box kept
    // the focus, so typing landed in the wrong field.
    unsafe {
        keybd_event(VK_CONTROL, 0, 0, 0);
        keybd_event(b'L', 0, 0, 0);
        keybd_event(b'L', 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
    }
    std::thread::sleep(std::time::Duration::from_millis(130));
    let Some(edit) = pick_edit(dialog, true) else {
        crate::debug_log::qs_log("navigate: address bar did not open");
        return false;
    };
    set_text(edit, path);
    std::thread::sleep(std::time::Duration::from_millis(30));
    unsafe {
        keybd_event(VK_RETURN, 0, 0, 0);
        keybd_event(VK_RETURN, 0, KEYEVENTF_KEYUP, 0);
    }
    crate::debug_log::qs_log(&format!("navigate: dialog {dialog:#x} -> {path}"));
    true
}

/// Put `path` in the filename box so the user can accept it themselves.
/// Deliberately does **not** press OK: choosing what to do with the path is
/// the user's call.
pub fn fill_filename(dialog: HWND, path: &str) -> bool {
    let Some(edit) = filename_edit(dialog) else {
        return false;
    };
    set_text(edit, path);
    // Select the text: the user can overwrite it by typing, or press Enter
    // themselves when they are sure.
    unsafe {
        SendMessageW(edit, 0x00B1 /* EM_SETSEL */, 0, -1);
    }
    activate(dialog);
    crate::debug_log::qs_log(&format!("fill filename: {path}"));
    true
}

pub fn pixel_color(x: i32, y: i32) -> (u8, u8, u8) {
    unsafe {
        let hdc = GetDC(0);
        if hdc == 0 {
            return (0, 0, 0);
        }
        let c = GetPixel(hdc, x, y);
        ReleaseDC(0, hdc);
        if c == u32::MAX {
            return (0, 0, 0);
        }
        ((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8)
    }
}

pub fn cursor_pos() -> (i32, i32) {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut pt);
    }
    (pt.x, pt.y)
}

pub fn screen_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) }
}

/// Force a dark/light native titlebar (used if a window ever keeps native decorations).
pub fn set_dark_titlebar(hwnd: HWND, dark: bool) {
    let value: i32 = if dark { 1 } else { 0 };
    unsafe {
        DwmSetWindowAttribute(hwnd, 20, &value as *const i32 as *const c_void, 4);
        DwmSetWindowAttribute(hwnd, 19, &value as *const i32 as *const c_void, 4);
    }
}
