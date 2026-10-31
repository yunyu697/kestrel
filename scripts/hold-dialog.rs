// Open a real common file dialog and keep it open, so the Quick Switch has
// time to attach to it. Closes itself after `hold` milliseconds.
//
//   hold-dialog.exe [milliseconds]
#![cfg(windows)]

type HWND = isize;

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

#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
}

fn main() {
    let hold: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);

    let title: Vec<u16> = "KestrelHold".encode_utf16().chain(std::iter::once(0)).collect();
    std::thread::spawn(move || {
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
        // OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST
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
    std::thread::sleep(std::time::Duration::from_millis(hold));
    println!("holding done");
}
