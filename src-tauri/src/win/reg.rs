//! Just enough registry access for the "Run" features.
//!
//! Three callers, one shape of question: what is the default value of this key
//! (`App Paths`), what are all the values of this key (`Shell Folders`,
//! `RunMRU`), and write one back (`RunMRU`). That is the whole surface, so it
//! is hand-written next to the other bindings rather than pulled in as a
//! crate — the same reason stated in `win/mod.rs`.
//!
//! Every function here returns `None`/`false` instead of an error code: a
//! missing key is the normal case (most machines have no `App Paths` entry for
//! a given name), and no caller can do anything with the code.

use super::{from_wide, wide, HKEY, MAX_PATH};
use std::ffi::c_void;
use std::ptr;

/// The two hives the Run features read from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Root {
    CurrentUser,
    LocalMachine,
}

impl Root {
    fn hkey(self) -> HKEY {
        match self {
            // HKEY_CURRENT_USER / HKEY_LOCAL_MACHINE, sign-extended.
            Root::CurrentUser => 0x8000_0001u32 as isize,
            Root::LocalMachine => 0x8000_0002u32 as isize,
        }
    }
}

const KEY_QUERY_VALUE: u32 = 0x0001;
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_WRITE: u32 = 0x20006;
const REG_SZ: u32 = 1;

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        hkey: HKEY,
        sub_key: *const u16,
        options: u32,
        sam: u32,
        out: *mut HKEY,
    ) -> i32;
    fn RegQueryValueExW(
        hkey: HKEY,
        name: *const u16,
        reserved: *const u32,
        ty: *mut u32,
        data: *mut u8,
        cb: *mut u32,
    ) -> i32;
    fn RegEnumValueW(
        hkey: HKEY,
        index: u32,
        name: *mut u16,
        cch: *mut u32,
        reserved: *const u32,
        ty: *mut u32,
        data: *mut u8,
        cb: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        hkey: HKEY,
        name: *const u16,
        reserved: u32,
        ty: u32,
        data: *const u8,
        cb: u32,
    ) -> i32;
    fn RegCreateKeyExW(
        hkey: HKEY,
        sub_key: *const u16,
        reserved: u32,
        class: *const u16,
        options: u32,
        sam: u32,
        attrs: *const c_void,
        out: *mut HKEY,
        disposition: *mut u32,
    ) -> i32;
    fn RegDeleteValueW(hkey: HKEY, name: *const u16) -> i32;
    fn RegCloseKey(hkey: HKEY) -> i32;
}

fn open(root: Root, sub: &str, sam: u32) -> Option<HKEY> {
    let sub = wide(sub);
    let mut key: HKEY = 0;
    let rc = unsafe { RegOpenKeyExW(root.hkey(), sub.as_ptr(), 0, sam, &mut key) };
    if rc == 0 && key != 0 {
        Some(key)
    } else {
        None
    }
}

/// Raw bytes of one value, whatever its type.
///
/// The size is asked for first and the buffer is then allocated for it, which
/// is the only way to read a value that grows between the two calls (a
/// `RunMRU` entry can be up to `MAX_PATH` characters).
fn query_raw(key: HKEY, name: &str) -> Option<Vec<u8>> {
    let name = wide(name);
    let mut ty: u32 = 0;
    let mut cb: u32 = 0;
    let rc = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null(),
            &mut ty,
            ptr::null_mut(),
            &mut cb,
        )
    };
    if rc != 0 || cb == 0 {
        return None;
    }
    let mut buf = vec![0u8; cb as usize + 2];
    let rc = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null(),
            &mut ty,
            buf.as_mut_ptr(),
            &mut cb,
        )
    };
    if rc != 0 {
        return None;
    }
    buf.truncate(cb as usize);
    Some(buf)
}

/// Decode a UTF-16LE buffer, dropping the terminator and anything after it.
fn utf16_string(bytes: &[u8]) -> String {
    if bytes.len() < 2 {
        return String::new();
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    from_wide(&units)
}

/// One string value. `name` may be empty — that is the key's default value.
pub fn read_string(root: Root, sub: &str, name: &str) -> Option<String> {
    let key = open(root, sub, KEY_QUERY_VALUE)?;
    let raw = query_raw(key, name);
    unsafe { RegCloseKey(key) };
    let s = utf16_string(&raw?);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Every value in a key as (name, string), skipping non-string types.
///
/// `Shell Folders` stores the names the shell understands ("Startup",
/// "Common Startup", …) — that table is what `shell:startup` is resolved
/// against, so the full enumeration is the point.
pub fn values(root: Root, sub: &str) -> Vec<(String, String)> {
    let Some(key) = open(root, sub, KEY_QUERY_VALUE) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut index = 0u32;
    loop {
        let mut name = [0u16; MAX_PATH];
        let mut cch = name.len() as u32;
        let mut cb = 0u32;
        let rc = unsafe {
            RegEnumValueW(
                key,
                index,
                name.as_mut_ptr(),
                &mut cch,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut cb,
            )
        };
        if rc != 0 {
            break; // ERROR_NO_MORE_ITEMS, or a name too long to be ours.
        }
        index += 1;
        let key_name = String::from_utf16_lossy(&name[..cch as usize]);
        let Some(raw) = query_raw(key, &key_name) else {
            continue;
        };
        let value = utf16_string(&raw);
        if !value.is_empty() {
            out.push((key_name, value));
        }
    }
    unsafe { RegCloseKey(key) };
    out
}

/// Create (or open) a key and set one string value in it.
pub fn write_string(root: Root, sub: &str, name: &str, value: &str) -> bool {
    let sub_w = wide(sub);
    let mut key: HKEY = 0;
    let mut disposition: u32 = 0;
    let rc = unsafe {
        RegCreateKeyExW(
            root.hkey(),
            sub_w.as_ptr(),
            0,
            ptr::null(),
            0,
            KEY_WRITE | KEY_QUERY_VALUE,
            ptr::null(),
            &mut key,
            &mut disposition,
        )
    };
    if rc != 0 || key == 0 {
        return false;
    }
    let name_w = wide(name);
    // The length includes the terminator, exactly as the shell stores it.
    let data: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = data.len() * 2;
    let rc = unsafe {
        RegSetValueExW(
            key,
            name_w.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr() as *const u8,
            bytes as u32,
        )
    };
    unsafe { RegCloseKey(key) };
    rc == 0
}

pub fn delete_value(root: Root, sub: &str, name: &str) -> bool {
    let Some(key) = open(root, sub, KEY_SET_VALUE) else {
        return false;
    };
    let name = wide(name);
    let rc = unsafe { RegDeleteValueW(key, name.as_ptr()) };
    unsafe { RegCloseKey(key) };
    rc == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UTF-16 buffer shorter than one code unit has no string in it; the
    /// caller must get an empty value instead of a panic.
    #[test]
    fn utf16_needs_a_whole_code_unit() {
        assert_eq!(utf16_string(&[]), "");
        assert_eq!(utf16_string(&[0x41]), "");
    }

    #[test]
    fn utf16_stops_at_the_terminator() {
        let mut bytes = Vec::new();
        for u in "abc\0zzz".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(utf16_string(&bytes), "abc");
    }

    /// Reading a key that does not exist has to be a quiet `None`: it is the
    /// normal answer for `App Paths` lookups.
    #[test]
    fn a_missing_key_reads_as_nothing() {
        let missing = r"Software\Kestrel\ThereIsNoSuchKeyForThisTest";
        assert!(read_string(Root::CurrentUser, missing, "").is_none());
        assert!(values(Root::CurrentUser, missing).is_empty());
    }

    /// `Shell Folders` is present on every Windows profile and is what the
    /// `shell:` names resolve against, so it doubles as a read smoke test.
    #[test]
    fn shell_folders_enumerates() {
        let vals = values(
            Root::CurrentUser,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders",
        );
        assert!(
            vals.iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Startup") && v.contains('\\')),
            "启动文件夹不在 Shell Folders 里: {:?}",
            vals.iter().take(5).collect::<Vec<_>>()
        );
    }
}
