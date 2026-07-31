//! Permanent gate: the `@eN` staleness marker discriminates (GAP-042).
//!
//! # Why this file exists
//!
//! The snapshot stays opt-in to keep the envelope small, and the marker is the
//! other half of that trade: the caller is told its tree went stale instead of
//! discovering it a step later, on the wrong command.
//!
//! A marker only carries information if it is sometimes ABSENT. Stamping every
//! envelope would satisfy any positive control and destroy the signal, which is
//! why the reading case below matters more than the mutating one. It is the same
//! shape as the `cookie` exclusion in `tests/view_precondition_gate.rs`: the
//! question is never "does the field appear" but "does it appear only when it
//! should".
//!
//! # What ties the marker to reality
//!
//! `scripts/fixtures/ref_invalidation/page.html` really rebuilds its list on
//! press — one node removed, two inserted at the front — so references taken
//! before the action genuinely point elsewhere afterwards. Each case reads a DOM
//! signature before and after, so a marker is never accepted on a tree that did
//! not move, and absence is never accepted on a tree that did.
//!
//! # The four committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | a mutating action marks, and the tree really changed |
//! | negative | pure readers do not mark, and change nothing |
//! | declared exclusion | `eval` and `scroll` mark even when nothing moved |
//! | table guard | the classifier never admits a reading command |
//!
//! # What this file does NOT cover
//!
//! - It does not check that a stale `@eN` actually fails afterwards. Asserting
//!   the consequence would pin behaviour that belongs to ref resolution, not to
//!   the marker.
//! - It does not cover `--include-snapshot`, which is the opt-in half of the
//!   trade and unchanged by this gap.
//! - It does not cover one-shot invocations, only steps inside `run`.
//! - It says nothing about `view` refusing on a blank page; that dimension has
//!   its own gate in `tests/view_precondition_gate.rs`.
//!
//! # Skip policy
//!
//! The table guard needs no browser and always runs. The three behavioural cases
//! SKIP LOUDLY without a binary, a fixture or Chrome.

use std::path::PathBuf;
use std::process::Command;

use browser_automation_cli::capability::invalidates_refs;

/// Commands that only read. None of them may ever mark the tree stale.
const PURE_READERS: &[&str] = &["view", "text", "attr", "console", "net", "page", "wait"];

/// Commands that mark. Kept here so the table guard fails on a silent removal.
const MUTATORS: &[&str] = &[
    "press", "click", "write", "fill", "type", "keys", "hover", "drag", "submit", "upload", "goto",
    "back", "forward", "reload",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary() -> Option<PathBuf> {
    let p = root().join("target/debug/browser-automation-cli");
    p.exists().then_some(p)
}

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/ref_invalidation/page.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// Run a script through `run` and return the parsed envelope.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ref-inval-gate-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = Command::new(&bin)
        .args(["-q", "--timeout", "120", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    serde_json::from_slice(&out.stdout).ok()
}

/// `refs_invalidated` on the nth step with the given `cmd`, `None` when absent.
fn marker_at(env: &serde_json::Value, cmd: &str, nth: usize) -> Option<bool> {
    env.pointer("/data/steps")?
        .as_array()?
        .iter()
        .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some(cmd))
        .nth(nth)?
        .pointer("/data/refs_invalidated")?
        .as_bool()
}

/// Every `eval` result in order, used to read the DOM signature around actions.
fn eval_results(env: &serde_json::Value) -> Vec<String> {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
                .map(|s| {
                    s.pointer("/data/result")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default()
}

const SIGNATURE: &str = r#"{"cmd":"eval","expression":"window.signature()"}"#;

/// True when the host cannot run the behavioural cases. Never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP ref_invalidation_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    if fixture_url().is_none() {
        eprintln!(
            "SKIP ref_invalidation_gate: fixture scripts/fixtures/ref_invalidation/page.html \
             absent. This is NOT a pass."
        );
        return true;
    }
    let probe = Command::new(binary().expect("binary"))
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output();
    let chrome_ok = probe
        .ok()
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.get("ok").and_then(|b| b.as_bool()))
        .unwrap_or(false);
    if !chrome_ok {
        eprintln!(
            "SKIP ref_invalidation_gate: doctor reports the host is not ready for Chrome. \
             This is NOT a pass."
        );
        return true;
    }
    false
}

/// POSITIVE CONTROL: a mutating action marks the tree stale, and the tree really
/// did change.
///
/// The signature around the press is what keeps this from being a tautology. A
/// marker on a tree that never moved would be a false alarm, and this case would
/// no longer be evidence that the classification means anything.
#[test]
fn a_mutating_action_marks_the_tree_stale_and_the_tree_really_moved() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        SIGNATURE.to_string(),
        r##"{"cmd":"press","target":"#mutate"}"##.to_string(),
        SIGNATURE.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the script must run to completion: {env}"
    );
    assert_eq!(
        marker_at(&env, "press", 0),
        Some(true),
        "an interaction that rebuilds the DOM must declare the refs stale: {env}"
    );
    assert_eq!(
        marker_at(&env, "goto", 0),
        Some(true),
        "navigation replaces the document outright and must mark: {env}"
    );

    let signatures = eval_results(&env);
    assert_eq!(
        signatures.len(),
        2,
        "expected two signatures: {signatures:?}"
    );
    assert_ne!(
        signatures[0], signatures[1],
        "the fixture must actually rebuild its list, otherwise the marker above \
         is asserted on a tree that never moved and proves nothing; got {signatures:?}"
    );
}

/// NEGATIVE: reading commands do not mark, and they change nothing.
///
/// This is the case that gives the marker its value. A marker stamped on every
/// envelope would pass the positive control and carry no information at all, so
/// its ABSENCE here is the whole point.
#[test]
fn pure_reads_do_not_mark_the_tree_stale() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        SIGNATURE.to_string(),
        r#"{"cmd":"view"}"#.to_string(),
        r##"{"cmd":"text","target":"#title"}"##.to_string(),
        r##"{"cmd":"attr","target":"#field","name":"id"}"##.to_string(),
        SIGNATURE.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the reading script must succeed: {env}"
    );
    for cmd in ["view", "text", "attr"] {
        assert_eq!(
            marker_at(&env, cmd, 0),
            None,
            "`{cmd}` only reads and must NOT mark the tree stale. \
             A marker on every envelope carries no information: {env}"
        );
    }

    let signatures = eval_results(&env);
    assert_eq!(
        signatures.len(),
        2,
        "expected two signatures: {signatures:?}"
    );
    assert_eq!(
        signatures[0], signatures[1],
        "the reading commands must leave the tree untouched, otherwise their \
         silence would be wrong rather than correct; got {signatures:?}"
    );
}

/// DECLARED EXCLUSION: `eval` and `scroll` mark even when nothing moved.
///
/// The classification is by COMMAND, not by observed effect, and it errs toward
/// warning: arbitrary script can mutate anything, and scrolling can trigger
/// lazy loading. `1+1` moves nothing and still marks.
///
/// A false alarm costs the caller one `view`; a missed alarm costs a stale
/// reference resolved against the wrong node, discovered a step later on the
/// wrong command. The asymmetry is the reason for the bias, and pinning it here
/// stops the over-approximation from being read as a defect.
#[test]
fn eval_and_scroll_mark_conservatively_even_when_nothing_moved() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        SIGNATURE.to_string(),
        r#"{"cmd":"eval","expression":"1+1"}"#.to_string(),
        r#"{"cmd":"scroll","dy":40}"#.to_string(),
        SIGNATURE.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the script must succeed: {env}"
    );
    // The signature evals are themselves `eval` steps, so the arithmetic one is
    // the second of the three.
    assert_eq!(
        marker_at(&env, "eval", 1),
        Some(true),
        "`eval` marks by command, because a script may mutate anything: {env}"
    );
    assert_eq!(
        marker_at(&env, "scroll", 0),
        Some(true),
        "`scroll` marks by command, because scrolling can trigger lazy loading: {env}"
    );

    let signatures = eval_results(&env);
    assert_eq!(
        signatures.len(),
        3,
        "expected three eval results, two of them signatures: {signatures:?}"
    );
    assert_eq!(
        signatures[0], signatures[2],
        "this case is only a declared over-approximation if the tree really did \
         NOT move; got {signatures:?}"
    );
}

/// TABLE GUARD: the classifier never admits a reading command.
///
/// The behavioural cases above cover three readers. This one covers the rest
/// without a browser, and it is the check that catches the edit most likely to
/// happen: someone adding a command to the invalidating list "to be safe".
#[test]
fn the_classifier_separates_readers_from_mutators() {
    for cmd in PURE_READERS {
        assert!(
            !invalidates_refs(cmd),
            "`{cmd}` only reads, so marking it would erode the signal until the \
             field means nothing"
        );
    }
    for cmd in MUTATORS {
        assert!(
            invalidates_refs(cmd),
            "`{cmd}` can change the DOM, so dropping it from the table brings \
             back exactly the silent stale-tree failure this marker removed"
        );
    }
    // Normalisation is part of the contract: `run` accepts both spellings.
    assert!(
        invalidates_refs("fill_form") && invalidates_refs("fill-form"),
        "underscore and hyphen spellings must classify the same, otherwise one \
         of them silently stops marking"
    );
    assert!(
        invalidates_refs("  PRESS  "),
        "the classifier trims and lowercases; a caller-shaped string must not slip through"
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
