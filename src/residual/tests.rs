// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual module unit tests.

use std::collections::HashSet;
use std::fs;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use super::classify::{
    cmdline_holds_path, entry_holds_path_protective, entry_is_browser_strict,
    entry_is_live_cli_chrome_strict, entry_is_owning_cli, is_google_chrome_tmp_name,
    is_singleton_only_or_small,
};
use super::proc::ProcessEntry;
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

/// The argv a real Chrome child carries, and that anything else can also carry.
///
/// This exact string is the regression: it is what the reproduction script was
/// launched with, and every predicate below is judged against it.
const BROWSER_SHAPED_ARGV: &str = concat!(
    "--user-data-dir=/tmp/browser-automation-cli-chrome-abc",
    " --type=renderer --headless=new"
);

/// A process that only *mentions* the marker is not a browser, whatever it says.
///
/// # Why the old version of this test proved nothing
///
/// It asserted on `"bash -c ls /tmp/browser-automation-cli-chrome-abc"`. That
/// string contains no `--user-data-dir=` and no `--type=`, so it failed the
/// browser-shape test on the FIRST condition and never reached the denylist the
/// test was named after. It passed by construction while the real input — a
/// shell carrying browser flags — was counted as a live Chrome, failed `doctor`
/// on a clean host, and put an unrelated pid in front of the reaper.
///
/// A fixture easier than the field is a fixture that measures nothing.
#[test]
fn impostor_shell_with_browser_flags_is_not_a_browser() {
    let impostor = ProcessEntry::new(
        4242,
        None,
        format!("/bin/bash run.sh {BROWSER_SHAPED_ARGV}"),
    )
    .with_exe("/usr/bin/bash");
    assert!(
        !entry_is_browser_strict(&impostor),
        "argv is self-declared; the kernel says this is bash"
    );
    assert!(
        !entry_is_live_cli_chrome_strict(&impostor),
        "counting this as a live CLI Chrome is what failed doctor on a clean host"
    );

    // The denylist that used to guard this cannot: it never listed a shell.
    for interpreter in [
        "/usr/bin/python3",
        "/usr/bin/node",
        "/bin/sh",
        "/usr/bin/perl",
    ] {
        let e = ProcessEntry::new(1, None, format!("{interpreter} x {BROWSER_SHAPED_ARGV}"))
            .with_exe(interpreter);
        assert!(
            !entry_is_browser_strict(&e),
            "{interpreter} must never classify as a browser"
        );
    }
}

/// The same argv IS a browser once the kernel agrees.
#[test]
fn same_argv_with_a_browser_exe_is_a_browser() {
    for exe in [
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome-stable",
        "/opt/microsoft/msedge/msedge",
        "C:\\Program Files\\Google\\Chrome\\chrome.exe",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome Helper (Renderer)",
    ] {
        let e = ProcessEntry::new(7, None, format!("{exe} {BROWSER_SHAPED_ARGV}")).with_exe(exe);
        assert!(
            entry_is_browser_strict(&e),
            "{exe} must classify as browser"
        );
        assert!(
            entry_is_live_cli_chrome_strict(&e),
            "{exe} holds our marker"
        );
    }
}

/// Unknown identity is ignorance, and the two polarities read it in opposite ways.
#[test]
fn unknown_exe_is_strict_false_but_still_protective() {
    let unknown = ProcessEntry::new(99, None, format!("? {BROWSER_SHAPED_ARGV}"));
    assert!(
        unknown.exe.is_none(),
        "fixture models an unreadable /proc/pid/exe"
    );

    // Verdict and kill: never act on what we cannot name.
    assert!(!entry_is_browser_strict(&unknown));
    assert!(!entry_is_live_cli_chrome_strict(&unknown));

    // Protection: a profile that might be in use is never collected. Deleting a
    // live sibling's profile is the expensive mistake; keeping a directory one
    // run too long is not.
    assert!(
        entry_holds_path_protective(&unknown, "/tmp/browser-automation-cli-chrome-abc"),
        "unknown identity must still pin the profile it names"
    );
}

/// Ownership is read from the kernel, so a URL in argv cannot revoke it.
///
/// The argv predicate rejects anything browser-shaped, because our marker prefix
/// contains the product binary name and every Chrome child would otherwise look
/// like its own owner. That rule has a mirror image: a real invocation whose argv
/// merely mentions `chromium` used to read as a browser, stop being an owner, and
/// leave its own live tree looking orphaned.
#[test]
fn owning_cli_is_identified_by_exe_not_by_argv() {
    let bin = crate::constants::PRODUCT_BIN_NAME;

    let scraping_chromium_org = ProcessEntry::new(
        10,
        None,
        format!("/usr/bin/{bin} scrape https://www.chromium.org --format text"),
    )
    .with_exe(format!("/usr/bin/{bin}"));
    assert!(
        entry_is_owning_cli(&scraping_chromium_org),
        "a live owner must not lose ownership because a URL says chromium"
    );

    // And the exclusion it replaces still holds: a Chrome child carries the
    // product name inside `--user-data-dir` and is never an owner.
    let chrome_child = ProcessEntry::new(11, None, format!("chromium {BROWSER_SHAPED_ARGV}"))
        .with_exe("/usr/bin/chromium");
    assert!(!entry_is_owning_cli(&chrome_child));
}

#[test]
fn missing_cli_marker_dir_detects_ghost_profile_path() {
    // Non-marker path is never a ghost profile.
    assert!(!super::report::is_missing_cli_marker_dir(
        "/tmp/other-chrome-profile"
    ));
    // Marker-shaped path that does not exist is a ghost.
    let ghost = std::env::temp_dir().join(format!(
        "{}ghost-{}",
        CLI_CHROME_MARKER_PREFIX,
        uuid::Uuid::new_v4()
    ));
    assert!(!ghost.exists());
    assert!(super::report::is_missing_cli_marker_dir(
        ghost.to_str().expect("utf8 path")
    ));
    // Existing marker dir is not a ghost.
    let live = marker_fixture("ghost-live");
    assert!(!super::report::is_missing_cli_marker_dir(
        live.to_str().expect("utf8 path")
    ));
    let _ = fs::remove_dir_all(&live);
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
