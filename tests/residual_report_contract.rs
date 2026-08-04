// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gates for the residual report contract: pid dedupe, scanned roots, escape hatch.

use std::collections::HashSet;
use std::time::{Duration, SystemTime};

use browser_automation_cli::residual::{
    abandoned_marker_dirs, browsers_pinning, browsers_reapable, residual_disk_report,
    write_owner_pid, LiveProcessIndex, ProcessEntry, CLI_CHROME_MARKER_PREFIX,
};

/// A browser-shaped command line holding `profile` as its user-data-dir.
fn chrome_cmdline(profile: &str, kind: &str) -> String {
    format!("/usr/lib/chromium/chromium --type={kind} --user-data-dir={profile} --headless=new")
}

/// A Chrome process: browser-shaped argv AND a browser executable.
///
/// The executable is what makes it a browser. Fixtures that set only the command
/// line describe a process claiming to be Chrome, which is a different thing and
/// has its own fixture below.
fn chrome_proc(pid: u32, ppid: Option<u32>, profile: &str, kind: &str) -> ProcessEntry {
    ProcessEntry::new(pid, ppid, chrome_cmdline(profile, kind))
        .with_exe("/usr/lib/chromium/chromium")
}

/// A process that merely names a profile, with no browser executable behind it.
fn impostor_proc(pid: u32, ppid: Option<u32>, cmdline: String, exe: &str) -> ProcessEntry {
    ProcessEntry::new(pid, ppid, cmdline).with_exe(exe)
}

#[test]
fn live_marker_processes_are_counted_once_per_pid() {
    let profile = format!("/tmp/{CLI_CHROME_MARKER_PREFIX}aaaa");
    // One invocation, four Chrome processes: renderer, gpu, utility, zygote.
    // The same pid is also listed twice, which is what the old string-counting
    // implementation multiplied instead of collapsing.
    let entries = vec![
        chrome_proc(101, None, &profile, "renderer"),
        chrome_proc(101, None, &profile, "renderer"),
        chrome_proc(102, Some(101), &profile, "gpu-process"),
        chrome_proc(103, Some(101), &profile, "utility"),
        impostor_proc(104, None, format!("rg --files {profile}"), "/usr/bin/rg"),
    ];
    let index = LiveProcessIndex::from_entries(entries);

    let counted = index.count_distinct_pids(|entry| {
        entry.cmdline.contains(CLI_CHROME_MARKER_PREFIX)
            && entry
                .exe
                .as_deref()
                .and_then(std::path::Path::to_str)
                .is_some_and(|exe| exe.contains("chromium"))
    });
    assert_eq!(
        counted, 3,
        "pid 101 appears twice and must collapse to one; the `rg` process is \
         not a browser and must not be counted at all"
    );
}

/// A process that only claims to be Chrome must not be counted as one.
///
/// The regression: `/bin/bash script.sh --user-data-dir=<marker> --type=renderer`
/// satisfied the old argv-substring classifier, so `ghost_marker_processes`
/// reported it and `doctor --offline --quick` exited 1 on a host with no Chrome
/// running at all. argv is written by the process; the executable is not.
#[test]
fn a_process_that_only_claims_to_be_chrome_is_not_counted() {
    let profile = format!("/tmp/{CLI_CHROME_MARKER_PREFIX}impostor");
    let index = LiveProcessIndex::from_entries(vec![
        impostor_proc(
            300,
            None,
            format!("/bin/bash run.sh --user-data-dir={profile} --type=renderer"),
            "/usr/bin/bash",
        ),
        // A real one alongside it, so the assertion cannot pass by counting zero.
        chrome_proc(301, None, &profile, "renderer"),
    ]);

    let counted = index.count_distinct_pids(|entry| {
        entry.cmdline.contains(CLI_CHROME_MARKER_PREFIX)
            && entry
                .exe
                .as_deref()
                .and_then(std::path::Path::to_str)
                .is_some_and(|exe| exe.contains("chromium"))
    });
    assert_eq!(
        counted, 1,
        "only the process with a browser executable counts"
    );
}

#[test]
fn report_emits_the_roots_it_scanned() {
    let report = residual_disk_report();
    assert!(
        !report.scanned_roots.is_empty(),
        "a zero count without its roots is unfalsifiable: chrome_profiles_dir \
         derives from cache_dir, so a different XDG_CACHE_HOME scans a \
         different tree and reports an honest but misleading zero"
    );
    for root in &report.scanned_roots {
        assert!(!root.is_empty(), "every scanned root must be a real path");
    }
}

#[test]
fn report_counts_never_exceed_the_marker_dirs_they_describe() {
    let report = residual_disk_report();
    assert!(
        report.sibling_live_processes <= report.cli_marker_dirs,
        "sibling invocations are a subset of the marker dirs on disk: {report:?}"
    );
    assert!(
        report.orphan_marker_dirs <= report.cli_marker_dirs,
        "orphans are a subset of the marker dirs on disk: {report:?}"
    );
}

#[test]
fn foreign_root_orphans_are_reported_separately() {
    // The field exists so residue outside the scanned roots stops being
    // invisible. On a clean host it is zero; the type-level guarantee that it is
    // reported at all is what this asserts.
    let report = residual_disk_report();
    let json = serde_json::to_value(&report).expect("report must serialize");
    assert!(
        json.get("foreign_root_orphans").is_some(),
        "the doctor envelope needs this field to explain a residue it cannot collect"
    );
    assert!(
        json.get("scanned_roots").is_some(),
        "the doctor envelope needs the roots to qualify its zeros"
    );
}

#[test]
fn a_live_owner_pid_protects_a_concurrent_invocation() {
    let temp = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}live-owner-gate"));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("fixture profile");
    write_owner_pid(&temp).expect("owner marker");

    // Our own pid is alive by construction, so this profile is a live sibling.
    let index = LiveProcessIndex::from_parts(HashSet::from([std::process::id()]), Vec::new());
    let abandoned = abandoned_marker_dirs(
        std::slice::from_ref(&temp),
        &index,
        SystemTime::now(),
        Duration::ZERO,
    );

    assert!(
        abandoned.is_empty(),
        "a profile whose owner pid is alive is a healthy concurrent \
         invocation and must never be force-collected"
    );
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn a_dead_owner_past_the_age_floor_becomes_collectable() {
    let temp = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}dead-owner-gate"));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("fixture profile");
    // pid 0 is never a live user process, so it stands in for a dead owner.
    std::fs::write(temp.join(".browser-automation-cli-owner-pid"), "0").expect("owner marker");

    let index = LiveProcessIndex::from_parts(HashSet::from([std::process::id()]), Vec::new());

    // Below the floor: still protected, because a brand-new invocation looks
    // exactly like this before its Chrome reaches the process table.
    let protected = abandoned_marker_dirs(
        std::slice::from_ref(&temp),
        &index,
        SystemTime::now(),
        Duration::from_secs(3600),
    );
    assert!(
        protected.is_empty(),
        "the age floor must protect a young profile"
    );

    // Above the floor: collectable, which is what breaks the mutual-protection
    // deadlock between an orphaned browser and the profile it pins.
    let collectable = abandoned_marker_dirs(
        std::slice::from_ref(&temp),
        &index,
        SystemTime::now(),
        Duration::ZERO,
    );
    assert_eq!(
        collectable.len(),
        1,
        "a dead owner past the floor is provable residue"
    );

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn an_unmarked_profile_is_never_force_collected() {
    let temp = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}no-owner-gate"));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("fixture profile");
    // No owner-pid file: the directory cannot be attributed to a dead owner.

    let index = LiveProcessIndex::from_parts(HashSet::from([std::process::id()]), Vec::new());
    let abandoned = abandoned_marker_dirs(
        std::slice::from_ref(&temp),
        &index,
        SystemTime::now(),
        Duration::ZERO,
    );
    assert!(
        abandoned.is_empty(),
        "without an owner marker the escape hatch has no proof and must defer \
         to the ordinary stale-GC path"
    );
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn browsers_pinning_matches_the_exact_user_data_dir() {
    let profile = std::path::PathBuf::from(format!("/tmp/{CLI_CHROME_MARKER_PREFIX}exact"));
    let other = format!("/tmp/{CLI_CHROME_MARKER_PREFIX}exact-but-different");
    let index = LiveProcessIndex::from_entries(vec![
        chrome_proc(201, None, &profile.display().to_string(), "renderer"),
        chrome_proc(202, None, &other, "renderer"),
    ]);

    let pinning = browsers_pinning(&profile, &index);
    assert_eq!(
        pinning,
        vec![201],
        "a prefix match would select 202 and kill an unrelated invocation"
    );
}

/// Pinning answers "what shape is this tree"; reaping answers "what may I kill".
///
/// `browsers_pinning` must stay unfiltered because `browsers_are_reparented`
/// reads it as topology: a pid is recognised as a child by finding its parent in
/// the same set. Under Flatpak the tree root is `bwrap`, which is not a browser
/// executable — filtering it out would leave its children parentless in the set,
/// re-read every one of them as an orphaned root, and turn a live tree into a
/// target. Identity therefore belongs at the point of destruction only.
#[test]
fn pinning_keeps_the_sandbox_root_that_reaping_refuses_to_kill() {
    let dir = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}flatpak-shape"));
    let profile = dir.display().to_string();
    let index = LiveProcessIndex::from_entries(vec![
        // Flatpak tree root: holds the profile, but the executable is the sandbox.
        impostor_proc(
            500,
            Some(1),
            chrome_cmdline(&profile, "browser"),
            "/usr/bin/bwrap",
        ),
        chrome_proc(501, Some(500), &profile, "renderer"),
    ]);

    let mut pinning = browsers_pinning(&dir, &index);
    pinning.sort_unstable();
    assert_eq!(
        pinning,
        vec![500, 501],
        "the sandbox root must stay in the pin set or the child looks like a root"
    );

    assert_eq!(
        browsers_reapable(&dir, &index),
        vec![501],
        "only the positively identified browser may ever be signalled"
    );
}

/// An unidentified holder makes the whole profile off-limits, not partly off-limits.
///
/// Killing the identified subset and wiping anyway would leave the unidentified
/// holder running against a deleted `--user-data-dir` — the exact definition of
/// `ghost_marker_processes`. A partial cleanup would manufacture the very defect
/// the report exists to catch, so the pass declines the directory entirely.
#[test]
fn an_unidentified_holder_makes_the_profile_off_limits() {
    let dir = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}mixed-holders"));
    let profile = dir.display().to_string();
    let index = LiveProcessIndex::from_entries(vec![
        chrome_proc(600, None, &profile, "renderer"),
        impostor_proc(
            601,
            None,
            format!("/usr/bin/python3 tool.py --user-data-dir={profile}"),
            "/usr/bin/python3",
        ),
    ]);

    let pinning = browsers_pinning(&dir, &index);
    let reapable = browsers_reapable(&dir, &index);
    assert_eq!(pinning.len(), 2, "both processes name the profile");
    assert_eq!(reapable, vec![600], "only one is a browser");
    assert_ne!(
        pinning.len(),
        reapable.len(),
        "this inequality is what makes reconcile skip the directory whole"
    );
}

/// The reaper must see profiles that only the process table knows about.
///
/// `list_cli_chrome_marker_dirs` walks `residual_scan_roots()`, and one of those
/// roots derives from `XDG_CACHE_HOME`. Two invocations under different cache
/// homes are mutually blind, so a profile outside our roots can never be listed
/// — while `foreign_root_orphans` reports it correctly, because detection reads
/// command lines rather than directories.
///
/// Measured on 2026-08-02: an orphaned tree under
/// `~/.cache/browser-automation-cli/chrome-profiles/` survived repeated 0.1.7
/// invocations whose cache home pointed elsewhere. The report named residue that
/// the reaper had no way to reach. This gate pins the union so detection and
/// action keep the same scope.
#[test]
fn reaper_discovers_marker_profiles_outside_the_scanned_roots() {
    let foreign = format!("/home/someone/.cache/browser-automation-cli/chrome-profiles/{CLI_CHROME_MARKER_PREFIX}ffff");
    let index = LiveProcessIndex::from_entries(vec![
        chrome_proc(900, None, &foreign, "renderer"),
        chrome_proc(901, Some(900), &foreign, "gpu-process"),
    ]);

    let found = browser_automation_cli::residual::marker_dirs_from_live_cmdlines(&index);
    assert_eq!(
        found,
        vec![std::path::PathBuf::from(&foreign)],
        "a profile named only in a live browser's argv must still reach the reaper"
    );
    // Deduplicated: four Chrome processes of one invocation are one profile.
    assert_eq!(found.len(), 1, "one invocation must yield one directory");
}

/// A directory that does not carry the product prefix is never adopted.
///
/// The cmdline filter already demands the marker, but the reaper kills processes
/// and deletes directories, so the path itself is re-checked rather than trusted
/// transitively.
#[test]
fn reaper_ignores_foreign_chromium_profiles() {
    let alien = "/tmp/some-other-tool-profile";
    let index = LiveProcessIndex::from_entries(vec![chrome_proc(910, None, alien, "renderer")]);
    assert!(
        browser_automation_cli::residual::marker_dirs_from_live_cmdlines(&index).is_empty(),
        "a Chromium profile without the product prefix is not ours to reap"
    );
}

/// A legacy profile with no owner marker is reaped when its browser is orphaned.
///
/// Profiles created before GAP-052 carry no `.browser-automation-cli-owner-pid`
/// file. Requiring that marker made them permanently unreapable: measured on
/// 2026-08-02, a tree from an earlier build survived every 0.1.7 invocation
/// because its directory holds only `Local State` and `BrowserMetrics-spare.pma`.
///
/// The substitute proof is read from the kernel instead of from a file: the tree
/// root's parent is not a live CLI of this product, so nothing owns it.
#[test]
fn legacy_profile_without_marker_is_reaped_when_reparented() {
    let dir = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}legacy-orphan"));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    // No write_owner_pid: that is the whole point of this gate.
    let profile = dir.display().to_string();
    let index = LiveProcessIndex::from_entries(vec![
        // Tree root, reparented to the user subreaper (pid 1 stands in for it).
        chrome_proc(700, Some(1), &profile, "browser"),
        // Child of the root: identified by its parent being inside the set.
        chrome_proc(701, Some(700), &profile, "renderer"),
        // The subreaper itself is not this product.
        impostor_proc(
            1,
            None,
            "/usr/lib/systemd/systemd --user".to_string(),
            "/usr/lib/systemd/systemd",
        ),
    ]);

    let abandoned = abandoned_marker_dirs(
        &[dir.clone()],
        &index,
        SystemTime::now() + Duration::from_secs(86_400),
        Duration::from_secs(1),
    );
    assert_eq!(
        abandoned,
        vec![dir.clone()],
        "an orphaned legacy tree must be collectable without an owner marker"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same legacy profile is protected while a real CLI still owns it.
///
/// This is the safety half of the previous gate: dropping the owner marker as a
/// requirement must not turn a healthy concurrent invocation into a target.
#[test]
fn legacy_profile_is_protected_while_its_cli_lives() {
    let dir = std::env::temp_dir().join(format!("{CLI_CHROME_MARKER_PREFIX}legacy-owned"));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    let profile = dir.display().to_string();
    let index = LiveProcessIndex::from_entries(vec![
        chrome_proc(800, Some(42), &profile, "browser"),
        // A live CLI of this product is the parent: the tree is owned.
        impostor_proc(
            42,
            None,
            format!(
                "/usr/bin/{} run --script steps.jsonl",
                env!("CARGO_PKG_NAME")
            ),
            &format!("/usr/bin/{}", env!("CARGO_PKG_NAME")),
        ),
    ]);

    let abandoned = abandoned_marker_dirs(
        &[dir.clone()],
        &index,
        SystemTime::now() + Duration::from_secs(86_400),
        Duration::from_secs(1),
    );
    assert!(
        abandoned.is_empty(),
        "a tree whose parent is a live CLI must never be reaped"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The `residual_disk` check and `data.residual` must describe the same thing.
///
/// # Why this can drift
///
/// `data.residual` serialises `ResidualDiskReport` through serde, so a new field
/// appears there for free. The check builds its object field-by-field inside a
/// `json!`, so the same field appears there only if someone remembers. Measured
/// on 2026-08-02: `scanned_roots` and `foreign_root_orphans` shipped in 0.1.7,
/// reached `data.residual`, and never reached the check — two views of one fact
/// disagreeing about what the fact even consists of.
///
/// A consumer reading the check would conclude residual-zero without ever
/// learning which roots were scanned, which is precisely the blindness those two
/// fields were added to remove.
#[test]
fn the_residual_check_carries_every_field_the_report_does() {
    // `CARGO_BIN_EXE_` and not `cargo_bin`: the latter resolves to a path any
    // build overwrites, with no guarantee it matches this test's feature set.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_browser-automation-cli"))
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output()
        .expect("spawn cli");
    let env: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("doctor emits one JSON envelope");

    let report = env["data"]["residual"]
        .as_object()
        .expect("data.residual is an object");
    let check = env["data"]["checks"]
        .as_array()
        .expect("data.checks is an array")
        .iter()
        .find(|c| c["id"] == "residual_disk")
        .expect("residual_disk check present")
        .as_object()
        .expect("check is an object");

    // Envelope-only keys the check adds; everything else must be shared.
    const CHECK_ONLY: &[&str] = &["id", "status", "message"];

    let missing: Vec<&String> = report
        .keys()
        .filter(|k| !check.contains_key(k.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "residual_disk check is missing fields that data.residual reports: {missing:?}"
    );

    let extra: Vec<&String> = check
        .keys()
        .filter(|k| !CHECK_ONLY.contains(&k.as_str()) && !report.contains_key(k.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "residual_disk check invents fields absent from data.residual: {extra:?}"
    );
}

/// A live sibling invocation must never fail the check.
///
/// GAP-002/GAP-006 established that overlapping invocations are healthy: only a
/// marker dir past the age floor with a dead owner proves DIE failed. The status
/// ladder encodes that, and this pins it so a future "tighten the gate" change
/// cannot quietly turn concurrency into a failure.
#[test]
fn the_residual_check_verdict_depends_on_orphans_not_on_raw_counts() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_browser-automation-cli"))
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output()
        .expect("spawn cli");
    let env: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("doctor emits one JSON envelope");
    let check = env["data"]["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .find(|c| c["id"] == "residual_disk")
        .expect("residual_disk check present");

    let orphans = check["orphan_marker_dirs"].as_u64().unwrap_or(0);
    let ghosts = check["ghost_marker_processes"].as_u64().unwrap_or(0);
    let status = check["status"].as_str().unwrap_or("");

    if orphans == 0 && ghosts == 0 {
        assert_ne!(
            status, "fail",
            "zero orphans/ghosts must never fail: raw marker counts include live siblings"
        );
    } else {
        assert_eq!(
            status, "fail",
            "a proven orphan or ghost holder must fail the check"
        );
    }

    // The message has to name the verdict, not only the counters, or a reader
    // cannot tell a healthy concurrent host from an abandoned one.
    let message = check["message"].as_str().unwrap_or("");
    assert!(
        message.contains("clean")
            || message.contains("live siblings only")
            || message.contains("abandoned residue")
            || message.contains("ghost holders"),
        "message must state the verdict in words, got: {message}"
    );
}
