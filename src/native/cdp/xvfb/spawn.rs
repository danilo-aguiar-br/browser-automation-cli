// SPDX-License-Identifier: MIT OR Apache-2.0
//! Starting a private X server and owning its lifetime.

use std::process::Child;
use std::time::{Duration, Instant};

use super::auth::XAuthority;
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
    /// The cookie clients must present. Declared after `child` so the file is
    /// removed only after the server that reads it is gone.
    authority: XAuthority,
}

impl XvfbGuard {
    /// The `DISPLAY` value a child must be given to draw here.
    #[must_use]
    pub fn display_value(&self) -> String {
        display::display_value(self.display_number)
    }

    /// The `XAUTHORITY` value a child must be given to be let in.
    #[must_use]
    pub fn xauthority_value(&self) -> String {
        self.authority.path().to_string_lossy().into_owned()
    }
}

impl Drop for XvfbGuard {
    fn drop(&mut self) {
        let pid = self.child.id();
        let was_alive = matches!(self.child.try_wait(), Ok(None));
        if was_alive {
            // SIGTERM first: the server then removes its own lock and socket.
            // SIGKILL, the fallback inside `wait_child_or_kill`, leaves both.
            #[cfg(unix)]
            if let Ok(raw) = i32::try_from(pid) {
                // SAFETY:
                // - Contract: ask a child this guard spawned and has not reaped to exit.
                // - Invariant: `kill` is a plain syscall; `raw` is the pid of a live,
                //   unreaped child, so it cannot name a recycled process.
                // - See: `man 2 kill`.
                unsafe {
                    libc::kill(raw, libc::SIGTERM);
                }
            }
            crate::platform::wait_child_or_kill(
                &mut self.child,
                Duration::from_millis(crate::constants::XVFB_TERM_GRACE_MS),
            );
        } else {
            let _ = self.child.wait();
        }
        // Remove the files ONLY when they name this server. A server that died
        // on its own may have lost its number to a sibling process, and
        // unlinking that sibling's lock would let a third process start a second
        // X server on the same socket.
        if display::lock_owner(self.display_number) == Some(pid) {
            let _ = std::fs::remove_file(display::lock_path(self.display_number));
            let _ = std::fs::remove_file(display::socket_path(self.display_number));
        }
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

/// Why a readiness wait ended without a usable server.
#[derive(Debug)]
enum NotReady {
    /// The server exited, typically because a sibling took the number first.
    Exited(String),
    /// The server kept running but never came up.
    TimedOut(String),
}

/// Start a private X server, or explain why not.
///
/// # Errors
///
/// Returns a human-readable reason when the binary is missing, when the cookie
/// file cannot be created, when every display number in the search range is
/// taken or lost to another process, or when a server does not come up inside
/// [`crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS`].
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

    // Geometry comes from named constants, not literals: a virtual display at
    // 800x600 is itself a fingerprint, because almost no human desktop reports
    // one. See `constants::stealth`.
    let screen = format!(
        "{}x{}x{}",
        crate::constants::DEFAULT_XVFB_WIDTH,
        crate::constants::DEFAULT_XVFB_HEIGHT,
        crate::constants::DEFAULT_XVFB_DEPTH
    );

    let mut last_loss = None;
    for display_number in display::free_displays() {
        let authority = XAuthority::create(display_number)?;
        let args = vec![
            display::display_value(display_number),
            "-screen".to_string(),
            "0".to_string(),
            screen.clone(),
            // No TCP listener, and only clients holding the cookie get in.
            "-nolisten".to_string(),
            "tcp".to_string(),
            "-auth".to_string(),
            authority.path().to_string_lossy().into_owned(),
        ];
        let mut request = SpawnRequest::new(program.clone(), args);
        // Nobody reads Xvfb's output, and it grows by roughly a kilobyte of
        // xkbcomp warnings per client: left in a pipe, it would eventually
        // block the server and Chrome with it.
        request.discard_output = true;
        let guarded = spawn_guarded(request)?;
        let mut guard = XvfbGuard {
            child: guarded.child,
            display_number,
            authority,
        };

        match wait_until_ready(&mut guard.child, display_number) {
            Ok(()) => {
                tracing::debug!(
                    target: "browser_automation_cli::xvfb",
                    display = display_number,
                    "private display ready"
                );
                return Ok(guard);
            }
            Err(NotReady::Exited(reason)) => {
                // Lost the number; the guard's Drop leaves the winner's files.
                last_loss = Some(reason);
            }
            Err(NotReady::TimedOut(reason)) => return Err(reason),
        }
    }
    Err(last_loss.unwrap_or_else(|| {
        format!(
            "no free X display in :{}..:{}",
            crate::constants::XVFB_DISPLAY_SEARCH_START,
            crate::constants::XVFB_DISPLAY_SEARCH_START
                + crate::constants::XVFB_DISPLAY_SEARCH_SPAN
        )
    }))
}

/// Block until THIS server holds the display, or it exits, or the deadline passes.
///
/// The socket alone is not the signal: a server that lost the race for the
/// number exits, while the socket it waited for exists because the WINNER made
/// it. Ready means the lock names our pid and the socket is there.
///
/// # Why the sleep is handed to the runtime
///
/// This function is synchronous, and its only caller — the self-spawn launch
/// path — is `async`. A bare `std::thread::sleep` here therefore parks a Tokio
/// worker for the whole startup window, which on a small worker pool is a
/// visible stall of every other task. [`tokio::task::block_in_place`] moves the
/// worker's remaining tasks to another thread before blocking, which is the
/// documented way to block from inside a multi-thread runtime without turning a
/// synchronous signature into an async one. Outside a runtime — unit tests, and
/// any future non-async caller — the plain loop is used unchanged.
fn wait_until_ready(child: &mut Child, display_number: u32) -> Result<(), NotReady> {
    match tokio::runtime::Handle::try_current() {
        // `block_in_place` panics on a current-thread runtime, so the flavor is
        // checked rather than assumed.
        Ok(handle) if handle.runtime_flavor() != tokio::runtime::RuntimeFlavor::CurrentThread => {
            tokio::task::block_in_place(|| poll_until_owned(child, display_number))
        }
        _ => poll_until_owned(child, display_number),
    }
}

/// The readiness poll itself, with no assumption about the calling context.
fn poll_until_owned(child: &mut Child, display_number: u32) -> Result<(), NotReady> {
    let socket = display::socket_path(display_number);
    let deadline =
        Instant::now() + Duration::from_secs(crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS);
    let poll = Duration::from_millis(crate::constants::XVFB_READY_POLL_MS);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(NotReady::Exited(format!(
                    "Xvfb on :{display_number} exited before it was ready ({status})"
                )));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(NotReady::Exited(format!(
                    "Xvfb on :{display_number} could not be polled: {e}"
                )));
            }
        }
        if display::lock_owner(display_number) == Some(child.id())
            && std::path::Path::new(&socket).exists()
        {
            return Ok(());
        }
        std::thread::sleep(poll);
    }
    Err(NotReady::TimedOut(format!(
        "Xvfb on :{display_number} did not create {socket} within {}s",
        crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A long-lived child that never becomes an X server.
    #[cfg(unix)]
    fn idle_child() -> Option<Child> {
        let cat = crate::platform::which_bin("cat")?;
        std::process::Command::new(cat)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .ok()
    }

    #[test]
    #[cfg(unix)]
    fn readiness_gives_up_instead_of_hanging() {
        // A display number nothing will ever bind. The point is that the wait
        // ends: an unbounded wait here would hang every browser command on a
        // host where Xvfb fails to start.
        let far = crate::constants::XVFB_DISPLAY_SEARCH_START
            + crate::constants::XVFB_DISPLAY_SEARCH_SPAN
            + 7;
        if std::path::Path::new(&display::socket_path(far)).exists() {
            // NOT routed through `skip_unit_test`, and the difference matters:
            // that helper turns a skip into a failure under `strict-gates`
            // because a missing TOOL is fixable by installing it. This branch is
            // the opposite shape — the host happens to occupy the display this
            // test needs free, so the assertion is unreachable by construction
            // and no install makes it reachable. Failing here would punish a
            // host for a state it is entitled to be in.
            eprintln!("skip: display :{far} unexpectedly exists on this host");
            return;
        }
        let Some(mut child) = idle_child() else {
            crate::test_utils::skip_unit_test("xvfb_ready", "cat not found on this host.");
            return;
        };
        let started = Instant::now();
        let outcome = wait_until_ready(&mut child, far);
        let _ = child.kill();
        let _ = child.wait();
        assert!(matches!(outcome, Err(NotReady::TimedOut(_))), "{outcome:?}");
        assert!(
            started.elapsed()
                >= Duration::from_secs(crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS),
            "gave up before the deadline it promises"
        );
    }

    #[test]
    #[cfg(unix)]
    fn a_server_that_exits_is_reported_at_once_not_at_the_deadline() {
        // The race loser exits immediately; waiting out the whole deadline for
        // it would turn every lost race into a ten-second stall.
        let Some(program) = crate::platform::which_bin("true") else {
            crate::test_utils::skip_unit_test("xvfb_ready", "true not found on this host.");
            return;
        };
        let Ok(mut child) = std::process::Command::new(program).spawn() else {
            crate::test_utils::skip_unit_test("xvfb_ready", "true could not be spawned.");
            return;
        };
        let far = crate::constants::XVFB_DISPLAY_SEARCH_START
            + crate::constants::XVFB_DISPLAY_SEARCH_SPAN
            + 9;
        let started = Instant::now();
        let outcome = wait_until_ready(&mut child, far);
        assert!(matches!(outcome, Err(NotReady::Exited(_))), "{outcome:?}");
        assert!(
            started.elapsed()
                < Duration::from_secs(crate::constants::DEFAULT_XVFB_STARTUP_TIMEOUT_SECS),
            "an exited server must not cost the whole deadline"
        );
    }

    #[test]
    fn a_host_without_the_binary_reports_it_rather_than_panicking() {
        if xvfb_available() {
            // Mutually exclusive with the host state, exactly like the skip in
            // `readiness_gives_up_instead_of_hanging`: this test asserts what
            // happens when Xvfb is ABSENT, so a host that has it can never reach
            // the assertion. Left out of `strict-gates` on purpose — see the
            // longer note there.
            eprintln!("skip: this host has Xvfb, so the missing-binary path cannot run");
            return;
        }
        match start_private_display() {
            Ok(_) => panic!("no binary on PATH, yet a display was reported as started"),
            Err(err) => assert!(err.contains("Xvfb"), "{err}"),
        }
    }
}
