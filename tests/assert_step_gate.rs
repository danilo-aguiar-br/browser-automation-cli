//! Permanent gate: every `assert` kind can FAIL the run.
//!
//! # Why this file exists
//!
//! `assert` exists so a domain verdict reaches the process exit code. Without
//! it, `exit 0` means only "the automation ran", and a failure reported inside
//! an `eval` result still exits zero.
//!
//! Nothing under `tests/` executed the step before this file: a cross-check of
//! the `run` dispatch inventory against every literal step, helper invocation
//! and argv form in `tests/` and `scripts/` found zero. So the command that
//! turns verdicts into exit codes had no test proving it can produce a non-zero
//! one.
//!
//! # Why the negative half is the whole gate
//!
//! An `assert` that always returned `ok` would satisfy every positive control
//! here and would silently disarm every script that relies on it — the exit code
//! would go back to meaning "the automation ran". The six failing cases are
//! therefore not redundancy: each one pins a different kind, and a kind that
//! stopped discriminating would show up in exactly one of them.
//!
//! # The nine committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | all six kinds pass together when every condition holds |
//! | six negatives | each kind FAILS the run on its own when the condition does not |
//! | declared exclusion: no previous | `kind: step` first in a script is a PRECONDITION error |
//! | declared exclusion: unknown kind | an unknown kind is a USAGE error, not a silent pass |
//! | environment guard | the host really ran the cases above |
//!
//! The two exclusions matter because both could plausibly be "just skip it".
//! Skipping either would let a typo in `kind` disarm an assertion while the run
//! reported success, which is the same false green the step exists to remove.
//!
//! # What this file does NOT cover
//!
//! - It does not cover the flag-shaped `assert` subcommand, only the `run` step.
//! - It does not cover the shorthand forms without `kind` (`{"cmd":"assert",
//!   "url":"..."}`), only the explicit ones.
//! - It does not cover the `level` and `max` matrix of `kind: console` beyond
//!   the zero-error case.
//! - It says nothing about console CAPTURE surviving shutdown, which is covered
//!   by `tests/failure_dump_gate.rs`.
//!
//! # Why `--capture-console` is always passed
//!
//! Three of the six kinds read the console ring, which only exists when the
//! capture flag is on. Passing it for all cases keeps the scripts identical
//! except for the assertion under test, so a difference in outcome cannot come
//! from a difference in flags.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY, and
//! `the_host_can_actually_run_this_gate` turns that skip into exactly one red
//! case instead of nine silent greens.

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "assert_step_gate";

/// Token the fixture writes to the console, and to nothing else in the tree.
const CONSOLE_TOKEN: &str = "ASSERT_TOKEN_E5F6";

fn fixture_url(query: &str) -> Option<String> {
    let p = root().join("scripts/fixtures/assert_step/page.html");
    p.exists().then(|| {
        if query.is_empty() {
            format!("file://{}", p.display())
        } else {
            format!("file://{}?{}", p.display(), query)
        }
    })
}

/// Run a script through `run` with console capture, and return the envelope.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-assert-gate-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = common::isolated_cmd(&bin)
        .args([
            "-q",
            "--timeout",
            "120",
            "--capture-console",
            "--json",
            "run",
            "--script",
        ])
        .arg(&script)
        .output()
        .ok()?;

    serde_json::from_slice(&out.stdout).ok()
}

/// A `goto` step for the fixture, quiet or noisy.
fn goto(noisy: bool) -> String {
    let url = fixture_url(if noisy { "noisy=1" } else { "" }).expect("fixture url");
    format!(r#"{{"cmd":"goto","url":"{url}"}}"#)
}

/// Read the page's `#detail` text, so `kind: step` has a payload to assert on.
const READ_DETAIL: &str =
    r#"{"cmd":"eval","expression":"document.getElementById('detail').textContent"}"#;

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url("").is_none() {
        common::skip_with_reason(
            "assert_step_gate",
            "fixture scripts/fixtures/assert_step/page.html absent.",
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// Assert that one script fails, with the expected error kind, and say why.
fn expect_failure(label: &str, kind: &str, lines: &[String]) {
    let env = run_script(lines).unwrap_or_else(|| panic!("{label}: no envelope"));
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "{label}: a false condition must FAIL the run. Succeeding here means the \
         exit code stopped carrying the verdict, which is the entire reason the \
         assert step exists: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some(kind),
        "{label}: expected error kind `{kind}`: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("assert"),
        "{label}: the failure must name the assertion so the caller is not sent \
         looking at the previous step; got {message}"
    );
}

/// POSITIVE CONTROL: all six kinds pass together when every condition holds.
///
/// They share one launch on purpose. If any kind stopped accepting a true
/// condition the whole script fails, and the step index in the envelope names
/// which one.
#[test]
fn every_assert_kind_passes_when_its_condition_holds() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        goto(false),
        r#"{"cmd":"assert","kind":"url","value":"assert_step","contains":true}"#.to_string(),
        r#"{"cmd":"assert","kind":"text","value":"Verdict OK"}"#.to_string(),
        r#"{"cmd":"assert","kind":"console_empty"}"#.to_string(),
        format!(r#"{{"cmd":"assert","kind":"console_no_match","pattern":"{CONSOLE_TOKEN}"}}"#),
        r#"{"cmd":"assert","kind":"console","level":"error","max":0}"#.to_string(),
        READ_DETAIL.to_string(),
        r#"{"cmd":"assert","kind":"step","path":"result","equals":"status: pass"}"#.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "every condition here is true, so the run must succeed: {env}"
    );
    let steps = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|s| s.len())
        .unwrap_or(0);
    assert_eq!(
        steps, 8,
        "all eight steps must have run; a short list means the script stopped early: {env}"
    );
}

/// NEGATIVE: `kind: url` fails when the URL does not match.
#[test]
fn assert_url_fails_on_a_url_that_does_not_match() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "url",
        "data",
        &[
            goto(false),
            r#"{"cmd":"assert","kind":"url","value":"this-substring-is-not-there","contains":true}"#
                .to_string(),
        ],
    );
}

/// NEGATIVE: `kind: text` fails when the text is absent from the page.
#[test]
fn assert_text_fails_on_text_the_page_does_not_contain() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "text",
        "data",
        &[
            goto(false),
            r#"{"cmd":"assert","kind":"text","value":"Verdict FAIL"}"#.to_string(),
        ],
    );
}

/// NEGATIVE: `kind: console_empty` fails when the page logged an error.
#[test]
fn assert_console_empty_fails_on_a_noisy_page() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "console_empty",
        "data",
        &[
            goto(true),
            r#"{"cmd":"assert","kind":"console_empty"}"#.to_string(),
        ],
    );
}

/// NEGATIVE: `kind: console_no_match` fails when the pattern IS present.
///
/// The token exists only in the fixture, so a match here cannot come from
/// unrelated browser noise.
#[test]
fn assert_console_no_match_fails_when_the_pattern_is_present() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "console_no_match",
        "data",
        &[
            goto(true),
            format!(r#"{{"cmd":"assert","kind":"console_no_match","pattern":"{CONSOLE_TOKEN}"}}"#),
        ],
    );
}

/// NEGATIVE: `kind: console` fails when the error count exceeds `max`.
#[test]
fn assert_console_fails_when_the_error_count_exceeds_max() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "console",
        "data",
        &[
            goto(true),
            r#"{"cmd":"assert","kind":"console","level":"error","max":0}"#.to_string(),
        ],
    );
}

/// NEGATIVE: `kind: step` fails when the previous payload does not match.
///
/// This is the kind the step was opened for: a domain verdict living inside an
/// `eval` result, which without this reaches the caller as exit 0.
#[test]
fn assert_step_fails_when_the_previous_payload_does_not_match() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "step",
        "data",
        &[
            goto(false),
            READ_DETAIL.to_string(),
            r#"{"cmd":"assert","kind":"step","path":"result","equals":"status: fail"}"#.to_string(),
        ],
    );
}

/// DECLARED EXCLUSION: `kind: step` with nothing before it is a PRECONDITION
/// error, not a usage error and not a silent pass.
///
/// The distinction is what the caller acts on: `usage` says fix the argv, and
/// the argv is fine here. What is wrong is the ORDER, and only a precondition
/// kind says so.
#[test]
fn assert_step_without_a_previous_step_is_a_precondition_error() {
    if cannot_run() {
        return;
    }
    expect_failure(
        "step-without-previous",
        "precondition",
        &[r#"{"cmd":"assert","kind":"step","path":"result","equals":"anything"}"#.to_string()],
    );
}

/// DECLARED EXCLUSION: an unknown kind is a USAGE error.
///
/// Silently skipping an unrecognised kind is the worst available behaviour: a
/// typo would disarm the assertion while the run reported success, which is
/// precisely the false green this step exists to remove.
#[test]
fn an_unknown_assert_kind_is_a_usage_error_and_not_a_silent_pass() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        goto(false),
        r#"{"cmd":"assert","kind":"kind-that-does-not-exist"}"#.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "an unknown kind must not be skipped: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("usage"),
        "an unrecognised kind is malformed input, so the taxonomy is usage: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("kind-that-does-not-exist"),
        "the message must echo the offending kind so the typo is visible; got {message}"
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
