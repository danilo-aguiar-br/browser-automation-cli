//! Xvfb: a headed launch on Linux renders into a private display and leaves
//! nothing behind.
//!
//! # What this gate is for
//!
//! `--no-xvfb` shipped for a whole version as a flag that changed the help
//! text and nothing else, because no code path ever spawned Xvfb. A flag that
//! promises and does not deliver is worse than an absent flag: the caller
//! reads the help, believes the display is private, and runs a headed browser
//! onto the operator's own screen.
//!
//! So the property under test is not "Xvfb starts". It is that the product's
//! *claim* about Xvfb matches what the host can actually do, and that a headed
//! run cleans up the display it created. A silent regression back to the
//! no-op flag would leave `doctor` reporting a capability the launch path no
//! longer uses.
//!
//! # Why this skips instead of failing
//!
//! Xvfb is an optional host package, and this product cannot install it: the
//! CLI does not invoke a package manager and does not invoke `sudo`. A host
//! without Xvfb is a supported host, so its absence is a skip with a printed
//! reason — never a red run that trains the reader to ignore red runs.

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

/// Whether an `Xvfb` binary is reachable on `PATH`.
///
/// Resolved the same way the product resolves it — by walking `PATH` — rather
/// than by shelling out to `which`, so the gate and the product cannot
/// disagree about what "available" means.
fn xvfb_on_path() -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join("Xvfb").is_file())
}

fn doctor_json() -> serde_json::Value {
    let out = Command::new(BIN)
        .args(["--json", "doctor", "--offline", "--quick"])
        .output()
        .expect("doctor must run");
    serde_json::from_slice(&out.stdout).expect("doctor must emit a JSON envelope")
}

fn xvfb_check(doc: &serde_json::Value) -> serde_json::Value {
    doc["data"]["checks"]
        .as_array()
        .expect("doctor must carry a checks array")
        .iter()
        .find(|c| c["id"] == "xvfb")
        .cloned()
        .expect("doctor must publish an `xvfb` check on every platform")
}

#[test]
fn doctor_reports_xvfb_and_never_omits_the_check() {
    // The check is unconditional by design: on a platform that always has a
    // compositor it reports `info` and `not applicable`. A check that appears
    // only on Linux is indistinguishable, from a parser's side, from a check
    // that was deleted.
    let check = xvfb_check(&doctor_json());
    let status = check["status"].as_str().unwrap_or_default();
    assert!(
        matches!(status, "pass" | "info" | "warn"),
        "unexpected xvfb status: {status}"
    );
    assert!(
        check["message"].as_str().is_some_and(|m| !m.is_empty()),
        "the xvfb check must explain itself, not just carry a status"
    );
}

#[test]
fn the_doctor_claim_matches_what_the_host_can_do() {
    // The regression this guards: `doctor` keeps saying Xvfb is usable after
    // the launch path stops using it, or keeps saying it is missing on a host
    // where it is installed. Either way the operator is told one thing and
    // gets another.
    if !cfg!(target_os = "linux") {
        eprintln!("skip: Xvfb is a Linux concern; this platform always has a compositor");
        return;
    }
    let check = xvfb_check(&doctor_json());
    assert_eq!(
        check["xvfb_present"].as_bool(),
        Some(xvfb_on_path()),
        "doctor disagrees with PATH about whether Xvfb exists"
    );
}

#[test]
fn a_headed_run_leaves_no_display_lock_behind() {
    // The property: whatever display the product allocated is released at DIE.
    // A leaked `/tmp/.X{N}-lock` makes the next run skip that number forever,
    // and a leaked socket is a file the operator did not ask this CLI to
    // create and will never think to look for.
    if !cfg!(target_os = "linux") {
        eprintln!("skip: no private display is allocated off Linux");
        return;
    }
    if !xvfb_on_path() {
        eprintln!("skip: Xvfb is not installed; install it to exercise the headed path");
        return;
    }

    let locks_before = display_locks();

    // about:blank keeps the assertion about the display, not about the
    // network: a headed launch is all that is needed to allocate one.
    let status = Command::new(BIN)
        .args(["--json", "--headed", "goto", "about:blank"])
        .output()
        .expect("headed goto must run");
    assert!(
        status.status.success() || !status.stdout.is_empty(),
        "the headed launch produced neither success nor an envelope"
    );

    let locks_after = display_locks();
    let leaked: Vec<_> = locks_after.difference(&locks_before).cloned().collect();
    assert!(
        leaked.is_empty(),
        "headed run leaked display locks: {leaked:?}"
    );
}

/// The set of `/tmp/.X{N}-lock` files present right now.
///
/// Compared as a *difference* across the run rather than as an absolute count,
/// because a developer machine legitimately runs its own X server and a
/// concurrent test may hold a display of its own.
fn display_locks() -> std::collections::BTreeSet<String> {
    let mut found = std::collections::BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(Path::new("/tmp")) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".X") && name.ends_with("-lock") {
            found.insert(name);
        }
    }
    found
}
