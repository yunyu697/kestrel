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
    /// How many text inputs / pickers the dialog has. A folder picker asks
    /// one question; a form (IDM's "download file information", a login box,
    /// an options page) asks many, and one of them merely happens to show a
    /// folder.
    pub edits: usize,
    pub combos: usize,
    pub children: usize,
}

impl DialogShape {
    /// Fold one child class name into the fingerprint.
    fn add_class(&mut self, class: &str) {
        if class.eq_ignore_ascii_case("ComboBoxEx32") || class.eq_ignore_ascii_case("ComboBox") {
            self.has_name_combo = true;
            self.combos += 1;
        } else if class.eq_ignore_ascii_case("Edit") {
            self.has_edit = true;
            self.edits += 1;
        } else if class.eq_ignore_ascii_case("SHELLDLL_DefView")
            || class.eq_ignore_ascii_case("DUIViewWndClassName")
        {
            self.has_shell_view = true;
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

    /// Too many input fields to be "pick a folder".
    ///
    /// A download manager's "file information" dialog also shows a folder
    /// (where the file will be saved), but it is a form: URL, file name,
    /// credentials, comments, several drop-downs. IDM 6.43's dialog has nine
    /// edits and four combos; 7-Zip's extract box has one edit. The threshold
    /// is deliberately loose — only clearly form-shaped windows are rejected,
    /// so a real folder picker with an extra option or two still works.
    pub fn is_form(&self) -> bool {
        self.edits > 4 || self.combos > 2
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

/// Everything the Quick Switch decision is allowed to look at, gathered in one
/// walk of the dialog's child tree.
///
/// Plain data on purpose: [`qs_decision`] is a pure function of this struct,
/// so an uninstaller, a download form or a 7-Zip extract box can be tested
/// without creating a window.
#[derive(Debug, Default)]
struct DialogFacts {
    shape: DialogShape,
    /// Bare executable name, lower-cased.
    exe: String,
    /// Window title, lower-cased.
    title: String,
    /// Visible button captions, lower-cased.
    buttons: Vec<String>,
    /// A writable edit that holds a folder path.
    has_folder_edit: bool,
    /// Some edit holds a link: this dialog is *about* a download.
    saw_url: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QsDecision {
    Attach(&'static str),
    Skip(&'static str),
}

/// Installers, uninstallers and their wizards.
///
/// They are the biggest source of stray Quick Switches: a wizard page shows
/// the install folder, often beside a "Browse" button and a feature tree, so
/// it can satisfy every weak signal the old content rule had. What it never
/// does is offer to *navigate to* a folder. Reject the whole category instead
/// of adding one more exception per app.
///
/// This gates only an app's own dialogs. If an installer opens a real system
/// folder picker, rule 1 in [`qs_decision`] still attaches there.
fn is_setup_or_uninstall(f: &DialogFacts) -> bool {
    const EXE: &[&str] = &[
        "unins",
        "uninstall",
        "setup",
        "install",
        "msiexec",
        "au_.exe",
        "installshield",
        "_iu",
    ];
    const TITLE: &[&str] = &[
        "卸载",
        "解除安装",
        "安装",
        "向导",
        "维护",
        "修复",
        "更改程序",
        "uninstall",
        "uninstal",
        "setup",
        "install",
        "wizard",
        "maintenance",
        "repair",
        "modify",
        "remove programs",
        "programs and features",
    ];
    const BUTTON: &[&str] = &[
        "卸载",
        "解除安装",
        "安装",
        "下一步",
        "完成",
        "修复",
        "更改",
        "uninstall",
        "install",
        "next",
        "finish",
        "repair",
        "remove",
        "modify",
    ];
    EXE.iter().any(|k| f.exe.contains(k))
        || TITLE.iter().any(|k| f.title.contains(k))
        || f.buttons
            .iter()
            .any(|b| BUTTON.iter().any(|k| b.contains(k)))
}

/// Positive evidence that this dialog asks the user to choose a folder.
///
/// The old rule was "no reason *not* to": any `#32770` with a folder-looking
/// edit got a Quick Switch, which is why it appeared on uninstallers,
/// settings pages and download-information forms. An app dialog now has to
/// say what it is for — in its title, through a browse button next to a
/// writable path field, or by belonging to an app known to ask for a
/// destination folder.
fn has_folder_intent(f: &DialogFacts) -> bool {
    const TITLE: &[&str] = &[
        "解压",
        "提取",
        "复制到",
        "移动到",
        "选择文件夹",
        "浏览文件夹",
        "目标文件夹",
        "目标路径",
        "输出目录",
        "保存位置",
        // Deliberately no bare "文件夹" / "folder": Explorer's own
        // "文件夹选项 / Folder Options" would otherwise qualify as a picker.
        "extract",
        "copy to",
        "move to",
        "select folder",
        "choose folder",
        "browse for folder",
        "destination",
        "output folder",
        "unpack",
    ];
    if TITLE.iter().any(|k| f.title.contains(k)) {
        return true;
    }

    const BROWSE: &[&str] = &[
        "...",
        "…",
        "浏览",
        "browse",
        "choose",
        "选择文件夹",
        "select folder",
    ];
    if f.has_folder_edit
        && f.buttons
            .iter()
            .any(|b| BROWSE.iter().any(|k| b.contains(k)))
    {
        return true;
    }

    // Archive managers and file managers ask "which folder?" with their own
    // controls, and some of them leave the title empty or generic.
    const APPS: &[&str] = &[
        "7zfm",
        "7zg",
        "7z.exe",
        "winrar",
        "unrar",
        "bandizip",
        "peazip",
        "winzip",
        "nanazip",
        "360zip",
        "haozip",
        "totalcmd",
        "freecommander",
        "dopus",
        "xyplorer",
        "explorer++",
        "q-dir",
    ];
    APPS.iter().any(|k| f.exe.contains(k))
}

/// The whole rule, in one place.
fn qs_decision(f: &DialogFacts) -> QsDecision {
    use QsDecision::*;

    // 1. The shell's own Open/Save/folder picker. Two structural markers, no
    //    text guessing and no exceptions: this is what Quick Switch is for.
    if f.shape.is_file_dialog() {
        return Attach("shell file dialog");
    }
    // 2. A wizard is never a folder picker, whatever it happens to display.
    if is_setup_or_uninstall(f) {
        return Skip("installer / uninstaller / wizard");
    }
    // 3. An app's own dialog needs to *say* it is choosing a folder.
    if !has_folder_intent(f) {
        return Skip("no folder-picking intent");
    }
    // 4. Cheap guards that also make the reason visible in the debug log.
    if f.shape.is_form() {
        return Skip("form, not a folder picker");
    }
    if f.saw_url {
        return Skip("download link, not a folder picker");
    }
    // 5. Either WinRAR's editable-combo + tree shape, or 7-Zip's bare path
    //    field. In both cases there is a writable folder field to fill.
    if f.shape.is_folder_dialog() || f.has_folder_edit {
        return Attach("app folder dialog");
    }
    Skip("no writable folder field")
}

/// Read the parts of a dialog that the weak rules need. Called only after the
/// fast structural test has failed, so the common case never pays for a
/// cross-process text read.
fn scan_dialog_facts(hwnd: HWND, shape: DialogShape) -> DialogFacts {
    const GWL_STYLE: i32 = -16;
    const ES_READONLY: isize = 0x0800;

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    let exe = process_path(pid)
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .to_lowercase();

    let mut f = DialogFacts {
        shape,
        exe,
        title: hwnd_string(hwnd).to_lowercase(),
        ..Default::default()
    };

    // A wizard can often be rejected from its process and title alone; no
    // reason to touch a single control in another process in that case.
    if is_setup_or_uninstall(&f) {
        return f;
    }

    let mut stack: Vec<HWND> = Vec::new();
    unsafe {
        let mut c = GetWindow(hwnd, GW_CHILD);
        while c != 0 {
            stack.push(c);
            c = GetWindow(c, GW_HWNDNEXT);
        }
    }
    while let Some(h) = stack.pop() {
        let class = class_name(h);
        if class.eq_ignore_ascii_case("Edit") {
            // A read-only field only *displays* a folder (the install location
            // on an uninstaller page, the save-to path on a download form).
            // There is nothing for Quick Switch to write into it.
            let readonly = unsafe { GetWindowLongPtrW(h, GWL_STYLE) } & ES_READONLY != 0;
            if !readonly {
                let text = get_text_timeout(h, 150);
                if is_url_like(&text) {
                    f.saw_url = true;
                } else if !f.has_folder_edit && looks_like_folder_path(&text) {
                    f.has_folder_edit = true;
                }
            }
        } else if class.eq_ignore_ascii_case("Button") && f.buttons.len() < 24 {
            let t = get_text_timeout(h, 100).trim().to_lowercase();
            if !t.is_empty() {
                f.buttons.push(t);
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
    f
}

/// Can the Quick Switch attach here?
pub fn is_quick_switch_target(hwnd: HWND) -> bool {
    let Some(shape) = dialog_shape(hwnd) else {
        return false;
    };
    // Fast path: the common case needs no text reads at all.
    if shape.is_file_dialog() {
        return true;
    }
    let facts = scan_dialog_facts(hwnd, shape);
    let decision = qs_decision(&facts);
    crate::debug_log::qs_log_verbose(&format!(
        "quick switch {hwnd:#x}: exe={} title={:?} edits={} combos={} decision={decision:?}",
        facts.exe, facts.title, facts.shape.edits, facts.shape.combos
    ));
    matches!(decision, QsDecision::Attach(_))
}

/// Ctrl+G is an explicit request, so it gets a wider rule than the automatic
/// attach.
///
/// The strict rule above deliberately ignores apps it cannot name: a false
/// positive is far more annoying than a missing box. But if the user *asks*
/// for Quick Switch on a dialog we failed to recognise, refusing would make
/// the feature useless there. Accept any `#32770` with a writable folder field
/// — while still refusing wizards, forms, download links and read-only fields,
/// because even an explicit keypress has nothing to write into those.
pub fn is_manual_quick_switch_target(hwnd: HWND) -> bool {
    let Some(shape) = dialog_shape(hwnd) else {
        return false;
    };
    if shape.is_file_dialog() {
        return true;
    }
    let facts = scan_dialog_facts(hwnd, shape);
    if is_setup_or_uninstall(&facts) || facts.shape.is_form() || facts.saw_url {
        return false;
    }
    facts.has_folder_edit || facts.shape.is_folder_dialog()
}

/// The text field of an app dialog that holds a folder path.
///
/// Used to *locate* the field once the dialog has been accepted. The decision
/// itself lives in [`qs_decision`]; this helper deliberately applies the same
/// read-only rule so a rejected dialog can never be written into by accident.
fn folder_path_edit(hwnd: HWND) -> Option<HWND> {
    const GWL_STYLE: i32 = -16;
    const ES_READONLY: isize = 0x0800;

    struct Ctx {
        hit: Option<HWND>,
        saw_url: bool,
    }
    extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(l as *mut Ctx) };
        if class_name(h) != "Edit" {
            return 1;
        }
        if unsafe { GetWindowLongPtrW(h, GWL_STYLE) } & ES_READONLY != 0 {
            return 1;
        }
        let text = get_text_timeout(h, 150);
        if is_url_like(&text) {
            ctx.saw_url = true;
            return 0;
        }
        if ctx.hit.is_none() && looks_like_folder_path(&text) {
            ctx.hit = Some(h);
        }
        1
    }
    let mut ctx = Ctx {
        hit: None,
        saw_url: false,
    };
    unsafe {
        EnumChildWindows(hwnd, cb, &mut ctx as *mut Ctx as LPARAM);
    }
    crate::debug_log::qs_log_verbose(&format!(
        "folder field probe {hwnd:#x} -> {:?} (url={})",
        ctx.hit, ctx.saw_url
    ));
    if ctx.saw_url {
        None
    } else {
        ctx.hit
    }
}

/// A field holding a link, not a path. Deliberately narrow: a Windows path
/// contains `:\` but never `://`.
fn is_url_like(text: &str) -> bool {
    let t = text.trim();
    if t.len() > 2048 {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.starts_with("magnet:")
        || lower.starts_with("www.")
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
    // Manual Ctrl+G uses a wider rule than the automatic attach; a box that
    // is already on screen must still be able to reach its path field.
    if !is_manual_quick_switch_target(hwnd) {
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
    use super::super::{
        EnumWindows, GetWindowTextW, IsWindowVisible, PostMessageW, BOOL, HWND, LPARAM,
    };
    use super::{
        class_name, is_file_dialog, is_manual_quick_switch_target, is_quick_switch_target,
        looks_like_folder_path, qs_decision, same_folder, DialogFacts, DialogShape, QsDecision,
    };

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

    /// IDM 6.43's "下载文件信息" dialog, captured live on 2026-11-04. It is a
    /// `#32770` form: 9 `Edit`s (3 of them inside a combo), 4 combos, 8
    /// buttons and 6 labels. One edit holds the folder the download will be
    /// saved to — the content rule used to mistake that for a folder picker.
    const IDM_DOWNLOAD_INFO: &[&str] = &[
        "Edit",
        "Edit",
        "Edit",
        "Edit",
        "Edit",
        "Edit",
        "ComboBox>Edit",
        "ComboBox>Edit",
        "ComboBox>Edit",
        "ComboBox",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Button",
        "Static",
        "Static",
        "Static",
        "Static",
        "Static",
        "Static",
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

    /// A decision input with one writable folder field, no window needed.
    fn facts(exe: &str, title: &str, buttons: &[&str]) -> DialogFacts {
        let mut f = DialogFacts {
            exe: exe.to_lowercase(),
            title: title.to_lowercase(),
            buttons: buttons.iter().map(|b| b.to_lowercase()).collect(),
            has_folder_edit: true,
            ..Default::default()
        };
        f.shape.has_edit = true;
        f.shape.edits = 1;
        f
    }

    fn attaches(exe: &str, title: &str, buttons: &[&str]) -> bool {
        matches!(
            qs_decision(&facts(exe, title, buttons)),
            QsDecision::Attach(_)
        )
    }

    #[test]
    fn uninstallers_are_never_targets() {
        // Inno Setup, NSIS, MSI and their many one-off wrappers. Some of them
        // show the install folder in an editable field next to a Browse
        // button, so every weak signal can be present at once.
        for (exe, title, buttons) in [
            ("unins000.exe", "Foo 卸载", &["卸载(&U)"][..]),
            ("Au_.exe", "Foo Uninstall", &["Uninstall"][..]),
            ("FooUninstaller.exe", "Foo", &["卸载", "取消"][..]),
            ("msiexec.exe", "Foo Setup", &["Next >", "Cancel"][..]),
            ("Foo_Setup.exe", "欢迎使用 Foo", &["下一步(&N)"][..]),
            ("setup.exe", "Foo Maintenance", &["Repair", "Remove"][..]),
            (
                "foo.exe",
                "Programs and Features",
                &["Uninstall/Change"][..],
            ),
        ] {
            assert!(
                !attaches(exe, title, buttons),
                "stray Quick Switch on {exe} / {title}"
            );
        }
    }

    #[test]
    fn ordinary_dialogs_need_evidence_of_a_folder_choice() {
        // The regression the old content rule caused: merely displaying a
        // folder path was enough. Titles like these are everywhere.
        for title in [
            "选项",
            "首选项",
            "关于 Foo",
            "下载文件信息",
            "Properties",
            "Preferences",
            "",
        ] {
            assert!(
                !attaches("someapp.exe", title, &["确定", "取消"]),
                "a folder path alone must not trigger Quick Switch: {title:?}"
            );
        }
    }

    #[test]
    fn real_folder_dialogs_keep_working() {
        assert!(attaches("7zFM.exe", "解压到", &["确定", "取消"]));
        assert!(attaches("7zG.exe", "Extract to", &["OK"]));
        assert!(attaches("WinRAR.exe", "解压路径和选项", &["确定"]));
        assert!(attaches("Bandizip.exe", "Copy to", &["OK"]));
        // An app we do not know, but its title says exactly what it is for.
        assert!(attaches("mytool.exe", "选择文件夹", &["确定"]));
        assert!(attaches("mytool.exe", "Browse for Folder", &["OK"]));
        // No title keyword, but a browse button next to a writable path.
        assert!(attaches("mytool.exe", "输出", &["...", "确定"]));
        // The archive managers also work when their title is generic.
        assert!(attaches("7zFM.exe", "", &["OK"]));
    }

    #[test]
    fn the_shell_file_dialog_wins_over_every_other_rule() {
        // An installer may open a real system folder picker; that is still a
        // file dialog and Quick Switch belongs there.
        let mut f = facts("setup.exe", "Foo Setup", &["卸载"]);
        f.shape.has_name_combo = true;
        f.shape.has_shell_view = true;
        assert!(matches!(qs_decision(&f), QsDecision::Attach(_)));
    }

    #[test]
    fn forms_links_and_readonly_fields_are_still_rejected() {
        let mut f = facts("7zFM.exe", "解压到", &["确定"]);
        f.shape.edits = 9;
        f.shape.combos = 4;
        assert!(matches!(qs_decision(&f), QsDecision::Skip(_)), "a form");

        let mut f = facts("7zFM.exe", "解压到", &["确定"]);
        f.saw_url = true;
        assert!(
            matches!(qs_decision(&f), QsDecision::Skip(_)),
            "a download link"
        );

        let mut f = facts("7zFM.exe", "解压到", &["确定"]);
        f.has_folder_edit = false;
        assert!(
            matches!(qs_decision(&f), QsDecision::Skip(_)),
            "no writable field"
        );
    }

    #[test]
    fn idms_download_dialog_is_a_form_not_a_folder_picker() {
        let s = shape_of(IDM_DOWNLOAD_INFO);
        assert_eq!(s.edits, 9);
        assert_eq!(s.combos, 4);
        assert!(
            s.is_form(),
            "a nine-field form must not reach the content rule"
        );
        assert!(!s.is_file_dialog(), "no shell view");
        assert!(!s.is_folder_dialog(), "no folder tree");
    }

    #[test]
    fn real_folder_dialogs_are_not_mistaken_for_forms() {
        assert!(!shape_of(OPEN_DIALOG).is_form());
        assert!(!shape_of(SAVE_DIALOG).is_form());
        assert!(!shape_of(WINRAR_EXTRACT).is_form());
        assert!(!shape_of(MODERN_FOLDER_PICKER).is_form());
        assert!(!shape_of(IDM_MAIN).is_form());
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
        wstr(&mut t, "解压到"); // 7-Zip's extract-to box: an explicit folder intent
                                // WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP
        dialog_item(
            &mut t,
            "Edit",
            path_text,
            0x5080_0080,
            1000,
            (10, 10, 200, 14),
        );
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

    /// Build a hidden `#32770` with one `Edit` per entry. Lets the tests feed
    /// real control text to the content rule instead of only a class list.
    fn with_edits(texts: &[&str], f: impl FnOnce(HWND)) {
        let mut t: Vec<u8> = Vec::new();
        w32(&mut t, 0x80C8_0080); // WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME
        w32(&mut t, 0);
        w16(&mut t, texts.len() as u16);
        w16(&mut t, 0); // x
        w16(&mut t, 0); // y
        w16(&mut t, 260); // cx
        w16(&mut t, (24 + 20 * texts.len()) as u16); // cy
        w16(&mut t, 0); // no menu
        w16(&mut t, 0); // default class
        wstr(&mut t, "解压到"); // 7-Zip's extract-to box: an explicit folder intent
        for (i, text) in texts.iter().enumerate() {
            let y = 8 + 20 * i as i16;
            dialog_item(
                &mut t,
                "Edit",
                text,
                0x5080_0080, // WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP
                1000 + i as u16,
                (10, y, 230, 14),
            );
        }
        let hwnd = unsafe { CreateDialogIndirectParamW(0, t.as_ptr(), 0, 0, 0) };
        assert_ne!(hwnd, 0, "could not build the probe dialog");
        f(hwnd);
        unsafe { DestroyWindow(hwnd) };
    }

    /// Build a hidden `#32770` with a title, push buttons and edit fields
    /// (each optionally read-only). Lets the runtime probe be exercised on
    /// realistic uninstaller and archive-manager dialogs.
    fn with_dialog(title: &str, buttons: &[&str], edits: &[(&str, bool)], f: impl FnOnce(HWND)) {
        let mut t: Vec<u8> = Vec::new();
        w32(&mut t, 0x80C8_0080); // WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME
        w32(&mut t, 0);
        w16(&mut t, (buttons.len() + edits.len()) as u16);
        w16(&mut t, 0);
        w16(&mut t, 0);
        w16(&mut t, 260);
        w16(&mut t, (40 + 20 * (buttons.len() + edits.len())) as u16);
        w16(&mut t, 0); // no menu
        w16(&mut t, 0); // default class
        wstr(&mut t, title);
        let mut y = 8i16;
        for (text, readonly) in edits {
            let style = 0x5080_0080 | if *readonly { 0x0800 } else { 0 };
            dialog_item(
                &mut t,
                "Edit",
                text,
                style,
                1000 + y as u16,
                (10, y, 230, 14),
            );
            y += 20;
        }
        for text in buttons {
            // WS_CHILD | WS_VISIBLE | WS_TABSTOP, BS_PUSHBUTTON (0)
            dialog_item(
                &mut t,
                "Button",
                text,
                0x5001_0000,
                2000 + y as u16,
                (10, y, 100, 16),
            );
            y += 20;
        }
        let hwnd = unsafe { CreateDialogIndirectParamW(0, t.as_ptr(), 0, 0, 0) };
        assert_ne!(hwnd, 0, "could not build the probe dialog");
        f(hwnd);
        unsafe { DestroyWindow(hwnd) };
    }

    #[test]
    fn an_uninstaller_page_never_gets_a_quick_switch() {
        // The regression the user hit: an uninstall wizard shows the install
        // folder in a field, with a browse button and buttons labelled 卸载.
        with_dialog(
            "Foo 卸载",
            &["卸载(&U)", "取消"],
            &[(r"C:\Program Files\Foo", false)],
            |hwnd| {
                assert!(!super::scan_dialog_children(hwnd).is_file_dialog());
                assert!(!is_quick_switch_target(hwnd));
            },
        );
    }

    #[test]
    fn a_generic_dialog_showing_a_folder_gets_nothing() {
        with_dialog(
            "选项",
            &["确定", "取消"],
            &[(r"C:\Windows", false)],
            |hwnd| {
                assert!(
                    !is_quick_switch_target(hwnd),
                    "a folder path alone is not a reason to attach"
                );
            },
        );
    }

    #[test]
    fn an_extract_dialog_with_a_browse_button_still_works() {
        with_dialog(
            "解压到",
            &["...", "确定"],
            &[(r"C:\Windows", false)],
            |hwnd| {
                assert!(
                    is_quick_switch_target(hwnd),
                    "7-Zip's extract box regressed"
                );
            },
        );
    }

    #[test]
    fn a_readonly_folder_field_is_not_a_target() {
        with_dialog("解压到", &["..."], &[(r"C:\Windows", true)], |hwnd| {
            assert!(
                !is_quick_switch_target(hwnd),
                "a read-only field has nothing to write into"
            );
        });
    }

    #[test]
    fn ctrl_g_still_works_on_an_unrecognised_app_dialog() {
        with_dialog("选项", &["确定", "取消"], &[(r"C://Windows", false)], |hwnd| {
            assert!(!is_quick_switch_target(hwnd), "auto-attach must stay strict");
            assert!(
                is_manual_quick_switch_target(hwnd),
                "Ctrl+G must still work when the user asks for it"
            );
        });
    }

    #[test]
    fn ctrl_g_cannot_rescue_an_uninstaller_or_a_readonly_field() {
        with_dialog(
            "Foo 卸载",
            &["卸载(&U)"],
            &[(r"C://Program Files//Foo", false)],
            |hwnd| assert!(!is_manual_quick_switch_target(hwnd)),
        );
        with_dialog("解压到", &["..."], &[(r"C://Windows", true)], |hwnd| {
            assert!(!is_manual_quick_switch_target(hwnd));
        });
    }

    #[test]
    fn a_download_form_is_not_a_folder_target() {
        // A URL, the folder it will be saved to, and a dozen unrelated
        // fields: the shape guard must reject this before any text is read.
        with_edits(
            &[
                "https://example.com/archive.zip",
                "C:\\Windows",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ],
            |hwnd| {
                let shape = super::scan_dialog_children(hwnd);
                assert!(shape.is_form());
                assert!(
                    !is_quick_switch_target(hwnd),
                    "stray Quick Switch on a form"
                );
            },
        );
    }

    #[test]
    fn a_link_beside_a_folder_is_not_a_folder_target() {
        // Only two fields, so the shape guard cannot help: the URL guard has
        // to do the work on its own.
        with_edits(
            &["https://example.com/archive.zip", "C:\\Windows"],
            |hwnd| {
                let shape = super::scan_dialog_children(hwnd);
                assert!(!shape.is_form());
                assert!(
                    !is_quick_switch_target(hwnd),
                    "a download link was read as a folder"
                );
            },
        );
    }

    #[test]
    fn a_lone_folder_field_is_still_a_target() {
        with_edits(&["C:\\Windows"], |hwnd| {
            assert!(
                is_quick_switch_target(hwnd),
                "the 7-Zip case must keep working"
            );
        });
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
            assert!(
                super::apply_folder(hwnd, path),
                "the folder was not applied"
            );
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
