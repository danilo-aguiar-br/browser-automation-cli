//! Permanent gate: failure evidence reaches DISK, not just the envelope (GAP-039).
//!
//! # Why this file exists
//!
//! `tests/preflight_no_browser.rs` asserts that the two call sites producing
//! `failure_dump_path` still exist in the source. That guard is worth keeping —
//! deleting either one breaks nothing that compiles — but it is a source scan.
//! It never launches a browser, so it cannot tell whether the artifact is
//! actually written, whether it contains the captured rings, or whether a run
//! WITHOUT the flag stays clean.
//!
//! The property the gap asks for is survival: after the process is gone, the
//! console and network the failing run saw must still be readable from disk.
//! That can only be shown by failing a real run and then reading the file.
//!
//! # What makes the artifact evidence rather than decoration
//!
//! `scripts/fixtures/failure_dump/noisy.html` logs two tokens that appear
//! nowhere else in the repository. A dump that wrote an empty payload, or a
//! synthesized one, would carry the right KEYS and still not carry these
//! strings. Asserting the tokens is what separates "a file was created" from
//! "the evidence survived".
//!
//! # The four committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | a failing run writes the rings to disk and names the path |
//! | negative | the same failure without the flag writes nothing |
//! | declared exclusion: success | a run that succeeds produces no artifact |
//! | declared exclusion: no capture | the flag alone preserves nothing |
//!
//! The negative case is checked AFTER a settle, because the dump is written on
//! the way out. Asserting absence too early would report a clean directory that
//! is merely a directory not yet written to.
//!
//! # What this file does NOT cover
//!
//! - It does not cover size limits or expiry of the artifact, which the gap
//!   lists as a separate requirement and which no measurement here would see.
//! - It does not cover the allowed-roots policy applied to the dump path; that
//!   dimension has its own gate in `tests/allowed_roots_gate.rs`.
//! - It does not cover the streaming path in `src/commands/run/stream.rs`, only
//!   the scripted `run`.
//!
//! # Declared coupling, measured and pinned below
//!
//! `--dump-on-failure` alone preserves nothing: `dump_failure_evidence` in
//! `src/browser/support.rs` returns early when neither capture ring is active.
//! That is deliberate and commented in the source, and it is pinned here so the
//! coupling is visible rather than discovered during an incident.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY. A silent green here
//! would rebuild the blind spot this gate exists to remove.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Tokens the fixture writes to the console and to no other file in the tree.
const CONSOLE_TOKEN: &str = "EVIDENCE_LOG_A1B2";
const WARN_TOKEN: &str = "EVIDENCE_WARN_C3D4";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary() -> Option<PathBuf> {
    let p = root().join("target/debug/browser-automation-cli");
    p.exists().then_some(p)
}

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/failure_dump/noisy.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// A private artifacts directory for one case.
fn artifacts_dir(tag: &str) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "failure-dump-gate-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("artifacts dir");
    dir
}

/// Files currently present in `dir`.
fn dumps_in(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default()
}

/// Run a script and return the parsed envelope.
///
/// `extra` carries the global flags under test, so each case differs only by
/// the flags and never by the script.
fn run_with(extra: &[&str], lines: &[String], arts: &Path) -> Option<serde_json::Value> {
    let bin = binary()?;
    let dir = arts.join("script");
    std::fs::create_dir_all(&dir).ok()?;
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let mut cmd = Command::new(&bin);
    cmd.args(["-q", "--timeout", "120", "--json"]);
    cmd.args(extra);
    cmd.args(["--artifacts-dir"]).arg(arts);
    cmd.args(["run", "--script"]).arg(&script);
    let out = cmd.output().ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    serde_json::from_slice(&out.stdout).ok()
}

/// A script that loads the noisy fixture and then fails on a missing target.
fn failing_script() -> Vec<String> {
    let url = fixture_url().expect("fixture url");
    vec![
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"press","target":"#this-node-does-not-exist"}"##.to_string(),
    ]
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP failure_dump_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    if fixture_url().is_none() {
        eprintln!(
            "SKIP failure_dump_gate: fixture scripts/fixtures/failure_dump/noisy.html absent. \
             This is NOT a pass."
        );
        return true;
    }
    let probe = Command::new(binary().expect("binary"))
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output();
    let chrome_ok = probe
        .ok()
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.get("ok").and_then(|b| b.as_bool()))
        .unwrap_or(false);
    if !chrome_ok {
        eprintln!(
            "SKIP failure_dump_gate: doctor reports the host is not ready for Chrome. \
             This is NOT a pass."
        );
        return true;
    }
    false
}

/// POSITIVE CONTROL: the failing run leaves its console and network on disk and
/// says where.
///
/// The path is read from the envelope rather than guessed from the directory:
/// an artifact an agent cannot locate is not much better than no artifact, and
/// the gap asks for both.
#[test]
fn a_failing_run_writes_the_captured_rings_to_disk_and_names_the_path() {
    if cannot_run() {
        return;
    }
    let arts = artifacts_dir("positive");
    let env = run_with(
        &[
            "--capture-console",
            "--capture-network",
            "--dump-on-failure",
        ],
        &failing_script(),
        &arts,
    )
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "the script must fail for there to be anything to dump: {env}"
    );

    let path = env
        .pointer("/data/failure_dump_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("error envelope must carry failure_dump_path: {env}"));
    assert!(
        path.starts_with(&arts),
        "the dump must land in --artifacts-dir; got {} expected under {}",
        path.display(),
        arts.display()
    );
    assert!(
        path.exists(),
        "envelope names {} but it does not exist. \
         A path reported without a file is worse than no path: it ends the search.",
        path.display()
    );

    let raw = std::fs::read_to_string(&path).expect("read dump");
    let dump: serde_json::Value = serde_json::from_str(&raw).expect("dump is json");
    for key in ["console", "network", "error"] {
        assert!(
            dump.get(key).is_some(),
            "the dump must carry `{key}`; got keys {:?}",
            dump.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );
    }

    // The tokens are the evidence. Keys alone would also be present in a dump
    // that preserved nothing.
    assert!(
        raw.contains(CONSOLE_TOKEN) && raw.contains(WARN_TOKEN),
        "the dump must contain the console the page actually produced; \
         neither {CONSOLE_TOKEN} nor {WARN_TOKEN} was found in {}",
        path.display()
    );
    let requests = dump
        .pointer("/network/total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        requests > 0,
        "the fixture fetches a subresource, so the network ring cannot be empty; \
         dump says {requests} in {}",
        path.display()
    );
}

/// NEGATIVE: the identical failure without the flag writes nothing at all.
///
/// Without this case a dump written unconditionally would satisfy the positive
/// control, and the flag would be decoration. The check happens after a settle
/// because the artifact is produced on the way out.
#[test]
fn the_same_failure_without_the_flag_leaves_no_artifact() {
    if cannot_run() {
        return;
    }
    let arts = artifacts_dir("negative");
    let env = run_with(
        &["--capture-console", "--capture-network"],
        &failing_script(),
        &arts,
    )
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "the same script must still fail: {env}"
    );
    assert!(
        env.pointer("/data/failure_dump_path").is_none(),
        "no flag, no path: {env}"
    );

    // Absence has to be measured after the writer would have run.
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let found = dumps_in(&arts);
    assert!(
        found.is_empty(),
        "no artifact may be written without --dump-on-failure; found {found:?}"
    );
}

/// DECLARED EXCLUSION: a run that SUCCEEDS produces no artifact.
///
/// The dump is failure evidence. Writing one on every run would fill the
/// artifacts directory with files nobody asked for and would make the presence
/// of a dump meaningless as a signal.
#[test]
fn a_successful_run_produces_no_artifact_even_with_the_flag() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let arts = artifacts_dir("success");
    let env = run_with(
        &[
            "--capture-console",
            "--capture-network",
            "--dump-on-failure",
        ],
        &[format!(r#"{{"cmd":"goto","url":"{url}"}}"#)],
        &arts,
    )
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "this script must succeed: {env}"
    );
    assert!(
        env.pointer("/data/failure_dump_path").is_none(),
        "a successful run must not report a failure dump: {env}"
    );

    std::thread::sleep(std::time::Duration::from_millis(1500));
    let found = dumps_in(&arts);
    assert!(
        found.is_empty(),
        "a successful run must leave the artifacts directory clean; found {found:?}"
    );
}

/// DECLARED EXCLUSION: the flag alone preserves nothing.
///
/// `dump_failure_evidence` returns early when neither capture ring is active,
/// so `--dump-on-failure` on its own writes no file. The source comments the
/// decision, and pinning it here makes the coupling discoverable before an
/// incident rather than during one.
///
/// This is the case most likely to change: the gap asks for evidence "without
/// requiring the operator to have predicted", and requiring a second flag is a
/// partial answer. Whoever decides to always arm the rings under
/// `--dump-on-failure` should replace this case rather than delete it.
#[test]
fn the_flag_alone_without_a_capture_ring_writes_nothing() {
    if cannot_run() {
        return;
    }
    let arts = artifacts_dir("uncaptured");
    let env = run_with(&["--dump-on-failure"], &failing_script(), &arts).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "the script must still fail: {env}"
    );
    assert!(
        env.pointer("/data/failure_dump_path").is_none(),
        "with no ring armed there is nothing to preserve, so no path may be \
         reported: {env}"
    );

    std::thread::sleep(std::time::Duration::from_millis(1500));
    let found = dumps_in(&arts);
    assert!(
        found.is_empty(),
        "an empty dump is worse than none: it looks like preserved evidence. \
         Found {found:?}"
    );
}

/// ENVIRONMENT GUARD: this one never skips.
///
/// The other cases in this file return early when the host is not ready, and a
/// test that returns counts as a PASS. On a machine without Chrome that turns
/// the whole file green while it tested nothing, and the honest SKIP lines this
/// file writes to stderr are easy to lose in `cargo test` output.
///
/// A test that fails the ENVIRONMENT is not a test that fails the CODE. Keeping
/// them apart is what lets the behavioural cases skip gracefully for someone
/// developing without a browser, while an unusable host still turns exactly one
/// case RED in one place.
#[test]
fn the_host_can_actually_run_this_gate() {
    assert!(
        !cannot_run(),
        "host cannot run this gate: every other case in this file skipped, and a \
         skip is NOT a pass. The SKIP line on stderr names the missing \
         precondition (binary, fixture, or Chrome)."
    );
}
