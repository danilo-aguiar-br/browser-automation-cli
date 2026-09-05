//! Permanent gate: the cookie jar is really written, read and emptied.
//!
//! # Why this file exists
//!
//! Nothing under `tests/` executed the `cookie` step before this file. It was
//! mentioned in two places and neither ran it: the header table of
//! `tests/view_precondition_gate.rs`, as prose about blank-page policy, and a
//! list of names that `tests/devtools_envelope_behavior.rs` passes to
//! `schema --cmd`. A name in a schema list is not a jar being written.
//!
//! # Why the round trip is the evidence
//!
//! `set` reporting `set_count: 1` proves only that the command counted its own
//! input. The jar is real when a LATER `list` finds what `set` put there, when
//! `clear` makes it disappear, and when a URL filter returns a strict subset.
//!
//! Each of those is a separate case here, because each kills a different
//! degenerate implementation: `set` that counts and discards, `list` that
//! replays the last `set`, and `clear` that reports success and empties nothing.
//!
//! # Why there is no fixture
//!
//! Every other browser gate in this directory commits an HTML file, and this one
//! deliberately does not. The jar is browser state and none of these assertions
//! reads the document, so a fixture would be decoration that later has to be
//! kept alive. `about:blank` is navigable, re-executable and needs nothing on
//! disk.
//!
//! That choice is itself covered: `cookie` on a blank page SUCCEEDS by design,
//! which is the policy decided in `tests/view_precondition_gate.rs` — `view`
//! refuses on a blank page because an empty read is indistinguishable from a
//! forgotten navigation, while an empty jar is a TRUE answer about browser
//! state. Using `about:blank` here exercises that decision instead of avoiding
//! it.
//!
//! # The six committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | `set` then `list` returns the cookie that was stored |
//! | discrimination: clear | `clear` really empties the jar |
//! | discrimination: filter | a URL filter returns a strict subset, so the jar is queried |
//! | declared exclusion | an empty jar SUCCEEDS, and says `empty` |
//! | negatives of argv | unknown action and missing payload are USAGE errors |
//! | environment guard | the host really ran the cases above |
//!
//! # What this file does NOT cover
//!
//! - It does not cover `httpOnly`, `secure` or expiry semantics beyond what the
//!   envelope echoes back.
//! - It does not cover the one-shot `cookie` subcommand, only the `run` step.
//! - It does not check that the PAGE can read the cookie via `document.cookie`,
//!   which would need an http origin; `file://` and `about:blank` cannot.
//! - It says nothing about the blank-page policy itself, which is decided and
//!   asserted in `tests/view_precondition_gate.rs`.
//!
//! # Skip policy
//!
//! No binary or no Chrome means SKIP LOUDLY, and
//! `the_host_can_actually_run_this_gate` turns that skip into exactly one red
//! case instead of five silent greens.

mod common;
use common::{binary, chrome_not_ready, missing_binary};

const GATE: &str = "cookie_jar_gate";

/// Values that exist nowhere else in the repository.
const TOKEN_ALPHA: &str = "COOKIE_A7B8";
const TOKEN_BETA: &str = "COOKIE_C9D0";

/// Run a script through `run` and return the parsed envelope.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-cookie-gate-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "120", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;

    serde_json::from_slice(&out.stdout).ok()
}

/// `data` of every `cookie` step, in order.
fn cookie_steps(env: &serde_json::Value) -> Vec<serde_json::Value> {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("cookie"))
                .filter_map(|s| s.get("data").cloned())
                .collect()
        })
        .unwrap_or_default()
}

const GOTO_BLANK: &str = r#"{"cmd":"goto","url":"about:blank"}"#;
const LIST: &str = r#"{"cmd":"cookie","action":"list"}"#;
const CLEAR: &str = r#"{"cmd":"cookie","action":"clear"}"#;

/// A `cookie set` step for one name/value/origin triple.
fn set(name: &str, value: &str, url: &str) -> String {
    format!(
        r#"{{"cmd":"cookie","action":"set","cookies":[{{"name":"{name}","value":"{value}","url":"{url}"}}]}}"#
    )
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// POSITIVE CONTROL: what `set` stored is what a later `list` finds.
///
/// The assertion is on the cookie's own fields rather than on `set_count`, which
/// a command could report without storing anything.
#[test]
fn a_cookie_that_was_set_is_found_again_by_list() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        GOTO_BLANK.to_string(),
        set("gate_token", TOKEN_ALPHA, "https://alpha.example"),
        LIST.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "storing and reading a cookie must succeed: {env}"
    );
    let steps = cookie_steps(&env);
    assert_eq!(steps.len(), 2, "expected a set and a list: {env}");

    assert_eq!(
        steps[0].get("set_count").and_then(|v| v.as_u64()),
        Some(1),
        "the set step must report what it stored: {:?}",
        steps[0]
    );

    let listed = steps[1]
        .get("cookies")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        listed.len(),
        1,
        "exactly one cookie was stored: {:?}",
        steps[1]
    );
    let c = &listed[0];
    assert_eq!(
        c.get("name").and_then(|v| v.as_str()),
        Some("gate_token"),
        "the stored name must come back: {c}"
    );
    assert_eq!(
        c.get("value").and_then(|v| v.as_str()),
        Some(TOKEN_ALPHA),
        "the stored VALUE is the evidence; a jar that only kept names would pass \
         a name-only assertion: {c}"
    );
    assert_eq!(
        c.get("domain").and_then(|v| v.as_str()),
        Some("alpha.example"),
        "the origin from the set payload must survive into the jar: {c}"
    );
    assert_eq!(
        steps[1].get("empty").and_then(|v| v.as_bool()),
        Some(false),
        "a jar with a cookie in it is not empty: {:?}",
        steps[1]
    );
}

/// DISCRIMINATION: `clear` really empties the jar.
///
/// Without this case a `set` that stored nothing and a `list` that replayed the
/// last `set` would both pass the positive control. Here the same `list` has to
/// return a DIFFERENT answer after `clear`, which neither degenerate version can
/// produce.
#[test]
fn clear_really_empties_the_jar() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        GOTO_BLANK.to_string(),
        set("gate_token", TOKEN_ALPHA, "https://alpha.example"),
        CLEAR.to_string(),
        LIST.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "clearing a jar is not an error: {env}"
    );
    let steps = cookie_steps(&env);
    assert_eq!(steps.len(), 3, "expected set, clear and list: {env}");
    assert_eq!(
        steps[1].get("cleared").and_then(|v| v.as_bool()),
        Some(true),
        "the clear step must say it cleared: {:?}",
        steps[1]
    );
    assert_eq!(
        steps[2].get("count").and_then(|v| v.as_u64()),
        Some(0),
        "after clear the jar must be empty. A non-zero count here means clear \
         reported success and removed nothing: {:?}",
        steps[2]
    );
    assert_eq!(
        steps[2].get("empty").and_then(|v| v.as_bool()),
        Some(true),
        "the emptiness must be stated, not inferred from the count: {:?}",
        steps[2]
    );
}

/// DISCRIMINATION: a URL filter returns a strict subset.
///
/// Two cookies on two origins, then a filtered list. A `list` that ignored the
/// filter would return both, and a `list` that replayed the last `set` would
/// return the wrong one. Only a real query over a real jar returns exactly the
/// alpha cookie.
#[test]
fn a_url_filter_returns_a_strict_subset_of_the_jar() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        GOTO_BLANK.to_string(),
        set("gate_token", TOKEN_ALPHA, "https://alpha.example"),
        set("other_token", TOKEN_BETA, "https://beta.example"),
        LIST.to_string(),
        r#"{"cmd":"cookie","action":"list","url":"https://alpha.example"}"#.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "listing with and without a filter must succeed: {env}"
    );
    let steps = cookie_steps(&env);
    assert_eq!(steps.len(), 4, "expected two sets and two lists: {env}");

    assert_eq!(
        steps[2].get("count").and_then(|v| v.as_u64()),
        Some(2),
        "the unfiltered list must see both origins: {:?}",
        steps[2]
    );
    assert_eq!(
        steps[3].get("count").and_then(|v| v.as_u64()),
        Some(1),
        "the filter must narrow the answer. Returning 2 here means the filter is \
         accepted and then ignored: {:?}",
        steps[3]
    );
    assert_eq!(
        steps[3].get("url_filter").and_then(|v| v.as_str()),
        Some("https://alpha.example"),
        "the envelope must echo which filter was applied: {:?}",
        steps[3]
    );
    let listed = steps[3]
        .get("cookies")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        listed
            .first()
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str()),
        Some(TOKEN_ALPHA),
        "the surviving cookie must be the alpha one, not whichever was set last: {:?}",
        steps[3]
    );
}

/// DECLARED EXCLUSION: an empty jar is a SUCCESS, and says so.
///
/// This is the half of the blank-page policy that `cookie` keeps: an empty jar
/// is a true answer about browser state, unlike an empty page read, which is
/// indistinguishable from a forgotten navigation. See
/// `tests/view_precondition_gate.rs` for the decision and the other half.
#[test]
fn an_empty_jar_succeeds_and_reports_itself_as_empty() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[GOTO_BLANK.to_string(), LIST.to_string()]).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "an empty jar is an answer, not a precondition failure: {env}"
    );
    let steps = cookie_steps(&env);
    assert_eq!(
        steps[0].get("empty").and_then(|v| v.as_bool()),
        Some(true),
        "emptiness must be explicit so the caller can tell it from a missing \
         field: {:?}",
        steps[0]
    );
    assert_eq!(
        steps[0].get("count").and_then(|v| v.as_u64()),
        Some(0),
        "an empty jar has a count, and the count is zero: {:?}",
        steps[0]
    );
}

/// NEGATIVES OF ARGV: an unknown action and a missing payload are USAGE errors.
///
/// Skipping either would let a typo disarm the step while the run reported
/// success, which is the false green every gate in this directory exists to
/// remove.
///
/// # Why this case does NOT wait for Chrome
///
/// It used to, and that was the bug. The question here is about parsing, not
/// about browsing, yet the script opens with a `goto` and the rejection lived
/// inside the dispatcher — so the run paid a whole browser to reach an answer
/// argv alone decides. A launch that lost a contended host therefore failed a
/// test that never needed a browser, and that is exactly what was observed: one
/// `exit_code 69 / unavailable` per full suite run, landing on a different test
/// each time.
///
/// `run --script` now validates `action` and required payloads before BORN, so
/// dropping the Chrome guard is not a convenience — it is the ASSERTION. If the
/// rejection ever moves back behind the launch, this case fails on any host
/// without Chrome instead of passing quietly.
#[test]
fn an_unknown_action_and_a_missing_payload_are_usage_errors() {
    if missing_binary(GATE) {
        return;
    }
    for (label, step) in [
        (
            "unknown action",
            r#"{"cmd":"cookie","action":"action-that-does-not-exist"}"#,
        ),
        ("set without payload", r#"{"cmd":"cookie","action":"set"}"#),
    ] {
        let env = run_script(&[GOTO_BLANK.to_string(), step.to_string()])
            .unwrap_or_else(|| panic!("{label}: no envelope"));
        assert_eq!(
            env.get("ok").and_then(|v| v.as_bool()),
            Some(false),
            "{label}: malformed input must not be skipped: {env}"
        );
        assert_eq!(
            env.pointer("/error/kind").and_then(|v| v.as_str()),
            Some("usage"),
            "{label}: malformed input is a usage error: {env}"
        );
        let message = env
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            message.contains("cookie"),
            "{label}: the failure must name the step; got {message}"
        );
    }
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
         precondition (binary or Chrome)."
    );
}

/// The SAME rejection on the `exec` surface, and it needs no browser either.
///
/// `exec` runs one step and used to launch the browser as its very first
/// instruction — before it had even read the `cmd` — so `exec cookie --action
/// nonsense` paid a full Chrome to reach a usage error decided by argv. Same
/// defect as the script path, second surface. This case carries no Chrome
/// guard, so a regression that moves the check back behind the launch fails
/// here rather than hiding on a host that happens to have Chrome.
///
/// Measured 2026-08-18 on this host: the rejection returns in ~200 ms while the
/// same command with a VALID action takes ~2130 ms, because that one really
/// does launch a browser. Ten times the cost is what the check removes.
#[test]
fn exec_rejects_an_unknown_action_without_launching_a_browser() {
    if missing_binary(GATE) {
        return;
    }
    let bin = binary().expect("binary");
    let out = common::isolated_cmd(&bin)
        .args([
            "-q",
            "--json",
            "exec",
            "cookie",
            "--action",
            "action-that-does-not-exist",
        ])
        .output()
        .expect("spawn browser-automation-cli");
    let env: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be JSON; CLEAN STDOUT is contract");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "a typo must not be treated as a runnable step: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("usage"),
        "malformed argv is a usage error, not an unavailable browser: {env}"
    );
    assert!(
        env.pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("cookie"),
        "the failure must name the step: {env}"
    );
}
