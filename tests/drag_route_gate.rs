//! Permanent gate for the drag ROUTE, not for the drag helpers (GAP-030).
//!
//! # Why this file exists
//!
//! `src/native/interaction/tests.rs` already covers payload normalization,
//! drop anchors and route tags. Those are pure functions, and they stay GREEN
//! if the page-level `Input.dragIntercepted` forwarder in
//! `src/native/cdp/client/page_attach.rs` is removed — while the drag silently
//! degrades to a synthetic mouse gesture that never touches the page handler.
//!
//! Function coverage is not path coverage. The question a gate must answer is
//! not "what does this assert" but "what has to BREAK for this to fail". For
//! this gap the answer is: the interception path. So that is what is asserted
//! here, end to end, against real Chrome.
//!
//! # The three committed cases
//!
//! | case | fixture | expected |
//! |---|---|---|
//! | positive control | `positive_control.html` | `route == "intercepted"` and list reorders |
//! | negative | `negative.html` | `route == "synthetic_mouse"` plus a warning |
//! | declared exclusion | positive fixture + `synthetic_payload` | `route == "synthetic_payload"` |
//!
//! The negative case is what proves the gate discriminates: a gate that only
//! ever saw the positive page would also pass while asserting nothing.
//!
//! # Skip policy
//!
//! No Chrome means SKIP LOUDLY. A silent green here would rebuild exactly the
//! blind spot this gate removes.

mod common;
use common::{binary, chrome_not_ready, missing_binary, root};

const GATE: &str = "drag_route_gate";

fn fixture_url(name: &str) -> Option<String> {
    let p = root().join("scripts/fixtures/drag_route").join(name);
    p.exists().then(|| format!("file://{}", p.display()))
}

/// Run a script through `run` and return the parsed envelope.
fn run_script(lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // The three cases run as threads of ONE test binary, so a directory keyed
    // only by pid is SHARED: they overwrite each other's script and the routes
    // get crossed. Each invocation needs its own directory.
    // A `TempDir` and not a pid+counter path: the counter only ever resolved
    // COLLISION between the threads of this one binary, never cleanup, so an
    // assertion that panicked left the directory behind for good. The guard is
    // bound to a NAMED variable on purpose — `let _ = ...` drops it on the spot
    // and deletes the script before the child process can read it.
    let scratch = tempfile::Builder::new()
        .prefix("bac-drag-route-gate-")
        .tempdir()
        .ok()?;
    let dir = scratch.path();
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = common::isolated_cmd(&bin)
        .args(["-q", "--timeout", "180", "--json", "run", "--script"])
        .arg(&script)
        .output()
        .ok()?;

    serde_json::from_slice(&out.stdout).ok()
}

fn drag_step_data(env: &serde_json::Value) -> Option<serde_json::Value> {
    env.pointer("/data/steps")?
        .as_array()?
        .iter()
        .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("drag"))
        .and_then(|s| s.get("data").cloned())
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run(fixture: &str) -> bool {
    if missing_binary(GATE) {
        return true;
    }
    if fixture_url(fixture).is_none() {
        common::skip_with_reason(
            "drag_route_gate",
            &format!("fixture scripts/fixtures/drag_route/{fixture} absent."),
        );
        return true;
    }
    if chrome_not_ready(GATE, &binary().expect("binary")) {
        return true;
    }
    false
}

/// POSITIVE CONTROL: the page owns `dragstart`, so the drop must carry the
/// page's own DataTransfer and the list must actually reorder.
///
/// Removing the page-level `Input.dragIntercepted` forwarder makes this fail.
#[test]
fn drag_uses_the_page_dragstart_and_reports_intercepted() {
    if cannot_run("positive_control.html") {
        return;
    }
    let url = fixture_url("positive_control.html").expect("fixture");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#list li","min_count":3,"wait_timeout_ms":8000}"##.into(),
        r##"{"cmd":"drag","from":"#a","to":"#c"}"##.into(),
        r##"{"cmd":"eval","expression":"Array.from(document.querySelectorAll('#list li')).map(x=>x.id).join(',')"}"##.into(),
    ])
    .expect("run envelope");

    let data = drag_step_data(&env).expect("drag step data");
    let route = data
        .get("route")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    assert_eq!(
        route, "intercepted",
        "drag must exercise the page's own dragstart. Got route={route}. \
         The usual cause is a missing page-level Input.dragIntercepted forwarder \
         in src/native/cdp/client/page_attach.rs; browser-level listeners never \
         receive target-session events."
    );
    assert_eq!(
        data.get("exercised_page_dragstart")
            .and_then(|v| v.as_bool()),
        Some(true),
        "envelope must state that the page dragstart ran"
    );

    // The token proves the payload came from the page, not from the CLI.
    let payload = serde_json::to_string(&data.get("data_transfer").cloned().unwrap_or_default())
        .unwrap_or_default();
    assert!(
        payload.contains("PAGE_OWNED:a"),
        "dropped DataTransfer must be the one the page built; got {payload}"
    );

    // Effect, not just protocol: the page's drop handler rejects foreign
    // payloads, so a reorder is only possible with the real one.
    let order = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .and_then(|s| s.last())
        .and_then(|s| s.pointer("/data/result"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(
        order, "b,a,c",
        "the page's drop handler must have reordered the list"
    );
}

/// NEGATIVE: no `draggable`, no `dragstart`. Interception cannot produce a
/// payload, so the CLI must report the degraded route WITH a warning.
///
/// This is the case that proves the gate discriminates between routes rather
/// than merely asserting that some string is present.
#[test]
fn drag_without_page_dragstart_reports_the_degraded_route_and_warns() {
    if cannot_run("negative.html") {
        return;
    }
    let url = fixture_url("negative.html").expect("fixture");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#list li","min_count":3,"wait_timeout_ms":8000}"##.into(),
        r##"{"cmd":"drag","from":"#a","to":"#c"}"##.into(),
    ])
    .expect("run envelope");

    let data = drag_step_data(&env).expect("drag step data");
    let route = data
        .get("route")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    assert_eq!(
        route, "synthetic_mouse",
        "a page with no dragstart cannot yield an intercepted drag. Got route={route}. \
         If this says `intercepted`, the route is being reported without evidence."
    );
    assert_eq!(
        data.get("exercised_page_dragstart")
            .and_then(|v| v.as_bool()),
        Some(false),
        "the degraded route must NOT claim the page dragstart ran"
    );
    let warning = data.get("warning").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        !warning.is_empty(),
        "the degraded route must carry an explicit warning; silence here is the \
         false positive GAP-030 was opened for"
    );
}

/// DECLARED EXCLUSION: `synthetic_payload` is an opt-in bypass of the page's
/// `dragstart`. It must be reported as its own route, never as `intercepted`.
#[test]
fn declared_synthetic_payload_is_reported_as_its_own_route() {
    if cannot_run("positive_control.html") {
        return;
    }
    let url = fixture_url("positive_control.html").expect("fixture");
    let env = run_script(&[
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#list li","min_count":3,"wait_timeout_ms":8000}"##.into(),
        r##"{"cmd":"drag","from":"#a","to":"#c","synthetic_payload":{"items":[{"mimeType":"text/plain","data":"injected"}]}}"##.into(),
    ])
    .expect("run envelope");

    let data = drag_step_data(&env).expect("drag step data");
    let route = data
        .get("route")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    assert_eq!(
        route, "synthetic_payload",
        "an explicitly injected payload must report its own route, got {route}"
    );
    assert_eq!(
        data.get("exercised_page_dragstart")
            .and_then(|v| v.as_bool()),
        Some(false),
        "an injected payload bypasses the page dragstart and must say so"
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
///
/// Every fixture this file uses is checked, so a single missing HTML file is
/// reported by name instead of silently disabling the cases that need it.
#[test]
fn the_host_can_actually_run_this_gate() {
    for fixture in ["positive_control.html", "negative.html"] {
        assert!(
            !cannot_run(fixture),
            "host cannot run this gate with fixture {fixture}: the other cases in              this file skipped, and a skip is NOT a pass. The SKIP line on stderr              names the missing precondition (binary, fixture, or Chrome)."
        );
    }
}
