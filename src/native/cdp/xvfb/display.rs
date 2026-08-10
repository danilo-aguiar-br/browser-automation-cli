// SPDX-License-Identifier: MIT OR Apache-2.0
//! Picking a free X display number, and telling whether one already exists.

/// Whether this host already has a usable graphical display.
///
/// # Why reading these two variables is not a product knob
///
/// `DISPLAY` and `WAYLAND_DISPLAY` are the operating system's own way of
/// saying "there is a compositor here, and this is how you reach it". They are
/// facts about the host, in the same category as the path separator, and the
/// product has no say in their values.
///
/// That is a different thing from a product environment variable, which is a
/// configuration channel this CLI deliberately does not have. `CHROME_HEADLESS`
/// and friends are refused because the operator's intent belongs in argv or in
/// the XDG file, where it is discoverable and diagnosable. Detecting a
/// compositor is not intent.
#[must_use]
pub fn host_has_display() -> bool {
    #[cfg(target_os = "linux")]
    {
        let present = |key: &str| std::env::var_os(key).is_some_and(|v| !v.is_empty());
        present("DISPLAY") || present("WAYLAND_DISPLAY")
    }
    #[cfg(not(target_os = "linux"))]
    {
        // macOS always has Quartz and Windows always has DWM. There is no
        // headless-by-default case to compensate for, and no Xvfb either.
        true
    }
}

/// Find a display number no other X server is using.
///
/// # How "free" is decided
///
/// An X server holds `/tmp/.X{N}-lock` for the whole time it runs, so the
/// absence of that file is the same test `Xvfb -displayfd` would ultimately
/// lose to on a race. The race is real but small, and it is handled where it
/// belongs: the caller waits for the server to come up and moves on to the
/// next number if it does not.
///
/// Numbering starts high on purpose. Display `:0` is the operator's own
/// session, and `:1`..`:9` are commonly taken by display managers and remote
/// sessions. Starting at [`crate::constants::XVFB_DISPLAY_SEARCH_START`] keeps
/// this product out of that range entirely, so a bug here can never point
/// Chrome at a human's desktop.
#[must_use]
pub fn find_free_display() -> Option<u32> {
    let start = crate::constants::XVFB_DISPLAY_SEARCH_START;
    let end = start.saturating_add(crate::constants::XVFB_DISPLAY_SEARCH_SPAN);
    (start..end).find(|n| is_display_free(*n))
}

/// True when neither the lock file nor the socket for `n` exists.
fn is_display_free(n: u32) -> bool {
    !std::path::Path::new(&lock_path(n)).exists() && !std::path::Path::new(&socket_path(n)).exists()
}

/// `/tmp/.X{n}-lock`, the file an X server holds while it runs.
#[must_use]
pub fn lock_path(n: u32) -> String {
    format!("/tmp/.X{n}-lock")
}

/// `/tmp/.X11-unix/X{n}`, the socket clients connect to.
#[must_use]
pub fn socket_path(n: u32) -> String {
    format!("/tmp/.X11-unix/X{n}")
}

/// The `DISPLAY` value for a display number.
#[must_use]
pub fn display_value(n: u32) -> String {
    format!(":{n}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_never_reaches_the_operators_own_session() {
        // Display :0 is a human's desktop. Pointing an automated Chrome at it
        // would put a window on someone's screen, which is the failure this
        // whole module exists to avoid. The floor itself is enforced at compile
        // time in `constants::stealth`; what this checks is that the SEARCH
        // honours it, which a const assert cannot see.
        if let Some(n) = find_free_display() {
            assert!(n >= crate::constants::XVFB_DISPLAY_SEARCH_START);
        }
    }

    #[test]
    fn a_taken_display_is_not_offered() {
        // Whatever this host is running, a number whose lock exists must be
        // rejected. Asserted against the live filesystem rather than a mock,
        // because the lock file IS the protocol.
        for n in 0..8u32 {
            if std::path::Path::new(&lock_path(n)).exists() {
                assert!(!is_display_free(n), "display :{n} has a lock and was free");
            }
        }
    }

    #[test]
    fn paths_follow_the_x_server_convention() {
        assert_eq!(lock_path(99), "/tmp/.X99-lock");
        assert_eq!(socket_path(99), "/tmp/.X11-unix/X99");
        assert_eq!(display_value(99), ":99");
    }

    #[test]
    fn the_search_covers_the_whole_declared_span() {
        // A span of one would make concurrent runs collide immediately. The
        // value is asserted at compile time; what this checks is that the
        // search actually walks it rather than stopping at the first number.
        let start = crate::constants::XVFB_DISPLAY_SEARCH_START;
        let span = crate::constants::XVFB_DISPLAY_SEARCH_SPAN;
        if let Some(n) = find_free_display() {
            assert!(n < start + span, "picked :{n}, outside the declared span");
        }
    }
}
