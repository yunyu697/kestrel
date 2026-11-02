//! Window enumeration, activation, file-dialog detection, window management.

use super::*;
use serde::Serialize;
use std::ffi::c_void;

#[link(name = "user32")]
extern "system" {
    fn EnumChildWindows(
        hwnd: HWND,
        cb: extern "system" fn(HWND, LPARAM) -> BOOL,
        lparam: LPARAM,
    ) -> BOOL;
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
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
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
    /// An **editable** combo box, i.e. one whose text field is a real `Edit`
    /// child. That is the "you can type a path in here" marker: a plain
    /// drop-down list (`CBS_DROPDOWNLIST`) has no `Edit` inside it, so a
    /// settings dialog with a picker and a search box does not qualify.
    pub has_path_edit: bool,
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

    /// A dialog that chooses a **folder** without hosting the shell's item
    /// view: an editable path field plus a folder tree.
    ///
    /// WinRAR's "extract path and options" dialog is the model case — the user
    /// picks a destination folder there, which is exactly what the Quick
    /// Switch is for, yet it is an ordinary app dialog (`#32770`, a
    /// `ComboBox` with an `Edit` inside, and a `SysTreeView32`), not a shell
    /// dialog, so the file-dialog rule never matched it.
    ///
    /// The editable combo is what keeps this narrow. IDM's main window (the
    /// regression the file-dialog rule was written for) has an `Edit` and a
    /// `SysTreeView32` as well, but no combo, so it stays rejected.
    ///
    /// The test is deliberately structural — no cross-process text reads in
    /// the attach loop, which runs up to eight times per dialog. A settings
    /// dialog that happens to put an editable combo next to a category tree
    /// can therefore still match. The cost is a stray box: attaching changes
    /// nothing, and the only way to alter the dialog is for the user to pick
    /// a folder in it.
    pub fn is_folder_dialog(&self) -> bool {
        self.has_path_edit && self.has_tree_view
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
            let class = from_wide(&buf[..len as usize]);
            shape.add_class(&class);
            // An `Edit` whose parent is a combo means the combo is editable:
            // a drop-down *list* keeps its text inside the control and has no
            // child window, so this is exactly the "type a path here" signal.
            if class.eq_ignore_ascii_case("Edit") {
                let parent = unsafe { GetParent(h) };
                if parent != 0 {
                    let pc = class_name(parent);
                    if pc.eq_ignore_ascii_case("ComboBox")
                        || pc.eq_ignore_ascii_case("ComboBoxEx32")
                    {
                        shape.has_path_edit = true;
                    }
                }
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
    shape
}

/// A file dialog is a `#32770` window with the shell's item view and a file
/// name combo. See [`DialogShape::is_file_dialog`] for why the weaker
/// "edit + list" heuristic was not enough.
pub fn is_file_dialog(hwnd: HWND) -> bool {
    dialog_shape(hwnd)
        .map(|s| s.is_file_dialog())
        .unwrap_or(false)
}

/// Can the Quick Switch attach here?
///
/// Broader than [`is_file_dialog`] on purpose: an app that asks "which folder?"
/// with its own dialog (WinRAR's extract-to dialog) is just as good a place to
/// pick a folder as the shell's picker. What differs is only *how* the choice
/// is applied — see [`apply_folder`].
pub fn is_quick_switch_target(hwnd: HWND) -> bool {
    let Some(shape) = dialog_shape(hwnd) else {
        return false;
    };
    if shape.is_file_dialog() || shape.is_folder_dialog() {
        return true;
    }
    // Third shape: an app that asks "which folder?" with a dialog built from
    // its own controls. 7-Zip's "copy to" / "move to" / "extract to" boxes are
    // the model case — the path field need not be an editable combo, and the
    // folder tree, when there is one at all, is not a `SysTreeView32`, so
    // neither structural rule above can see them. What they always have is a
    // text field holding a *folder*, and that folder is what the user is
    // choosing.
    folder_path_edit(hwnd).is_some()
}

/// The text field of an app dialog that holds a folder path.
///
/// 7-Zip asks "extract to where?" with an ordinary `#32770` whose only
/// machine-readable clue is the content of one text field — `D:\Work\`. The
/// content is also what keeps the rule narrow: a dialog whose text field holds
/// a *file* name (WinRAR's "Archive name", 7-Zip's "Archive") is asking a
/// different question, and gluing a folder box to it would be wrong.
fn folder_path_edit(hwnd: HWND) -> Option<HWND> {
    struct Ctx {
        hit: Option<HWND>,
    }
    extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(l as *mut Ctx) };
        if class_name(h) != "Edit" {
            return 1;
        }
        if looks_like_folder_path(&get_text_timeout(h, 150)) {
            ctx.hit = Some(h);
            return 0; // stop enumerating
        }
        1
    }
    let mut ctx = Ctx { hit: None };
    unsafe {
        EnumChildWindows(hwnd, cb, &mut ctx as *mut Ctx as LPARAM);
    }
    crate::debug_log::qs_log_verbose(&format!(
        "folder field probe {hwnd:#x} -> {:?}",
        ctx.hit
    ));
    ctx.hit
}

/// Does `text` name a folder?
///
/// `D:\`, `D:\Work\`, `\\server\share` — or a path that exists as a directory.
/// A path that neither exists nor ends with a separator is rejected, because
/// that is exactly what a *file* name looks like.
pub fn looks_like_folder_path(text: &str) -> bool {
    let t = text.trim().trim_matches('"');
    let b = t.as_bytes();
    let absolute = (b.len() >= 3
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b[2] == b'\\' || b[2] == b'/'))
        || t.starts_with("\\\\");
    if !absolute {
        return false;
    }
    if t.ends_with('\\') || t.ends_with('/') {
        return true;
    }
    std::path::Path::new(t).is_dir()
}

fn dialog_shape(hwnd: HWND) -> Option<DialogShape> {
    if class_name(hwnd) != "#32770" {
        return None;
    }
    let shape = scan_dialog_children(hwnd);
    crate::debug_log::qs_log_verbose(&format!(
        "dialog shape {hwnd:#x}: combo={} path_edit={} shell={} edit={} list={} tree={} children={}",
        shape.has_name_combo,
        shape.has_path_edit,
        shape.has_shell_view,
        shape.has_edit,
        shape.has_list_view,
        shape.has_tree_view,
        shape.children
    ));
    Some(shape)
}

/// The text field that holds a path, if any.
///
/// On a shell dialog that is the "File name" box; on an app's own folder
/// dialog (WinRAR's destination path) it is the editable combo's field. Both
/// are the place a chosen path belongs.
pub fn dialog_edit(hwnd: HWND) -> Option<HWND> {
    if !is_quick_switch_target(hwnd) {
        return None;
    }
    // Prefer an Edit nested in a ComboBox (that is the filename field).
    if let Some(combo) = find_child_deep(hwnd, "ComboBox") {
        if let Some(edit) = find_child_deep(combo, "Edit") {
            return Some(edit);
        }
    }
    // No editable combo: the field is a bare Edit. Take the one that holds a
    // path — the first Edit in a dialog can be a search box or a password
    // field, and writing a folder into one of those is worse than doing
    // nothing at all.
    folder_path_edit(hwnd).or_else(|| find_child_deep(hwnd, "Edit"))
}

/// The folder tree of an app's own folder dialog, if it has one.
pub fn dialog_tree(hwnd: HWND) -> Option<HWND> {
    find_child_deep(hwnd, "SysTreeView32")
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

/// Read a control's text without risking a hang.
///
/// `WM_GETTEXT` is a synchronous cross-process call: sent to a process that is
/// not pumping messages it blocks the caller — and this runs on the foreground
/// event queue, where blocking means every later dialog is missed.
fn get_text_timeout(hwnd: HWND, timeout_ms: u32) -> String {
    let mut buf = [0u16; 1024];
    let mut out: usize = 0;
    let ok = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_GETTEXT,
            buf.len(),
            buf.as_mut_ptr() as LPARAM,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            timeout_ms,
            &mut out,
        )
    };
    if ok == 0 {
        return String::new();
    }
    from_wide(&buf)
}

/// Best-effort guess of the folder currently shown in a file dialog.
extern "system" fn scan_address_text(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lparam as *mut Option<String>) };
    if class_name(hwnd) != "ToolbarWindow32" {
        return 1;
    }
    let t = get_text(hwnd);
    // The band's text is "地址: C:\Windows" — the path starts after the last
    // space that precedes the drive colon.
    if let Some(idx) = t.find(":\\") {
        let start = t[..idx].rfind(|c| c == ' ').map(|i| i + 1).unwrap_or(0);
        let candidate = &t[start..];
        if std::path::Path::new(candidate).is_dir() {
            *out = Some(candidate.to_string());
            return 0; // stop enumerating
        }
    }
    1
}

pub fn dialog_current_folder(hwnd: HWND) -> Option<String> {
    // The address band exposes the path through a toolbar's text. There are
    // several toolbars (navigation buttons, up band, address band, address
    // segments) and only one of them holds a path, so every one has to be
    // checked — taking the first ToolbarWindow32 found "导航按钮" and made this
    // function answer None for every real file dialog.
    let mut found: Option<String> = None;
    unsafe {
        EnumChildWindows(
            hwnd,
            scan_address_text,
            &mut found as *mut Option<String> as LPARAM,
        );
    }
    if found.is_some() {
        return found;
    }
    // Fallback: the path field may already hold a full path. A file dialog's
    // name box holds a *file*, so its folder is the parent; an app's folder
    // dialog holds the folder itself (WinRAR's destination path).
    if let Some(edit) = dialog_edit(hwnd) {
        let t = get_text(edit);
        if t.contains(":\\") {
            let p = std::path::PathBuf::from(&t);
            if p.is_dir() {
                return Some(t);
            }
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
    let mut ctx = EditBand {
        dlg_top: r.top,
        want_top,
        hit: None,
        hit_pos: 0,
    };
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
const GA_ROOT: u32 = 2;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;

/// Two paths are the same folder?
fn same_folder(a: &str, b: &str) -> bool {
    fn norm(s: &str) -> String {
        s.trim_end_matches(['\\', '/'])
            .replace('/', "\\")
            .to_lowercase()
    }
    !a.is_empty() && norm(a) == norm(b)
}

/// The address band's text field.
///
/// It exists **before** Ctrl+L — the shell creates it up front and merely keeps
/// it hidden until the band enters edit mode — and its ancestor chain always
/// contains `Address Band Root`. The file name box is a direct child of the
/// dialog, so it can never be mistaken for this one. That matters: Enter in the
/// *name* box confirms the dialog instead of navigating.
fn address_band_edit(dialog: HWND) -> Option<HWND> {
    struct Ctx {
        dialog: HWND,
        hit: Option<HWND>,
    }
    extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(lparam as *mut Ctx) };
        if class_name(hwnd) != "Edit" {
            return 1;
        }
        let mut cur = hwnd;
        for _ in 0..8 {
            cur = unsafe { GetParent(cur) };
            if cur == 0 || cur == ctx.dialog {
                break;
            }
            if class_name(cur) == "Address Band Root" {
                ctx.hit = Some(hwnd);
                return 0; // found it: stop enumerating
            }
        }
        1
    }
    let mut ctx = Ctx { dialog, hit: None };
    unsafe {
        EnumChildWindows(dialog, cb, &mut ctx as *mut Ctx as LPARAM);
    }
    ctx.hit
}

/// Navigate by writing into the address band and posting Enter **to that
/// edit**, without entering edit mode.
///
/// This is what makes a jump invisible: no address bar appears, the breadcrumb
/// stays a breadcrumb, and the dialog never has to take the foreground — so
/// there is no focus fight with the Quick Switch box either. Measured on both
/// the classic comdlg32 dialog and the DirectUI `IFileDialog`: the shell
/// accepts the path and switches folders.
///
/// The shell loads the folder asynchronously, so the result is confirmed
/// against the dialog's own address band. Without that check a failed jump
/// would report success and the caller would never retry.
fn jump_via_address_band(dialog: HWND, path: &str) -> bool {
    let Some(edit) = address_band_edit(dialog) else {
        return false;
    };
    set_text(edit, &path.replace('/', "\\"));
    unsafe {
        SendMessageW(edit, WM_KEYDOWN, VK_RETURN as WPARAM, 0);
        SendMessageW(edit, WM_KEYUP, VK_RETURN as WPARAM, 0);
    }
    // The band updates when the shell has actually switched, which takes a few
    // hundred milliseconds on a cold folder; poll rather than guess a delay.
    for _ in 0..25 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        if dialog_closing(dialog) {
            return false;
        }
        if let Some(cur) = dialog_current_folder(dialog) {
            if same_folder(&cur, path) {
                crate::debug_log::qs_log(&format!("navigate: {path} (address band, no Ctrl+L)"));
                return true;
            }
        }
    }
    false
}

/// True when the dialog is on its way out.
///
/// `IsWindow` alone answers "yes" for another few hundred milliseconds after
/// the user has cancelled: measured on a common dialog, `IsWindowVisible` drops
/// 100 ms after clicking Cancel and 290 ms after Esc, while `IsWindow` keeps
/// returning true until 600 / 980 ms. That gap is exactly the window in which a
/// jump used to fire keystrokes at a dialog that no longer existed.
pub fn dialog_closing(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 }
}

/// True when `fg` is the dialog itself or one of its own child windows.
///
/// Accepting *any* window of the dialog's process was too loose: when the user
/// closes the dialog, the foreground falls back to the application's main
/// window, which shares that process. Measured on Notepad3 — the jump then sent
/// Ctrl+L into the editor and looked for an address bar on a destroyed window.
/// `GetAncestor(GA_ROOT)` walks a child up to its top-level window, so the
/// legitimate "foreground is a child of the dialog" case still passes while a
/// sibling window of the same process no longer does.
fn is_dialog_or_child(fg: HWND, dialog: HWND) -> bool {
    if fg == dialog {
        return true;
    }
    if unsafe { IsWindow(fg) } == 0 {
        return false;
    }
    unsafe { GetAncestor(fg, GA_ROOT) == dialog }
}

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
    // Fast, invisible path: the address band's edit is already there, so the
    // folder can be typed into it without ever showing the address bar.
    if jump_via_address_band(dialog, path) {
        return true;
    }
    if dialog_closing(dialog) {
        crate::debug_log::qs_log("navigate: dialog closed before the jump");
        return false;
    }
    // Slow path, kept for dialogs whose address band does not exist (older
    // shells, custom templates): Ctrl+L turns the breadcrumb into an editable
    // address bar and hands it the caret.
    //
    // The box itself has the focus when the user clicks a folder in it, so the
    // dialog has to be activated first — and we must *verify* that happened
    // before sending keys. Alt+D fired at the wrong window would open a menu
    // somewhere else entirely.
    let mut focused = false;
    for _ in 0..6 {
        if dialog_closing(dialog) {
            crate::debug_log::qs_log("navigate: dialog closed before the jump");
            return false;
        }
        activate(dialog);
        // Same dialog or one of its own children — nothing else will do.
        if is_dialog_or_child(foreground(), dialog) {
            focused = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    if !focused {
        // A closing dialog cannot take focus, so report the real reason
        // instead of blaming the focus transfer.
        if dialog_closing(dialog) {
            crate::debug_log::qs_log("navigate: dialog closed before the jump");
        } else {
            crate::debug_log::qs_log("navigate: dialog did not take focus, keys not sent");
        }
        return false;
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    // Focus can still move during that pause (the user clicking away, the
    // dialog closing). Sending Ctrl+L blind would drive whatever app is in
    // front now, so verify once more right before the keystroke.
    if !is_dialog_or_child(foreground(), dialog) {
        crate::debug_log::qs_log("navigate: lost focus before Ctrl+L");
        return false;
    }
    // Ctrl+L: turn the breadcrumb into an editable address bar AND give it the
    // caret. Plain Alt+D only made the edit visible while the filename box kept
    // the focus, so typing landed in the wrong field.
    let mut edit = None;
    for attempt in 0..2 {
        unsafe {
            keybd_event(VK_CONTROL, 0, 0, 0);
            keybd_event(b'L', 0, 0, 0);
            keybd_event(b'L', 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
        }
        std::thread::sleep(std::time::Duration::from_millis(130));
        // A closed dialog and a swallowed keystroke look identical from the
        // caller's side, so tell them apart here.
        if dialog_closing(dialog) {
            crate::debug_log::qs_log("navigate: dialog closed while jumping");
            return false;
        }
        edit = pick_edit(dialog, true);
        if edit.is_some() || attempt > 0 {
            break;
        }
        // The keystroke can be swallowed while the dialog is still settling
        // after activation. Retry once — Ctrl+L on an already open address bar
        // is a no-op, so the retry costs nothing when it was not needed.
        crate::debug_log::qs_log("navigate: address bar did not open, retrying");
        if !is_dialog_or_child(foreground(), dialog) {
            crate::debug_log::qs_log("navigate: lost focus before retry");
            return false;
        }
        activate(dialog);
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    let Some(edit) = edit else {
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

/// Apply a chosen folder to whatever kind of dialog is in front.
///
/// The three kinds need different mechanics. A shell dialog has an address bar,
/// and typing a folder there only navigates. An app's own folder dialog has a
/// folder tree instead, and the tree is what the user is looking at — so the
/// selection is moved there and the dialog works out the path itself. Only
/// when the tree cannot express the path (a hidden or system folder has no
/// node at all) does the path get written into the field, and even then
/// nothing is pressed: Enter in WinRAR's extract dialog starts extracting.
///
/// A dialog recognised only by its folder field (7-Zip) has no standard tree
/// to drive and no address bar to type into, so the field is the whole
/// mechanism there — and the field is authoritative for that app.
pub fn apply_folder(dialog: HWND, path: &str) -> bool {
    let Some(shape) = dialog_shape(dialog) else {
        return false;
    };
    if shape.is_file_dialog() {
        return navigate_to_folder(dialog, path);
    }
    if shape.is_folder_dialog() {
        if super::tree::select_folder(dialog, path) {
            activate(dialog);
            return true;
        }
        return fill_path_field(dialog, path);
    }
    if folder_path_edit(dialog).is_some() {
        return fill_path_field(dialog, path);
    }
    false
}

/// Write `path` into an app dialog's path field, without pressing anything.
fn fill_path_field(dialog: HWND, path: &str) -> bool {
    let Some(edit) = dialog_edit(dialog) else {
        return false;
    };
    set_text(edit, path);
    // Select it so the user can simply overwrite, or accept it as it is.
    unsafe {
        SendMessageW(edit, 0x00B1 /* EM_SETSEL */, 0, -1);
    }
    activate(dialog);
    crate::debug_log::qs_log(&format!("fill path: dialog {dialog:#x} -> {path}"));
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
        (
            (c & 0xFF) as u8,
            ((c >> 8) & 0xFF) as u8,
            ((c >> 16) & 0xFF) as u8,
        )
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
    use super::super::{EnumWindows, GetWindowTextW, IsWindowVisible, PostMessageW, BOOL, HWND, LPARAM};
    use super::DialogShape;
    use super::{class_name, is_file_dialog, is_quick_switch_target, looks_like_folder_path, same_folder};

    #[test]
    fn tells_a_folder_field_from_a_file_field() {
        // 7-Zip's "extract to" field holds a folder; its "archive" field holds
        // a file. Only the first one means "pick a folder here" — attaching a
        // folder box to the second would overwrite the archive name.
        assert!(looks_like_folder_path("D:\\"));
        assert!(looks_like_folder_path("D:\\Work\\"));
        assert!(looks_like_folder_path("C:/Windows/"));
        assert!(looks_like_folder_path("  \"C:\\Windows\\\\\"  "));
        // No trailing separator: the path has to exist as a folder.
        assert!(looks_like_folder_path("C:\\Windows"));
        assert!(!looks_like_folder_path("D:\\Work\\archive.rar"));
        assert!(!looks_like_folder_path("C:\\no-such-folder-here"));
        assert!(!looks_like_folder_path("archive.rar"));
        assert!(!looks_like_folder_path("D:"));
        assert!(!looks_like_folder_path(""));
        // A search box holds neither.
        assert!(!looks_like_folder_path("report"));
    }

    #[test]
    fn compares_folder_paths_the_way_the_shell_does() {
        // The address band answers with backslashes and no trailing separator;
        // the caller may pass either. Case never matters on Windows.
        assert!(same_folder("C:\\Windows", "c:\\windows"));
        assert!(same_folder("C:\\Windows\\", "C:/Windows"));
        assert!(same_folder("C:/Users/x/", "C:\\Users\\x"));
        assert!(!same_folder("C:\\Windows", "C:\\Windows\\System32"));
        // An empty answer must never count as "already there".
        assert!(!same_folder("", ""));
    }

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
            let title: Vec<u16> = "KestrelOpen"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
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
            MessageBoxA(0, b"hello\0".as_ptr(), b"KestrelMsg\0".as_ptr(), 0);
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
            println!(
                "IDM window {hwnd:#x} title={title:?} shape={:?}",
                super::scan_dialog_children(*hwnd)
            );
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
    ///
    /// `"ComboBox>Edit"` spells out a parent/child pair a flat class list
    /// cannot express: an **editable** combo, whose text field is a real `Edit`
    /// child window. That relation is the "you can type a path here" marker.
    fn shape_of(classes: &[&str]) -> DialogShape {
        let mut s = DialogShape::default();
        for c in classes {
            match c.strip_suffix(">Edit") {
                Some(parent) => {
                    s.add_class(parent);
                    s.add_class("Edit");
                    s.has_path_edit = true;
                }
                None => s.add_class(c),
            }
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
        "ComboBoxEx32>Edit",
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

    /// `IFileOpenDialog` with `FOS_PICKFOLDERS` (the "choose a folder" dialog
    /// Windows shows today). Captured with tools/probe-dialogs.rs — it is a
    /// full shell dialog, so the file-dialog rule already covered it.
    const MODERN_FOLDER_PICKER: &[&str] = &[
        "DUIViewWndClassName",
        "DirectUIHWND",
        "CtrlNotifySink",
        "NamespaceTreeControl",
        "SysTreeView32",
        "SHELLDLL_DefView",
        "ScrollBar",
        "Static",
        "ComboBoxEx32>Edit",
        "ComboBox",
        "Button",
        "Button",
        "ListBox",
        "WorkerW",
        "ReBarWindow32",
        "TravelBand",
        "ToolbarWindow32",
        "Address Band Root",
        "Breadcrumb Parent",
        "msctls_progress32",
    ];

    /// WinRAR 7.23's "extract path and options" dialog. An ordinary app dialog:
    /// a destination path in an editable combo and a folder tree, but no shell
    /// view and no address bar. This is what the Quick Switch used to skip.
    const WINRAR_EXTRACT: &[&str] = &[
        "#32770",
        "Static",
        "ComboBox>Edit",
        "SysTreeView32",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "SysTabControl32",
    ];

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
        // The legacy SHBrowseForFolder tree has nothing to put a path into:
        // no combo, no address bar, and its selection is not readable. The
        // Quick Switch has no business attaching to it.
        let s = shape_of(FOLDER_PICKER);
        assert!(!s.is_file_dialog());
        assert!(!s.is_folder_dialog());
    }

    #[test]
    fn recognises_the_modern_folder_picker() {
        // IFileOpenDialog + FOS_PICKFOLDERS is a shell dialog: shell view and
        // a "Folder:" combo, so the file-dialog rule already covers it.
        let s = shape_of(MODERN_FOLDER_PICKER);
        assert!(s.has_shell_view && s.has_name_combo);
        assert!(s.is_file_dialog());
    }

    #[test]
    fn recognises_an_apps_folder_dialog() {
        // WinRAR's extract-to dialog: an editable path field and a folder
        // tree, no shell view. It picks a folder, so the Quick Switch belongs
        // there even though it is not a shell dialog.
        let s = shape_of(WINRAR_EXTRACT);
        assert!(s.has_path_edit && s.has_tree_view);
        assert!(!s.is_file_dialog(), "no address bar to navigate");
        assert!(s.is_folder_dialog());
    }

    #[test]
    fn a_drop_down_list_is_not_a_path_field() {
        // The editable-combo marker is what separates WinRAR's dialog from any
        // dialog that merely has a picker and a tree: a CBS_DROPDOWNLIST keeps
        // its text inside the control and has no Edit child.
        let mut s = shape_of(WINRAR_EXTRACT);
        s.has_path_edit = false;
        assert!(!s.is_folder_dialog());
    }

    #[test]
    fn rejects_idms_main_window() {
        // The regression this rule was written for: an edit box plus a list
        // view is not enough — IDM's dialog has both.
        let s = shape_of(IDM_MAIN);
        assert!(s.has_edit && s.has_list_view, "IDM shape changed");
        assert!(!s.has_name_combo && !s.has_shell_view);
        assert!(!s.is_file_dialog());
        // And it must stay out under the folder-dialog rule too: no editable
        // combo, even though it does have a tree view.
        assert!(s.has_tree_view && !s.has_path_edit);
        assert!(!s.is_folder_dialog());
    }

    #[test]
    fn rejects_message_boxes() {
        assert!(!shape_of(MESSAGE_BOX).is_file_dialog());
    }

    // ---- a dialog built at runtime ---------------------------------------
    //
    // 7-Zip's extract box cannot be opened from a test — it needs the app, an
    // archive and a click — so the shape it has is built here instead: an
    // ordinary `#32770` holding a bare `Edit` with a folder in it and a folder
    // tree beside it. That is precisely what the structural rules cannot see
    // (the field is not an editable combo), and what the content rule must.

    #[link(name = "user32")]
    extern "system" {
        fn CreateDialogIndirectParamW(
            hinstance: isize,
            template: *const u8,
            parent: HWND,
            dlgproc: isize,
            init: isize,
        ) -> HWND;
        fn DestroyWindow(hwnd: HWND) -> BOOL;
    }

    #[link(name = "comctl32")]
    extern "system" {
        fn InitCommonControlsEx(icce: *const INITCOMMONCONTROLSEX) -> BOOL;
    }

    #[repr(C)]
    struct INITCOMMONCONTROLSEX {
        size: u32,
        icc: u32,
    }

    /// `ICC_TREEVIEW_CLASSES` — without it `SysTreeView32` does not exist yet.
    const ICC_TREEVIEW_CLASSES: u32 = 0x0000_0002;

    fn align4(v: &mut Vec<u8>) {
        while v.len() % 4 != 0 {
            v.push(0);
        }
    }

    fn w16(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    fn w32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    fn wstr(v: &mut Vec<u8>, s: &str) {
        for u in s.encode_utf16() {
            w16(v, u);
        }
        w16(v, 0);
    }

    /// Append one control to a dialog template.
    fn dialog_item(
        v: &mut Vec<u8>,
        class: &str,
        text: &str,
        style: u32,
        id: u16,
        rect: (i16, i16, i16, i16),
    ) {
        align4(v);
        w32(v, style);
        w32(v, 0); // extended style
        w16(v, rect.0 as u16);
        w16(v, rect.1 as u16);
        w16(v, rect.2 as u16);
        w16(v, rect.3 as u16);
        w16(v, id);
        wstr(v, class);
        wstr(v, text);
        w16(v, 0); // no creation data
    }

    /// Build a hidden `#32770` whose path field holds `path_text`, and hand
    /// its handle to `f`.
    fn with_app_dialog(path_text: &str, f: impl FnOnce(HWND)) {
        unsafe {
            let icc = INITCOMMONCONTROLSEX {
                size: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                icc: ICC_TREEVIEW_CLASSES,
            };
            InitCommonControlsEx(&icc);
        }
        let mut t: Vec<u8> = Vec::new();
        // WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME
        w32(&mut t, 0x80C8_0080);
        w32(&mut t, 0);
        w16(&mut t, 2); // two controls
        w16(&mut t, 0); // x
        w16(&mut t, 0); // y
        w16(&mut t, 220); // cx
        w16(&mut t, 140); // cy
        w16(&mut t, 0); // no menu
        w16(&mut t, 0); // default class
        w16(&mut t, 0); // empty title
        // WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP
        dialog_item(&mut t, "Edit", path_text, 0x5080_0080, 1000, (10, 10, 200, 14));
        // WS_CHILD | WS_VISIBLE | WS_BORDER | TVS_HASBUTTONS | TVS_LINESATROOT
        dialog_item(
            &mut t,
            "SysTreeView32",
            "",
            0x5080_0005,
            1001,
            (10, 30, 200, 100),
        );
        let hwnd = unsafe { CreateDialogIndirectParamW(0, t.as_ptr(), 0, 0, 0) };
        assert_ne!(hwnd, 0, "could not build the probe dialog");
        f(hwnd);
        unsafe { DestroyWindow(hwnd) };
    }

    #[test]
    fn recognises_an_app_dialog_by_its_folder_field() {
        with_app_dialog("C:\\Windows", |hwnd| {
            let shape = super::scan_dialog_children(hwnd);
            println!("probe dialog shape: {shape:?}");
            // The structural rules cannot see this dialog: the path field is a
            // bare `Edit`, not the text half of an editable combo.
            assert!(shape.has_edit, "the probe dialog lost its text field");
            assert!(!shape.is_file_dialog());
            assert!(!shape.is_folder_dialog());
            // It is still a place to pick a folder, and that is what counts.
            assert!(
                is_quick_switch_target(hwnd),
                "a folder field was not recognised"
            );
        });
    }

    #[test]
    fn an_app_dialog_showing_a_file_is_not_a_folder_target() {
        // The same dialog with a *file* in the field — WinRAR's "Archive
        // name", 7-Zip's "Archive". Writing a folder there would destroy what
        // the user typed.
        with_app_dialog("C:\\Windows\\notepad.exe", |hwnd| {
            assert!(
                !is_quick_switch_target(hwnd),
                "a file name was treated as a folder field"
            );
        });
    }

    #[test]
    fn choosing_a_folder_writes_it_into_an_app_dialog() {
        with_app_dialog("C:\\Windows", |hwnd| {
            let path = "C:\\Windows\\System32";
            assert!(super::apply_folder(hwnd, path), "the folder was not applied");
            let edit = super::dialog_edit(hwnd).expect("no path field");
            assert_eq!(super::get_text(edit), path);
        });
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
