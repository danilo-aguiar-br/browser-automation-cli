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
    // A `TempDir` guard: the `remove_dir_all` below is on the happy path only.
    // The marker prefix survives the move, which is what matters — the product
    // DISCOVERS these directories by that prefix, and the suffix is free.
    let dir_guard = tempfile::Builder::new()
        .prefix("browser-automation-cli-chrome-test-")
        .tempdir()
        .expect("create marker fixture");
    let dir = dir_guard.path().to_path_buf();
    // Snapshot first, then confirm the fixture outlived it. A concurrent CLI
    // invocation on this host runs BORN GC over the same roots and can collect
    // the fixture mid-test; asserting anyway would measure the neighbour.
    let found = list_cli_chrome_marker_dirs();
    let survived = dir.exists();
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
    // A `TempDir` guard: the `remove_dir_all` at the end of this test is on the
    // happy path only, so a failing assertion left the sandbox on disk. `uuid`
    // already gave the name uniqueness; what it never gave was removal on unwind.
    let root_guard = tempfile::Builder::new()
        .prefix("bac-stale-gc-")
        .tempdir()
        .expect("mkdir sandbox root");
    let root = root_guard.path().to_path_buf();
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
    let wiped = scavenge_stale_singleton_orphans_in_roots(&[root], Duration::ZERO);
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
        // Same shared-root window as `stamp_or_skip` documents: this case
        // stamps nothing, so the fixture spends its whole life owner-less and
        // a concurrent sweep may take it before the wipe is even called.
        if !dir.exists() {
            eprintln!(
                "wipe_refuses_when_process_table_unavailable: fixture was swept \\
                 by a concurrent invocation; assertion skipped"
            );
            return;
        }
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
    if !stamp_or_skip(
        &dir,
        &sibling_pid.to_string(),
        "wipe_spares_profile_of_live_sibling",
    ) {
        return;
    }
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
    if !stamp_or_skip(
        &dir,
        "999999",
        "dead_owner_still_spared_while_orphaned_browser_holds_it",
    ) {
        return;
    }
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

/// Stamp a fixture with `pid`, or report that a neighbour swept it first.
///
/// # Why stamping can fail at all
///
/// The fixture lives under the SHARED scan roots and `marker_fixture` creates
/// the directory BEFORE anything stamps it. In that window the directory
/// carries the CLI marker prefix with no owner behind it, which is exactly the
/// shape `wipe.rs:44` is entitled to collect, and `SHARED_ROOTS` only
/// serializes tests inside THIS process: any other invocation on this host —
/// an integration gate launching the binary, a developer's own run — sweeps
/// the same roots at BORN.
///
/// Measured 2026-09-04 on two sibling tests that assert through the discovery
/// path: one failed once at 1077 of 1078 in a full-suite run and passed 5/5
/// alone. The tests below assert on the WIPE rather than on discovery, so they
/// have never been seen to fail, but the window is the same one and a vanished
/// fixture would make them measure the neighbour instead of the wipe.
///
/// Returning `false` lets the caller skip out loud rather than panic inside
/// `expect`, which would report a write error and hide the real cause.
fn stamp_or_skip(dir: &std::path::Path, pid: &str, test: &str) -> bool {
    if fs::write(owner_pid_path(dir), pid).is_err() || !dir.exists() {
        // Never silent: a fixture swept on every run would otherwise turn the
        // gate into an empty green.
        eprintln!(
            "{test}: fixture was swept by a concurrent invocation before it \
             could be stamped; assertions skipped"
        );
        return false;
    }
    true
}

/// A supervised teardown is not residue; a disowned browser is.
///
/// FINALIZE removes the profile dir and then waits for Chrome to exit, so every
/// healthy shutdown spends a window looking exactly like a ghost. Measured
/// 2026-08-06: that window read `ghost_marker_processes=22` on a host that was
/// clean minutes later with no cleanup performed, and it turned
/// `concurrent_invocation_keeps_doctor_exit_zero` red.
///
/// The three cases below are the whole rule. Without the second one the counter
/// is blind and the leak it exists to catch walks straight through; without the
/// first it fails healthy concurrency; without the third it re-inflates on
/// Chrome subprocesses.
#[test]
fn disowned_ghosts_are_counted_and_supervised_ones_are_not() {
    // Path under a marker name that does not exist: "profile already deleted".
    let missing = std::env::temp_dir()
        .join("browser-automation-cli-chrome-ghost-fixture-never-created")
        .display()
        .to_string();
    let browser = format!("/usr/bin/chromium --user-data-dir={missing}");

    // Identity comes from the kernel-reported executable, never from argv, so a
    // fixture without `with_exe` is not a browser at all and every assertion
    // below would pass while measuring nothing. Caught exactly that way on the
    // first run of this test: case 1 was green because the entry was invisible.
    let cli_exe = format!("/usr/bin/{}", crate::constants::PRODUCT_BIN_NAME);
    let chrome = |pid: u32, ppid: u32, cmd: &str| {
        ProcessEntry::new(pid, Some(ppid), cmd).with_exe("/usr/bin/chromium")
    };

    // 1. SUPERVISED: the launcher is alive, so it still owns the teardown.
    let cli = ProcessEntry::new(100, Some(1), "browser-automation-cli goto").with_exe(&cli_exe);
    let index = super::proc::LiveProcessIndex::from_entries(vec![cli, chrome(200, 100, &browser)]);
    assert_eq!(
        super::report::count_disowned_ghosts(&index),
        0,
        "a browser whose launcher is still alive is mid-teardown, not residue"
    );

    // 2. DISOWNED: same browser, launcher gone. Nobody is left to collect it.
    let index = super::proc::LiveProcessIndex::from_entries(vec![chrome(200, 1, &browser)]);
    assert_eq!(
        super::report::count_disowned_ghosts(&index),
        1,
        "a browser with no live launcher is exactly the leak this field catches"
    );

    // 3. SUBPROCESS: renderer parented by the ghost root counts once, not twice.
    let renderer_cmd = format!("{browser} --type=renderer");
    let index = super::proc::LiveProcessIndex::from_entries(vec![
        chrome(200, 1, &browser),
        chrome(201, 200, &renderer_cmd),
    ]);
    assert_eq!(
        super::report::count_disowned_ghosts(&index),
        1,
        "only tree roots are judged; Chrome children must not inflate the count"
    );
}

/// The `/tmp` singleton directory is claimed from the profile's own symlink.
///
/// The regression this guards is a measured leak: every browser launch left
/// one `/tmp/org.chromium.Chromium.*` behind, because the scan-and-match path
/// found nothing to match on. Chrome names neither the pid nor the profile in
/// that directory, and it creates the directory during startup — before the
/// timestamp the scan compares against. The symlink is the only deterministic
/// link between our profile and Chrome's temp directory.
#[cfg(unix)]
#[test]
fn the_tmp_singleton_dir_is_claimed_through_the_profile_symlink() {
    // The profile takes a `TempDir` guard so a failing assertion below cannot
    // keep it. `tmp_dir` deliberately does NOT: the `org.chromium.Chromium.<8
    // hex>` SHAPE of that name is the predicate under test, and `TempDir` picks
    // the suffix itself, so a guard there would change what this measures.
    let profile_guard = tempfile::Builder::new()
        .prefix("bac-sym-prof-")
        .tempdir()
        .expect("mkdir profile");
    let profile = profile_guard.path().to_path_buf();
    let tmp_dir = std::env::temp_dir().join(format!(
        "org.chromium.Chromium.{}",
        &uuid::Uuid::new_v4().to_string()[..8]
    ));
    fs::create_dir_all(&tmp_dir).expect("mkdir chromium tmp");
    let target = tmp_dir.join("SingletonSocket");
    let _ = fs::write(&target, b"");
    std::os::unix::fs::symlink(&target, profile.join("SingletonSocket")).expect("symlink");

    let claimed = owned_chromium_tmp_dir_via_profile(&profile);
    assert_eq!(
        claimed.as_ref(),
        Some(&tmp_dir),
        "the profile symlink must resolve to the directory Chrome created"
    );

    let _ = fs::remove_dir_all(&tmp_dir);
}

/// A symlink pointing somewhere that is not a Chromium temp dir is refused.
///
/// The return value feeds a recursive delete, so a target that does not carry
/// the Chromium name is left alone. Leaking a directory is a smaller harm than
/// deleting one this product does not own.
#[cfg(unix)]
#[test]
fn a_symlink_outside_the_chromium_shape_is_never_claimed() {
    // A `TempDir` guard: the `remove_dir_all` below runs only on the happy path,
    // so a failing assertion left the fixture on disk. `uuid` already gave the
    // name uniqueness; what it never gave was removal under unwind.
    let profile_guard = tempfile::Builder::new()
        .prefix("bac-sym-bad-")
        .tempdir()
        .expect("mkdir profile");
    let profile = profile_guard.path().to_path_buf();
    let decoy_guard = tempfile::Builder::new()
        .prefix("bac-not-chromium-")
        .tempdir()
        .expect("mkdir decoy");
    let decoy = decoy_guard.path().to_path_buf();
    let target = decoy.join("SingletonSocket");
    let _ = fs::write(&target, b"");
    std::os::unix::fs::symlink(&target, profile.join("SingletonSocket")).expect("symlink");

    assert!(
        owned_chromium_tmp_dir_via_profile(&profile).is_none(),
        "a target without the Chromium temp name must never be handed to a delete"
    );
}

/// A profile with no symlink yields nothing, rather than a parent directory.
#[test]
fn a_profile_without_the_symlink_claims_nothing() {
    // A `TempDir` guard: the `remove_dir_all` below runs only on the happy path,
    // so a failing assertion left the fixture on disk. `uuid` already gave the
    // name uniqueness; what it never gave was removal under unwind.
    let profile_guard = tempfile::Builder::new()
        .prefix("bac-sym-none-")
        .tempdir()
        .expect("mkdir profile");
    let profile = profile_guard.path().to_path_buf();
    assert!(owned_chromium_tmp_dir_via_profile(&profile).is_none());
}

/// A LIVE sibling's profile is never claimed by this invocation's FINALIZE.
///
/// # The defect this pins
///
/// `discover_owned_chromium_tmp_side_channels` accepted any marker directory
/// that was recent and owned by this uid, under the comment "always ours when
/// marker + recent + our uid". None of those three identifies an invocation:
/// the prefix is shared by every run of the product, the uid by every process
/// this user starts, and "recent" is exactly what a LATER, still-running
/// sibling's profile looks like.
///
/// The result was a cross-invocation delete. A finishing one-shot collected a
/// live sibling's profile and `wipe_owned_path` removed it — no age floor, no
/// liveness check — while that sibling's Chrome was still booting. Measured
/// 2026-08-18 in `cargo test --tests`: exit 21 with `Failed to create
/// <profile>/SingletonLock: No such file or directory`, one victim per full
/// run, a different test each time, and never reproducible alone because it
/// takes a second invocation to happen at all.
#[test]
fn a_marker_profile_owned_by_another_pid_is_not_claimed() {
    let _shared_roots = shared_roots_guard();
    let sibling = marker_fixture("sibling-live");
    // pid 1 is init: always live, and definitively not this process.
    fs::write(super::owner::owner_pid_path(&sibling), "1").expect("stamp foreign owner");

    let found = super::discover::discover_owned_chromium_tmp_side_channels(
        None,
        None,
        SystemTime::UNIX_EPOCH,
    );
    let claimed = found.iter().any(|p| p == &sibling);
    let _ = fs::remove_dir_all(&sibling);

    assert!(
        !claimed,
        "a profile stamped with another pid belongs to that invocation, \
         and FINALIZE must not delete it"
    );
}

/// Our OWN profile is still claimed, so the fix does not disarm the cleanup.
///
/// Without this the change above could be satisfied by claiming nothing at all,
/// which would leak one profile directory per launch.
#[test]
fn our_own_marker_profile_is_still_claimed() {
    let _shared_roots = shared_roots_guard();
    let mine = marker_fixture("own-profile");
    super::owner::write_owner_pid(&mine).expect("stamp own owner");

    let found = super::discover::discover_owned_chromium_tmp_side_channels(
        Some(&mine),
        None,
        SystemTime::UNIX_EPOCH,
    );
    // Read survival AFTER the scan: the fixture lives under the SHARED roots,
    // and `SHARED_ROOTS` only serializes this process. Any other invocation on
    // this host -- an integration gate launching the binary, a developer's own
    // run -- sweeps the same roots at BORN, and `wipe.rs:44` collects every
    // `browser-automation-cli-chrome-` directory with no live process behind
    // it. `marker_fixture` creates the directory and only then stamps the
    // owner pid, so between those two calls the fixture is indistinguishable
    // from an orphan and a concurrent sweep is entitled to delete it.
    //
    // Measured 2026-09-04: this test failed once inside a full-suite run and
    // passed 5/5 alone and 5/5 under `--lib` alone, which is the signature of a
    // neighbour and not of this code. Asserting through a vanished fixture
    // would measure the neighbour, exactly as the sibling case above already
    // documents.
    let survived = mine.exists();
    let claimed = found.iter().any(|p| p == &mine);
    let _ = fs::remove_dir_all(&mine);

    if !survived {
        // Never silent: a permanently swept fixture would turn this gate into
        // an empty green, and the line says so out loud under `--nocapture`.
        eprintln!(
            "our_own_marker_profile_is_still_claimed: fixture was swept by a \
             concurrent invocation before the scan finished; assertion skipped"
        );
        return;
    }

    assert!(
        claimed,
        "the profile this invocation owns must still be collected at FINALIZE"
    );
}

/// A stamped profile is claimed by the process whose pid it names, path aside.
///
/// The `profile` argument is not the only way to be the owner: a launch may
/// allocate more than one marker directory, and the stamp is what says which
/// process they belong to.
#[test]
fn a_profile_stamped_with_our_own_pid_is_claimed_without_the_profile_arg() {
    let _shared_roots = shared_roots_guard();
    let mine = marker_fixture("own-stamp");
    super::owner::write_owner_pid(&mine).expect("stamp own owner");

    let found = super::discover::discover_owned_chromium_tmp_side_channels(
        None,
        None,
        SystemTime::UNIX_EPOCH,
    );
    // Same shape and same window as the sibling above: `marker_fixture`
    // creates the directory and only then stamps the owner pid, so in between
    // the fixture is indistinguishable from an orphan and `wipe.rs:44` on any
    // concurrent invocation of this host is entitled to collect it.
    // `SHARED_ROOTS` serializes this process, never the host, so read survival
    // AFTER the scan rather than asserting through a fixture that may be gone.
    let survived = mine.exists();
    let claimed = found.iter().any(|p| p == &mine);
    let _ = fs::remove_dir_all(&mine);

    if !survived {
        // Never silent: a fixture swept every time would turn this gate into
        // an empty green, and the line says so out loud under `--nocapture`.
        eprintln!(
            "a_profile_stamped_with_our_own_pid_is_claimed_without_the_profile_arg: \\
             fixture was swept by a concurrent invocation before the scan \\
             finished; assertion skipped"
        );
        return;
    }

    assert!(claimed, "our own pid in the marker is proof of ownership");
}
