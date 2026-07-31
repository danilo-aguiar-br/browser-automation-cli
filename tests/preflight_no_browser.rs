// SPDX-License-Identifier: MIT OR Apache-2.0
//! GAP-012 / GAP-029: an invalid script must fail before Chrome is launched.
//!
//! The proof is residual: a launch leaves a `browser-automation-cli-chrome-*`
//! marker profile behind for the duration of the run. If preflight worked, no
//! marker ever appears and the failure is far faster than a launch.

use std::process::Command;
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

/// A run that must fail at preflight, returning (exit code, stdout, elapsed).
fn run_script(body: &str, extra: &[&str]) -> (i32, String, Duration) {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("s.jsonl");
    std::fs::write(&script, body).expect("write script");

    let markers_before = browser_automation_cli::residual::list_cli_chrome_marker_dirs().len();
    let started = Instant::now();
    let mut args: Vec<&str> = vec!["--json"];
    args.extend_from_slice(extra);
    args.push("run");
    args.push("--script");
    let out = Command::new(BIN)
        .args(&args)
        .arg(&script)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn run");
    let elapsed = started.elapsed();
    let markers_after = browser_automation_cli::residual::list_cli_chrome_marker_dirs().len();

    assert!(
        markers_after <= markers_before,
        "preflight failure must not launch Chrome: markers {markers_before} -> {markers_after}"
    );
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        elapsed,
    )
}

/// GAP-012: an unknown command in the LAST step still fails before any effect.
#[test]
fn unknown_cmd_in_last_step_fails_without_launching() {
    let (code, stdout, elapsed) = run_script(
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"view\"}\n{\"cmd\":\"parametro_inexistente_teste\"}\n",
        &[],
    );
    assert_eq!(code, 2, "unknown cmd is a usage error: {stdout}");
    assert!(
        stdout.contains("unknown script cmd"),
        "must name the offending cmd: {stdout}"
    );
    // A real launch costs seconds; preflight is parse-only.
    assert!(
        elapsed < Duration::from_secs(5),
        "preflight took {elapsed:?}, which suggests a browser launch"
    );
}

/// GAP-029: a console step without `--capture-console` fails before BORN.
#[test]
fn missing_capture_flag_fails_without_launching() {
    let (code, stdout, _) = run_script(
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"console\",\"action\":\"list\"}\n",
        &[],
    );
    assert_eq!(
        code, 64,
        "capability-disabled is exit 64, not usage 2: {stdout}"
    );
    assert!(
        stdout.contains("--capture-console"),
        "must name the missing flag: {stdout}"
    );
}

/// GAP-011: a gated heap action reports capability-disabled, not usage.
#[test]
fn gated_heap_action_reports_capability_disabled() {
    let (code, stdout, _) = run_script(
        "{\"cmd\":\"heap\",\"action\":\"retainers\",\"path\":\"/tmp/x.heapsnapshot\",\"node\":1}\n",
        &[],
    );
    assert_eq!(code, 64, "expected capability-disabled: {stdout}");
    assert!(
        stdout.contains("capability-disabled"),
        "envelope kind must be capability-disabled: {stdout}"
    );
    assert!(
        stdout.contains("--category-memory"),
        "must name the flag: {stdout}"
    );
}

/// An include cycle is caught at load, not by exhausting the stack.
#[test]
fn include_cycle_fails_without_launching() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("a.jsonl");
    let b = dir.path().join("b.jsonl");
    std::fs::write(&a, "{\"cmd\":\"include\",\"path\":\"b.jsonl\"}\n").expect("write a");
    std::fs::write(&b, "{\"cmd\":\"include\",\"path\":\"a.jsonl\"}\n").expect("write b");

    let out = Command::new(BIN)
        .args(["--json", "run", "--script"])
        .arg(&a)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(65),
        "cycle is a data error: {stdout}"
    );
    assert!(
        stdout.contains("include cycle"),
        "must name the cycle: {stdout}"
    );
}

/// Every missing flag is listed in one failure, not discovered one launch at a time.
#[test]
fn all_missing_flags_reported_in_one_run() {
    let (code, stdout, _) = run_script(
        "{\"cmd\":\"console\",\"action\":\"list\"}\n{\"cmd\":\"net\",\"action\":\"list\"}\n",
        &[],
    );
    assert_eq!(code, 64, "{stdout}");
    for flag in ["--capture-console", "--capture-network"] {
        assert!(stdout.contains(flag), "{flag} missing from: {stdout}");
    }
}

/// Wiring guard: the two call sites that produce `failure_dump_path` must stay.
///
/// They can be deleted without breaking compilation — the command keeps working
/// and only the envelope field disappears — so a source assertion is the only
/// thing that catches it. `engine.rs` captures the evidence while the session is
/// still alive, and `scripts.rs` carries it onto the error envelope.
///
/// Scope note: this asserts only that the call sites EXIST. It does not run a
/// session, so it cannot show that console and network capture actually survive
/// shutdown and reach the dump. That behaviour has no gate yet.
#[test]
fn failure_dump_wiring_is_not_silently_removed() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let engine =
        std::fs::read_to_string(root.join("src/commands/run/engine.rs")).expect("read engine.rs");
    assert!(
        engine.contains("dump_failure_evidence"),
        "engine.rs must capture failure evidence before the session is dropped"
    );
    assert!(
        engine.contains("failure_dump_path"),
        "engine.rs must put failure_dump_path on the fail-fast envelope"
    );

    let scripts =
        std::fs::read_to_string(root.join("src/commands/nav/scripts.rs")).expect("read scripts.rs");
    assert!(
        scripts.contains("failure_dump_path"),
        "scripts.rs rebuilds the error envelope and must carry failure_dump_path"
    );
}
