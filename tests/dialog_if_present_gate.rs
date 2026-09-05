//! Permanent gate: `dialog --if-present` tolerates absence WITHOUT hiding it.
//!
//! # Why this file exists
//!
//! No file under `tests/` referenced `if_present` before this one. The other
//! mentions in the tree — `actions_browser.rs`, `browser_nav.rs`,
//! `nav/wait.rs` — declare the CLI and schema surface; none exercises the
//! behaviour.
//!
//! That was found the hard way: an automated split destroyed the handling block
//! in `src/commands/run/execute/page_steps/page.rs` and the loss was noticed only
//! because the cut happened to leave an unbalanced brace. At a syntactically
//! valid boundary the file would have compiled, the whole suite would have
//! passed, and the feature would have been dead in the product with a
//! good-faith report of success.
//!
//! # Why the third case is the one that matters
//!
//! `if_present` exists so a step can tolerate a dialog that may not be there.
//! The failure mode is not that it errors — it is that it degenerates into
//! always-on. If `dialog` never fails when no dialog is showing, the flag
//! distinguishes nothing and a script author loses the ability to REQUIRE that
//! the dialog was there.
//!
//! So absence is asserted twice, from both sides: tolerated WITH the flag, and
//! fatal WITHOUT it. Either case alone would be satisfied by a degenerate
//! implementation.
//!
//! # What makes the answer evidence
//!
//! `scripts/fixtures/dialog_if_present/page.html` writes the outcome of its own
//! `confirm()` into `#answer`. A command that reported success without actually
//! answering the dialog would leave that paragraph at `none`, so the assertion
//! is on what the PAGE observed rather than on what the envelope claims.
//!
//! The page is read with `text` after `dialog` (GAP-054 settle in the product).
//!
//! # The five committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | a pending dialog is accepted, and the page saw it |
//! | declared arm | `dismiss` is a distinct answer, not a synonym for accept |
//! | negative: tolerated absence | no dialog plus the flag SUCCEEDS and says so |
//! | discrimination | no dialog WITHOUT the flag FAILS |
//! | environment guard | the host really ran the four above |
//!
//! # What this file does NOT cover
//!
//! - It does not cover `prompt()` text entry, only `confirm()`.
//! - It does not cover the one-shot `dialog` subcommand, only the `run` step.
//! - It does not cover the dialog PRECONDITION that blocks other commands while
//!   one is open; that lives in `src/capability/` and has its own unit tests.
//! - It says nothing about `refs_invalidated`, which `dialog` also stamps and
//!   which is covered by `tests/ref_invalidation_gate.rs`.
//!
//! # GAP-054 closed: dialog settles before return
//!
//! `dialog` suppresses stale `javascriptDialogOpening` and waits for
//! `javascriptDialogClosed` (or the XDG `dialog_settle_ms` budget) so the next
//! page-observing step is not refused with precondition / exit 75. This gate
//! asserts that path with `text` immediately after `dialog` (no artificial wait).
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY, and
//! `the_host_can_actually_run_this_gate` turns that skip into exactly one red
//! case instead of four silent greens.

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "dialog_if_present_gate";

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/dialog_if_present/page.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// Run a script through `run` and return the parsed envelope.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-dialog-gate-")
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

/// `data` of the first step with the given `cmd`.
fn step_data(env: &serde_json::Value, cmd: &str) -> Option<serde_json::Value> {
    env.pointer("/data/steps")?
        .as_array()?
        .iter()
        .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some(cmd))
        .and_then(|s| s.get("data").cloned())
}

/// Page text from the `text` step after dialog (GAP-054: no artificial settle).
fn matched_page_text(env: &serde_json::Value) -> String {
    step_data(env, "text")
        .and_then(|d| {
            d.get("text")
                .or_else(|| d.get("value"))
                .or_else(|| d.get("result"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

/// Script that opens a dialog, answers it, then reads `#answer` with `text`.
///
/// GAP-054: product settles after `handleJavaScriptDialog`, so no wait between
/// `dialog` and `text` is required.
fn answer_script(action: &str) -> Vec<String> {
    let url = fixture_url().expect("fixture url");
    vec![
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"press","target":"#ask"}"##.to_string(),
        format!(r#"{{"cmd":"dialog","action":"{action}","if_present":true}}"#),
        r##"{"cmd":"text","target":"#answer"}"##.to_string(),
    ]
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url().is_none() {
        common::skip_with_reason(
            "dialog_if_present_gate",
            "fixture scripts/fixtures/dialog_if_present/page.html absent.",
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// POSITIVE CONTROL: a pending dialog is accepted and the PAGE saw the answer.
///
/// Asserting only the envelope would accept a command that reported success
/// without answering anything. `#answer` is written by the page's own handler
/// from the return value of `confirm()`, so it cannot be forged from outside.
#[test]
fn a_pending_dialog_is_accepted_and_the_page_records_it() {
    if cannot_run() {
        return;
    }
    let env = run_script(&answer_script("accept")).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "answering a dialog that IS showing must succeed: {env}"
    );
    let data = step_data(&env, "dialog").expect("dialog step data");
    assert_eq!(
        data.get("dialog").and_then(|v| v.as_str()),
        Some("accept"),
        "the envelope must report the action taken: {data}"
    );
    assert_eq!(
        data.get("dialog_settled").and_then(|v| v.as_bool()),
        Some(true),
        "GAP-054: Page.javascriptDialogClosed must be observed so the agent \
         envelope does not lie with dialog_settled:false after a real answer: {data}"
    );
    assert!(
        data.get("dialog_shown").is_none(),
        "`dialog_shown` belongs to the tolerated-absence path and must not appear \
         when a dialog really was answered: {data}"
    );
    assert_eq!(
        matched_page_text(&env),
        "accepted",
        "the page own confirm() must have returned true, and text after dialog \
         must read it (GAP-054). Anything else means the dialog was reported as \
         handled without being answered."
    );
}

/// DECLARED ARM: `dismiss` is a distinct answer, not a synonym for accept.
///
/// The two arms sit next to each other in the handler and a split or a refactor
/// can collapse them without breaking anything that compiles. The page records
/// which one arrived, so a collapse shows up here rather than in production.
#[test]
fn dismiss_is_a_distinct_answer_and_not_a_synonym_for_accept() {
    if cannot_run() {
        return;
    }
    let env = run_script(&answer_script("dismiss")).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "dismissing a dialog that IS showing must succeed: {env}"
    );
    assert_eq!(
        step_data(&env, "dialog")
            .and_then(|d| d.get("dialog").and_then(|v| v.as_str()).map(str::to_string))
            .unwrap_or_default(),
        "dismiss",
        "the envelope must report the dismissal: {env}"
    );
    assert_eq!(
        matched_page_text(&env),
        "dismissed",
        "the page's confirm() must have returned false, and text after dialog \
         must read it (GAP-054). If the two arms collapsed into one, the page would say \
         `accepted` and this wait would time out instead of matching."
    );
}

/// NEGATIVE: with no dialog pending, the flag makes the step SUCCEED — and the
/// envelope says the dialog was not there.
///
/// Tolerating absence silently would be almost as bad as failing: the caller
/// could not tell a handled dialog from a missing one. `dialog_shown: false` is
/// what keeps the tolerance honest.
#[test]
fn no_dialog_with_the_flag_succeeds_and_reports_the_absence() {
    if cannot_run() {
        return;
    }
    // Both spellings run as steps of ONE script, on purpose. An earlier draft
    // launched a second browser just to check the camelCase alias, and that
    // second launch made the case fail intermittently under load — an
    // environment failure arriving dressed as an alias defect. One launch for
    // two assertions is both cheaper and less ambiguous when it breaks.
    let url = fixture_url().expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r#"{"cmd":"dialog","action":"accept","if_present":true}"#.to_string(),
        r#"{"cmd":"dialog","action":"accept","ifPresent":true}"#.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "tolerating a missing dialog is the whole point of the flag: {env}"
    );

    let dialogs: Vec<serde_json::Value> = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("dialog"))
                .filter_map(|s| s.get("data").cloned())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        dialogs.len(),
        2,
        "expected both spellings to have run: {env}"
    );

    for (spelling, data) in ["if_present", "ifPresent"].iter().zip(&dialogs) {
        assert_eq!(
            data.get("dialog_shown").and_then(|v| v.as_bool()),
            Some(false),
            "with `{spelling}` the absence must be visible in the envelope, not \
             swallowed: {data}"
        );
        assert_eq!(
            data.get("if_present").and_then(|v| v.as_bool()),
            Some(true),
            "with `{spelling}` the envelope must record that tolerance was \
             requested: {data}"
        );
    }
}

/// DISCRIMINATION: with no dialog pending and NO flag, the step must FAIL.
///
/// This is the case that stops `if_present` from degenerating into always-on.
/// Without it, an implementation that never failed on a missing dialog would
/// satisfy every other case in this file while the flag distinguished nothing,
/// and a script author would lose the ability to REQUIRE that the dialog was
/// there.
#[test]
fn no_dialog_without_the_flag_fails() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r#"{"cmd":"dialog","action":"accept"}"#.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "without the flag, a missing dialog is an error. Succeeding here would \
         make `if_present` mean nothing: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert!(
        message.contains("dialog"),
        "the failure must name the dialog so the caller is not sent looking at \
         the previous step; got {message}"
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
