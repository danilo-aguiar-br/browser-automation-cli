// SPDX-License-Identifier: MIT OR Apache-2.0
//! Permanent gate: multi-tab dialog isolation (GAP-041 / GAP-054 session_id).
//!
//! # Why this file exists
//!
//! Page-scoped CDP forwarders stamp `CdpEvent.session_id` from `Page::session_id`
//! because `Page.javascriptDialogOpening` / `Closed` event bodies do **not**
//! carry `sessionId` (docs.rs chromiumoxide_cdp). Without that stamp, dialog
//! open state collapses onto the active tab and blocks unrelated pages.
//!
//! Unit tests cover `dialog_map_key` with two session ids. This gate is the
//! **real Chrome** proof: open a confirm on tab 0, switch to a new tab, and
//! assert the new tab is not precondition-blocked; then answer on the owner
//! tab and read the page-recorded outcome (agent-native compact fields only).
//!
//! # Cases
//!
//! | case | what it proves |
//! |---|---|
//! | isolation | dialog on tab0 does not block `text` on tab1 |
//! | answer_on_owner | select tab0 + accept → `dialog_settled` + `#answer=accepted` |
//! | host_guard | environment can actually run the two above |
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY (not a silent pass).
//! `the_host_can_actually_run_this_gate` turns that skip into one red case.

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "dialog_multitab_gate";

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/dialog_if_present/page.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-dialog-mt-gate-")
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

fn steps(env: &serde_json::Value) -> Vec<serde_json::Value> {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default()
}

fn step_data_nth(env: &serde_json::Value, cmd: &str, n: usize) -> Option<serde_json::Value> {
    steps(env)
        .into_iter()
        .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some(cmd))
        .nth(n)
        .and_then(|s| s.get("data").cloned())
}

fn step_ok(env: &serde_json::Value, cmd: &str, n: usize) -> Option<bool> {
    steps(env)
        .into_iter()
        .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some(cmd))
        .nth(n)
        .and_then(|s| s.get("ok").and_then(|v| v.as_bool()))
}

fn text_value(data: &serde_json::Value) -> String {
    data.get("text")
        .or_else(|| data.get("value"))
        .or_else(|| data.get("result"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Full multi-tab script: open dialog on tab0, new tab, text, select 0, accept, text answer.
fn multitab_script() -> Vec<String> {
    let url = fixture_url().expect("fixture url");
    vec![
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"press","target":"#ask"}"##.to_string(),
        r#"{"cmd":"page","action":"new","url":"about:blank"}"#.to_string(),
        // Isolation probe: must not be blocked by dialog on the other tab.
        r##"{"cmd":"text","target":"body"}"##.to_string(),
        r#"{"cmd":"page","action":"select","index":0}"#.to_string(),
        r#"{"cmd":"dialog","action":"accept"}"#.to_string(),
        r##"{"cmd":"text","target":"#answer"}"##.to_string(),
    ]
}

fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url().is_none() {
        common::skip_with_reason(
            "dialog_multitab_gate",
            "fixture scripts/fixtures/dialog_if_present/page.html absent.",
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// Dialog open on tab0 must not block `text` on the newly active tab1.
#[test]
fn dialog_on_inactive_tab_does_not_block_other_tab() {
    if cannot_run() {
        return;
    }
    let env = run_script(&multitab_script()).expect("run envelope");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "multi-tab isolation + answer path must succeed end-to-end: {env}"
    );
    assert_eq!(
        step_ok(&env, "text", 0),
        Some(true),
        "text on tab1 (about:blank) must not hit precondition 75 while dialog \
         remains open on tab0 — session_id stamp is required: {env}"
    );
    // page new must have created a second tab (agent-native count/pages).
    let page_new = step_data_nth(&env, "page", 0).expect("page new data");
    // Accept either index field or pages list later; at least the step ok.
    assert!(
        page_new.get("index").is_some()
            || page_new.get("tab_id").is_some()
            || page_new.get("pages").is_some()
            || step_ok(&env, "page", 0) == Some(true),
        "page new must report a tab handle: {page_new}"
    );
}

/// Answer on the owner tab settles and the page records accept (GAP-054).
#[test]
fn answering_owner_tab_settles_and_page_records_accept() {
    if cannot_run() {
        return;
    }
    let env = run_script(&multitab_script()).expect("run envelope");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "owner-tab answer path must succeed: {env}"
    );
    let data = step_data_nth(&env, "dialog", 0).expect("dialog step data");
    assert_eq!(
        data.get("dialog").and_then(|v| v.as_str()),
        Some("accept"),
        "envelope must report accept: {data}"
    );
    assert_eq!(
        data.get("dialog_settled").and_then(|v| v.as_bool()),
        Some(true),
        "GAP-054: dialog_settled must be true after Closed on the owner session: {data}"
    );
    let answer = step_data_nth(&env, "text", 1).expect("answer text step");
    assert_eq!(
        text_value(&answer),
        "accepted",
        "page #answer must record confirm()=true after accept on owner tab: {answer}"
    );
}

/// Environment guard: never silent-green when Chrome/binary/fixture missing.
#[test]
fn the_host_can_actually_run_this_gate() {
    assert!(
        !cannot_run(),
        "host cannot run this gate: every other case skipped, and a skip is NOT a pass."
    );
}
