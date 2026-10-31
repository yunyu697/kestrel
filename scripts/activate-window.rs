// Bring a window to the front by title substring — used to reproduce the
// "IDM's dialog pops the Quick Switch" case on demand.
//
//   activate-window.exe "Internet Download Manager"
#![cfg(windows)]

type HWND = isize;

#[link(name = "user32")]
extern "system" {
    fn GetClassNameW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    fn GetWindowTextW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    fn EnumWindows(cb: extern "system" fn(HWND, isize) -> i32, l: isize) -> i32;
    fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
    fn SetForegroundWindow(hwnd: HWND) -> i32;
    fn BringWindowToTop(hwnd: HWND) -> i32;
    fn GetForegroundWindow() -> HWND;
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

fn main() {
    let needle = std::env::args().nth(1).expect("need a title substring");
    let needle = needle.to_lowercase();

    extern "system" fn cb(hwnd: HWND, l: isize) -> i32 {
        let needle = unsafe { &*(l as *const String) };
        let t = text_of(hwnd);
        if !t.is_empty() && t.to_lowercase().contains(needle.as_str()) {
            println!("found hwnd={hwnd:#x} class={:?} text={:?}", class_of(hwnd), t);
            unsafe {
                ShowWindow(hwnd, 5); // SW_SHOW
                BringWindowToTop(hwnd);
                SetForegroundWindow(hwnd);
            }
            let fg = unsafe { GetForegroundWindow() };
            println!(
                "foreground now={fg:#x} ({} {})",
                class_of(fg),
                text_of(fg)
            );
            return 0;
        }
        1
    }

    unsafe { EnumWindows(cb, &needle as *const String as isize) };
}
