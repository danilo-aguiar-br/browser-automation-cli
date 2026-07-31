// SPDX-License-Identifier: MIT OR Apache-2.0
//! Windows App Paths registry discovery (OS platform only).

use std::path::PathBuf;

#[cfg(windows)]
use super::path_util::is_executable_file;

/// Resolve an application path from Windows `App Paths` registry (no product env).
///
/// Looks up `HKLM` then `HKCU` under
/// `SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe_name}` (default value).
/// Returns `None` on non-Windows or when the key/value is missing / not executable.
///
/// Multiplatform rules: registry is **OS discovery** (like `$PATH`), not product
/// configuration — product overrides remain XDG `chrome_path` / CLI flags only.
pub fn registry_app_path(exe_name: &str) -> Option<PathBuf> {
    if exe_name.is_empty() {
        return None;
    }
    #[cfg(windows)]
    {
        registry_app_path_windows(exe_name)
    }
    #[cfg(not(windows))]
    {
        let _ = exe_name;
        None
    }
}

#[cfg(windows)]
fn registry_app_path_windows(exe_name: &str) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ, KEY_WOW64_64KEY, REG_SZ,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn query_default(root: HKEY, subkey_wide: &[u16]) -> Option<PathBuf> {
        let mut hkey: HKEY = std::ptr::null_mut();
        // SAFETY: subkey is NUL-terminated; KEY_READ|KEY_WOW64_64KEY is valid SAM.
        // See: RegOpenKeyExW — App Paths default value is the absolute executable.
        let open = unsafe {
            RegOpenKeyExW(
                root,
                subkey_wide.as_ptr(),
                0,
                KEY_READ | KEY_WOW64_64KEY,
                &mut hkey,
            )
        };
        if open != ERROR_SUCCESS {
            // Retry without WOW64 flag (32-bit hosts / pure 32-bit keys).
            let open32 =
                unsafe { RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey) };
            if open32 != ERROR_SUCCESS {
                return None;
            }
        }
        let mut ty: u32 = 0;
        let mut size: u32 = 0;
        // SAFETY: query size of default value (lpValueName = null).
        let size_rc = unsafe {
            RegQueryValueExW(
                hkey,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut ty,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if size_rc != ERROR_SUCCESS || size == 0 || ty != REG_SZ {
            unsafe {
                let _ = RegCloseKey(hkey);
            }
            return None;
        }
        let mut buf = vec![0u16; (size as usize / 2).max(1)];
        let mut size2 = size;
        // SAFETY: buffer sized from prior query; default value name is null.
        let data_rc = unsafe {
            RegQueryValueExW(
                hkey,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut ty,
                buf.as_mut_ptr() as *mut u8,
                &mut size2,
            )
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        if data_rc != ERROR_SUCCESS || ty != REG_SZ {
            return None;
        }
        // Trim trailing NULs from REG_SZ.
        while buf.last().copied() == Some(0) {
            buf.pop();
        }
        if buf.is_empty() {
            return None;
        }
        let os = std::ffi::OsString::from_wide(&buf);
        let path = PathBuf::from(os);
        if is_executable_file(&path) {
            Some(path)
        } else {
            None
        }
    }

    let subkey = format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe_name}");
    let wide = to_wide(&subkey);
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        if let Some(p) = query_default(root, &wide) {
            return Some(p);
        }
    }
    None
}
