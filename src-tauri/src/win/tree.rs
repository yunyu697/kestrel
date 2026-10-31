//! Driving a folder tree (`SysTreeView32`) that lives in another process.
//!
//! Why this exists: an app that asks "which folder?" with its own dialog
//! (WinRAR's "解压路径和选项") has no address bar to type into. It has a folder
//! tree, and the tree is the thing the user is looking at — so the Quick
//! Switch should move *the tree*, not paste a string into a text box. It turns
//! out that selecting a node is enough: the dialog derives its destination
//! path from the selection and updates its own field.
//!
//! A tree view keeps its item text in its own address space, so the only way
//! to read it from outside is `TVM_GETITEMW` with the `TVITEMW` payload placed
//! in the **target** process. That is a scratch page, not code injection: one
//! `VirtualAllocEx`, a handful of `WriteProcessMemory`/`ReadProcessMemory`
//! calls, and the page is released on drop. Nothing is ever executed there.
//!
//! With item text plus the control's own parent/child links in hand, a path is
//! walked one component at a time and the last node selected with
//! `TVM_SELECTITEM`.

use super::*;
use std::ffi::c_void;
use std::mem::size_of;

const TVM_GETNEXTITEM: u32 = 0x110A;
const TVM_GETITEMW: u32 = 0x113E;
const TVM_EXPAND: u32 = 0x1102;
const TVM_SELECTITEM: u32 = 0x110B;
const TVM_ENSUREVISIBLE: u32 = 0x1114;

const TVGN_ROOT: usize = 0;
const TVGN_NEXT: usize = 1;
const TVGN_CHILD: usize = 4;
const TVGN_CARET: usize = 9;
const TVE_EXPAND: usize = 0x0002;
const TVE_COLLAPSE: usize = 0x0001;

const TVIF_TEXT: u32 = 0x0001;
const TVIF_STATE: u32 = 0x0008;
const TVIF_HANDLE: u32 = 0x0010;
const TVIF_CHILDREN: u32 = 0x0040;

/// Scratch page carved out of the target process.
const PAGE_SIZE: usize = 0x1000;
/// Where the item text starts inside that page.
const TEXT_OFFSET: usize = 0x100;
const TEXT_MAX: i32 = 260;

/// The drives sit at most this deep (desktop → this PC → drive).
const DRIVE_DEPTH: usize = 2;
/// A pathological tree must not turn into an unbounded walk.
const MAX_CHILDREN: usize = 4096;
/// How long to wait for a lazily enumerated folder to list its children.
const EXPAND_POLLS: usize = 100;
const EXPAND_POLL_MS: u64 = 15;

/// One tree item: its label and whether the control thinks it has children.
struct Item {
    text: String,
    children: i32,
}

/// A `SysTreeView32` plus the scratch page used to read its item text.
pub struct TreeView {
    hwnd: HWND,
    process: HANDLE,
    remote: *mut c_void,
}

impl Drop for TreeView {
    fn drop(&mut self) {
        unsafe {
            VirtualFreeEx(self.process, self.remote, 0, MEM_RELEASE);
            CloseHandle(self.process);
        }
    }
}

impl TreeView {
    /// Attach to the folder tree inside `dialog`, if it has one.
    pub fn attach(dialog: HWND) -> Option<TreeView> {
        let hwnd = window::dialog_tree(dialog)?;
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == 0 {
            return None;
        }
        let access = PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE;
        let process = unsafe { OpenProcess(access, 0, pid) };
        if process == 0 {
            return None;
        }
        let remote = unsafe {
            VirtualAllocEx(
                process,
                std::ptr::null_mut(),
                PAGE_SIZE,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if remote.is_null() {
            unsafe { CloseHandle(process) };
            return None;
        }
        Some(TreeView {
            hwnd,
            process,
            remote,
        })
    }

    fn send(&self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> usize {
        let mut out: usize = 0;
        let ok = unsafe {
            SendMessageTimeoutW(
                self.hwnd,
                msg,
                wparam,
                lparam,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                2000,
                &mut out,
            )
        };
        if ok == 0 {
            0
        } else {
            out
        }
    }

    fn next(&self, flag: usize, item: isize) -> isize {
        self.send(TVM_GETNEXTITEM, flag, item) as isize
    }

    /// Read one item: its label and whether it has children.
    fn item(&self, item: isize) -> Option<Item> {
        let mut payload = TVITEMW {
            mask: TVIF_TEXT | TVIF_STATE | TVIF_HANDLE | TVIF_CHILDREN,
            h_item: item,
            state: 0,
            state_mask: u32::MAX,
            psz_text: (self.remote as usize + TEXT_OFFSET) as *mut u16,
            cch_text_max: TEXT_MAX,
            i_image: 0,
            i_selected_image: 0,
            c_children: 0,
            l_param: 0,
        };
        let mut moved: usize = 0;
        unsafe {
            if WriteProcessMemory(
                self.process,
                self.remote,
                &payload as *const TVITEMW as *const c_void,
                size_of::<TVITEMW>(),
                &mut moved,
            ) == 0
            {
                return None;
            }
            if self.send(TVM_GETITEMW, 0, self.remote as LPARAM) == 0 {
                return None;
            }
            if ReadProcessMemory(
                self.process,
                self.remote,
                &mut payload as *mut TVITEMW as *mut c_void,
                size_of::<TVITEMW>(),
                &mut moved,
            ) == 0
            {
                return None;
            }
            let mut buf = [0u16; TEXT_MAX as usize];
            if ReadProcessMemory(
                self.process,
                (self.remote as usize + TEXT_OFFSET) as *const c_void,
                buf.as_mut_ptr() as *mut c_void,
                TEXT_MAX as usize * 2,
                &mut moved,
            ) == 0
            {
                return None;
            }
            Some(Item {
                text: from_wide(&buf),
                children: payload.c_children,
            })
        }
    }

    fn children(&self, item: isize) -> Vec<isize> {
        let mut out = Vec::new();
        let mut child = self.next(TVGN_CHILD, item);
        while child != 0 && out.len() < MAX_CHILDREN {
            out.push(child);
            child = self.next(TVGN_NEXT, child);
        }
        out
    }

    /// Scroll an item into view. A shell tree only enumerates a folder once it
    /// is on screen, so this has to happen *before* expanding it.
    fn reveal(&self, item: isize) {
        self.send(TVM_ENSUREVISIBLE, 0, item);
    }

    /// Expand an item and wait for its children to appear.
    fn expand(&self, item: isize) -> bool {
        self.send(TVM_EXPAND, TVE_EXPAND, item);
        for _ in 0..EXPAND_POLLS {
            if self.next(TVGN_CHILD, item) != 0 {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(EXPAND_POLL_MS));
        }
        false
    }

    fn collapse(&self, item: isize) {
        self.send(TVM_EXPAND, TVE_COLLAPSE, item);
    }

    /// Make sure `item` has listed its children, expanding it if necessary.
    fn ensure_children(&self, item: isize) -> bool {
        if self.next(TVGN_CHILD, item) != 0 {
            return true;
        }
        match self.item(item) {
            Some(it) if it.children != 0 => {
                self.reveal(item);
                self.expand(item)
            }
            _ => false,
        }
    }

    fn select(&self, item: isize) {
        self.send(TVM_SELECTITEM, TVGN_CARET, item);
        self.reveal(item);
    }

    /// The node for drive `letter`, e.g. `Data (D:)` for `D:`.
    ///
    /// Searching by expanding is a last resort: a dialog that offers a
    /// destination at all already has its drive list materialised, and
    /// expanding while searching would burst the tree open in front of the
    /// user (measured: 39 items became 322).
    fn find_drive(&self, letter: char) -> Option<isize> {
        let root = self.next(TVGN_ROOT, 0);
        if root == 0 {
            return None;
        }
        let want = format!("({}:)", letter.to_ascii_uppercase());
        if let Some(hit) = self.scan_materialized(root, &want, 0) {
            return Some(hit);
        }
        self.scan_expanding(root, &want, 0)
    }

    /// Walk what the control has already materialised, without opening
    /// anything.
    fn scan_materialized(&self, first: isize, want: &str, depth: usize) -> Option<isize> {
        if depth > DRIVE_DEPTH {
            return None;
        }
        let mut item = first;
        while item != 0 {
            if let Some(it) = self.item(item) {
                if it.text.to_uppercase().ends_with(want) {
                    return Some(item);
                }
            }
            let child = self.next(TVGN_CHILD, item);
            if child != 0 {
                if let Some(hit) = self.scan_materialized(child, want, depth + 1) {
                    return Some(hit);
                }
            }
            item = self.next(TVGN_NEXT, item);
        }
        None
    }

    /// The drive list is still collapsed somewhere: open one branch at a time
    /// and fold it back up when it holds no drive, so the tree ends up looking
    /// the way it did.
    fn scan_expanding(&self, first: isize, want: &str, depth: usize) -> Option<isize> {
        let mut item = first;
        while item != 0 {
            let info = self.item(item);
            if let Some(it) = &info {
                if it.text.to_uppercase().ends_with(want) {
                    return Some(item);
                }
            }
            let has_children = info.map(|i| i.children != 0).unwrap_or(false);
            if depth < DRIVE_DEPTH && has_children && self.next(TVGN_CHILD, item) == 0 {
                self.reveal(item);
                if self.expand(item) {
                    let child = self.next(TVGN_CHILD, item);
                    if child != 0 {
                        if let Some(hit) = self.scan_expanding(child, want, depth + 1) {
                            return Some(hit);
                        }
                    }
                    self.collapse(item);
                }
            }
            item = self.next(TVGN_NEXT, item);
        }
        None
    }

    fn find_child(&self, parent: isize, display: &str, raw: &str) -> Option<isize> {
        self.ensure_children(parent);
        self.children(parent)
            .into_iter()
            .find(|&c| match self.item(c) {
                Some(it) => name_matches(&it.text, display, raw),
                None => false,
            })
    }

    /// Walk `path` down the tree and leave the caret on its last component.
    pub fn select_path(&self, path: &str) -> bool {
        let Some((letter, parts)) = split_path(path) else {
            return false;
        };
        let Some(mut node) = self.find_drive(letter) else {
            crate::debug_log::qs_log_verbose(&format!("tree: drive {letter}: is not listed"));
            return false;
        };
        let mut walked = format!("{}:\\", letter.to_ascii_uppercase());
        for part in &parts {
            walked = format!("{}\\{}", walked.trim_end_matches('\\'), part);
            // A shell tree shows the *display* name, so "Documents" appears as
            // "文档" on a Chinese system. The raw name is kept as a fallback
            // for folders without a shell name.
            let display = display_name(&walked).unwrap_or_else(|| part.clone());
            match self.find_child(node, &display, part) {
                Some(next) => node = next,
                None => {
                    crate::debug_log::qs_log_verbose(&format!(
                        "tree: '{part}' (shown as '{display}') is not under its parent"
                    ));
                    return false;
                }
            }
        }
        self.select(node);
        true
    }
}

/// Does a node's label stand for the wanted folder?
fn name_matches(label: &str, display: &str, raw: &str) -> bool {
    label.eq_ignore_ascii_case(display) || label.eq_ignore_ascii_case(raw)
}

/// `D:\Data\Apps` → ('D', ["Data", "Apps"]).
fn split_path(path: &str) -> Option<(char, Vec<String>)> {
    let mut parts = path.split(['\\', '/']).filter(|s| !s.is_empty());
    let drive = parts.next()?;
    let bytes: Vec<char> = drive.chars().collect();
    if bytes.len() != 2 || bytes[1] != ':' || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    Some((bytes[0], parts.map(str::to_string).collect()))
}

/// The name the shell shows for `path` — what a shell tree puts in a node.
fn display_name(path: &str) -> Option<String> {
    let w = wide(path);
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        SHGetFileInfoW(
            w.as_ptr(),
            0,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_DISPLAYNAME,
        )
    };
    if ok == 0 {
        return None;
    }
    let name = from_wide(&info.szDisplayName);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Move the folder tree inside `dialog` to `path`.
///
/// Returns false when the dialog has no tree, when the path is not a plain
/// drive path, or when a component simply is not in the tree — a shell tree
/// hides hidden and system folders, so `C:\Windows\Fonts` has no node at all.
/// The caller falls back to writing the path field in that case.
pub fn select_folder(dialog: HWND, path: &str) -> bool {
    let Some(tree) = TreeView::attach(dialog) else {
        return false;
    };
    let ok = tree.select_path(path);
    crate::debug_log::qs_log(&format!("tree jump: dialog {dialog:#x} -> {path} ({ok})"));
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_drive_path_into_its_components() {
        assert_eq!(
            split_path("D:\\Data\\Apps"),
            Some(('D', vec!["Data".to_string(), "Apps".to_string()]))
        );
        assert_eq!(
            split_path("C:/Windows"),
            Some(('C', vec!["Windows".to_string()]))
        );
        assert_eq!(split_path("D:\\"), Some(('D', vec![])));
        // Trailing separators are not components.
        assert_eq!(
            split_path("D:\\Data\\"),
            Some(('D', vec!["Data".to_string()]))
        );
    }

    #[test]
    fn refuses_paths_a_drive_tree_cannot_express() {
        assert_eq!(split_path(""), None);
        assert_eq!(split_path("\\\\server\\share"), None);
        assert_eq!(split_path("relative\\path"), None);
        assert_eq!(split_path("D"), None);
    }

    #[test]
    fn matches_a_node_by_display_name_or_real_name() {
        // Chinese Windows shows Documents as 文档; either spelling identifies
        // the same node.
        assert!(name_matches("文档", "文档", "Documents"));
        assert!(name_matches("Documents", "文档", "Documents"));
        // Case never matters on Windows.
        assert!(name_matches("apps", "Apps", "apps"));
        assert!(!name_matches("Apps", "Data", "ProgramData"));
        assert!(!name_matches("", "Apps", "Apps"));
    }

    #[test]
    fn the_item_payload_has_the_c_layout() {
        // The offsets are what the target process reads back; getting them
        // wrong returns garbage instead of an error.
        assert_eq!(
            size_of::<TVITEMW>(),
            if cfg!(target_pointer_width = "64") {
                56
            } else {
                40
            }
        );
        assert_eq!(std::mem::offset_of!(TVITEMW, h_item), 8);
        assert_eq!(std::mem::offset_of!(TVITEMW, psz_text), 24);
        assert_eq!(std::mem::offset_of!(TVITEMW, c_children), 44);
    }
}
