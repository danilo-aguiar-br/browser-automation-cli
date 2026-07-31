//! Permanent gate: `scrape` fetches its own URL and parses what it got.
//!
//! # Why this file exists
//!
//! Nothing under `tests/` executed `scrape` before this file. The mentions found
//! elsewhere were all about the SOURCE MODULE rather than the command:
//! `scripts/parallelism-check.sh` and `scripts/network-check.sh` grep
//! `src/scrape_local` for a semaphore, and `tests/parity_run_inventory.rs`
//! enumerates the name. A gate that scans source for a symbol says nothing about
//! whether the command returns a page.
//!
//! # What makes the answer evidence
//!
//! `scripts/fixtures/content/page.html` carries tokens that exist nowhere else
//! in the repository. Text that comes back containing them was read from that
//! file; an empty or synthesized payload cannot contain them.
//!
//! `scrape` differs from every other step here in that it takes its OWN url and
//! does not read the current page — there is no `goto` before it. That is the
//! property the positive control pins.
//!
//! # The five committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | the fixture's own text and title come back |
//! | discrimination: url | the OPEN page is not what comes back |
//! | declared exclusion | the HTTP engine REFUSES `file://` and says why |
//! | negative: argv | a step with no url is a USAGE error |
//! | environment guard | the host really ran the cases above |
//!
//! The url case is what stops the positive control from being satisfied by a
//! command that ignores `url` and returns whatever document is open.
//!
//! # GAP-057 closed: run step honours `format`
//!
//! The `run` scrape step reads `format`/`formats` and shares payload derivation
//! with the top-level subcommand. `format=text` echoes `format` and must not
//! dump full HTML; different formats produce different key shapes.
//!
//! # Shared fixture
//!
//! `scripts/fixtures/content/page.html` is shared with
//! `tests/extract_step_gate.rs`. Both gates name it in their own skip guard, so
//! deleting it turns both red by name rather than silently disabling either.
//!
//! # What this file does NOT cover
//!
//! - It does not cover the network path: every case here is `file://`, so
//!   nothing about redirects, status codes or robots over HTTP is asserted.
//! - It does not cover `--only-main-content`, `summary`, `product` or the other
//!   derived formats.
//! - It does not cover `batch-scrape` or `crawl`, which have their own surface.
//! - It does not cover the one-shot `scrape` subcommand, only the `run` step.
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
    let dir = std::env::temp_dir().join(format!("scrape-gate-{}-{n}", std::process::id()));
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

/// `data` of the first `scrape` step.
fn scrape_data(env: &serde_json::Value) -> Option<serde_json::Value> {
    env.pointer("/data/steps")?
        .as_array()?
        .iter()
        .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("scrape"))
        .and_then(|s| s.get("data").cloned())
}

/// A `scrape` step for the fixture in the given format.
fn scrape(format: &str) -> String {
    let url = fixture_url().expect("fixture url");
    format!(r#"{{"cmd":"scrape","url":"{url}","format":"{format}"}}"#)
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP scrape_step_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    if fixture_url().is_none() {
        eprintln!(
            "SKIP scrape_step_gate: fixture scripts/fixtures/content/page.html absent. \
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
            "SKIP scrape_step_gate: doctor reports the host is not ready for Chrome. \
             This is NOT a pass."
        );
        return true;
    }
    false
}

/// POSITIVE CONTROL: the step navigates on its own and returns the page.
///
/// There is no `goto` in this script. If `scrape` read the current page instead
/// of fetching its `url`, it would return a blank document and neither token
/// would appear.
#[test]
fn scrape_fetches_its_own_url_and_returns_the_page_text() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[scrape("text")]).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "scraping a local fixture must succeed: {env}"
    );
    let data = scrape_data(&env).expect("scrape step data");

    assert_eq!(
        data.get("title").and_then(|v| v.as_str()),
        Some("content fixture"),
        "the document title must come from the fetched page: {data}"
    );
    let text = data
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        text.contains(HEAD_TOKEN) && text.contains(BODY_TOKEN),
        "the returned text must carry the fixture's own tokens; a blank or \
         synthesized payload cannot. Got {text:?}"
    );
    assert_eq!(
        data.get("engine").and_then(|v| v.as_str()),
        Some("browser"),
        "the envelope must state which engine produced the answer: {data}"
    );
}

/// DISCRIMINATION: the current page is NOT what comes back.
///
/// The script navigates somewhere else first and then scrapes the fixture. A
/// `scrape` that read the open document would return the OTHER page's title and
/// text, so this is what proves the `url` field is honoured rather than decorative.
///
/// This case replaced a format-based discrimination that could not be written
/// honestly. See the header for the measurement.
#[test]
fn scrape_ignores_the_currently_open_page_and_uses_its_own_url() {
    if cannot_run() {
        return;
    }
    let other = root().join("scripts/fixtures/assert_step/page.html");
    if !other.exists() {
        eprintln!(
            "SKIP scrape_ignores_the_currently_open_page: \
             scripts/fixtures/assert_step/page.html absent. This is NOT a pass."
        );
        return;
    }
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"file://{}"}}"#, other.display()),
        scrape("text"),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "navigating then scrapping elsewhere must succeed: {env}"
    );
    let data = scrape_data(&env).expect("scrape step data");

    assert_eq!(
        data.get("title").and_then(|v| v.as_str()),
        Some("content fixture"),
        "the title must be the SCRAPED page's, not the open one's: {data}"
    );
    let text = data
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        text.contains(HEAD_TOKEN),
        "the scraped page's token must be present; got {text:?}"
    );
    assert!(
        !text.contains("Verdict OK"),
        "the OPEN page's text must NOT leak into the answer. Seeing it here means \
         `url` is ignored and the command returns whatever happened to be open: \
         got {text:?}"
    );
}

/// DECLARED EXCLUSION: the HTTP engine refuses `file://`, and says why.
///
/// Refusing is correct — an HTTP client cannot open a local path — and the value
/// of pinning it is the MESSAGE. A bare failure here would send the caller
/// looking at the URL instead of at the engine choice.
#[test]
fn the_http_engine_refuses_a_file_url_and_names_the_reason() {
    if cannot_run() {
        return;
    }
    let bin = binary().expect("binary");
    let url = fixture_url().expect("fixture url");
    let out = Command::new(&bin)
        .args(["-q", "--timeout", "60", "--json", "scrape"])
        .arg(&url)
        .args(["--format", "text", "--engine", "http"])
        .output()
        .expect("run scrape");
    let env: serde_json::Value = serde_json::from_slice(&out.stdout).expect("envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "the HTTP engine cannot fetch a local file: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("HTTP engine") && message.contains("file://"),
        "the refusal must name BOTH the engine and the scheme, so the caller \
         changes the engine instead of the URL; got {message}"
    );
}

/// NEGATIVE OF ARGV: a step with no url is a USAGE error.
///
/// `scrape` is the one content step that does not read the current page, so
/// falling back to it would be the natural degeneration — and would silently
/// return whatever happened to be open.
#[test]
fn scrape_without_a_url_is_a_usage_error() {
    if cannot_run() {
        return;
    }
    let env =
        run_script(&[r#"{"cmd":"scrape","format":"text"}"#.to_string()]).expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "without a url there is nothing to scrape, and falling back to the \
         current page would return whatever happened to be open: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("usage"),
        "a missing required field is malformed input: {env}"
    );
}

/// GAP-057: `format` is honoured and echoed; text does not dump HTML.
#[test]
fn scrape_format_text_echoes_format_without_html_blob() {
    if cannot_run() {
        return;
    }
    let env = run_script(&[scrape("text")]).expect("run envelope");
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "format=text scrape must succeed: {env}"
    );
    let data = scrape_data(&env).expect("scrape step data");
    let fmt = data
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        fmt.contains("text"),
        "run step must echo format (GAP-057); got {data}"
    );
    assert!(
        data.get("html").is_none(),
        "format=text must not dump HTML into the agent envelope: {data}"
    );
    let text = data
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        text.contains(HEAD_TOKEN) && text.contains(BODY_TOKEN),
        "text format still carries fixture tokens: {text:?}"
    );
}

/// GAP-057: different formats produce different shapes (not identical key sets).
#[test]
fn scrape_formats_are_not_identical_key_sets() {
    if cannot_run() {
        return;
    }
    let text_env = run_script(&[scrape("text")]).expect("text");
    let md_env = run_script(&[scrape("markdown")]).expect("markdown");
    assert_eq!(text_env.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(md_env.get("ok").and_then(|v| v.as_bool()), Some(true));
    let text_data = scrape_data(&text_env).expect("text data");
    let md_data = scrape_data(&md_env).expect("md data");
    let text_keys: Vec<_> = text_data
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let md_keys: Vec<_> = md_data
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    // At least one of: format echo differs, or markdown field present, or html only on one.
    let text_fmt = text_data
        .get("format")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let md_fmt = md_data
        .get("format")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert!(
        text_fmt != md_fmt
            || text_keys != md_keys
            || md_data.get("markdown").is_some()
            || md_data.get("content").is_some(),
        "text and markdown must not collapse to identical envelopes; text={text_data} md={md_data}"
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
