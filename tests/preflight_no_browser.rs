// SPDX-License-Identifier: MIT OR Apache-2.0
//! GAP-012 / GAP-029: an invalid script must fail before Chrome is launched.
//!
//! # What the marker assertion proves, and what it does not
//!
//! This header used to say the proof was residual: a launch leaves a
//! `browser-automation-cli-chrome-*` marker profile behind for the duration of
//! the run. The first half is true; the conclusion drawn from it was not.
//!
//! `Command::output()` WAITS for the child to exit, and a clean one-shot
//! removes its own profile on the way out — which is the product law
//! `tests/residual_one_shot.rs` exists to enforce. Measured 2026-09-01: a
//! script that really launches Chrome, runs `eval` and answers `ok` leaves ZERO
//! markers behind. The marker therefore cannot be observed for a child that has
//! finished, whether or not it ever launched.
//!
//! The assertion inside `run_script` is a LEAK guard, and it is kept on those
//! terms because it costs almost nothing. What actually carries the preflight
//! claim is the error SHAPE each test below asserts: exit 2, 64 or 65 with a
//! message only the preflight path emits, produced without the product ever
//! reaching the dispatcher.
//!
//! A real launch guard would have to observe the child WHILE it lives. That is
//! not built here, and saying so is better than a green assertion that cannot
//! fail.

mod common;

/// The two roots the CHILD of ONE `run_script` call writes marker profiles into.
///
/// Both live under that call's own `tempfile::tempdir()`, because `run_script`
/// pins the child's `TMPDIR` and `XDG_CACHE_HOME` there. The cache layout
/// mirrors `xdg::chrome_profiles_dir()`, exactly as `tests/residual_one_shot.rs`
/// spells it: `$XDG_CACHE_HOME/<pkg>/chrome-profiles`.
///
/// # Why not `common::sandbox_root()`
///
/// That was the first fix and it was still too coarse. `sandbox_root()` is keyed
/// on `std::process::id()`, so it is shared by every test in this BINARY, and
/// libtest runs them on parallel threads.
///
/// `preflight_accepts_every_key_the_dispatcher_reads` is the reason that
/// matters: it is the one test here that feeds a VALID script, so it launches
/// Chrome on purpose and leaves a marker on purpose. Measured 2026-09-01, with
/// the roots shared, five of the eight tests failed by observing that marker —
/// a sibling's correct behaviour read as their own regression.
///
/// A witness for "this invocation launched nothing" has to be owned by THAT
/// invocation. Anything wider is a race with whoever else is running.
fn marker_roots(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    vec![
        dir.to_path_buf(),
        dir.join(env!("CARGO_PKG_NAME")).join("chrome-profiles"),
    ]
}

/// A run that must fail at preflight, returning (exit code, stdout).
///
/// # Why elapsed time is no longer part of the contract
///
/// This used to return the wall time as well, and one caller asserted it stayed
/// under five seconds "which suggests a browser launch". That was a PROXY for
/// the claim, and a fragile one: on a loaded machine a parse-only failure can
/// exceed any threshold and turn a correct product into a red test, while on a
/// fast machine a real launch could slip in under it.
///
/// The direct proof is a marker profile, which a launch leaves behind and a
/// preflight failure never creates. This file therefore asserts NO timing.
///
/// # Why the marker is counted in EXPLICIT roots
///
/// It was not, and the assertion was broken in both directions at once.
/// Measured 2026-09-01: the count came from `list_cli_chrome_marker_dirs()`,
/// which resolves its roots from the env of the TEST process — the operator's
/// real cache plus the shared `/tmp`.
///
/// The child, meanwhile, has been isolated since `common::cmd()` gained
/// `sandbox_env()`: it writes under `$CARGO_TARGET_TMPDIR/xdg-sandbox-<pid>`.
/// So the counter looked where the child never writes and watched a directory
/// every other process on the host shares.
///
/// False NEGATIVE, and this is the serious half: had preflight regressed and
/// launched Chrome, the marker would land inside the sandbox, the shared count
/// would not move, and the test would pass. The assertion could not fail for
/// the reason the file exists.
///
/// False POSITIVE: six of these tests failed on 2026-09-01 with
/// `markers 0 -> 1` while other agents on the same host were driving the
/// product. Nothing about the code under test had changed.
///
/// `markers_after <= markers_before` was the other half of the mistake: a
/// concurrent REMOVAL elsewhere on the host could absorb a marker this run
/// created and still satisfy it. Owning the roots makes the honest assertion
/// available, and it is the one used now — the set must be EMPTY.
fn run_script(body: &str, extra: &[&str]) -> (i32, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("s.jsonl");
    std::fs::write(&script, body).expect("write script");
    let roots = marker_roots(dir.path());

    let mut args: Vec<&str> = vec!["--json"];
    args.extend_from_slice(extra);
    args.push("run");
    args.push("--script");
    let out = common::cmd()
        .args(&args)
        .arg(&script)
        .env("NO_COLOR", "1")
        .env("TMPDIR", dir.path())
        .env("XDG_CACHE_HOME", dir.path())
        .output()
        .expect("spawn run");
    let leaked = browser_automation_cli::residual::list_cli_chrome_marker_dirs_in_roots(&roots);

    assert!(
        leaked.is_empty(),
        "preflight failure must not launch Chrome, but marker profiles appeared \
         in this run's own roots: {leaked:?}"
    );
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// GAP-012: an unknown command in the LAST step still fails before any effect.
#[test]
fn unknown_cmd_in_last_step_fails_without_launching() {
    let (code, stdout) = run_script(
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"view\"}\n{\"cmd\":\"parametro_inexistente_teste\"}\n",
        &[],
    );
    assert_eq!(code, 2, "unknown cmd is a usage error: {stdout}");
    assert!(
        stdout.contains("unknown script cmd"),
        "must name the offending cmd: {stdout}"
    );
    // "It did not launch a browser" is proved by the marker count inside
    // `run_script`, not by a stopwatch.
}

/// A required payload missing from a command with NO `action` fails before BORN.
///
/// `validate_action` only reaches commands that branch on `action`, so `eval`
/// and `goto` used to travel all the way to the dispatcher to be told the same
/// thing about their own argv — after a browser had been launched and paid for.
///
/// The marker assertion inside `run_script` is what proves no launch happened;
/// this file asserts no timing, deliberately, for the reason stated above.
#[test]
fn missing_required_payload_fails_without_launching() {
    for (script, want) in [
        ("{\"cmd\":\"eval\"}\n", "eval requires expression"),
        ("{\"cmd\":\"goto\"}\n", "goto requires url"),
    ] {
        let (code, stdout) = run_script(script, &[]);
        assert_eq!(code, 2, "missing payload is a usage error: {stdout}");
        assert!(stdout.contains(want), "must say `{want}`, got: {stdout}");
    }
}

/// The payload check must not reject a step the dispatcher would have run.
///
/// `eval` accepts `expression`, `function` or `js`. Preflight asks the
/// dispatcher's own reader rather than keeping a second copy of that list, and
/// this is the direction that catches a copy going stale: if preflight ever
/// narrowed the accepted keys, these would fail as usage errors.
#[test]
fn preflight_accepts_every_key_the_dispatcher_reads() {
    for key in ["expression", "function", "js"] {
        let script = format!("{{\"cmd\":\"eval\",\"{key}\":\"1+1\"}}\n");
        let (code, stdout) = run_script(&script, &[]);
        assert_ne!(
            code, 2,
            "`{key}` is a key the dispatcher reads and preflight rejected it: {stdout}"
        );
    }
}

/// GAP-029: a console step without `--capture-console` fails before BORN.
#[test]
fn missing_capture_flag_fails_without_launching() {
    let (code, stdout) = run_script(
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
    let (code, stdout) = run_script(
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

    let out = common::cmd()
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
    let (code, stdout) = run_script(
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
