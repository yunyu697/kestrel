// Probe several flavours of "file picking" dialog so the detector can be
// told apart from look-alikes without regressing on the older ones.
//
//   1. modern  — GetOpenFileName + OFN_EXPLORER (the Vista+ common dialog)
//   2. legacy  — GetOpenFileName without OFN_EXPLORER (classic Win32 dialog)
//   3. folder  — SHBrowseForFolder (tree picker)
//   4. save    — GetSaveFileName + OFN_EXPLORER
#![cfg(windows)]

type HWND = isize;
type HRESULT = i32;

#[repr(C)]
struct OPENFILENAMEW {
    lStructSize: u32,
    hwndOwner: HWND,
    hInstance: isize,
    lpstrFilter: *const u16,
    lpstrCustomFilter: *mut u16,
    nMaxCustFilter: u32,
    nFilterIndex: u32,
    lpstrFile: *mut u16,
    nMaxFile: u32,
    lpstrFileTitle: *mut u16,
    nMaxFileTitle: u32,
    lpstrInitialDir: *const u16,
    lpstrTitle: *const u16,
    flags: u32,
    nFileOffset: u16,
    nFileExtension: u16,
    lpstrDefExt: *const u16,
    lCustData: isize,
    lpfnHook: isize,
    lpTemplateName: *const u16,
    pvReserved: *mut isize,
    dwReserved: u32,
    flagsEx: u32,
}

#[repr(C)]
struct BROWSEINFOA {
    hwndOwner: HWND,
    pidlRoot: *mut isize,
    pszDisplayName: *mut i8,
    lpszTitle: *const i8,
    ulFlags: u32,
    lpfn: isize,
    lParam: isize,
    iImage: i32,
}

#[link(name = "user32")]
extern "system" {
    fn GetClassNameW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    fn GetWindowTextW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    fn EnumWindows(cb: extern "system" fn(HWND, isize) -> i32, l: isize) -> i32;
    fn EnumChildWindows(parent: HWND, cb: extern "system" fn(HWND, isize) -> i32, l: isize) -> i32;
    fn IsWindowVisible(hwnd: HWND) -> i32;
    fn PostMessageW(hwnd: HWND, msg: u32, wp: isize, lp: isize) -> i32;
}

#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
    fn GetSaveFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn SHBrowseForFolderA(bi: *mut BROWSEINFOA) -> i32;
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), 256) };
    String::from_utf16_lossy(&buf[..n as usize])
}

fn text_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), 512) };
    String::from_utf16_lossy(&buf[..n as usize])
}

fn dump(target: HWND, label: &str) {
    println!("=== {label}: hwnd={:#x} class={:?} text={:?} ===", target, class_of(target), text_of(target));
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    extern "system" fn cb(hwnd: HWND, l: isize) -> i32 {
        let map = unsafe { &mut *(l as *mut std::collections::BTreeMap<String, usize>) };
        *map.entry(class_of(hwnd)).or_insert(0) += 1;
        1
    }
    unsafe { EnumChildWindows(target, cb, &mut counts as *mut _ as isize) };
    for (c, n) in &counts {
        println!("    {c:<32} x{n}");
    }
    let has = |needle: &str| counts.keys().any(|k| k.eq_ignore_ascii_case(needle));
    println!(
        "    --> ComboBoxEx32={} SHELLDLL_DefView={} DUIViewWndClassName={} NamespaceTreeControl={} Edit={} ListBox={} SysListView32={} SysTreeView32={} DirectUIHWND={}",
        has("ComboBoxEx32"),
        has("SHELLDLL_DefView"),
        has("DUIViewWndClassName"),
        has("NamespaceTreeControl"),
        has("Edit"),
        has("ListBox"),
        has("SysListView32"),
        has("SysTreeView32"),
        has("DirectUIHWND"),
    );
}

static mut TARGET: HWND = 0;
static mut MARKER: u32 = 0;

static mut SEEN: isize = 0;

extern "system" fn top_cb(hwnd: HWND, _l: isize) -> i32 {
    unsafe {
        if class_of(hwnd) == "#32770" && IsWindowVisible(hwnd) != 0 && MARKER != 0 && hwnd != SEEN {
            TARGET = hwnd;
            MARKER = 0;
            return 0;
        }
    }
    1
}

fn wait_and_dump(label: &str) {
    let mut target = 0;
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        unsafe { EnumWindows(top_cb, 0) };
        target = unsafe { TARGET };
        if target != 0 {
            break;
        }
    }
    if target == 0 {
        println!("=== {label}: NOT FOUND ===");
        return;
    }
    dump(target, label);
    unsafe { SEEN = target; PostMessageW(target, 0x0010, 0, 0) };
    std::thread::sleep(std::time::Duration::from_millis(600));
}

fn ofn(title: &str, flags: u32) -> (OPENFILENAMEW, Vec<u16>) {
    let t: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: 0,
        hInstance: 0,
        lpstrFilter: std::ptr::null(),
        lpstrCustomFilter: std::ptr::null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 0,
        lpstrFile: std::ptr::null_mut(),
        nMaxFile: 260,
        lpstrFileTitle: std::ptr::null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: t.as_ptr(),
        flags,
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: std::ptr::null(),
        lCustData: 0,
        lpfnHook: 0,
        lpTemplateName: std::ptr::null(),
        pvReserved: std::ptr::null_mut(),
        dwReserved: 0,
        flagsEx: 0,
    };
    (ofn, t)
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());

    if mode == "all" || mode == "modern" {
        unsafe { MARKER = 1 };
        std::thread::spawn(|| {
            let (mut o, _t) = ofn("KestrelModern", 0x0008_0000 | 0x0000_1000 | 0x0000_0800);
            let mut buf = [0u16; 260];
            o.lpstrFile = buf.as_mut_ptr();
            unsafe { GetOpenFileNameW(&mut o) };
        });
        wait_and_dump("MODERN open dialog (OFN_EXPLORER)");
    }

    if mode == "all" || mode == "legacy" {
        unsafe { MARKER = 1 };
        std::thread::spawn(|| {
            let (mut o, _t) = ofn("KestrelLegacy", 0x0000_1000 | 0x0000_0800);
            let mut buf = [0u16; 260];
            o.lpstrFile = buf.as_mut_ptr();
            unsafe { GetOpenFileNameW(&mut o) };
        });
        wait_and_dump("LEGACY open dialog (no OFN_EXPLORER)");
    }

    if mode == "all" || mode == "save" {
        unsafe { MARKER = 1 };
        std::thread::spawn(|| {
            let (mut o, _t) = ofn("KestrelSave", 0x0008_0000 | 0x0000_1000 | 0x0000_0800);
            let mut buf = [0u16; 260];
            o.lpstrFile = buf.as_mut_ptr();
            unsafe { GetSaveFileNameW(&mut o) };
        });
        wait_and_dump("SAVE dialog (OFN_EXPLORER)");
    }

    if mode == "all" || mode == "folder" {
        unsafe { MARKER = 1 };
        std::thread::spawn(|| unsafe {
            let mut display = [0i8; 260];
            let title: &[u8] = b"Choose a folder\0";
            let mut bi = BROWSEINFOA {
                hwndOwner: 0,
                pidlRoot: std::ptr::null_mut(),
                pszDisplayName: display.as_mut_ptr(),
                lpszTitle: title.as_ptr() as *const i8,
                ulFlags: 0,
                lpfn: 0,
                lParam: 0,
                iImage: 0,
            };
            SHBrowseForFolderA(&mut bi);
        });
        wait_and_dump("FOLDER picker (SHBrowseForFolder)");
    }
}
