// SPDX-License-Identifier: MIT OR Apache-2.0
//! Preflight unit tests: validation and capability gating.

use std::path::{Path, PathBuf};

use crate::browser::CaptureOpts;
use crate::error::ErrorKind;

use super::super::flags::RunFlags;
use super::include::step_cmd;
use super::preflight_script;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write fixture");
    p
}

/// GAP-012: a typo in the last step fails before anything is launched.

#[test]
fn unknown_cmd_is_rejected_at_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"parametro_inexistente_teste\"}\n",
    );
    let err = preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect_err("unknown cmd must fail preflight");
    assert_eq!(err.kind(), ErrorKind::Usage);
    assert!(
        err.message().contains("script step 1"),
        "must name the offending step: {}",
        err.message()
    );
}

/// GAP-012: unknown fields are caught before BORN too.

#[test]
fn unknown_field_is_rejected_at_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\",\"nope\":1}\n",
    );
    let err = preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect_err("unknown field must fail preflight");
    assert_eq!(err.kind(), ErrorKind::Usage);
}

/// GAP-029: a console step without `--capture-console` fails before BORN.

#[test]
fn missing_capture_flag_fails_before_born() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"console\",\"action\":\"list\"}\n",
    );
    let err = preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect_err("missing capture flag must fail preflight");
    assert_eq!(err.kind(), ErrorKind::CapabilityDisabled);
    assert_eq!(err.exit_code(), 64);
    assert!(
        err.message().contains("--capture-console"),
        "must name the flag: {}",
        err.message()
    );
}

/// Every missing flag is reported at once, not one launch at a time.

#[test]
fn all_missing_flags_are_reported_together() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"console\",\"action\":\"list\"}\n{\"cmd\":\"net\",\"action\":\"list\"}\n{\"cmd\":\"heap\",\"action\":\"retainers\"}\n",
    );
    let err = preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect_err("missing flags must fail preflight");
    let msg = err.message();
    for flag in [
        "--capture-console",
        "--capture-network",
        "--category-memory",
    ] {
        assert!(msg.contains(flag), "{flag} missing from: {msg}");
    }
}

/// The capture flags satisfy the console/net steps.

#[test]
fn enabled_capture_passes_preflight() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"console\",\"action\":\"list\"}\n{\"cmd\":\"net\",\"action\":\"list\"}\n",
    );
    let capture = CaptureOpts {
        console: true,
        network: true,
    };
    let steps = preflight_script(&script, RunFlags::default(), capture).expect("must pass");
    assert_eq!(steps.len(), 2);
}

/// `heap take` stays free while a gated action fails (GAP-010 through preflight).

#[test]
fn heap_take_is_free_but_retainers_is_not() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ok = write(
        dir.path(),
        "take.jsonl",
        "{\"cmd\":\"heap\",\"action\":\"take\",\"path\":\"/tmp/x.heapsnapshot\"}\n",
    );
    preflight_script(&ok, RunFlags::default(), CaptureOpts::default())
        .expect("heap take needs no flag");

    let gated = write(
        dir.path(),
        "ret.jsonl",
        "{\"cmd\":\"heap\",\"action\":\"retainers\",\"path\":\"/tmp/x.heapsnapshot\"}\n",
    );
    let err = preflight_script(&gated, RunFlags::default(), CaptureOpts::default())
        .expect_err("heap retainers is gated");
    assert_eq!(err.kind(), ErrorKind::CapabilityDisabled);
}

/// Includes are spliced in order.

#[test]
fn include_is_expanded_in_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "inner.jsonl",
        "{\"cmd\":\"view\"}\n{\"cmd\":\"text\",\"target\":\"@e1\"}\n",
    );
    let outer = write(
        dir.path(),
        "outer.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"include\",\"path\":\"inner.jsonl\"}\n{\"cmd\":\"reload\"}\n",
    );
    let steps = preflight_script(&outer, RunFlags::default(), CaptureOpts::default())
        .expect("include must expand");
    let cmds: Vec<&str> = steps.iter().map(step_cmd).collect();
    assert_eq!(cmds, vec!["goto", "view", "text", "reload"]);
}

/// A self-referencing include is a cycle, not a stack overflow.

#[test]
fn direct_include_cycle_is_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write(
        dir.path(),
        "a.jsonl",
        "{\"cmd\":\"include\",\"path\":\"a.jsonl\"}\n",
    );
    let err = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect_err("cycle must fail");
    assert_eq!(err.kind(), ErrorKind::Data);
    assert!(
        err.message().contains("include cycle"),
        "must name the cycle: {}",
        err.message()
    );
}

/// An `action` with no dispatch arm is rejected BEFORE the browser is launched.
///
/// This is the case that made `tests/cookie_jar_gate.rs` depend on Chrome. The
/// gate asks whether malformed argv is a usage error — a question about
/// parsing, not about browsing — but the rejection used to happen inside the
/// dispatcher, after BORN. A launch that lost a contended host therefore failed
/// a test that never needed a browser. Answering here removes the coupling.
#[test]
fn unknown_action_is_rejected_before_born() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n\
         {\"cmd\":\"cookie\",\"action\":\"action-that-does-not-exist\"}\n",
    );
    let err = preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect_err("unknown action must fail preflight");
    assert_eq!(err.kind(), ErrorKind::Usage);
    assert!(
        err.message().contains("script step 1") && err.message().contains("unknown cookie action"),
        "must name the step and the action: {}",
        err.message()
    );
}

/// A required payload that is absent is rejected before BORN as well.
#[test]
fn cookie_set_without_payload_is_rejected_before_born() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(
        dir.path(),
        "s.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"cookie\",\"action\":\"set\"}\n",
    );
    let err = preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect_err("cookie set without payload must fail preflight");
    assert_eq!(err.kind(), ErrorKind::Usage);
    assert!(
        err.message().contains("cookie set requires json/cookies"),
        "must name what is missing: {}",
        err.message()
    );
}

/// Every action the dispatcher accepts still passes preflight.
///
/// The slices and the `match` arms are separate declarations, so this is what
/// keeps them honest in the direction that matters: a name dropped from a slice
/// would make preflight reject a step that runs perfectly well.
#[test]
fn every_dispatchable_action_survives_preflight() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (cmd, actions) in [
        ("cookie", &["list", "clear"][..]),
        (
            "page",
            &["info", "", "list", "tab-id", "tab_id", "get_tab_id"][..],
        ),
        ("dialog", &["accept", "dismiss"][..]),
        ("console", &["list", "get", "clear", "dump"][..]),
        ("net", &["list", "get"][..]),
        ("perf", &["start", "stop", "insight"][..]),
        ("screencast", &["start", "stop"][..]),
        (
            "heap",
            &["take", "summary", "close", "dup-strings", "dup_strings"][..],
        ),
    ] {
        for action in actions {
            let body = format!("{{\"cmd\":\"{cmd}\",\"action\":\"{action}\"}}\n");
            let script = write(dir.path(), "ok.jsonl", &body);
            // Every capability enabled: this test is about ACTION names, and a
            // capability rejection would mask the answer it is asking for.
            let flags = RunFlags {
                experimental_vision: true,
                experimental_screencast: true,
                category_memory: true,
                category_extensions: true,
                category_third_party: true,
                category_webmcp: true,
                ..RunFlags::default()
            };
            let capture = CaptureOpts {
                console: true,
                network: true,
            };
            let out = preflight_script(&script, flags, capture);
            assert!(
                out.is_ok(),
                "{cmd} action {action:?} must survive preflight: {:?}",
                out.err().map(|e| e.message().to_string())
            );
        }
    }
}

/// An ABSENT `action` is never a preflight failure.
///
/// Every arm that reads one applies a default, so absence is a valid script.
#[test]
fn an_absent_action_is_not_a_rejection() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = write(dir.path(), "s.jsonl", "{\"cmd\":\"cookie\"}\n");
    preflight_script(&script, RunFlags::default(), CaptureOpts::default())
        .expect("absent action defaults and must pass");
}
