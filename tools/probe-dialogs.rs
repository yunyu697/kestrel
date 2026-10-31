// Test helper: show the two kinds of folder-selection dialog so their window
// trees can be probed against Kestrel's dialog fingerprint.
//
//   probe-dialogs.exe folder-modern   -> IFileOpenDialog + FOS_PICKFOLDERS
//   probe-dialogs.exe folder-legacy   -> SHBrowseForFolder
//
// Build: rustc probe-dialogs.rs -o probe-dialogs.exe
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;

#[repr(C)]
struct Guid {
    d1: u32,
    d2: u16,
    d3: u16,
    d4: [u8; 8],
}

const CLSID_FILE_OPEN_DIALOG: Guid = Guid {
    d1: 0xDC1C_5A9C,
    d2: 0xE88A,
    d3: 0x4dde,
    d4: [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7],
};
const IID_IFILE_OPEN_DIALOG: Guid = Guid {
    d1: 0xd57c_7288,
    d2: 0xd4ad,
    d3: 0x4768,
    d4: [0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60],
};

const FOS_PICKFOLDERS: u32 = 0x0000_0020;

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> i32;
    fn CoCreateInstance(
        clsid: *const Guid,
        outer: *mut c_void,
        clsctx: u32,
        iid: *const Guid,
        out: *mut *mut c_void,
    ) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn SHBrowseForFolderW(info: *mut BrowseInfoW) -> *mut c_void;
    fn SHGetPathFromIDListW(pidl: *const c_void, path: *mut u16) -> i32;
}

#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemFree(p: *mut c_void);
}

#[repr(C)]
struct BrowseInfoW {
    owner: isize,
    pidl_root: *mut c_void,
    display_name: *mut u16,
    title: *const u16,
    flags: u32,
    callback: isize,
    lparam: isize,
    image: i32,
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

type ShowFn = unsafe extern "system" fn(*mut c_void, isize) -> i32;
type SetOptionsFn = unsafe extern "system" fn(*mut c_void, u32) -> i32;
type SetTitleFn = unsafe extern "system" fn(*mut c_void, *const u16) -> i32;
type ReleaseFn = unsafe extern "system" fn(*mut c_void) -> u32;

unsafe fn vfn<T>(obj: *mut c_void, index: usize) -> T {
    let table = *(obj as *const *const usize);
    let slot = *table.add(index);
    std::mem::transmute_copy(&slot)
}

fn folder_modern(title: &str) {
    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), 0x2 /* STA */);
        let mut obj: *mut c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_FILE_OPEN_DIALOG,
            std::ptr::null_mut(),
            0x1 /* INPROC_SERVER */,
            &IID_IFILE_OPEN_DIALOG,
            &mut obj,
        );
        if hr < 0 || obj.is_null() {
            println!("CoCreateInstance failed: {hr:#x}");
            return;
        }
        let t = wide(title);
        let set_title: SetTitleFn = vfn(obj, 17);
        set_title(obj, t.as_ptr());
        let set_options: SetOptionsFn = vfn(obj, 9);
        set_options(obj, FOS_PICKFOLDERS);
        let show: ShowFn = vfn(obj, 3);
        let r = show(obj, 0);
        let release: ReleaseFn = vfn(obj, 2);
        release(obj);
        println!("IFileOpenDialog(FOS_PICKFOLDERS) -> {r:#x}");
    }
}

fn folder_legacy(title: &str) {
    unsafe {
        let t = wide(title);
        let mut display = [0u16; 512];
        let mut info = BrowseInfoW {
            owner: 0,
            pidl_root: std::ptr::null_mut(),
            display_name: display.as_mut_ptr(),
            title: t.as_ptr(),
            flags: 0x1 | 0x10, // BIF_RETURNONLYFSDIRS | BIF_EDITBOX
            callback: 0,
            lparam: 0,
            image: 0,
        };
        let pidl = SHBrowseForFolderW(&mut info);
        if pidl.is_null() {
            println!("SHBrowseForFolder -> cancelled");
            return;
        }
        let mut path = [0u16; 512];
        SHGetPathFromIDListW(pidl, path.as_mut_ptr());
        CoTaskMemFree(pidl);
        println!("SHBrowseForFolder -> ok");
    }
}

fn main() {
    let kind = std::env::args().nth(1).unwrap_or_default();
    let title = "QS-PROBE-FOLDER";
    match kind.as_str() {
        "folder-modern" => folder_modern(title),
        "folder-legacy" => folder_legacy(title),
        other => println!("unknown kind {other:?}; use folder-modern | folder-legacy"),
    }
    // Keep the process alive briefly so scripts observe post-close state.
    std::thread::sleep(std::time::Duration::from_millis(1500));
}
