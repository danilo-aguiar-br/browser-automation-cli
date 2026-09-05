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

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "scrape_step_gate";

const HEAD_TOKEN: &str = "CONTENT_HEAD_K1L2";
const BODY_TOKEN: &str = "CONTENT_BODY_P5Q6";

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/content/page.html");
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
        .prefix("bac-scrape-gate-")
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
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url().is_none() {
        common::skip_with_reason(
            "scrape_step_gate",
            "fixture scripts/fixtures/content/page.html absent.",
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
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
        common::skip_with_reason(
            "scrape_ignores_the_currently_open_page",
            "scripts/fixtures/assert_step/page.html absent.",
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
    let out = common::isolated_cmd(&bin)
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
    // Both envelopes are printed on failure. This case launches the browser
    // TWICE in sequence, so it is the one most exposed to a launch that loses a
    // contended host; measured 2026-08-18, the second launch failed inside the
    // full 65-binary suite while the same case passed 6 of 6 alone. The bare
    // `assert_eq!` that used to stand here reported only `Some(false)` against
    // `Some(true)`, discarding the `error` field that says WHICH launch failed
    // and why — the sibling assertions in this file already carry their envelope.
    assert_eq!(
        text_env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "format=text scrape must succeed: {text_env}"
    );
    assert_eq!(
        md_env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "format=markdown scrape must succeed: {md_env}"
    );
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

/// Run a script and return BOTH the process exit code and the parsed envelope.
///
/// `run_script` above discards the status, which is right for the cases that only
/// read the envelope. The `engine` refusal below must pin the exit code too: an
/// agent branches on the process status before it parses anything, so a usage
/// error that answered exit 0 would be read as success no matter what the JSON
/// said.
fn run_script_with_status(lines: &[String]) -> Option<(i32, serde_json::Value)> {
    let bin = binary()?;
    let scratch = tempfile::Builder::new()
        .prefix("bac-scrape-gate-")
        .tempdir()
        .ok()?;
    let script = scratch.path().join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "120", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;

    let env = serde_json::from_slice(&out.stdout).ok()?;
    Some((out.status.code().unwrap_or(-1), env))
}

/// `engine` inside a run step is REFUSED, and the refusal names the alternative.
///
/// # Why refusing beats honouring, and beats ignoring
///
/// Inside `run` the browser session is already live, so the engine was settled at
/// launch. Honouring the field would mean relaunching mid-script; the step has no
/// authority to do that.
///
/// Ignoring it is what the step used to do, and MEASURED 2026-08-31 that produced
/// the one shape a caller cannot detect: a step asking for `"engine":"http"`
/// returned `ok: true` carrying `engine: "browser"`. The envelope contradicted the
/// request and still reported success, so nothing an agent reads would reveal that
/// the field was thrown away.
///
/// The message is asserted, not just the kind. A bare usage error would send the
/// caller looking for a typo in the step instead of at the top-level `scrape`,
/// which is where the engine choice actually lives.
#[test]
fn scrape_step_refuses_engine_and_names_the_top_level_alternative() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture url");
    let step = format!(r#"{{"cmd":"scrape","url":"{url}","format":"text","engine":"http"}}"#);
    let (code, env) = run_script_with_status(&[step]).expect("run envelope");

    assert_eq!(
        code, 2,
        "a step field the command cannot honour is malformed input, and exit 2 is \
         what an agent branches on before parsing: {env}"
    );
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "accepting the key and discarding it is the defect this pins: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("usage"),
        "a field this command cannot honour is a usage error: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("engine"),
        "the refusal must name the field that caused it; got {message}"
    );
    assert!(
        message.contains("scrape") && message.contains("--engine"),
        "the refusal must name WHERE the engine can be chosen — the top-level \
         `scrape --engine` — or the caller edits the step forever; got {message}"
    );
}

/// CONTROL: the same step WITHOUT `engine` still succeeds.
///
/// This case is what stops the refusal above from being satisfied by a step that
/// broke for some unrelated reason. Without it, a `scrape` arm that started
/// failing on every input would keep the rejection test green while silently
/// destroying the feature.
#[test]
fn scrape_step_without_engine_still_succeeds() {
    if cannot_run() {
        return;
    }
    let (code, env) = run_script_with_status(&[scrape("text")]).expect("run envelope");

    assert_eq!(
        code, 0,
        "dropping `engine` must leave a working step, not a broken one: {env}"
    );
    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the control proves the refusal is about the FIELD, not about scrape: {env}"
    );
    let data = scrape_data(&env).expect("scrape step data");
    let text = data
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        text.contains(HEAD_TOKEN) && text.contains(BODY_TOKEN),
        "the control must return the fixture's own tokens, or it proves nothing \
         about the step still working; got {text:?}"
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
