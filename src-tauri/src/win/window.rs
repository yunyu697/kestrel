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

/// Structural fingerprint of a `#32770` dialog.
///
/// The class alone tells us nothing: message boxes, IDM's main window and
/// real Open/Save dialogs all share it. The class names of the children are
/// what separates them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DialogShape {
    /// The combo that holds the file name: `ComboBoxEx32` on the common
    /// dialog, a plain `ComboBox` in the Save dialog's variant.
    pub has_name_combo: bool,
    /// The shell view that lists the files: `SHELLDLL_DefView` (classic) or
    /// `DUIViewWndClassName` (DirectUI hosted).
    pub has_shell_view: bool,
    /// Any plain text edit — a search box, a message box input, ...
    pub has_edit: bool,
    /// A classic list view. IDM's download list is one of these.
    pub has_list_view: bool,
    /// A tree view. The navigation pane of a file dialog, IDM's category
    /// tree, ...
    pub has_tree_view: bool,
    /// The legacy list box of the old-style file dialog.
    pub has_list_box: bool,
    pub children: usize,
}

impl DialogShape {
    /// Fold one child class name into the fingerprint.
    fn add_class(&mut self, class: &str) {
        if class.eq_ignore_ascii_case("ComboBoxEx32") || class.eq_ignore_ascii_case("ComboBox") {
            self.has_name_combo = true;
        } else if class.eq_ignore_ascii_case("SHELLDLL_DefView")
            || class.eq_ignore_ascii_case("DUIViewWndClassName")
        {
            self.has_shell_view = true;
        } else if class.eq_ignore_ascii_case("Edit") {
            self.has_edit = true;
        } else if class.eq_ignore_ascii_case("SysListView32") {
            self.has_list_view = true;
        } else if class.eq_ignore_ascii_case("SysTreeView32")
            || class.eq_ignore_ascii_case("NamespaceTreeControl")
        {
            self.has_tree_view = true;
        } else if class.eq_ignore_ascii_case("ListBox") {
            self.has_list_box = true;
        }
    }

    /// Is this an Open/Save file dialog?
    ///
    /// Two independent markers have to be present:
    ///
    /// * the **shell view** (`SHELLDLL_DefView` / `DUIViewWndClassName`) —
    ///   only the shell's own item view creates it, and
    /// * the **file name combo** (`ComboBoxEx32` / `ComboBox`).
    ///
    /// The old rule ("has an Edit and something list-shaped") matched far too
    /// much: IDM's main window is a `#32770` holding a `SysListView32` (the
    /// download list) and an `Edit` (the search box), so the Quick Switch
    /// popped up under IDM's download notification. Requiring the shell view
    /// rejects it — and with it every other app that draws its own list in a
    /// dialog.
    pub fn is_file_dialog(&self) -> bool {
        self.has_name_combo && self.has_shell_view
    }
}

/// One single pass over the child tree of a dialog.
///
/// The old implementation called `find_child_deep` up to seven times, and
/// each call walked the whole tree (a modern file dialog has hundreds of
/// child windows). One walk with the class names compared on the way is
/// dramatically cheaper and the result is identical.
fn scan_dialog_children(parent: HWND) -> DialogShape {
    let mut shape = DialogShape::default();
    let mut stack: Vec<HWND> = Vec::new();

    unsafe {
        let mut child = GetWindow(parent, GW_CHILD);
        while child != 0 {
            stack.push(child);
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }

    while let Some(h) = stack.pop() {
        shape.children += 1;
        let mut buf = [0u16; 256];
        let len = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        if len > 0 {
            shape.add_class(&from_wide(&buf[..len as usize]));
        }
        unsafe {
            let mut c = GetWindow(h, GW_CHILD);
            while c != 0 {
                stack.push(c);
                c = GetWindow(c, GW_HWNDNEXT);
            }
        }
    }
    shape
}

/// A file dialog is a `#32770` window with the shell's item view and a file
/// name combo. See [`DialogShape::is_file_dialog`] for why the weaker
/// "edit + list" heuristic was not enough.
pub fn is_file_dialog(hwnd: HWND) -> bool {
    if class_name(hwnd) != "#32770" {
        return false;
    }
    let shape = scan_dialog_children(hwnd);
    crate::debug_log::qs_log_verbose(&format!(
        "is_file_dialog {hwnd:#x}: combo={} shell={} edit={} list={} tree={} children={}",
        shape.has_name_combo,
        shape.has_shell_view,
        shape.has_edit,
        shape.has_list_view,
        shape.has_tree_view,
        shape.children
    ));
    shape.is_file_dialog()
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

#[cfg(test)]
mod tests {
    use super::super::{EnumWindows, GetWindowTextW, IsWindowVisible, PostMessageW, BOOL, LPARAM};
    use super::DialogShape;
    use super::{class_name, is_file_dialog};

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxA(hwnd: isize, text: *const u8, caption: *const u8, flags: u32) -> i32;
    }

    #[repr(C)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwnd_owner: isize,
        hinstance: isize,
        lpstr_filter: *const u16,
        lpstr_custom_filter: *mut u16,
        n_max_cust_filter: u32,
        n_filter_index: u32,
        lpstr_file: *mut u16,
        n_max_file: u32,
        lpstr_file_title: *mut u16,
        n_max_file_title: u32,
        lpstr_initial_dir: *const u16,
        lpstr_title: *const u16,
        flags: u32,
        n_file_offset: u16,
        n_file_extension: u16,
        lpstr_def_ext: *const u16,
        l_cust_data: isize,
        lpfn_hook: isize,
        lp_template_name: *const u16,
        pv_reserved: *mut isize,
        dw_reserved: u32,
        flags_ex: u32,
    }

    /// The title the probe looks for. A `fn` callback cannot capture, so the
    /// wanted title lives in a static.
    static mut WANTED_TITLE: &str = "";

    /// Poll the top-level windows until a visible `#32770` with the wanted
    /// title shows up.
    fn find_visible_dialog(wanted: &'static str, timeout_ms: u64) -> Option<isize> {
        extern "system" fn cb(hwnd: isize, l: LPARAM) -> BOOL {
            let slot = l as *mut isize;
            let want = unsafe { WANTED_TITLE };
            unsafe {
                if class_name(hwnd) == "#32770" && IsWindowVisible(hwnd) != 0 {
                    let mut buf = [0u16; 256];
                    let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), 256);
                    let t = String::from_utf16_lossy(&buf[..n as usize]);
                    if t.contains(want) {
                        *slot = hwnd;
                        return 0;
                    }
                }
            }
            1
        }
        unsafe { WANTED_TITLE = wanted };
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        while std::time::Instant::now() < deadline {
            let mut found: isize = 0;
            unsafe { EnumWindows(cb, &mut found as *mut isize as LPARAM) };
            if found != 0 {
                return Some(found);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        None
    }

    /// Close a probe dialog and wait until it is really gone.
    fn close_dialog(hwnd: isize) {
        unsafe { PostMessageW(hwnd, 0x0010, 0, 0) };
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if unsafe { IsWindowVisible(hwnd) } == 0 {
                return;
            }
        }
    }

    /// A real common file dialog must be recognised.
    #[test]
    #[ignore = "opens a dialog on screen"]
    fn detects_a_real_file_dialog() {
        std::thread::spawn(|| {
            let title: Vec<u16> = "KestrelOpen".encode_utf16().chain(std::iter::once(0)).collect();
            let mut buf = [0u16; 260];
            let mut ofn = OPENFILENAMEW {
                lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
                hwnd_owner: 0,
                hinstance: 0,
                lpstr_filter: std::ptr::null(),
                lpstr_custom_filter: std::ptr::null_mut(),
                n_max_cust_filter: 0,
                n_filter_index: 0,
                lpstr_file: buf.as_mut_ptr(),
                n_max_file: 260,
                lpstr_file_title: std::ptr::null_mut(),
                n_max_file_title: 0,
                lpstr_initial_dir: std::ptr::null(),
                lpstr_title: title.as_ptr(),
                flags: 0x0008_0000 | 0x0000_1000 | 0x0000_0800,
                n_file_offset: 0,
                n_file_extension: 0,
                lpstr_def_ext: std::ptr::null(),
                l_cust_data: 0,
                lpfn_hook: 0,
                lp_template_name: std::ptr::null(),
                pv_reserved: std::ptr::null_mut(),
                dw_reserved: 0,
                flags_ex: 0,
            };
            unsafe { GetOpenFileNameW(&mut ofn) };
        });
        let hwnd = find_visible_dialog("KestrelOpen", 8000).expect("no dialog appeared");
        // The dialog shows itself before every control is in place, so give
        // the shell a moment to finish building the view.
        let mut detected = false;
        let mut shape = super::DialogShape::default();
        for _ in 0..30 {
            shape = super::scan_dialog_children(hwnd);
            detected = is_file_dialog(hwnd);
            if detected {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        println!("open dialog shape: {shape:?}");
        close_dialog(hwnd);
        assert!(detected, "a real Open dialog was not recognised");
    }

    /// A message box must not be mistaken for a file dialog.
    #[test]
    #[ignore = "opens a dialog on screen"]
    fn rejects_a_real_message_box() {
        std::thread::spawn(|| unsafe {
            MessageBoxA(
                0,
                b"hello\0".as_ptr(),
                b"KestrelMsg\0".as_ptr(),
                0,
            );
        });
        let hwnd = find_visible_dialog("KestrelMsg", 8000).expect("no message box appeared");
        let detected = is_file_dialog(hwnd);
        println!("message box shape: {:?}", super::scan_dialog_children(hwnd));
        close_dialog(hwnd);
        assert!(!detected, "a message box was mistaken for a file dialog");
    }

    /// IDM's own windows must not be mistaken for a file dialog.
    ///
    /// IDM 6.43 keeps its main window in the `#32770` class and fills it with
    /// a download list plus a search box — which is exactly what the old
    /// "edit + list" heuristic looked for. Its download-complete popup comes
    /// from `IDMMsgHost.exe`.
    #[test]
    #[ignore = "needs IDM running"]
    fn rejects_idms_live_windows() {
        extern "system" fn cb(hwnd: isize, l: LPARAM) -> BOOL {
            let out = l as *mut Vec<(isize, String)>;
            unsafe {
                if class_name(hwnd) == "#32770" {
                    let mut buf = [0u16; 256];
                    let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), 256);
                    (*out).push((hwnd, String::from_utf16_lossy(&buf[..n as usize])));
                }
            }
            1
        }
        let mut list: Vec<(isize, String)> = Vec::new();
        unsafe { EnumWindows(cb, &mut list as *mut Vec<(isize, String)> as LPARAM) };
        let mut checked = 0;
        for (hwnd, title) in &list {
            let looks_like_idm = title.contains("Internet Download Manager")
                || title.contains("拖至窗口")
                || title.contains("IDM")
                || title.contains("下载文件信息");
            if !looks_like_idm {
                continue;
            }
            checked += 1;
            println!("IDM window {hwnd:#x} title={title:?} shape={:?}", super::scan_dialog_children(*hwnd));
            assert!(
                !is_file_dialog(*hwnd),
                "IDM window {hwnd:#x} ({title}) was mistaken for a file dialog"
            );
        }
        println!("checked {checked} IDM windows");
    }

    /// Every live `#32770` that is *not* a file dialog must be rejected.
    ///
    /// Run with IDM open: its main window is a `#32770` with a download list
    /// and a search box, and it used to be accepted.
    #[test]
    #[ignore = "inspects the live desktop"]
    fn rejects_live_non_file_dialogs() {
        extern "system" fn cb(hwnd: isize, l: LPARAM) -> BOOL {
            let out = l as *mut Vec<isize>;
            unsafe {
                if class_name(hwnd) == "#32770" && IsWindowVisible(hwnd) != 0 {
                    (*out).push(hwnd);
                }
            }
            1
        }
        let mut dialogs: Vec<isize> = Vec::new();
        unsafe { EnumWindows(cb, &mut dialogs as *mut Vec<isize> as LPARAM) };
        for hwnd in dialogs {
            let shape = super::scan_dialog_children(hwnd);
            // A dialog that carries the shell view is a real file dialog and
            // must be accepted; everything else must be refused.
            assert_eq!(
                is_file_dialog(hwnd),
                shape.has_name_combo && shape.has_shell_view,
                "hwnd {hwnd:#x} classified inconsistently"
            );
        }
    }

    /// Build a fingerprint from a real child-class listing.
    fn shape_of(classes: &[&str]) -> DialogShape {
        let mut s = DialogShape::default();
        for c in classes {
            s.add_class(c);
            s.children += 1;
        }
        s
    }

    /// Child classes of the Windows 11 common "Open" dialog, captured with
    /// scripts/probe-dialog-variants.rs.
    const OPEN_DIALOG: &[&str] = &[
        "Address Band Root",
        "Breadcrumb Parent",
        "Button",
        "ComboBox",
        "ComboBoxEx32",
        "CtrlNotifySink",
        "DUIViewWndClassName",
        "DirectUIHWND",
        "Edit",
        "ListBox",
        "NamespaceTreeControl",
        "ReBarWindow32",
        "SHELLDLL_DefView",
        "ScrollBar",
        "Search Box",
        "SearchEditBoxWrapperClass",
        "SeparatorBand",
        "Shell Preview Extension Host",
        "Static",
        "SysTreeView32",
        "ToolbarWindow32",
        "TravelBand",
        "UniversalSearchBand",
        "UpBand",
        "WorkerW",
        "msctls_progress32",
    ];

    /// The Windows 11 "Save as" dialog: no ComboBoxEx32, the file name lives
    /// in a plain ComboBox. It must still be recognised.
    const SAVE_DIALOG: &[&str] = &[
        "Button",
        "ComboBox",
        "ComboBox",
        "CtrlNotifySink",
        "DUIViewWndClassName",
        "DirectUIHWND",
        "Edit",
        "FloatNotifySink",
        "ListBox",
        "NamespaceTreeControl",
        "ReBarWindow32",
        "SHELLDLL_DefView",
        "ScrollBar",
        "Search Box",
        "Static",
        "SysTreeView32",
        "ToolbarWindow32",
        "WorkerW",
    ];

    /// SHBrowseForFolder's tree picker: no shell view, no file name combo.
    const FOLDER_PICKER: &[&str] = &["Button", "Static", "Static", "SysTreeView32"];

    /// IDM 6.43's main window (a `#32770` with the download list and the
    /// search box). This is the window the Quick Switch used to attach to.
    const IDM_MAIN: &[&str] = &[
        "SysListView32",
        "SysHeader32",
        "SysTreeView32",
        "Button",
        "Static",
        "ToolbarWindow32",
        "ReBarWindow32",
        "#32770",
        "Static",
        "Edit",
        "Edit",
        "Button",
        "Button",
        "Button",
        "Button",
    ];

    /// A plain message box: static text, an icon and two buttons.
    const MESSAGE_BOX: &[&str] = &["Static", "Static", "Button", "Button"];

    #[test]
    fn recognises_the_open_dialog() {
        assert!(shape_of(OPEN_DIALOG).is_file_dialog());
    }

    #[test]
    fn recognises_the_save_dialog() {
        assert!(shape_of(SAVE_DIALOG).is_file_dialog());
    }

    #[test]
    fn rejects_the_folder_picker() {
        // A folder tree has nothing to put a file name into, so the Quick
        // Switch has no business attaching to it.
        assert!(!shape_of(FOLDER_PICKER).is_file_dialog());
    }

    #[test]
    fn rejects_idms_main_window() {
        // The regression this rule was written for: an edit box plus a list
        // view is not enough — IDM's dialog has both.
        let s = shape_of(IDM_MAIN);
        assert!(s.has_edit && s.has_list_view, "IDM shape changed");
        assert!(!s.has_name_combo && !s.has_shell_view);
        assert!(!s.is_file_dialog());
    }

    #[test]
    fn rejects_message_boxes() {
        assert!(!shape_of(MESSAGE_BOX).is_file_dialog());
    }

    #[test]
    fn a_shell_view_without_a_combo_is_not_a_file_dialog() {
        let mut s = DialogShape::default();
        s.add_class("SHELLDLL_DefView");
        s.add_class("DirectUIHWND");
        assert!(!s.is_file_dialog());
    }

    #[test]
    fn a_combo_without_a_shell_view_is_not_a_file_dialog() {
        let mut s = DialogShape::default();
        s.add_class("ComboBoxEx32");
        s.add_class("Edit");
        s.add_class("SysListView32");
        assert!(!s.is_file_dialog());
    }
}
