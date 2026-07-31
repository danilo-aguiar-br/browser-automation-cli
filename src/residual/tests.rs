// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual module unit tests.

use std::collections::HashSet;
use std::fs;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use super::classify::{
    cmdline_holds_path, is_google_chrome_tmp_name, is_live_cli_chrome_cmdline,
    is_singleton_only_or_small,
};
use super::wipe::wipe_safe_candidates_with_index;
use super::*;

#[test]
fn marker_scan_finds_and_filters() {
    let _shared_roots = shared_roots_guard();
    let tmp = std::env::temp_dir();
    let dir = tmp.join(format!(
        "browser-automation-cli-chrome-test-{}",
        uuid::Uuid::new_v4()
    ));
    let _ = fs::create_dir_all(&dir);
    // Snapshot first, then confirm the fixture outlived it. A concurrent CLI
    // invocation on this host runs BORN GC over the same roots and can collect
    // the fixture mid-test; asserting anyway would measure the neighbour.
    let found = list_cli_chrome_marker_dirs();
    let survived = dir.exists();
    let _ = fs::remove_dir_all(&dir);
    if survived {
        assert!(
            found.iter().any(|p| p == &dir),
            "expected marker dir in list"
        );
    }
}

#[test]
fn discover_respects_not_before() {
    let future = SystemTime::now() + Duration::from_secs(3600);
    let found = discover_owned_chromium_tmp_side_channels(None, None, future);
    assert!(
        found.is_empty(),
        "future not_before must yield no side channels"
    );
}

#[test]
fn stale_gc_removes_singleton_only_fixture() {
    // A private root, because the floor below is ZERO. Against the shared roots a
    // zero floor also matches the profile a concurrent invocation created
    // microseconds ago, before Chrome exists to hold it — the liveness guard
    // cannot protect what has not been spawned yet. Narrowing the roots keeps the
    // predicate under test intact and takes the neighbours out of range.
    let root = std::env::temp_dir().join(format!("bac-stale-gc-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("mkdir sandbox root");
    let dir = root.join(format!(
        "org.chromium.Chromium.{}",
        &uuid::Uuid::new_v4().to_string()[..8]
    ));
    let _ = fs::create_dir_all(&dir);
    // Singleton-shaped contents.
    let cookie = dir.join("SingletonCookie");
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink("12345", &cookie);
    #[cfg(not(unix))]
    let _ = fs::write(&cookie, b"12345");
    let sock = dir.join("SingletonSocket");
    // Regular empty file standing in for a dead socket.
    let _ = fs::write(&sock, b"");

    assert!(is_singleton_only_or_small(&dir));
    // Age floor zero in tests so we do not depend on utimensat/filetime.
    let wiped = scavenge_stale_singleton_orphans_in_roots(&[root.clone()], Duration::ZERO);
    if index_live_processes().is_some() {
        assert!(
            wiped.iter().any(|p| p == &dir) || !dir.exists(),
            "stale singleton fixture must be wiped: wiped={wiped:?} exists={}",
            dir.exists()
        );
    } else {
        // GAP-045: no process table means no proof of liveness, so the collector
        // refuses the wipe by design and the fixture must survive.
        assert!(
            wiped.is_empty() && dir.exists(),
            "fail-closed GC must keep the fixture: wiped={wiped:?}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn residual_disk_report_is_finite() {
    let _shared_roots = shared_roots_guard();
    let r = residual_disk_report();
    // Just ensure fields are accessible and non-panicking.
    let _ = r.cli_marker_dirs
        + r.chromium_tmp_singleton_orphans
        + r.scavenge_safe_candidates
        + r.live_cli_marker_processes;
}

#[test]
fn google_chrome_tmp_names_excluded_from_stale_gc_list() {
    assert!(is_google_chrome_tmp_name(".com.google.Chrome.XYZ"));
    assert!(is_google_chrome_tmp_name("com.google.Chrome.XYZ"));
    assert!(!is_google_chrome_tmp_name("org.chromium.Chromium.XYZ"));
}

#[test]
fn live_cli_chrome_cmdline_ignores_shell_mentions() {
    let shell = "bash -c ls /tmp/browser-automation-cli-chrome-abc";
    assert!(!is_live_cli_chrome_cmdline(shell));
    let chrome = "/usr/bin/chromium-browser --user-data-dir=/tmp/browser-automation-cli-chrome-abc --headless=new";
    assert!(is_live_cli_chrome_cmdline(chrome));
}

/// GAP-045: an unavailable process table must never authorize a wipe.
#[test]
fn wipe_refuses_when_process_table_unavailable() {
    let _shared_roots = shared_roots_guard();
    let dir = marker_fixture("unavailable");
    // `wipe_safe_candidates` resolves the index itself; simulate `None` by asserting
    // the documented contract of the public entry point on hosts without a backend,
    // and prove the fixture survives when liveness cannot be established.
    if index_live_processes().is_none() {
        let wiped = super::wipe::wipe_safe_candidates(&[dir.clone()]);
        assert!(
            wiped.is_empty() && dir.exists(),
            "unavailable process table must refuse the wipe: wiped={wiped:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// GAP-045/GAP-052: a live sibling's profile is never collected.
#[test]
fn wipe_spares_profile_of_live_sibling() {
    let _shared_roots = shared_roots_guard();
    let dir = marker_fixture("live-sibling");
    // Simulated sibling: an owner pid that the index reports as alive.
    let sibling_pid = 424_242_u32;
    fs::write(owner_pid_path(&dir), sibling_pid.to_string()).expect("write owner pid");
    let index = LiveProcessIndex::from_parts(HashSet::from([sibling_pid]), Vec::new());

    let wiped = wipe_safe_candidates_with_index(&[dir.clone()], &index);
    assert!(
        wiped.is_empty() && dir.exists(),
        "live sibling profile must survive: wiped={wiped:?}"
    );

    // Same fixture, owner now dead: collection proceeds.
    let empty = LiveProcessIndex::from_parts(HashSet::from([1_u32]), Vec::new());
    let wiped = wipe_safe_candidates_with_index(&[dir.clone()], &empty);
    assert!(
        wiped.contains(&dir) && !dir.exists(),
        "dead owner profile must be collected: wiped={wiped:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// GAP-052: an editor or `rg` mentioning the path must not pin it forever.
#[test]
fn cmdline_fallback_ignores_non_browser_mentions() {
    let path = "/tmp/browser-automation-cli-chrome-abc";
    assert!(!cmdline_holds_path(
        "nvim /tmp/browser-automation-cli-chrome-abc/x",
        path
    ));
    assert!(!cmdline_holds_path(
        "rg foo /tmp/browser-automation-cli-chrome-abc",
        path
    ));
    assert!(cmdline_holds_path(
        "/usr/bin/chromium --user-data-dir=/tmp/browser-automation-cli-chrome-abc",
        path
    ));
}

/// GAP-052: a dead owner pid frees the profile only when no browser still holds it.
#[test]
fn dead_owner_still_spared_while_orphaned_browser_holds_it() {
    let _shared_roots = shared_roots_guard();
    let dir = marker_fixture("orphan-browser");
    fs::write(owner_pid_path(&dir), "999999").expect("write owner pid");
    assert_eq!(read_owner_pid(&dir), Some(999_999));
    assert!(has_owner_pid(&dir));

    // Owner CLI is gone, but an orphaned Chrome is still writing to the profile.
    let holder = format!("/usr/bin/chromium --user-data-dir={}", dir.display());
    let index = LiveProcessIndex::from_parts(HashSet::from([1_u32]), vec![holder]);
    let wiped = wipe_safe_candidates_with_index(&[dir.clone()], &index);
    assert!(
        wiped.is_empty() && dir.exists(),
        "orphaned browser must still protect the profile: wiped={wiped:?}"
    );

    // Only a non-browser mention leaves it collectable.
    let editor = format!("nvim {}/prefs", dir.display());
    let index = LiveProcessIndex::from_parts(HashSet::from([1_u32]), vec![editor]);
    let wiped = wipe_safe_candidates_with_index(&[dir.clone()], &index);
    assert!(
        wiped.contains(&dir),
        "editor mention must not pin the profile: wiped={wiped:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// GAP-003: every root the report observes is a root the collector scans.
#[test]
fn observed_roots_are_collected_roots() {
    let _shared_roots = shared_roots_guard();
    let roots = super::roots::residual_scan_roots();
    for root in &roots {
        let dir = root.join(format!(
            "{}assert-{}",
            CLI_CHROME_MARKER_PREFIX,
            uuid::Uuid::new_v4()
        ));
        if fs::create_dir_all(&dir).is_err() {
            continue;
        }
        // Take both snapshots first, then confirm the fixture outlived them. Any
        // concurrent CLI invocation on this host may collect the fixture between
        // the two reads; when that happens there is nothing left to prove, and
        // asserting anyway would only measure the neighbour's timing.
        let observed = list_cli_chrome_marker_dirs();
        let candidates = super::discover::discover_stale_singleton_candidates(Duration::ZERO);
        let survived = dir.exists();
        let _ = fs::remove_dir_all(&dir);
        if !survived {
            continue;
        }
        assert!(
            observed.iter().any(|p| p == &dir),
            "marker under {} must be observed",
            root.display()
        );
        assert!(
            candidates.iter().any(|p| p == &dir),
            "marker under {} must be a collection candidate",
            root.display()
        );
    }
}

/// The owner-pid marker must not change Singleton-shape classification.
#[test]
fn owner_pid_file_keeps_singleton_shape() {
    let _shared_roots = shared_roots_guard();
    let dir = marker_fixture("shape");
    let _ = fs::write(dir.join("SingletonSocket"), b"");
    fs::write(owner_pid_path(&dir), "1").expect("write owner pid");
    assert!(is_singleton_only_or_small(&dir));
    let _ = fs::remove_dir_all(&dir);
}

/// Serializes tests that create fixtures under the **shared** scan roots or run
/// a global zero-floor scavenge.
///
/// A global scavenge wipes every collectable path, including another test's
/// fixture, so these tests cannot overlap. The guard keeps the default test
/// parallelism instead of forcing `--test-threads 1`.
static SHARED_ROOTS: Mutex<()> = Mutex::new(());

/// Take the shared-root guard, recovering from a poisoned lock.
fn shared_roots_guard() -> MutexGuard<'static, ()> {
    SHARED_ROOTS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Create an empty CLI marker profile under the OS temp root.
fn marker_fixture(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{}{}-{}",
        CLI_CHROME_MARKER_PREFIX,
        tag,
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&dir).expect("create marker fixture");
    dir
}
