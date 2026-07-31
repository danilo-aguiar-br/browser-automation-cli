//! Permanent gate: `extract` resolves its target on the LIVE page.
//!
//! # Why this file exists
//!
//! Nothing under `tests/` executed the `extract` step before this file. The only
//! mentions were `tests/parity_run_inventory.rs` and `tests/parity_semantics.rs`,
//! which enumerate NAMES, plus a match arm in `src/commands/run/argv.rs`, which
//! is production code. A name in an inventory is not a node being read.
//!
//! # Why one positive is not enough
//!
//! An `extract` that ignored its selector and returned the document text would
//! satisfy a single positive control, because the headline token appears in the
//! document text too. The gate therefore extracts TWO different nodes in one run
//! and requires two DIFFERENT answers. Only a command that actually resolves the
//! selector can produce that.
//!
//! The fixture's tokens exist nowhere else in the repository, so text that comes
//! back carrying one of them cannot have been synthesized.
//!
//! # The five committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | the named node's text comes back |
//! | discrimination | two selectors in one run give two different answers |
//! | negative: absent node | a selector that matches nothing FAILS |
//! | negative: argv | a missing selector is a USAGE error |
//! | environment guard | the host really ran the cases above |
//!
//! The absent-node case is the one that matters most: returning empty text for a
//! node that does not exist would be indistinguishable from a node that exists
//! and is empty, and the caller would act on a blank string.
//!
//! # Shared fixture
//!
//! `scripts/fixtures/content/page.html` is shared with
//! `tests/scrape_step_gate.rs`. Both gates name it in their own skip guard, so
//! deleting it turns both red by name rather than silently disabling either.
//!
//! # What this file does NOT cover
//!
//! - It does not cover `extract --llm`, which needs a key and a network call.
//! - It does not cover the one-shot `extract` subcommand, only the `run` step.
//! - It does not cover schema-guided extraction, only text of a single node.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY, and
//! `the_host_can_actually_run_this_gate` turns that skip into exactly one red
//! case instead of four silent greens.

use std::path::PathBuf;
use std::process::Command;

const HEAD_TOKEN: &str = "CONTENT_HEAD_K1L2";
const BODY_TOKEN: &str = "CONTENT_BODY_P5Q6";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary() -> Option<PathBuf> {
    let p = root().join("target/debug/browser-automation-cli");
    p.exists().then_some(p)
}

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/content/page.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// Run a script through `run` and return the parsed envelope.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("extract-gate-{}-{n}", std::process::id()));
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

/// The `text` reported by each `extract` step, in order.
fn extracted(env: &serde_json::Value) -> Vec<String> {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("extract"))
                .map(|s| {
                    s.pointer("/data/text")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn goto() -> String {
    let url = fixture_url().expect("fixture url");
    format!(r#"{{"cmd":"goto","url":"{url}"}}"#)
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP extract_step_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    if fixture_url().is_none() {
        eprintln!(
            "SKIP extract_step_gate: fixture scripts/fixtures/content/page.html absent. \
             This is NOT a pass."
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
            "SKIP extract_step_gate: doctor reports the host is not ready for Chrome. \
             This is NOT a pass."
        );
        return true;
    }
    false
}

/// POSITIVE CONTROL and DISCRIMINATION in one run.
///
/// Two selectors, two different tokens. A command that ignored the selector and
/// returned the document text would give the SAME answer twice, and the document
/// text contains both tokens — which is exactly why a single extraction would
/// not have caught it.
#[test]
fn two_selectors_in_one_run_resolve_to_two_different_nodes() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        goto(),
        r##"{"cmd":"extract","selector":"#headline"}"##.to_string(),
        r##"{"cmd":"extract","selector":"#detail"}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "both nodes exist, so both extractions must succeed: {env}"
    );
    let texts = extracted(&env);
    assert_eq!(texts.len(), 2, "expected two extractions: {env}");
    assert_eq!(
        texts[0], HEAD_TOKEN,
        "the first selector must return the headline node"
    );
    assert_eq!(
        texts[1], BODY_TOKEN,
        "the second selector must return the detail node"
    );
    assert_ne!(
        texts[0], texts[1],
        "two different selectors returning the same text means the selector is \
         accepted and then ignored; got {texts:?}"
    );
}

/// NEGATIVE: a selector that matches nothing must FAIL.
///
/// Returning empty text would be indistinguishable from a node that exists and
/// is empty, and the caller would act on a blank string believing it read the
/// page.
#[test]
fn a_selector_that_matches_nothing_fails_instead_of_returning_empty() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[
        goto(),
        r##"{"cmd":"extract","selector":"#node-that-does-not-exist"}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "a selector matching nothing must not succeed with empty text: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("extract"),
        "the failure must name the step so the caller is not sent looking at the \
         navigation; got {message}"
    );
}

/// NEGATIVE OF ARGV: a step with no selector is a USAGE error.
#[test]
fn extract_without_a_selector_is_a_usage_error() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[goto(), r#"{"cmd":"extract"}"#.to_string()]).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "a step with nothing to extract must not be skipped: {env}"
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
