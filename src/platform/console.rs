// SPDX-License-Identifier: MIT OR Apache-2.0
//! Console UTF-8 / VT configuration (Windows).

/// Configure Windows console for UTF-8 (CP 65001) and virtual terminal ANSI.
///
/// No-op on non-Windows. Failures are ignored (already UTF-8 / redirected handles).
pub fn configure_console() {
    #[cfg(windows)]
    {
        configure_console_windows();
    }
}

#[cfg(windows)]
fn configure_console_windows() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleCP, SetConsoleMode, SetConsoleOutputCP,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };

    // SAFETY: Win32 console APIs are process-wide and safe at single-threaded boot.
    // CP_UTF8 = 65001. VT mode enables ANSI on modern Windows Terminal / conhost.
    // See: https://learn.microsoft.com/windows/console/console-virtual-terminal-sequences
    const CP_UTF8: u32 = 65001;
    unsafe {
        let _ = SetConsoleOutputCP(CP_UTF8);
        let _ = SetConsoleCP(CP_UTF8);
        for nstd in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let h = GetStdHandle(nstd);
            if h == INVALID_HANDLE_VALUE || h.is_null() {
                continue;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(h, &mut mode) == 0 {
                continue;
            }
            let _ = SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}
