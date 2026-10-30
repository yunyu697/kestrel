// Test helper: shows a classic common file dialog (comdlg32) for exercising
// Kestrel's Quick Switch attach/follow/hide logic from automated scripts.
// Build: rustc test_dialog.rs -o test_dialog.exe
use std::os::windows::ffi::OsStrExt;

#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    fn CommDlgExtendedError() -> u32;
}

#[repr(C)]
struct OPENFILENAMEW {
    lstruct_size: u32,
    owner: isize,
    instance: isize,
    filter: *const u16,
    custom_filter: *mut u16,
    max_cust_filter: u32,
    filter_index: u32,
    file: *mut u16,
    max_file: u32,
    file_title: *mut u16,
    max_file_title: u32,
    initial_dir: *const u16,
    title: *const u16,
    flags: u32,
    file_offset: u16,
    file_extension: u16,
    def_ext: *const u16,
    cust_data: isize,
    hook: isize,
    template_name: *const u16,
    pv_reserved: isize,
    dw_reserved: u32,
    flags_ex: u32,
}

fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn main() {
    let mut file = [0u16; 512];
    let title = to_wide("QS-TEST-DIALOG");
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lstruct_size = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.file = file.as_mut_ptr();
    ofn.max_file = 512;
    ofn.title = title.as_ptr();
    ofn.flags = 0x00080000 | 0x00001000; // OFN_EXPLORER | OFN_FILEMUSTEXIST
    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    let err = unsafe { CommDlgExtendedError() };
    println!("GetOpenFileNameW -> {ok}, err={err:#x}, structsize={}", ofn.lstruct_size);
    // Keep the process alive briefly so scripts observe post-close state.
    std::thread::sleep(std::time::Duration::from_millis(1500));
}
