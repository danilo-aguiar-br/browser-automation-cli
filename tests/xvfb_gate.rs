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

mod common;

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
    let out = common::cmd()
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
        common::skip_with_reason(
            "xvfb_gate::preflight",
            "Xvfb is a Linux concern; this platform always has a compositor.",
        );
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
        common::skip_with_reason(
            "xvfb_gate::private_display",
            "no private display is allocated off Linux.",
        );
        return;
    }
    if !xvfb_on_path() {
        common::skip_with_remedy(
            "xvfb_gate::private_display",
            "Xvfb is not installed.",
            "install Xvfb to exercise the headed path.",
        );
        return;
    }

    let locks_before = display_locks();

    // about:blank keeps the assertion about the display, not about the
    // network: a headed launch is all that is needed to allocate one.
    let status = common::cmd()
        .args(["--json", "--headed", "goto", "about:blank"])
        .output()
        .expect("headed goto must run");
    // This used to be `success() || !stdout.is_empty()`, which one printed byte
    // satisfied — a panic message on stderr with a stray newline on stdout
    // would have passed it.
    //
    // The property that actually holds on BOTH paths is the agent-native
    // contract: under `--json` the product emits an envelope whether the launch
    // succeeds or fails, so the envelope SHAPE is assertable without knowing
    // which host this is. Success itself is not asserted, because a machine
    // with Xvfb installed but no usable Chrome fails here for a reason this
    // test is not about — and the property it IS about, the display lock,
    // is checked below and holds either way.
    let envelope: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap_or_else(|e| {
        panic!(
            "headed launch must emit a JSON envelope: {e}; stdout={} stderr={}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        )
    });
    assert!(
        envelope["ok"].is_boolean(),
        "envelope must carry a boolean `ok`: {envelope}"
    );
    assert!(
        envelope["schema_version"].is_number(),
        "envelope must carry `schema_version`: {envelope}"
    );

    // Release is ASYNCHRONOUS: the CLI exits, and only then does Xvfb tear its
    // display down and unlink the lock. Reading the set immediately after
    // `.output()` therefore catches a lock mid-release and blames the product
    // for a file that is already on its way out.
    //
    // MEASURED 2026-08-24, running the three tests in this file in sequence:
    // this assertion reported `.X101-lock` leaked, and the very next listing —
    // taken seconds later from the shell — no longer contained it. Nothing had
    // leaked; the instrument read too early. Isolated, the same test passed,
    // which is the signature of a timing artefact rather than a defect.
    //
    // So poll until the difference drains, and report only what SURVIVES the
    // deadline. The deadline is a hang guard: a lock still present after it is
    // a real leak, because no teardown takes that long.
    //
    // KNOWN LIMIT, measured in the same run: `.X100-lock` disappeared and came
    // back within five seconds, so another process on this machine allocates
    // displays concurrently. A set difference cannot tell whose lock appeared,
    // and the lock file carries the X server's pid rather than the pid of
    // whoever asked for it, so ownership is not recoverable from it. Polling
    // absorbs a short-lived foreign lock; a long-lived one would still be
    // misattributed here.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let leaked: Vec<String> = loop {
        let now: Vec<String> = display_locks().difference(&locks_before).cloned().collect();
        if now.is_empty() || std::time::Instant::now() >= deadline {
            break now;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(
        leaked.is_empty(),
        "headed run leaked display locks that survived the teardown window: {leaked:?}"
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
