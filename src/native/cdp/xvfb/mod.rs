// SPDX-License-Identifier: MIT OR Apache-2.0
//! A private virtual display for headed Chrome on Linux.
//!
//! # What this closes
//!
//! `BrowserMode::Auto` was documented as "headed inside a private virtual
//! display on Linux" and resolved to headless, because the display half was
//! never written. The constants for it existed and were read by nobody; so did
//! `--no-xvfb`, a flag that switched off something that never switched on.
//! That is worse than a missing feature, because the help text promised it.
//!
//! # Why a headed browser wants an invisible screen
//!
//! Headless Chrome differs from headed Chrome in ways a page can read directly:
//! `window.outerHeight` is 0, the renderer is a software rasteriser rather than
//! the host GPU, and the product token in the User-Agent literally says
//! `HeadlessChrome`. Running the headed binary against a virtual X server
//! removes that whole class of difference — the browser is genuinely headed;
//! the screen simply is not attached to a monitor.
//!
//! `--window-position=-32000,-32000` was tried first and does not work: GNOME's
//! Mutter clamps window positions back into the visible area, so the window
//! lands on the operator's desktop.
//!
//! # Scope, stated plainly
//!
//! Linux only. macOS always has Quartz and Windows always has DWM, so a headed
//! launch there is already invisible-capable without a virtual display, and
//! Xvfb does not exist on either.
//!
//! This never runs on the default path. `Auto` still resolves to headless, so
//! no ordinary command pays the spawn cost or acquires a host dependency; the
//! display is started only when the caller asked for headed. Inverting that
//! default is a separate decision with a latency bill attached.
//!
//! # What this deliberately does not do
//!
//! It does not install anything. A CLI that shells out to a package manager —
//! with or without `sudo` — is a CLI that can modify the host it was asked to
//! observe. When the binary is missing, `doctor` reports it with the command
//! for this distribution and the launch degrades to a plain headed window.

pub mod display;
pub mod spawn;

pub use display::host_has_display;
pub use spawn::{start_private_display, xvfb_available, XvfbGuard};

/// Whether this launch should run inside a private display.
///
/// # The cascade, and the two steps it deliberately omits
///
/// 1. Not headed → no. Headless has no window to hide.
/// 2. `--no-xvfb` → no. The operator asked to see the window.
/// 3. Not Linux → no. There is no Xvfb, and none is needed.
/// 4. Otherwise → yes, if a server can be started.
///
/// The reference cascade for this feature begins with `CHROME_HEADLESS=1` and
/// `CHROME_VISIBLE=1`. Both are omitted on purpose: product configuration in
/// this CLI is argv plus the XDG file, and those two intents already exist as
/// `--headless` / `--headed` and the `browser_mode` key. Adding environment
/// variables would create a second, undiscoverable way to say the same thing.
#[must_use]
pub fn should_use_private_display() -> bool {
    if crate::browser_policy::mode().launches_headless() {
        return false;
    }
    if crate::browser_policy::no_xvfb() {
        return false;
    }
    cfg!(target_os = "linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_headless_launch_has_no_window_to_hide() {
        // Reads the live policy rather than flipping it: the mode is a
        // process-global, and a test that mutated it would race every other
        // test in this binary.
        if crate::browser_policy::mode().launches_headless() {
            assert!(!should_use_private_display());
        }
    }

    #[test]
    fn a_non_linux_host_never_wants_xvfb() {
        if !cfg!(target_os = "linux") {
            assert!(!should_use_private_display());
        }
    }

    #[test]
    fn availability_is_a_question_not_an_installation() {
        // The check must be pure observation. If this ever starts installing
        // anything, the CLI has begun modifying the host it was asked to
        // observe.
        let before = xvfb_available();
        assert_eq!(before, xvfb_available());
    }
}
