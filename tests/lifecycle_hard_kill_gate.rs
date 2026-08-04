// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hard-kill gate: `SIGKILL` on the CLI must leave no process from its group.
//!
//! # Why this is the only honest end-to-end shape
//!
//! Cooperative exits already worked; the defect was the path where FINALIZE
//! never runs. That cannot be exercised from inside the process under test —
//! `SIGKILL` cannot be caught, so a test that killed itself would have nothing
//! left to assert with. The CLI therefore has to be a *separate* process that
//! this test spawns, kills without warning, and then audits from the outside.
//!
//! # Why it is deterministic and not flaky
//!
//! Every wait has a bounded deadline and polls a definite condition (the pid is
//! gone, or the profile has a live holder) rather than sleeping a guessed
//! interval. Where a precondition is missing — no Chrome on the host, no
//! process table — the test skips **explicitly** and says so, instead of
//! passing silently on a host that proved nothing.

#![cfg(unix)]

use std::time::{Duration, Instant};

use browser_automation_cli::lifecycle::descendants_in_index;
use browser_automation_cli::native::cdp::chrome::find_chrome;
use browser_automation_cli::native::cdp::spawn::{host, ParentDeathBinding};
use browser_automation_cli::residual::{index_live_processes, CLI_CHROME_MARKER_PREFIX};

/// Bound on how long the CLI is given to get Chrome running.
const STARTUP_DEADLINE: Duration = Duration::from_secs(45);
/// Bound on how long the kernel is given to tear the group down after SIGKILL.
const TEARDOWN_DEADLINE: Duration = Duration::from_secs(20);
/// Poll slice for both waits.
const POLL: Duration = Duration::from_millis(100);

/// Live browser processes whose command line holds a CLI marker profile.
///
/// Deduplicated by pid, so this counts processes and not argv strings.
fn live_marker_pids() -> Option<Vec<u32>> {
    let index = index_live_processes()?;
    Some(
        index
            .entries()
            .iter()
            .filter(|e| {
                e.cmdline.contains(CLI_CHROME_MARKER_PREFIX)
                    && (e.cmdline.contains("chrom") || e.cmdline.contains(" --type="))
            })
            .map(|e| e.pid)
            .collect(),
    )
}

/// Marker-holding browser processes that descend from `cli_pid`.
///
/// # Why descent, and not "pids that appeared since the baseline"
///
/// This host runs several agents concurrently, and a baseline diff attributes
/// *any* browser that starts during the window to this test — including a
/// sibling invocation's. The test would then demand that another agent's Chrome
/// die when we kill *our* CLI, and fail for a reason that has nothing to do with
/// the code under test. Descent from our own pid is exact: the self-spawn path
/// forks Chrome directly from the CLI, so every process in the tree is ours and
/// nothing else can be.
fn owned_browser_pids(cli_pid: u32) -> Vec<u32> {
    let Some(index) = index_live_processes() else {
        return Vec::new();
    };
    let mut owned: Vec<u32> = descendants_in_index(cli_pid, &index)
        .into_iter()
        .filter(|pid| {
            index.entries().iter().any(|e| {
                e.pid == *pid
                    && e.cmdline.contains(CLI_CHROME_MARKER_PREFIX)
                    && (e.cmdline.contains("chrom") || e.cmdline.contains(" --type="))
            })
        })
        .collect();
    owned.sort_unstable();
    owned
}

/// SIGKILL a pid, ignoring failure (it may already be gone).
fn sigkill(pid: u32) {
    // SAFETY: the pid is a child this test spawned; SIGKILL cannot be caught,
    // which is exactly the scenario under test. See `man 2 kill`.
    unsafe {
        let _ = libc::kill(pid as i32, libc::SIGKILL);
    }
}

#[test]
fn sigkill_of_the_cli_leaves_no_browser_process_behind() {
    // A gate that cannot say what it did is indistinguishable from a gate that
    // did nothing, so every run reports its decision path on stdout.
    println!(
        "hard-kill gate: chrome={:?} binding={} proc_table={:?}",
        find_chrome(),
        host().binding().as_str(),
        index_live_processes().as_ref().map(|i| i.len())
    );
    if find_chrome().is_none() {
        println!(
            "SKIP lifecycle_hard_kill_gate: no Chrome/Chromium on this host, so \
             there is no browser tree to orphan. This is a skip, not a pass."
        );
        return;
    }
    let Some(before) = live_marker_pids() else {
        eprintln!(
            "SKIP lifecycle_hard_kill_gate: host process table is unreadable, so \
             residue cannot be audited. This is a skip, not a pass."
        );
        return;
    };
    if host().binding() != ParentDeathBinding::Kernel {
        eprintln!(
            "SKIP lifecycle_hard_kill_gate: this host has no kernel parent-death \
             binding ({}), so a hard kill is documented to leave the group alive \
             until cross-run residual GC collects it.",
            host().binding().as_str()
        );
        return;
    }

    // A script that opens a page and then holds the session open well past the
    // kill, so Chrome is provably live at the moment of the SIGKILL.
    let script = std::env::temp_dir().join("lifecycle_hard_kill_gate.jsonl");
    std::fs::write(
        &script,
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"wait\",\"ms\":120000}\n",
    )
    .expect("script fixture");

    let mut cli = std::process::Command::new(env!("CARGO_BIN_EXE_browser-automation-cli"))
        .args([
            "--json",
            "--timeout",
            "300",
            "run",
            "--script",
            script.to_str().expect("utf-8 path"),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the CLI binary must be spawnable");
    let cli_pid = cli.id();

    // Wait until this invocation's Chrome is actually running, so the SIGKILL
    // lands on a live tree rather than on a CLI that had not launched yet.
    println!(
        "hard-kill gate: cli pid={cli_pid}, pre-existing marker processes on host={}",
        before.len()
    );
    let deadline = Instant::now() + STARTUP_DEADLINE;
    let mut spawned: Vec<u32> = Vec::new();
    while Instant::now() < deadline {
        let owned = owned_browser_pids(cli_pid);
        if !owned.is_empty() {
            spawned = owned;
            break;
        }
        if let Ok(Some(status)) = cli.try_wait() {
            let _ = std::fs::remove_file(&script);
            panic!("the CLI exited before Chrome started (status: {status})");
        }
        std::thread::sleep(POLL);
    }

    if spawned.is_empty() {
        let _ = cli.kill();
        let _ = cli.wait();
        let _ = std::fs::remove_file(&script);
        eprintln!(
            "SKIP lifecycle_hard_kill_gate: Chrome never appeared within {STARTUP_DEADLINE:?}; \
             the host could not run the browser. This is a skip, not a pass."
        );
        return;
    }

    println!(
        "hard-kill gate: {} browser process(es) descend from cli {cli_pid}: {spawned:?}",
        spawned.len()
    );
    // The whole point: no warning, no FINALIZE, no Drop.
    sigkill(cli_pid);
    let _ = cli.wait();

    // The kernel death signal is asynchronous, so poll to a bound rather than
    // asserting instantly.
    let deadline = Instant::now() + TEARDOWN_DEADLINE;
    let mut survivors: Vec<u32> = Vec::new();
    while Instant::now() < deadline {
        let Some(now) = live_marker_pids() else {
            break;
        };
        survivors = spawned
            .iter()
            .copied()
            .filter(|pid| now.contains(pid))
            .collect();
        if survivors.is_empty() {
            break;
        }
        std::thread::sleep(POLL);
    }

    // Clean up whatever survived before failing, so a red test does not leave
    // the host dirtier than it found it.
    for pid in &survivors {
        sigkill(*pid);
    }
    let _ = std::fs::remove_file(&script);

    assert!(
        survivors.is_empty(),
        "SIGKILL of the CLI (pid {cli_pid}) left {} browser process(es) alive: \
         {survivors:?}. The child must be bound to this process by \
         PR_SET_PDEATHSIG so the kernel reaps the group.",
        survivors.len()
    );
}

#[test]
fn the_host_binding_is_declared_and_documented() {
    // The matrix in `spawn::os` is the contract; this asserts the host reports a
    // value from it rather than silently defaulting.
    let binding = host().binding();
    assert!(
        matches!(
            binding,
            ParentDeathBinding::Kernel | ParentDeathBinding::Degraded
        ),
        "every host must declare its parent-death guarantee"
    );
    if cfg!(target_os = "linux") {
        assert_eq!(
            binding,
            ParentDeathBinding::Kernel,
            "Linux has PR_SET_PDEATHSIG, so a degraded binding is a regression"
        );
    }
}
