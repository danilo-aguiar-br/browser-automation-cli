//! Permanent gate for structured `eval` returns (GAP-035).
//!
//! # What has to break for this to fail
//!
//! `eval` used to hand back `data.result` as TEXT when the page returned a
//! serialized structure, so the consumer deserialized twice: once for the
//! envelope, once for the business payload. The gap asked for two things, and
//! both are asserted here, because closing only the visible half is how a gap
//! gets counted as fixed while still open:
//!
//! | requirement | assertion |
//! |---|---|
//! | structure delivered as structure | an object return parses as an object, not a string |
//! | type exposed for branching | `typed: true` reports `value_type` per JS type |
//!
//! # Declared exclusion
//!
//! `typed` is OPT-IN. The default shape stays lean, which is the same
//! token-economy rule that put command objects behind `--detail`. A default
//! that always carried type metadata would repeat the envelope-duplication
//! mistake that `envelope_shape_gate.rs` guards.
//!
//! # Skip policy
//!
//! No binary or no Chrome means SKIP LOUDLY.

mod common;
use common::{binary, binary_or_skip, chrome_not_ready};

const GATE: &str = "eval_typed_gate";

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    let Some(bin) = binary_or_skip(GATE) else {
        return true;
    };
    if chrome_not_ready(GATE, &bin) {
        return true;
    }
    false
}

/// Run a script and return the payload of every `eval` step, in order.
fn eval_payloads(steps: &[&str]) -> Option<Vec<serde_json::Value>> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-eval-typed-gate-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    let script = dir.join("steps.jsonl");

    let mut lines = vec![r#"{"cmd":"goto","url":"about:blank"}"#.to_string()];
    lines.extend(steps.iter().map(|s| (*s).to_string()));
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "90", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;

    let env: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(
        env.pointer("/data/steps")?
            .as_array()?
            .iter()
            .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
            .filter_map(|s| s.get("data").cloned())
            .collect(),
    )
}

/// GAP-035: an object return arrives as an OBJECT, not as text to re-parse.
#[test]
fn object_return_arrives_as_structure_not_text() {
    if cannot_run() {
        return;
    }
    let payloads =
        eval_payloads(&[r#"{"cmd":"eval","expression":"({a:1,b:[2,3]})"}"#]).expect("payloads");
    let result = payloads
        .first()
        .and_then(|p| p.get("result"))
        .expect("result");

    assert!(
        result.is_object(),
        "the page returned an object; delivering it as {} forces the caller to \
         deserialize twice, which is what this gap removed. Got: {result}",
        if result.is_string() {
            "text"
        } else {
            "a scalar"
        }
    );
    assert_eq!(result.pointer("/a"), Some(&serde_json::json!(1)));
    assert_eq!(result.pointer("/b/1"), Some(&serde_json::json!(3)));
}

/// GAP-035: `typed` reports the JS type so the consumer branches without guessing.
///
/// Asserted across three distinct types: a single type would pass on a constant.
#[test]
fn typed_mode_reports_the_value_type_per_js_type() {
    if cannot_run() {
        return;
    }
    let payloads = eval_payloads(&[
        r#"{"cmd":"eval","expression":"({a:1})","typed":true}"#,
        r#"{"cmd":"eval","expression":"42","typed":true}"#,
        r#"{"cmd":"eval","expression":"'texto'","typed":true}"#,
    ])
    .expect("payloads");
    assert_eq!(payloads.len(), 3, "expected three eval steps");

    let types: Vec<&str> = payloads
        .iter()
        .map(|p| p.get("value_type").and_then(|t| t.as_str()).unwrap_or(""))
        .collect();
    assert_eq!(
        types,
        vec!["object", "number", "string"],
        "typed mode must report the JS type of each return; got {types:?}"
    );

    // The value must still be there, and still be structured.
    assert!(payloads[0]
        .get("value")
        .map(|v| v.is_object())
        .unwrap_or(false));
    assert_eq!(payloads[1].get("value"), Some(&serde_json::json!(42)));
}

/// DECLARED EXCLUSION: `typed` is opt-in; the default payload stays lean.
///
/// Making type metadata unconditional would re-inflate every envelope, the
/// redundancy that `envelope_shape_gate.rs` guards.
#[test]
fn default_mode_stays_lean_without_type_metadata() {
    if cannot_run() {
        return;
    }
    let payloads = eval_payloads(&[r#"{"cmd":"eval","expression":"({a:1})"}"#]).expect("payloads");
    let payload = payloads.first().expect("payload");
    assert!(
        payload.get("value_type").is_none(),
        "default `eval` must not carry type metadata; that is what `typed` is for. \
         Got: {payload}"
    );
    assert!(
        payload.get("result").is_some(),
        "default `eval` must still carry the value under `result`. Got: {payload}"
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
