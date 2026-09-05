//! Permanent gate for the blank-page error TAXONOMY (GAP-020).
//!
//! # What has to break for this to fail
//!
//! `view --detailed` without prior navigation used to answer `kind: "usage"`
//! and exit `2`. The argv was already correct, so "fix your argv" sent the
//! agent into a correction loop that cannot converge — the agent needed to
//! NAVIGATE, not to re-read `--help`.
//!
//! The fix introduced a precondition kind with its own exit code. What must
//! never come back is `usage` on this path, so that is the assertion.
//!
//! | case | expectation |
//! |---|---|
//! | positive control | blank `view` answers a precondition kind, never `usage`/2 |
//! | negative | the same command with `--allow-empty` SUCCEEDS |
//! | declared exclusion | a genuinely malformed argv still answers `usage`/2 |
//!
//! The exclusion is what keeps the fix honest: moving everything off `usage`
//! would also pass a naive check, while destroying the distinction the gap is
//! about.
//!
//! # Unified blank-page policy (the other half of GAP-020, now CLOSED)
//!
//! The gap also asked to unify the blank-page policy across `view`, `cookie`,
//! `text` and `attr`. The rule that resolved it is about what each command
//! READS, not about keeping the four identical:
//!
//! | command | reads | blank page |
//! |---|---|---|
//! | `view` | page content | refuses, `precondition` / 75 |
//! | `text` | page content | refuses, `precondition` / 75 |
//! | `attr` | page content | refuses, `precondition` / 75 |
//! | `cookie list` | browser state | SUCCEEDS, exit 0 |
//!
//! `cookie` is deliberately NOT aligned: an empty cookie jar is a true answer,
//! while empty page content is indistinguishable from "the caller forgot to
//! navigate". Asserting `cookie` also refused would pin the wrong rule.
//!
//! # Skip policy
//!
//! No binary or no Chrome means SKIP LOUDLY.

mod common;
use common::{binary, binary_or_skip, chrome_not_ready};

const GATE: &str = "view_precondition_gate";

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

/// Run the CLI and return (exit code, parsed envelope).
fn run(args: &[&str]) -> Option<(i32, serde_json::Value)> {
    let bin = binary()?;
    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "90", "--json"])
        .args(args)
        .output()
        .ok()?;
    let parsed = serde_json::from_slice(&out.stdout).ok()?;
    Some((out.status.code().unwrap_or(-1), parsed))
}

/// GAP-020: a blank page is a PRECONDITION failure, never an argv failure.
///
/// This is the assertion that dies if someone routes the path back to `usage`.
#[test]
fn blank_view_is_a_precondition_failure_not_an_argv_failure() {
    if cannot_run() {
        return;
    }
    let (code, env) = run(&["view", "--detailed"]).expect("envelope");
    let kind = env
        .pointer("/error/kind")
        .and_then(|k| k.as_str())
        .unwrap_or("");

    assert_ne!(
        kind, "usage",
        "blank `view` answered `usage`, which tells the agent to fix an argv \
         that is already correct — the non-converging loop GAP-020 removed. \
         Envelope: {env}"
    );
    assert_ne!(
        code, 2,
        "blank `view` exited 2 (argv error). It must carry its own exit code."
    );
    assert!(
        !kind.is_empty() && kind.contains("precondition"),
        "expected a precondition kind, got {kind:?}. Envelope: {env}"
    );
}

/// GAP-020: the suggestion must point at NAVIGATION, not at `--help`.
///
/// Kind alone is not enough: an agent reads the suggestion. A correct kind with
/// a suggestion that still says "check your arguments" leaves the loop intact.
#[test]
fn blank_view_suggestion_points_at_navigating() {
    if cannot_run() {
        return;
    }
    let (_, env) = run(&["view", "--detailed"]).expect("envelope");
    let suggestion = env
        .pointer("/error/suggestion")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    assert!(
        !suggestion.is_empty(),
        "precondition failure must carry a suggestion. Envelope: {env}"
    );
    // Localized; match on the command name, which is stable in both locales.
    assert!(
        suggestion.contains("goto"),
        "suggestion must name the remedy (navigate with goto), got {suggestion:?}"
    );
}

/// NEGATIVE CASE: the operator opting into a blank snapshot must SUCCEED.
///
/// Without this, a gate could pass by making `view` fail unconditionally.
#[test]
fn view_with_allow_empty_succeeds_on_a_blank_page() {
    if cannot_run() {
        return;
    }
    let (code, env) = run(&["view", "--allow-empty"]).expect("envelope");
    assert_eq!(code, 0, "`--allow-empty` must succeed. Envelope: {env}");
    assert_eq!(env.get("ok").and_then(|v| v.as_bool()), Some(true), "{env}");
}

/// DECLARED EXCLUSION: genuinely malformed argv still answers `usage` / exit 2.
///
/// Keeps the fix honest. Moving every error off `usage` would satisfy the first
/// test while erasing the distinction the gap is about.
#[test]
fn malformed_argv_still_answers_usage() {
    if binary().is_none() {
        common::skip_with_remedy(
            "view_precondition_gate",
            "target/debug/browser-automation-cli absent.",
            "run `cargo build` first.",
        );
        return;
    }
    // No Chrome needed: clap rejects before any browser work.
    let (code, env) = run(&["view", "--this-flag-does-not-exist"]).expect("envelope");
    assert_eq!(
        code, 2,
        "a malformed flag must still exit 2. Envelope: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|k| k.as_str()),
        Some("usage"),
        "a malformed flag must still be `usage`. Envelope: {env}"
    );
}

/// GAP-020: `text` reads page content, so a blank page is a precondition
/// failure — the same class as `view`, never `usage`.
#[test]
fn blank_text_is_a_precondition_failure() {
    if cannot_run() {
        return;
    }
    // `body` exists on about:blank, so the read SUCCEEDS and returns nothing.
    // That is the case the rule is about: a successful read of no content.
    let (code, env) = run(&["text", "body"]).expect("envelope");
    let kind = env
        .pointer("/error/kind")
        .and_then(|k| k.as_str())
        .unwrap_or("");
    assert_ne!(
        code, 0,
        "blank `text` succeeded, which reports an empty page as a real answer          and hides the missing navigation. Envelope: {env}"
    );
    assert_ne!(kind, "usage", "argv was correct. Envelope: {env}");
    assert!(
        kind.contains("precondition"),
        "expected a precondition kind, got {kind:?}. Envelope: {env}"
    );
}

/// GAP-020: `attr` reads page content, so it follows the same rule as `text`.
#[test]
fn blank_attr_is_a_precondition_failure() {
    if cannot_run() {
        return;
    }
    let (code, env) = run(&["attr", "body", "id"]).expect("envelope");
    let kind = env
        .pointer("/error/kind")
        .and_then(|k| k.as_str())
        .unwrap_or("");
    assert_ne!(code, 0, "blank `attr` succeeded. Envelope: {env}");
    assert_ne!(kind, "usage", "argv was correct. Envelope: {env}");
    assert!(
        kind.contains("precondition"),
        "expected a precondition kind, got {kind:?}. Envelope: {env}"
    );
}

/// The declared exclusion: `cookie` reads BROWSER state, so empty is a true
/// answer and it must keep succeeding.
///
/// Without this case, aligning every command onto `precondition` would also
/// pass the two tests above while destroying the distinction the rule encodes.
#[test]
fn blank_cookie_list_still_succeeds() {
    if cannot_run() {
        return;
    }
    let (code, env) = run(&["cookie", "list"]).expect("envelope");
    assert_eq!(
        code, 0,
        "`cookie list` on a blank page must succeed: an empty jar is a real \
         answer, not a missing navigation. Envelope: {env}"
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
