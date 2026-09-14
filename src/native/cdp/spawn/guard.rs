// SPDX-License-Identifier: MIT OR Apache-2.0
//! The perennial spawn thread that keeps `PR_SET_PDEATHSIG` valid.
//!
//! # Why a dedicated thread exists
//!
//! `PR_SET_PDEATHSIG` is scoped to the **thread** that forked the child, not to
//! the process. When the spawning thread exits, the kernel delivers the signal
//! immediately — even though the CLI is still running and still driving CDP.
//!
//! Both obvious spawn sites are disqualified by that rule:
//!
//! - a Tokio worker thread can be parked, but the runtime may also shrink and
//!   retire threads, and the worker set is not a stable identity;
//! - a `spawn_blocking` thread retires after roughly ten seconds of idleness,
//!   which is well inside a normal browser session.
//!
//! Either choice produces a browser that is killed mid-session, which reads as a
//! random CDP disconnect. The fix is to own one thread for the whole process
//! lifetime and to fork every child from it.
//!
//! # Lifetime
//!
//! The thread is started on first use and never joined: its receiver stays alive
//! because the sender lives in a process-wide [`OnceLock`], and a one-shot CLI
//! exits by returning from `main`. That is deliberate — joining it would be the
//! very thread exit that fires the death signal.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Sender, SyncSender};
use std::sync::OnceLock;

use super::os;

/// One browser process to fork from the guard thread.
pub struct SpawnRequest {
    /// Absolute path to the engine binary.
    pub program: PathBuf,
    /// Full argument vector, excluding argv\[0\].
    pub args: Vec<String>,
    /// Environment entries to set on the child, on top of the inherited set.
    ///
    /// Exists for one reason: `DISPLAY`. A Chrome launched into a private
    /// virtual display is told about it through the environment and nowhere
    /// else, and this is the only fork site in the product — routing around it
    /// would mean forking from a thread that can retire, which is exactly the
    /// parent-death bug this module documents.
    ///
    /// Not a product configuration channel. Values reaching here are computed
    /// by the CLI, never read from the operator's environment.
    pub envs: Vec<(String, String)>,
    /// Inherited environment entries to drop from the child.
    ///
    /// Exists for one reason: `WAYLAND_DISPLAY`. It is NOT what keeps Chrome
    /// inside Xvfb — Chromium picks Wayland from `XDG_SESSION_TYPE` and finds
    /// `$XDG_RUNTIME_DIR/wayland-0` on its own, which `--ozone-platform=x11`
    /// overrides. Dropping the variable keeps the rest of the child's world
    /// consistent with that switch: Fedora's `/etc/chromium/chromium.conf`, for
    /// one, turns on Wayland-only GPU switches whenever it sees the variable.
    /// Same rule as `envs`: names reaching here are computed by the CLI.
    pub env_remove: Vec<String>,
    /// Send the child's stdout and stderr to the null device instead of pipes.
    ///
    /// For children nobody drains. A pipe with no reader blocks the writer once
    /// the kernel buffer fills, and the private Xvfb was measured writing about
    /// a kilobyte of xkbcomp warnings per client that connected.
    pub discard_output: bool,
    /// Chrome's ends of the DevTools pipe, placed where Chrome looks for them.
    ///
    /// Held here until the fork returns and dropped with the request, which
    /// closes them in this process: without that, the parent would never see
    /// end-of-file when Chrome exits.
    pub debug_pipe: Option<crate::native::cdp::pipe::ChildEnds>,
}

impl SpawnRequest {
    /// A request with no extra environment.
    #[must_use]
    pub fn new(program: PathBuf, args: Vec<String>) -> Self {
        Self {
            program,
            args,
            envs: Vec::new(),
            env_remove: Vec::new(),
            discard_output: false,
            debug_pipe: None,
        }
    }
}

/// A child forked by the guard thread, with its group already resolved.
pub struct GuardedChild {
    /// The live child. Ownership transfers to the caller, which must reap it.
    pub child: Child,
    /// POSIX process group of the child, when the host models one.
    ///
    /// `Some` enables a whole-tree `kill(-pgid, …)` at FINALIZE; `None` forces
    /// the pid-tree fallback in [`crate::lifecycle`].
    pub pgid: Option<i32>,
}

/// Work item handed to the guard thread.
struct Job {
    request: SpawnRequest,
    reply: SyncSender<Result<GuardedChild, String>>,
}

/// Process-wide handle to the guard thread, created on first spawn.
static GUARD: OnceLock<Sender<Job>> = OnceLock::new();

/// Fork `request` from the perennial guard thread and return the live child.
///
/// Blocking: the caller waits for the fork to complete. Async call sites must
/// wrap this in `spawn_blocking` so a Tokio worker is never pinned by the fork.
///
/// # Errors
///
/// Fails when the guard thread never started or has since died — the job
/// channel refuses the send, or the reply channel is dropped before an answer
/// arrives — and otherwise carries the `fork_child` message: `Command::spawn`
/// could not launch `request.program`, typically a missing or non-executable
/// binary.
pub fn spawn_guarded(request: SpawnRequest) -> Result<GuardedChild, String> {
    let sender = GUARD.get_or_init(start_guard_thread);
    let (reply, answer) = sync_channel(1);
    sender
        .send(Job { request, reply })
        .map_err(|_| "browser spawn guard thread is gone".to_string())?;
    answer
        .recv()
        .map_err(|_| "browser spawn guard thread dropped the reply".to_string())?
}

/// Start the guard thread and return its work channel.
fn start_guard_thread() -> Sender<Job> {
    let (tx, rx) = std::sync::mpsc::channel::<Job>();
    let started = std::thread::Builder::new()
        .name("bac-spawn-guard".to_string())
        .spawn(move || {
            // Runs until the process exits: every recv error means the sender in
            // GUARD was dropped, which only happens at process teardown.
            while let Ok(job) = rx.recv() {
                let _ = job.reply.send(fork_child(&job.request));
            }
        });
    if let Err(e) = started {
        tracing::warn!(error = %e, "browser spawn guard thread could not start");
    }
    tx
}

/// Fork one child with the host parent-death binding applied.
///
/// Always runs on the guard thread, which is what makes the binding outlive the
/// call site.
fn fork_child(request: &SpawnRequest) -> Result<GuardedChild, String> {
    let mut command = Command::new(&request.program);
    // Explicit Stdio: null stdin (automation), piped output for the drainers
    // unless the caller declared that nobody will drain it.
    let output = || {
        if request.discard_output {
            Stdio::null()
        } else {
            Stdio::piped()
        }
    };
    command
        .args(&request.args)
        .stdin(Stdio::null())
        .stdout(output())
        .stderr(output());
    for (key, value) in &request.envs {
        command.env(key, value);
    }
    for key in &request.env_remove {
        command.env_remove(key);
    }
    // On Windows the inheritable handles need no hook: they are alive in the
    // request, and the spawn inherits them under the values named in argv.
    #[cfg(unix)]
    if let Some(ends) = &request.debug_pipe {
        crate::native::cdp::pipe::install_child_ends(&mut command, ends);
    }
    os::host().bind_child(&mut command);

    let child = command.spawn().map_err(|e| {
        format!(
            "Failed to launch browser at {}: {e}",
            request.program.display()
        )
    })?;
    let pgid = os::host().process_group_of(child.id());
    Ok(GuardedChild { child, pgid })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial child proves the guard thread forks, binds, and replies.
    #[test]
    #[cfg(unix)]
    fn guard_thread_spawns_and_reports_group() {
        let Some(program) = crate::platform::which_bin("true") else {
            crate::test_utils::skip_unit_test("guard_thread", "/bin/true not found on this host.");
            return;
        };
        let mut guarded = spawn_guarded(SpawnRequest::new(program, Vec::new()))
            .expect("guard thread must fork /bin/true");

        // On Linux and macOS the child is put in its own group, so the group id
        // equals its pid and must differ from ours.
        if let Some(pgid) = guarded.pgid {
            assert_eq!(
                pgid as u32,
                guarded.child.id(),
                "setpgid(0, 0) makes the child its own group leader"
            );
        }
        let _ = guarded.child.wait();
    }

    #[test]
    fn repeated_spawns_reuse_one_guard_thread() {
        let Some(program) = crate::platform::which_bin("true") else {
            crate::test_utils::skip_unit_test("guard_thread", "/bin/true not found on this host.");
            return;
        };
        for _ in 0..3 {
            let mut guarded = spawn_guarded(SpawnRequest::new(program.clone(), Vec::new()))
                .expect("guard thread must stay alive across spawns");
            let _ = guarded.child.wait();
        }
        assert!(GUARD.get().is_some(), "guard channel must be initialized");
    }

    /// Extra environment must reach the child, or `DISPLAY` never arrives.
    #[test]
    #[cfg(unix)]
    fn extra_environment_reaches_the_child() {
        let Some(program) = crate::platform::which_bin("sh") else {
            crate::test_utils::skip_unit_test("guard_thread", "/bin/sh not found on this host.");
            return;
        };
        let guarded = spawn_guarded(SpawnRequest {
            envs: vec![("BAC_PROBE".to_string(), "ok".to_string())],
            ..SpawnRequest::new(
                program,
                vec!["-c".to_string(), "test \"$BAC_PROBE\" = ok".to_string()],
            )
        })
        .expect("guard thread must fork /bin/sh");
        let status = guarded
            .child
            .wait_with_output()
            .expect("child must be reapable");
        assert!(
            status.status.success(),
            "the child did not observe the env entry it was given"
        );
    }

    /// A removed variable must not reach the child, or `WAYLAND_DISPLAY` leaks.
    ///
    /// Uses `PATH` because the test runner always inherits it, so the check
    /// cannot pass merely because the variable was never there. `env` prints
    /// the environment it received and adds nothing of its own.
    #[test]
    #[cfg(unix)]
    fn removed_environment_never_reaches_the_child() {
        let Some(program) = crate::platform::which_bin("env") else {
            crate::test_utils::skip_unit_test("guard_thread", "env not found on this host.");
            return;
        };
        let guarded = spawn_guarded(SpawnRequest {
            env_remove: vec!["PATH".to_string()],
            ..SpawnRequest::new(program, Vec::new())
        })
        .expect("guard thread must fork env");
        let output = guarded
            .child
            .wait_with_output()
            .expect("child must be reapable");
        let printed = String::from_utf8_lossy(&output.stdout);
        assert!(
            !printed.lines().any(|line| line.starts_with("PATH=")),
            "the child still observed a variable it was told to drop: {printed}"
        );
    }
}
