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
}

impl SpawnRequest {
    /// A request with no extra environment.
    #[must_use]
    pub fn new(program: PathBuf, args: Vec<String>) -> Self {
        Self {
            program,
            args,
            envs: Vec::new(),
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
    command
        .args(&request.args)
        // Explicit Stdio: null stdin (automation), piped output for the drainers.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &request.envs {
        command.env(key, value);
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
            eprintln!("skip: /bin/true not found on this host");
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
            eprintln!("skip: /bin/true not found on this host");
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
            eprintln!("skip: /bin/sh not found on this host");
            return;
        };
        let guarded = spawn_guarded(SpawnRequest {
            program,
            args: vec!["-c".to_string(), "test \"$BAC_PROBE\" = ok".to_string()],
            envs: vec![("BAC_PROBE".to_string(), "ok".to_string())],
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
}
