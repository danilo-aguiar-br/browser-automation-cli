// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit gates for the FINALIZE group kill and its pid-tree fallback.
//!
//! These do not need a browser: what is under test is the escalation itself —
//! that a whole process group dies from one signal, and that the fallback finds
//! descendants when no group is known.

#![cfg(unix)]

use std::time::Duration;

use browser_automation_cli::lifecycle::{
    descendants_in_index, kill_unix_group_graceful, kill_unix_tree,
};
use browser_automation_cli::native::cdp::spawn::{spawn_guarded, SpawnRequest};
use browser_automation_cli::residual::{LiveProcessIndex, ProcessEntry};

/// Poll interval while waiting for a signalled process to actually disappear.
const REAP_POLL: Duration = Duration::from_millis(20);
/// Upper bound on how long a reap may take before the test calls it a failure.
const REAP_DEADLINE: Duration = Duration::from_secs(5);

/// True while `pid` still exists (`kill(pid, 0)` returns 0).
fn pid_alive(pid: u32) -> bool {
    // SAFETY: signal 0 probes existence without delivering anything; the pid is
    // one this test spawned. See `man 2 kill`.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Wait until `pid` is gone, or return false at the deadline.
fn wait_gone(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + REAP_DEADLINE;
    while std::time::Instant::now() < deadline {
        if !pid_alive(pid) {
            return true;
        }
        std::thread::sleep(REAP_POLL);
    }
    false
}

/// Spawn `sh -c 'sleep 300'` through the product guard, or skip when unavailable.
fn spawn_sleeper() -> Option<(u32, Option<i32>, std::process::Child)> {
    let sh = browser_automation_cli::platform::which_bin("sh")?;
    let guarded = spawn_guarded(SpawnRequest {
        program: sh,
        args: vec!["-c".to_string(), "sleep 300".to_string()],
    })
    .ok()?;
    let pid = guarded.child.id();
    Some((pid, guarded.pgid, guarded.child))
}

#[test]
fn guarded_child_lands_in_its_own_process_group() {
    let Some((pid, pgid, mut child)) = spawn_sleeper() else {
        eprintln!("skip: no /bin/sh on this host");
        return;
    };
    let pgid = pgid.expect("unix hosts must report a process group");
    assert_eq!(
        pgid as u32, pid,
        "setpgid(0, 0) must make the child its own group leader; sharing our \
         group would make a group kill suicidal"
    );
    // SAFETY: our own group id, no preconditions. See `man 2 getpgrp`.
    let own = unsafe { libc::getpgrp() };
    assert_ne!(pgid, own, "the child must not stay in the CLI's group");

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn group_kill_reaps_the_whole_group_not_just_the_leader() {
    let Some(sh) = browser_automation_cli::platform::which_bin("sh") else {
        eprintln!("skip: no /bin/sh on this host");
        return;
    };
    // The leader forks a grandchild and then waits: killing only the leader pid
    // would leave the grandchild alive, which is exactly the residue this
    // escalation exists to prevent.
    let guarded = spawn_guarded(SpawnRequest {
        program: sh,
        args: vec!["-c".to_string(), "sleep 300 & echo $! ; wait".to_string()],
    })
    .expect("guard thread must fork the leader");
    let leader = guarded.child.id();
    let pgid = guarded
        .pgid
        .expect("unix hosts must report a process group");
    let mut child = guarded.child;

    // Read the grandchild pid the shell printed on stdout.
    let grandchild = {
        use std::io::Read;
        let mut out = child.stdout.take().expect("piped stdout");
        let mut buf = String::new();
        let mut byte = [0u8; 1];
        // Read exactly one line; the shell prints the pid immediately.
        while out.read(&mut byte).unwrap_or(0) == 1 {
            if byte[0] == b'\n' {
                break;
            }
            buf.push(byte[0] as char);
        }
        buf.trim().parse::<u32>().expect("shell must print a pid")
    };
    assert!(pid_alive(grandchild), "grandchild must start alive");

    kill_unix_group_graceful(pgid, Duration::from_millis(200));

    assert!(
        wait_gone(grandchild),
        "group kill must reach the grandchild"
    );
    let _ = child.wait();
    assert!(wait_gone(leader), "group kill must reach the leader");
}

#[test]
fn group_kill_refuses_our_own_group() {
    // SAFETY: our own group id, no preconditions. See `man 2 getpgrp`.
    let own = unsafe { libc::getpgrp() };
    // If this were not guarded the call would SIGTERM the test runner itself,
    // so simply surviving the call is the assertion.
    kill_unix_group_graceful(own, Duration::from_millis(10));
    assert!(pid_alive(std::process::id()), "we must still be alive");
}

#[test]
fn group_kill_refuses_init_and_the_zero_group() {
    kill_unix_group_graceful(0, Duration::ZERO);
    kill_unix_group_graceful(1, Duration::ZERO);
    assert!(pid_alive(std::process::id()));
}

#[test]
fn tree_fallback_walks_descendants_deepest_first() {
    // 10 → 20 → 30, plus an unrelated 40 that must never be selected.
    let index = LiveProcessIndex::from_entries(vec![
        ProcessEntry::new(10, None, "root"),
        ProcessEntry::new(20, Some(10), "child"),
        ProcessEntry::new(30, Some(20), "grandchild"),
        ProcessEntry::new(40, Some(999), "unrelated"),
    ]);

    let found = descendants_in_index(10, &index);
    assert_eq!(found.len(), 2, "only 20 and 30 descend from 10: {found:?}");
    assert!(
        !found.contains(&40),
        "an unrelated pid must never be killed"
    );
    assert!(!found.contains(&10), "the root is signalled separately");
    assert_eq!(
        found,
        vec![30, 20],
        "deepest first, so a parent cannot fork a replacement mid-walk"
    );
}

#[test]
fn tree_fallback_terminates_on_a_ppid_cycle() {
    // A corrupted table where 10 and 20 claim each other as parent must not loop.
    let index = LiveProcessIndex::from_entries(vec![
        ProcessEntry::new(10, Some(20), "a"),
        ProcessEntry::new(20, Some(10), "b"),
    ]);
    let found = descendants_in_index(10, &index);
    assert_eq!(found, vec![20], "the cycle must be visited exactly once");
}

#[test]
fn tree_fallback_reaps_a_real_child_when_no_group_is_known() {
    let Some((pid, _pgid, mut child)) = spawn_sleeper() else {
        eprintln!("skip: no /bin/sh on this host");
        return;
    };
    assert!(pid_alive(pid), "sleeper must start alive");

    // `None` forces the pid-tree path even though a group exists.
    kill_unix_tree(pid, None, Duration::from_millis(200));

    let _ = child.wait();
    assert!(wait_gone(pid), "pid-tree fallback must reap the child");
}
