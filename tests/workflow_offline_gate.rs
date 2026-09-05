// SPDX-License-Identifier: MIT OR Apache-2.0
//! A manifest key the runner never reads must not be answered with `ok: true`.
//!
//! # Why this file exists at all
//!
//! Measured 2026-08-31: `workflow` had ZERO integration tests. A whole verb —
//! one that reads a user-authored manifest and executes it — was covered only
//! by whatever the unit tests of its parts happened to touch, and the parts
//! that were wrong were the ones nothing touched.
//!
//! # The defect being pinned
//!
//! `execute_offline_step` read four keys per command and accepted every other
//! one in silence. The consequential case was `engine`: the module pins the
//! engine to `http` because it launches no browser, so a manifest asking for
//! `browser` was quietly downgraded and the answer came from an engine the
//! author never chose — under `ok: true`, with nothing in the envelope
//! disagreeing with the request.
//!
//! # Why these cases need neither Chrome nor the network
//!
//! Every rejection here happens before any I/O, and the positive controls use
//! `echo`, which touches nothing. That is deliberate: a gate that needs a
//! browser to prove an argument-parsing rule gets skipped on the machines that
//! most need it, and a skipped gate is a green one.

mod common;

use std::io::Write;

/// Write a manifest and run it, returning the exit code and parsed envelope.
fn run_manifest(steps: &str) -> Option<(i32, serde_json::Value)> {
    let bin = common::binary()?;
    let scratch = tempfile::Builder::new()
        .prefix("bac-workflow-gate-")
        .tempdir()
        .ok()?;
    let manifest = scratch.path().join("wf.json");
    let mut f = std::fs::File::create(&manifest).ok()?;
    write!(f, r#"{{"name":"gate","steps":[{steps}]}}"#).ok()?;
    drop(f);

    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "60", "--json", "workflow", "run"])
        .arg("--manifest")
        .arg(&manifest)
        .output()
        .ok()?;
    let env = serde_json::from_slice(&out.stdout).ok()?;
    Some((out.status.code().unwrap_or(-1), env))
}

/// POSITIVE CONTROL: a manifest of keys the runner reads still succeeds.
///
/// Without this, every rejection below would stay green if `workflow run`
/// started refusing everything — the failure mode that turns a validation gate
/// into a test of nothing.
#[test]
fn a_manifest_using_only_read_keys_succeeds() {
    if common::missing_binary("workflow_offline_gate") {
        return;
    }
    let (code, env) =
        run_manifest(r#"{"id":"ping","cmd":"echo","args":{"message":"start"}}"#).expect("envelope");

    assert_eq!(code, 0, "the control manifest must run: {env}");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the control proves the refusals below are about the FIELD, not the verb: {env}"
    );
}

/// A key no handler reads is refused instead of dropped.
#[test]
fn an_unread_manifest_key_is_refused() {
    if common::missing_binary("workflow_offline_gate") {
        return;
    }
    let (code, env) =
        run_manifest(r#"{"id":"p","cmd":"parse","args":{"path":"/tmp/x","typo_key":1}}"#)
            .expect("envelope");

    assert_eq!(
        code, 2,
        "a key the runner cannot honour is malformed input, and exit 2 is what \
         an agent branches on before parsing: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("typo_key"),
        "the refusal must name the key that caused it, or the author re-reads \
         the whole manifest looking for it; got {message}"
    );
}

/// `engine: browser` is refused rather than silently downgraded to HTTP.
///
/// This is the case the whole file was written for. The old behaviour answered
/// `ok: true` from the HTTP engine, so the manifest's request and the envelope's
/// answer disagreed and nothing reported it.
#[test]
fn an_engine_the_offline_runner_cannot_launch_is_refused() {
    if common::missing_binary("workflow_offline_gate") {
        return;
    }
    let (code, env) = run_manifest(
        r#"{"id":"s","cmd":"scrape","args":{"url":"https://example.com","engine":"browser"}}"#,
    )
    .expect("envelope");

    assert_eq!(
        code, 2,
        "a downgraded engine must not report success: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("browser") && message.contains("http"),
        "the refusal must name BOTH the engine asked for and the one available, \
         or the author cannot tell which half to change; got {message}"
    );
}

/// CONTROL: `engine: http` is ACCEPTED, because it agrees with what runs.
///
/// `docs/COOKBOOK.md` publishes a manifest carrying exactly this. Refusing the
/// key outright would have broken a documented example to fix nothing, so the
/// rule is about the VALUE and this case is what pins that distinction.
///
/// The step is allowed to fail on the network — this asserts only that it is
/// not refused for its `engine`, which is decided before any request is made.
#[test]
fn the_engine_the_offline_runner_does_use_is_accepted() {
    if common::missing_binary("workflow_offline_gate") {
        return;
    }
    let (_, env) = run_manifest(
        r#"{"id":"s","cmd":"scrape","args":{"url":"https://example.com","engine":"http"}}"#,
    )
    .expect("envelope");

    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        !message.contains("cannot use engine"),
        "`http` is the engine this runner uses, so it must never be refused as \
         unavailable; got {message}"
    );
}

/// A failed run FAILS, instead of reporting success about a failure.
///
/// Measured 2026-08-31, before the fix: a manifest whose only step was refused
/// answered `ok: true` at the top of the envelope with `"status": "failed"`
/// nested inside, and exit 0. An agent branches on the exit code before it
/// parses anything, so the single field it reads said the workflow worked.
///
/// The journal and the per-step results must SURVIVE the failure, because a
/// caller debugging one needs more of that context, not less. That is what the
/// second half of this case pins.
#[test]
fn a_failed_run_reports_failure_at_the_top_and_keeps_its_journal() {
    if common::missing_binary("workflow_offline_gate") {
        return;
    }
    let (code, env) =
        run_manifest(r#"{"id":"p","cmd":"parse","args":{"path":"/tmp/x","typo_key":1}}"#)
            .expect("envelope");

    assert_ne!(code, 0, "a failed workflow must not exit 0: {env}");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "the top-level `ok` must agree with the nested status: {env}"
    );
    let data = env
        .get("data")
        .expect("the payload rides along on the error");
    assert_eq!(
        data.get("status").and_then(|v| v.as_str()),
        Some("failed"),
        "the nested status is still there: {env}"
    );
    assert!(
        data.get("journal").and_then(|v| v.as_str()).is_some(),
        "the journal path must survive the failure, or the caller loses the \
         one artefact that explains it: {env}"
    );
    assert!(
        data.get("steps")
            .and_then(|v| v.as_array())
            .is_some_and(|s| !s.is_empty()),
        "per-step results must survive too: {env}"
    );
}

/// CONTROL: `echo` still takes any key, because echoing is its contract.
///
/// The allowlist deliberately has no row for `echo`: it returns `args`
/// verbatim, so every key it receives IS read. Validating it would refuse the
/// one command whose entire purpose is to accept anything.
#[test]
fn echo_still_accepts_a_key_no_other_step_would() {
    if common::missing_binary("workflow_offline_gate") {
        return;
    }
    let (code, env) =
        run_manifest(r#"{"id":"e","cmd":"echo","args":{"anything":1,"engine":"browser"}}"#)
            .expect("envelope");

    assert_eq!(
        code, 0,
        "echo reads every key by definition, so validating it would break the \
         one command that cannot have an unread field: {env}"
    );
}
