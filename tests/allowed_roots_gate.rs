//! Permanent gate for local-scheme containment (GAP-026).
//!
//! # What has to break for this to fail
//!
//! `goto file:///etc/passwd` used to answer exit `0`. Paired with `grab --path`
//! it let rendered host content be written anywhere the process could reach.
//! The fix bounds local reads and artifact writes to allowed roots.
//!
//! A test that only asserted "the command errors" would stay green if the
//! refusal moved back to `usage`/exit 2, the taxonomy defect that
//! `view_precondition_gate.rs` guards: the agent would be told to fix an argv
//! that is already correct.
//! So this file asserts containment AND its classification.
//!
//! | case | expectation |
//! |---|---|
//! | positive control | a path INSIDE an allowed root is readable |
//! | negative | `/etc/passwd` is refused as policy, not as argv |
//! | declared exclusion | `--allow-outside-roots` restores access |
//!
//! The positive control is what stops a broken build from passing: a binary
//! that refused every `file://` would satisfy the negative case alone.
//!
//! # Skip policy
//!
//! No binary or no Chrome means SKIP LOUDLY.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary() -> Option<PathBuf> {
    let p = root().join("target/debug/browser-automation-cli");
    p.exists().then_some(p)
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    let Some(bin) = binary() else {
        eprintln!(
            "SKIP allowed_roots_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    };
    let chrome_ok = Command::new(&bin)
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output()
        .ok()
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.get("ok").and_then(|b| b.as_bool()))
        .unwrap_or(false);
    if !chrome_ok {
        eprintln!(
            "SKIP allowed_roots_gate: doctor reports the host is not ready for Chrome. \
             This is NOT a pass."
        );
        return true;
    }
    false
}

/// The host may not expose `/etc/passwd`; the negative case needs a real file
/// that is genuinely outside every allowed root.
fn outside_target() -> Option<&'static str> {
    ["/etc/passwd", "/etc/hostname"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
}

fn run(args: &[&str]) -> Option<(i32, serde_json::Value)> {
    let bin = binary()?;
    let out = Command::new(&bin)
        .args(["-q", "--timeout", "90", "--json"])
        .args(args)
        .output()
        .ok()?;
    let parsed = serde_json::from_slice(&out.stdout).ok()?;
    Some((out.status.code().unwrap_or(-1), parsed))
}

/// Write a fixture inside the system temp dir, which is a default allowed root.
fn fixture_inside_roots() -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("allowed-roots-gate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let file = dir.join("inside.html");
    std::fs::write(&file, b"<html><body>inside</body></html>").ok()?;
    Some(file)
}

/// POSITIVE CONTROL: a local file INSIDE an allowed root stays readable.
///
/// Without this, a binary that refused every `file://` URL would pass the
/// negative case while having broken the feature outright.
#[test]
fn local_file_inside_an_allowed_root_is_readable() {
    if cannot_run() {
        return;
    }
    let file = fixture_inside_roots().expect("fixture");
    let url = format!("file://{}", file.display());
    let (code, env) = run(&["goto", &url]).expect("envelope");
    let _ = std::fs::remove_file(&file);
    assert_eq!(
        code, 0,
        "a file under the system temp root must remain readable. Envelope: {env}"
    );
    assert_eq!(env.get("ok").and_then(|v| v.as_bool()), Some(true), "{env}");
}

/// GAP-026: a local path outside every allowed root is refused.
#[test]
fn local_file_outside_allowed_roots_is_refused() {
    if cannot_run() {
        return;
    }
    let Some(target) = outside_target() else {
        eprintln!(
            "SKIP allowed_roots_gate: no readable path outside the roots on this host. \
             This is NOT a pass."
        );
        return;
    };
    let url = format!("file://{target}");
    let (code, env) = run(&["goto", &url]).expect("envelope");
    assert_ne!(
        code, 0,
        "reading {target} must not succeed. Envelope: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert!(
        message.contains("outside allowed roots"),
        "the refusal must name the policy, got {message:?}"
    );
}

/// GAP-026: the refusal is a POLICY decision, never an argv error.
///
/// Separate from the test above on purpose: containment can be correct while
/// the classification sends the agent into the non-converging argv loop.
#[test]
fn the_refusal_is_classified_as_policy_not_argv() {
    if cannot_run() {
        return;
    }
    let Some(target) = outside_target() else {
        eprintln!("SKIP allowed_roots_gate: no path outside the roots. This is NOT a pass.");
        return;
    };
    let url = format!("file://{target}");
    let (code, env) = run(&["goto", &url]).expect("envelope");
    let kind = env
        .pointer("/error/kind")
        .and_then(|k| k.as_str())
        .unwrap_or("");
    assert_ne!(
        kind, "usage",
        "the path is outside policy but the ARGV is correct; `usage` restarts \
         the argv-correction loop that never converges. Envelope: {env}"
    );
    assert_ne!(code, 2, "policy refusal must not share the argv exit code");
    let suggestion = env
        .pointer("/error/suggestion")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    assert!(
        suggestion.contains("allow-outside-roots"),
        "the suggestion must name the escape flag, got {suggestion:?}"
    );
}

/// DECLARED EXCLUSION: the operator accepting the risk regains access.
///
/// A containment that could not be lifted would be a different product; the
/// gap asked for a bounded default, not a prohibition.
#[test]
fn the_risk_flag_restores_access() {
    if cannot_run() {
        return;
    }
    let Some(target) = outside_target() else {
        eprintln!("SKIP allowed_roots_gate: no path outside the roots. This is NOT a pass.");
        return;
    };
    let url = format!("file://{target}");
    let (code, env) = run(&["--allow-outside-roots", "goto", &url]).expect("envelope");
    assert_eq!(
        code, 0,
        "`--allow-outside-roots` must restore access. Envelope: {env}"
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
