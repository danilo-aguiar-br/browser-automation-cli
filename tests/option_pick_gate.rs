//! Permanent gate: choosing an option really changes the control.
//!
//! # Why this file exists
//!
//! Nothing under `tests/` executed `select-option` or `pick` before this file,
//! and the gap that absence hid was a FALSE POSITIVE: on a native `<select>`,
//! passing a CSS selector for the `<option>` returned `ok` and left the control
//! at its old value. Clicking an `<option>` is not how a browser changes a
//! `<select>`.
//!
//! A reported success that changed nothing is worse than a failure. The script
//! carries the stale value forward and breaks somewhere else, so the diagnosis
//! lands on the wrong command — the same shape as `press` on a look-alike button
//! in `tests/submit_form_gate.rs`.
//!
//! # Why one gate for two commands
//!
//! `select-option`, `select_option` and `pick` are the SAME dispatch arm. One
//! file with the aliases asserted together is honest; three files would imply
//! three behaviours.
//!
//! # What makes the choice evidence
//!
//! `scripts/fixtures/option_pick/page.html` writes the chosen value into
//! `#chosen` from its OWN handlers, and the two controls write different
//! prefixes: `native:` from the `<select>`'s `change` listener and `popover:`
//! from the custom list's click listener. So the assertion distinguishes not
//! only WHETHER something was chosen but WHICH route did it, and a command that
//! reported success without choosing leaves the paragraph at `none`.
//!
//! The popover options do not exist until the trigger is pressed, so reaching
//! one is also proof that the trigger was opened first.
//!
//! # The six committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive: native by text | a `<select>` really moves, and the page saw it |
//! | positive: native by value | the option is matched on value as well as text |
//! | positive: popover | the custom route still works and is reported as its own |
//! | negative: absent option | an option that does not exist FAILS, with the list |
//! | negative: argv | a missing option is a USAGE error |
//! | environment guard | the host really ran the cases above |
//!
//! The absent-option case is the one that pins the fix: before it, a missing
//! option on a `<select>` was the path that returned `ok` and changed nothing.
//!
//! # What this file does NOT cover
//!
//! - It does not cover `<select multiple>`.
//! - It does not cover options added after load, or `<optgroup>`.
//! - It does not cover the one-shot `exec select-option` argv path beyond its
//!   existence, which `tests/parity_run_inventory.rs` covers by name.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY, and
//! `the_host_can_actually_run_this_gate` turns that skip into exactly one red
//! case instead of five silent greens.

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "option_pick_gate";

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/option_pick/page.html");
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
        .prefix("bac-option-pick-gate-")
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

/// What the page recorded in `#chosen`, read through a `text` step.
fn page_choice(env: &serde_json::Value) -> String {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("text"))
                .and_then(|s| s.pointer("/data/text").cloned())
        })
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The `via` route reported by the choosing step, empty when absent.
fn route(env: &serde_json::Value) -> String {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .find(|s| {
                    matches!(
                        s.get("cmd").and_then(|c| c.as_str()),
                        Some("select-option") | Some("select_option") | Some("pick")
                    )
                })
                .and_then(|s| s.pointer("/data/via").cloned())
        })
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

const READ_CHOICE: &str = r##"{"cmd":"text","target":"#chosen"}"##;
const READ_EVENTS: &str = r##"{"cmd":"extract","selector":"#events"}"##;

/// The event names the page saw, in order, as a comma-joined string.
///
/// Read with `extract` rather than a second `text` step so `page_choice` keeps
/// finding the value and not this.
fn page_events(env: &serde_json::Value) -> String {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("extract"))
                .and_then(|s| s.pointer("/data/text").cloned())
        })
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// A choose-then-read script using the given command alias.
fn choose(cmd: &str, target: &str, option: &str) -> Vec<String> {
    let url = fixture_url().expect("fixture url");
    vec![
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        format!(r#"{{"cmd":"{cmd}","target":"{target}","option":"{option}"}}"#),
        READ_CHOICE.to_string(),
        READ_EVENTS.to_string(),
    ]
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url().is_none() {
        common::skip_with_reason(
            "option_pick_gate",
            "fixture scripts/fixtures/option_pick/page.html absent.",
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// POSITIVE CONTROL: a native `<select>` really moves, matched by option TEXT.
///
/// `#chosen` is written by the page's own `change` listener, so `native:alpha`
/// can only appear if the event was dispatched. Before the fix this path
/// reported success and left the paragraph at `none`.
#[test]
fn a_native_select_changes_when_the_option_is_named_by_text() {
    if cannot_run() {
        return;
    }
    let env = run_script(&choose("select-option", "#native", "Alpha")).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "choosing an existing option must succeed: {env}"
    );
    assert_eq!(
        route(&env),
        "native_select",
        "a `<select>` must take the native route, not the popover one: {env}"
    );
    assert_eq!(
        page_choice(&env),
        "native:alpha",
        "the page's own change listener must have run. Reading `none` here is \
         the false positive this gate exists for: the command reported success \
         and the control never moved."
    );

    // BOTH events, and this assertion is not decoration: an earlier draft of
    // this file recorded only `change`, and a mutation that removed the `input`
    // dispatch passed it unnoticed. Reactive forms listen for `input`, so
    // dispatching one of the two looks correct to half the pages in existence.
    assert_eq!(
        page_events(&env),
        "input,change",
        "the native route must dispatch `input` AND `change`, in that order. \
         Seeing only one of them means half the frameworks never notice the \
         value moved."
    );
    let dispatched = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("select-option"))
                .and_then(|s| s.pointer("/data/events_dispatched").cloned())
        })
        .unwrap_or(serde_json::Value::Null);
    assert_eq!(
        dispatched,
        serde_json::json!(["input", "change"]),
        "the envelope must state which events it sent, so a caller debugging a \
         framework that missed the change can see it without reading source: {env}"
    );
}

/// POSITIVE CONTROL: the option is also matched by its VALUE.
///
/// A caller reading the HTML sees `value="beta"`; a caller reading the rendered
/// page sees `Beta`. Both have to work, and matching only one of them would
/// send half the callers to a false `option not found`.
#[test]
fn a_native_select_changes_when_the_option_is_named_by_value() {
    if cannot_run() {
        return;
    }
    let env = run_script(&choose("select-option", "#native", "beta")).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "matching on value must succeed: {env}"
    );
    assert_eq!(
        page_choice(&env),
        "native:beta",
        "the value-matched option must be the one selected"
    );
}

/// POSITIVE CONTROL: the popover route still works, and reports itself.
///
/// The custom list is what the command was originally written for, and the
/// native route must not have cost it. The options do not exist until the
/// trigger is pressed, so `popover:High` is also proof the trigger was opened.
///
/// `pick` is used here and `select-option` above: they are the same dispatch
/// arm, and exercising both aliases is what keeps one from silently diverging.
#[test]
fn the_popover_route_still_works_and_is_reported_separately() {
    if cannot_run() {
        return;
    }
    let env = run_script(&choose("pick", "#trigger", "High")).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the custom popover must still be pickable: {env}"
    );
    assert_ne!(
        route(&env),
        "native_select",
        "a non-select target must NOT take the native route: {env}"
    );
    assert_eq!(
        page_choice(&env),
        "popover:High",
        "the popover's own click listener must have run, which also proves the \
         trigger was opened first — the options do not exist before that"
    );
}

/// NEGATIVE: an option that does not exist FAILS, and the error lists what does.
///
/// This is the case that pins the fix. The route it replaces returned `ok` for a
/// selector that matched nothing useful, so the absence of this case is exactly
/// how the false positive survived.
#[test]
fn an_option_that_does_not_exist_fails_and_lists_the_real_ones() {
    if cannot_run() {
        return;
    }
    let env = run_script(&choose("select-option", "#native", "Gamma")).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "an option the control does not have must NOT report success: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("data"),
        "the argv is well formed and the page disagrees with it, so this is a \
         data error rather than a usage one: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("Gamma"),
        "the failure must echo the option that was asked for; got {message}"
    );
}

/// NEGATIVE OF ARGV: a step with no option is a USAGE error.
#[test]
fn choosing_without_an_option_is_a_usage_error() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"select-option","target":"#native"}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "a step with nothing to choose must not be skipped: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("usage"),
        "a missing required field is malformed input: {env}"
    );
}

/// ENVIRONMENT GUARD: this one never skips.
///
/// The other cases in this file return early when the host is not ready, and a
/// test that returns counts as a PASS. On a machine without Chrome that turns
/// the whole file green while it tested nothing, and the honest SKIP lines this
/// file writes to stderr are easy to lose in `cargo test` output.
///
/// A test that fails the ENVIRONMENT is not a test that fails the CODE.
#[test]
fn the_host_can_actually_run_this_gate() {
    assert!(
        !cannot_run(),
        "host cannot run this gate: every other case in this file skipped, and a \
         skip is NOT a pass. The SKIP line on stderr names the missing \
         precondition (binary, fixture, or Chrome)."
    );
}
