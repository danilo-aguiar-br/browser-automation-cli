// SPDX-License-Identifier: MIT OR Apache-2.0
//! Starting a private X server and owning its lifetime.

use std::process::Child;
use std::time::{Duration, Instant};

use super::display;
use crate::native::cdp::spawn::guard::{spawn_guarded, SpawnRequest};

/// A private X server owned by this process.
///
/// # Why this is a guard type and not a pid
///
/// The product's residual contract is that a run leaves nothing behind. An X
/// server that outlives the CLI leaves three things: a process, a lock file at
/// `/tmp/.X{n}-lock`, and a socket at `/tmp/.X11-unix/X{n}`. None of them are
/// visible to the residual scanner, which classifies by the Chrome marker
/// prefix and would never look at an `Xvfb` cmdline.
///
/// Tying all three to a `Drop` is what keeps that contract true without asking
/// the residual scanner to learn a second vocabulary.
pub struct XvfbGuard {
    child: Child,
    display_number: u32,
}

impl XvfbGuard {
    /// The `DISPLAY` value a child must be given to draw here.
    #[must_use]
    pub fn display_value(&self) -> String {
        display::display_value(self.display_number)
    }
}

impl Drop for XvfbGuard {
    fn drop(&mut self) {
        // Kill before unlinking: removing the lock under a live server would
        // let a second server claim the same number and produce two X servers
        // fighting over one socket.
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Best-effort, and deliberately so. A failure here is a stale file in
        // `/tmp`, which the next run's search treats as "taken" and steps
        // over. Panicking in `Drop` during teardown would be strictly worse.
        let _ = std::fs::remove_file(display::lock_path(self.display_number));
        let _ = std::fs::remove_file(display::socket_path(self.display_number));
        tracing::debug!(
            target: "browser_automation_cli::xvfb",
            display = self.display_number,
            "private display torn down"
        );
    }
}

/// Whether an `Xvfb` binary is reachable on this host.
#[must_use]
pub fn xvfb_available() -> bool {
    crate::platform::which_bin("Xvfb").is_some()
}

/// Start a private X server, or explain why not.
///
/// # Errors
///
/// Returns a human-readable reason when the binary is missing, when every
/// display number in the search range is taken, or when the server does not
/// come up inside [`crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS`].
///
/// # Why the caller is expected to continue on failure
///
/// A private display improves the disguise; it is not what makes the browser
/// work. Refusing to launch because the host has no `Xvfb` would turn a
/// missing optional package into a hard outage on every browser command. The
/// caller degrades to a plain headed launch and says so.
pub fn start_private_display() -> Result<XvfbGuard, String> {
    let Some(program) = crate::platform::which_bin("Xvfb") else {
        return Err("Xvfb binary not found on PATH".to_string());
    };
    let Some(display_number) = display::find_free_display() else {
        return Err(format!(
            "no free X display in :{}..:{}",
            crate::constants::XVFB_DISPLAY_SEARCH_START,
            crate::constants::XVFB_DISPLAY_SEARCH_START
                + crate::constants::XVFB_DISPLAY_SEARCH_SPAN
        ));
    };

    // Geometry comes from named constants, not literals: a virtual display at
    // 800x600 is itself a fingerprint, because almost no human desktop reports
    // one. See `constants::stealth`.
    let screen = format!(
        "{}x{}x{}",
        crate::constants::DEFAULT_XVFB_WIDTH,
        crate::constants::DEFAULT_XVFB_HEIGHT,
        crate::constants::DEFAULT_XVFB_DEPTH
    );
    let args = vec![
        display::display_value(display_number),
        "-screen".to_string(),
        "0".to_string(),
        screen,
        // No host access control: the server is private to this process and
        // reachable only through a socket in `/tmp` that Drop removes.
        "-nolisten".to_string(),
        "tcp".to_string(),
    ];

    let guarded = spawn_guarded(SpawnRequest::new(program, args))?;
    let guard = XvfbGuard {
        child: guarded.child,
        display_number,
    };

    wait_until_ready(display_number)?;
    tracing::debug!(
        target: "browser_automation_cli::xvfb",
        display = display_number,
        "private display ready"
    );
    Ok(guard)
}

/// Block until the server's socket appears, or the deadline passes.
///
/// The socket is the readiness signal an X client actually needs: a running
/// process that has not yet bound is indistinguishable from a hung one, and
/// handing Chrome a `DISPLAY` that does not answer yet produces a launch
/// failure that blames Chrome.
fn wait_until_ready(display_number: u32) -> Result<(), String> {
    let socket = display::socket_path(display_number);
    let deadline =
        Instant::now() + Duration::from_secs(crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS);
    let poll = Duration::from_millis(crate::constants::XVFB_READY_POLL_MS);
    while Instant::now() < deadline {
        if std::path::Path::new(&socket).exists() {
            return Ok(());
        }
        std::thread::sleep(poll);
    }
    Err(format!(
        "Xvfb on :{display_number} did not create {socket} within {}s",
        crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_gives_up_instead_of_hanging() {
        // A display number nothing will ever bind. The point is that the wait
        // ends: an unbounded wait here would hang every browser command on a
        // host where Xvfb fails to start.
        let far = crate::constants::XVFB_DISPLAY_SEARCH_START
            + crate::constants::XVFB_DISPLAY_SEARCH_SPAN
            + 7;
        if std::path::Path::new(&display::socket_path(far)).exists() {
            eprintln!("skip: display :{far} unexpectedly exists on this host");
            return;
        }
        let started = Instant::now();
        assert!(wait_until_ready(far).is_err());
        assert!(
            started.elapsed()
                >= Duration::from_secs(crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS),
            "gave up before the deadline it promises"
        );
    }

    #[test]
    fn a_host_without_the_binary_reports_it_rather_than_panicking() {
        if xvfb_available() {
            eprintln!("skip: this host has Xvfb, so the missing-binary path cannot run");
            return;
        }
        match start_private_display() {
            Ok(_) => panic!("no binary on PATH, yet a display was reported as started"),
            Err(err) => assert!(err.contains("Xvfb"), "{err}"),
        }
    }
}
