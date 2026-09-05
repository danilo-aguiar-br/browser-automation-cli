//! Permanent gate for envelope payload duplication (GAP-019).
//!
//! # What has to break for this to fail
//!
//! `run` used to emit every step twice: `data` and `result` carried identical
//! content, so ~47% of the envelope was pure redundancy and a ten-step script
//! cost 3381 bytes instead of 1784. The fix removed `result`.
//!
//! Nothing prevents someone re-adding `result` as a convenience alias. A test
//! that only checked "the envelope parses" would stay green through that, so
//! this file asserts the two properties that actually die on regression:
//!
//! | property | assertion |
//! |---|---|
//! | no aliased payload | no step carries `data` and `result` with equal content |
//! | bounded size | a fixed ten-step script stays under [`MAX_ENVELOPE_BYTES`] |
//!
//! The size ceiling is the part that catches an alias under a *different*
//! name: the field check knows one spelling, the byte budget knows none.
//!
//! # Declared exclusion
//!
//! A step MAY carry `result` alone. Some payloads are naturally named that way
//! (`eval` returns a value). The violation is carrying BOTH with equal content,
//! which is duplication, not naming.
//!
//! # Skip policy
//!
//! No binary or no Chrome means SKIP LOUDLY. A silent green would rebuild the
//! blind spot this gate exists to remove.

use std::path::PathBuf;

mod common;
use common::{binary, binary_or_skip, chrome_not_ready, root};

const GATE: &str = "envelope_shape_gate";

/// Ceiling for the committed ten-step reference script.
///
/// Current size is ~1650 bytes. Restoring the `data`/`result` duplication takes
/// it to ~3300, so this bound discriminates while leaving room for honest
/// growth. Lower it when the envelope shrinks; never raise it to make a
/// regression fit.
const MAX_ENVELOPE_BYTES: usize = 2600;

fn reference_script() -> Option<PathBuf> {
    let p = root().join("scripts/fixtures/envelope_shape/ten_steps.jsonl");
    p.exists().then_some(p)
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    let Some(bin) = binary_or_skip(GATE) else {
        return true;
    };
    if reference_script().is_none() {
        common::skip_with_reason(
            "envelope_shape_gate",
            "scripts/fixtures/envelope_shape/ten_steps.jsonl absent.",
        );
        return true;
    }
    if chrome_not_ready(GATE, &bin) {
        return true;
    }
    false
}

/// Run the reference script and return raw stdout plus the parsed envelope.
fn run_reference() -> Option<(Vec<u8>, serde_json::Value)> {
    let bin = binary()?;
    let script = reference_script()?;
    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "120", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;
    let parsed = serde_json::from_slice(&out.stdout).ok()?;
    Some((out.stdout, parsed))
}

/// POSITIVE CONTROL: the reference script runs and every step reports a payload.
///
/// Without this, the two assertions below could pass on an empty envelope.
#[test]
fn reference_script_produces_ten_step_payloads() {
    if cannot_run() {
        return;
    }
    let (_, env) = run_reference().expect("run envelope");
    assert_eq!(env.get("ok").and_then(|v| v.as_bool()), Some(true), "{env}");
    let steps = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .expect("steps array");
    assert_eq!(steps.len(), 10, "reference script must keep ten steps");
    assert!(
        steps.iter().all(|s| s.get("data").is_some()),
        "every step must carry a payload, else the duplication check is vacuous"
    );
}

/// Steps that carry the same payload under both `data` and `result`.
///
/// Extracted so the detector can be exercised on a synthetic envelope. Proving
/// the gate REJECTS requires feeding it a violation, and mutating
/// `src/commands/run/engine.rs` to manufacture one would edit a file this task
/// does not own — and would leave the tree mutated if the run were killed.
fn duplicated_payload_steps(env: &serde_json::Value) -> Vec<String> {
    let Some(steps) = env.pointer("/data/steps").and_then(|s| s.as_array()) else {
        return Vec::new();
    };
    steps
        .iter()
        .enumerate()
        .filter(|(_, s)| match (s.get("data"), s.get("result")) {
            // Carrying `result` alone is naming, not duplication (see exclusion).
            (Some(d), Some(r)) => d == r,
            _ => false,
        })
        .map(|(i, s)| {
            format!(
                "step {i} cmd={}",
                s.get("cmd").and_then(|c| c.as_str()).unwrap_or("?")
            )
        })
        .collect()
}

/// GAP-019: no step may carry `data` and `result` with identical content.
#[test]
fn no_step_duplicates_its_payload_under_two_names() {
    if cannot_run() {
        return;
    }
    let (_, env) = run_reference().expect("run envelope");
    let offenders = duplicated_payload_steps(&env);
    assert!(
        offenders.is_empty(),
        "these steps carry the same payload under both `data` and `result`, \
         which is the ~47% redundancy GAP-019 removed:\n{}",
        offenders.join("\n")
    );
}

/// MUTATION PROOF, on synthetic input: the detector must REJECT a duplicated
/// envelope and ACCEPT the two legitimate shapes.
///
/// Without this, `no_step_duplicates_its_payload_under_two_names` could be
/// green because it never detects anything, the failure mode this version
/// keeps finding in its own gates.
#[test]
fn detector_rejects_duplication_and_accepts_the_declared_exclusion() {
    let duplicated = serde_json::json!({
        "data": { "steps": [
            { "cmd": "eval", "data": { "result": 2 }, "result": { "result": 2 } }
        ]}
    });
    assert_eq!(
        duplicated_payload_steps(&duplicated).len(),
        1,
        "detector must flag identical data/result"
    );

    // Declared exclusion: `result` ALONE is naming, not duplication.
    let result_only = serde_json::json!({
        "data": { "steps": [{ "cmd": "eval", "result": { "result": 2 } }] }
    });
    assert!(duplicated_payload_steps(&result_only).is_empty());

    // Both present but DIFFERENT is not duplication either.
    let both_differ = serde_json::json!({
        "data": { "steps": [
            { "cmd": "eval", "data": { "a": 1 }, "result": { "b": 2 } }
        ]}
    });
    assert!(duplicated_payload_steps(&both_differ).is_empty());
}

/// GAP-019: the reference envelope stays inside its byte budget.
///
/// Catches an alias reintroduced under a name this file does not know.
#[test]
fn reference_envelope_stays_within_its_byte_budget() {
    if cannot_run() {
        return;
    }
    let (raw, _) = run_reference().expect("run envelope");
    assert!(
        raw.len() <= MAX_ENVELOPE_BYTES,
        "reference envelope grew to {} bytes (budget {MAX_ENVELOPE_BYTES}). \
         If this is duplication, remove it. If the payload legitimately grew, \
         lower the budget deliberately — never raise it to fit a regression.",
        raw.len()
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
