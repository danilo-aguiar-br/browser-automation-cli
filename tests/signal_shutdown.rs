//! D-17: SIGTERM/SIGINT against a live CLI child (Unix).
//!
//! Strategy: spawn a long-lived `wait` step via `run` when Chrome is available;
//! otherwise exercise the signal path with `timeout(1)` + doctor offline so the
//! process cannot hang the suite.

#![cfg(unix)]

use std::io::Write;
use std::process::Stdio;
use std::thread;
use std::time::Duration;

mod common;
use common::{bin, chrome_hinted_by_doctor_text};

/// How a signalled child left the process table.
#[derive(Debug)]
enum Reaped {
    /// It exited on its own, within budget.
    ///
    /// The exit code is deliberately NOT carried: 0, 130 and the CLI error
    /// codes are all honest here, and the only failing outcome — the variant
    /// below — has no exit code at all, so a code field could never explain a
    /// red run.
    OnItsOwn,
    /// The budget expired and this test had to kill it.
    KilledByTest,
}

/// Poll `try_wait` until the deadline; kill on hang, and SAY which happened.
///
/// # Why the outcome is returned instead of swallowed
///
/// This used to return a bare `ExitStatus` after killing the child, and both
/// call sites discarded it with `let _ = status.code()`. A binary that IGNORED
/// SIGTERM outright therefore PASSED: the test blocked for the whole budget,
/// killed the child itself, reaped the corpse and reported ok. The single
/// behaviour this file exists to prove — that the CLI honours a signal — was
/// the one thing it could not fail on.
///
/// Killing is still the right thing to do on a hang, because a leaked child
/// outlives the suite. What changed is that the kill is now REPORTED, so the
/// caller can turn it into a failure.
fn wait_or_kill(child: &mut std::process::Child, budget: Duration) -> Reaped {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Reaped::OnItsOwn,
            Ok(None) if start.elapsed() >= budget => {
                let _ = child.kill();
                let _ = child.wait().expect("reap after kill");
                return Reaped::KilledByTest;
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(e) => panic!("try_wait: {e}"),
        }
    }
}

#[test]
fn sigterm_to_cli_does_not_hang() {
    // Prefer a multi-second wait step so the signal lands during EXECUTE.
    // Fallback: doctor offline (may finish before signal — still must not hang).
    let mut child = if chrome_hinted_by_doctor_text(&bin()) {
        let mut c = common::cmd()
            .args(["--json", "run", "-"])
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn run");
        if let Some(mut stdin) = c.stdin.take() {
            // Long wait so SIGTERM arrives mid-EXECUTE.
            let _ = writeln!(stdin, r#"{{"cmd":"wait","ms":8000}}"#);
        }
        c
    } else {
        common::cmd()
            .args(["doctor", "--offline", "--quick"])
            .env("NO_COLOR", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn doctor")
    };

    // A short delay so the signal has a chance to land after the handler is
    // installed rather than during process startup. It is not a correctness
    // knob: if the signal arrives before the handler exists, the SIGTERM
    // default terminates the process anyway, and the assertion below still
    // sees a child that left on its own.
    thread::sleep(Duration::from_millis(120));
    // SAFETY:
    // - Contract: `kill(2)` needs a pid the caller is allowed to signal; this
    //   test spawned the child itself and has not reaped it yet.
    // - Invariant: the pid stays valid until `wait_or_kill` below reaps it, so
    //   it cannot have been recycled by another process at this point.
    // - Caller/callee: sending SIGTERM mutates no memory this test owns.
    // - See: `man 2 kill`; the same pattern in `sigint_to_cli_does_not_hang`.
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }

    // The budget is deliberately SHORTER than the 8000 ms `wait` step above.
    // With a longer budget a CLI that ignored SIGTERM would finish the wait
    // naturally and look indistinguishable from one that honoured the signal.
    let reaped = wait_or_kill(&mut child, Duration::from_secs(5));
    // Exit code is not asserted: 0 means it finished before the signal, 130
    // means cancelled, and any other non-zero is the CLI error path — all three
    // are honest outcomes. What is NOT acceptable is still running.
    assert!(
        matches!(reaped, Reaped::OnItsOwn),
        "CLI ignored SIGTERM and had to be killed by the test: {reaped:?}"
    );
}

#[test]
fn sigint_to_cli_does_not_hang() {
    let mut child = common::cmd()
        .args(["doctor", "--offline", "--quick"])
        .env("NO_COLOR", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn doctor");
    thread::sleep(Duration::from_millis(50));
    // SAFETY:
    // - Contract: `kill(2)` needs a pid the caller is allowed to signal; this
    //   test spawned the child itself and has not reaped it yet.
    // - Invariant: the pid stays valid until the reap below, so it cannot have
    //   been recycled by another process at this point.
    // - Caller/callee: sending SIGINT mutates no memory this test owns.
    // - See: `man 2 kill`; the same pattern in `sigterm_to_cli_does_not_hang`.
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    // `doctor --offline --quick` may legitimately finish before the signal
    // arrives; that is `OnItsOwn` too. The failure this catches is the child
    // that is still running when the budget ends.
    let reaped = wait_or_kill(&mut child, Duration::from_secs(5));
    assert!(
        matches!(reaped, Reaped::OnItsOwn),
        "CLI ignored SIGINT and had to be killed by the test: {reaped:?}"
    );
}
