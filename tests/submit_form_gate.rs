//! Permanent gate for form submission as a distinct gesture from clicking (GAP-036).
//!
//! # Why this file exists
//!
//! The gap is not "there is no `submit` command". It is that clicking the button
//! that looks like the submit control reports success while nothing is sent, so
//! a failing script reads as a credential problem instead of a wrong gesture.
//!
//! A gate that asserted the command exists, or that its envelope has a
//! `submitted` field, would be green today and would stay green if `submit`
//! degraded into a plain click. So the assertions here are about what the PAGE
//! did, and about the command returning only once the outcome is known.
//!
//! # How the fixture makes the evidence unforgeable
//!
//! `scripts/fixtures/submit_form/login.html` appends `token=EVENT_RAN` to the
//! form from inside its own `submit` listener. The token therefore reaches the
//! destination URL only when the submit EVENT fired. A gesture that navigated by
//! any other route arrives without it.
//!
//! This is the same technique `tests/drag_route_gate.rs` uses, where the page's
//! drop handler rejects a payload it did not build.
//!
//! # The four committed cases
//!
//! | case | what it proves |
//! |---|---|
//! | positive control | a field target resolves to its form, the event fires, the command waits |
//! | discrimination | the look-alike click succeeds and changes nothing |
//! | negative | a cancelled submission is reported, not silently passed nor timed out |
//! | declared exclusion | constraint validation refuses instead of no-op |
//!
//! The discrimination case is the one that makes this a gate rather than a
//! smoke test: it pins the exact false success the gap describes, so the two
//! gestures can never quietly converge again.
//!
//! # What this file does NOT cover
//!
//! - It does not cover the `network` outcome, which needs a form whose handler
//!   issues a request; the fixture navigates instead.
//! - It does not cover submission inside an iframe.
//! - It does not check the command's presence in `commands` or in `schema`; that
//!   is surface, and surface is what this gate refuses to accept as evidence.
//! - It says nothing about the settling conditions of `wait`, which have their
//!   own gate in `tests/wait_conditions_gate.rs`.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY. A silent green here
//! would rebuild the blind spot this gate exists to remove.

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "submit_form_gate";

fn fixture_url(name: &str, query: &str) -> Option<String> {
    let p = root().join("scripts/fixtures/submit_form").join(name);
    p.exists().then(|| {
        if query.is_empty() {
            format!("file://{}", p.display())
        } else {
            format!("file://{}?{}", p.display(), query)
        }
    })
}

/// Run a script through `run` and return the parsed envelope.
///
/// Each invocation gets its own directory: the cases run as threads of ONE test
/// binary, so a path keyed only by pid would be shared and the scripts would
/// overwrite each other.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-submit-form-gate-")
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

/// Result of the last `eval` step, used to read `location.href` after the fact.
fn last_eval_result(env: &serde_json::Value) -> String {
    env.pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|steps| {
            steps
                .iter()
                .filter(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
                .next_back()
                .and_then(|s| s.pointer("/data/result").cloned())
        })
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if missing_binary(GATE) {
        return true;
    }
    for fixture in ["login.html", "landed.html"] {
        if fixture_url(fixture, "").is_none() {
            common::skip_with_reason(
                "submit_form_gate",
                &format!("fixture scripts/fixtures/submit_form/{fixture} absent."),
            );
            return true;
        }
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// POSITIVE CONTROL: a FIELD is enough of a target, the submit event fires, and
/// the command does not return until the navigation it caused has happened.
///
/// The `url_after` recorded by the step itself is the proof of the wait: it is
/// read inside `submit`, before the envelope is built, so it can only name the
/// destination if the command stayed until the page moved.
#[test]
fn submitting_a_field_runs_the_form_event_and_waits_for_the_navigation() {
    if cannot_run() {
        return;
    }
    let url = fixture_url("login.html", "").expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"submit","target":"#user","timeout_ms":8000}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "a valid form must submit: {env}"
    );
    let data = step_data(&env, "submit").expect("submit step data");

    assert_eq!(
        data.get("submit_event_fired").and_then(|v| v.as_bool()),
        Some(true),
        "the form's own submit event must have fired; a synthetic click on the \
         wrong node would not fire it: {data}"
    );
    assert_eq!(
        data.get("outcome").and_then(|v| v.as_str()),
        Some("navigation"),
        "this form navigates, so the reported outcome must be the navigation: {data}"
    );

    let url_after = data
        .get("url_after")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        url_after.contains("landed.html"),
        "submit must return only after the navigation completed; url_after={url_after}. \
         Seeing the login page here means the command returned early and the caller \
         would evaluate the old document."
    );
    assert!(
        url_after.contains("token=EVENT_RAN"),
        "the destination must carry the token the submit handler appended; \
         url_after={url_after}. Without it the page moved by some other route and \
         the form event never ran."
    );
    assert!(
        url_after.contains("user=alice"),
        "the field values must travel with the submission; url_after={url_after}"
    );
}

/// DISCRIMINATION: the look-alike button reports success and submits nothing.
///
/// This is the exact false success the gap was opened for, and pinning it is
/// what keeps the two gestures from quietly converging again. `press` is not
/// asserted to be broken — it did what it was asked, which is the problem.
#[test]
fn pressing_the_look_alike_button_succeeds_while_submitting_nothing() {
    if cannot_run() {
        return;
    }
    let url = fixture_url("login.html", "").expect("fixture url");

    // The settle step is load-bearing, and leaving it out was a real blind spot
    // in an earlier draft of this file. `press` does not wait for what it
    // causes: reading `location.href` on the very next step still sees the old
    // document even when a navigation is already in flight. Without the settle,
    // this case passed even after the fixture was mutated to make
    // `#fake-submit` a genuine submit control — it was reading the URL before
    // the move landed, not observing that no move happened.
    let pressed = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"press","target":"#fake-submit"}"##.to_string(),
        r#"{"cmd":"wait","dom_stable_ms":400,"ms":6000}"#.to_string(),
        r#"{"cmd":"eval","expression":"location.href"}"#.to_string(),
    ])
    .expect("press envelope");

    assert_eq!(
        pressed.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the click itself succeeds — that is precisely the trap: {pressed}"
    );
    let href = last_eval_result(&pressed);
    assert!(
        href.contains("login.html") && !href.contains("landed.html"),
        "clicking a non-submit control must leave the page where it was; href={href}. \
         If this reaches landed.html the fixture stopped discriminating and the rest \
         of this file proves nothing."
    );
    assert!(
        !href.contains("token=EVENT_RAN"),
        "no submit event ran, so the token cannot exist; href={href}"
    );

    // Same page, same form, the other gesture: this one moves.
    let submitted = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"submit","target":"#form","timeout_ms":8000}"##.to_string(),
    ])
    .expect("submit envelope");
    let after = step_data(&submitted, "submit")
        .and_then(|d| {
            d.get("url_after")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    assert!(
        after.contains("landed.html") && after.contains("token=EVENT_RAN"),
        "the dedicated command must do what the click did not; url_after={after}"
    );
}

/// NEGATIVE: a submission the page cancels is REPORTED, not passed off as
/// success and not turned into a timeout.
///
/// Both failure modes are worse than the truth: a bare `ok` hides that nothing
/// reached the server, and a timeout blames the budget for a decision the page
/// made deliberately.
#[test]
fn a_cancelled_submission_is_reported_rather_than_passed_or_timed_out() {
    if cannot_run() {
        return;
    }
    let url = fixture_url("login.html", "prevent=1").expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"submit","target":"#user","timeout_ms":4000}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "a cancelled submission is a known outcome, not a transport failure: {env}"
    );
    let data = step_data(&env, "submit").expect("submit step data");

    assert_eq!(
        data.get("outcome").and_then(|v| v.as_str()),
        Some("prevented"),
        "the page called preventDefault and issued nothing: {data}"
    );
    assert_eq!(
        data.get("default_prevented").and_then(|v| v.as_bool()),
        Some(true),
        "the cancellation must be visible in the envelope, not only in the outcome tag"
    );
    assert_eq!(
        data.get("requests_started").and_then(|v| v.as_u64()),
        Some(0),
        "nothing was sent, and the count is what says so: {data}"
    );
    let warning = data.get("warning").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        !warning.is_empty(),
        "a submission that reached no server must carry an explicit warning; \
         silence here is the same false success this gate exists to catch"
    );
}

/// DECLARED EXCLUSION: an invalid form is REFUSED, and that refusal is correct.
///
/// `requestSubmit()` runs constraint validation, unlike `submit()`. From the
/// outside the refusal reads like the command failing to do its job, so it is
/// worth pinning: the alternative — bypassing validation — would send data the
/// page had already rejected.
#[test]
fn constraint_validation_refuses_instead_of_silently_doing_nothing() {
    if cannot_run() {
        return;
    }
    let url = fixture_url("login.html", "require=1").expect("fixture url");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"submit","target":"#user","timeout_ms":4000}"##.to_string(),
    ])
    .expect("run envelope");

    assert_eq!(
        env.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "an empty required field must not submit: {env}"
    );
    assert_eq!(
        env.pointer("/error/kind").and_then(|v| v.as_str()),
        Some("data"),
        "a form the page itself rejects is a data problem, not a usage or timeout one: {env}"
    );
    let message = env
        .pointer("/error/message")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        message.contains("validation"),
        "the refusal must name constraint validation so the caller stops looking \
         for a credential problem; got {message}"
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
