//! Permanent gate for the input event TRACE (gaps.md G11, step 7 / "B7").
//!
//! # Why this file exists
//!
//! Every other input test in this repository asserts on an EFFECT: the document
//! scrolled, the field holds the text, the click landed. Those assertions have
//! a blind spot that is not theoretical -- it shipped.
//!
//! `scroll` used to move `scrollTop` from JavaScript and `type` used to fill a
//! field through `Input.insertText` alone. Both produced the correct effect and
//! neither delivered a single `wheel` or `keydown` to the page, so lazy-load,
//! infinite scroll, autocomplete and incremental validation stayed dead while
//! the suite stayed green. The gap was found by reading source, not by a test:
//! clippy, the full suite and ten gates all passed over it.
//!
//! gaps.md names the missing instrument in as many words -- "enquanto B7 não
//! existir, qualquer alegação sobre cinemática de input é não verificada por
//! construção". This file is B7. It asserts on WHAT THE PAGE RECEIVED.
//!
//! # Why the `direct` profile is asserted too
//!
//! A gate that only ever ran the `human` profile would also pass while
//! measuring nothing in particular. `--input-profile direct` is a real negative
//! control here, and the discrimination comes from the product's own contract:
//! `direct` keeps the scripted `scrollBy` path, which emits no `wheel` at all,
//! and the bare `insertText`, which emits no key events. So the same page and
//! the same steps must produce a trace under `human` and no trace under
//! `direct`. If a future change makes `human` stop synthesizing events, the
//! first two tests fail; if it makes `direct` start, the third fails.
//!
//! # Skip policy
//!
//! No Chrome, no binary or no fixture means SKIP LOUDLY. A silent green here
//! would rebuild exactly the blind spot this gate removes.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn binary() -> Option<PathBuf> {
    let p = root().join("target/debug/browser-automation-cli");
    p.exists().then_some(p)
}

fn fixture_url() -> Option<String> {
    let p = root().join("scripts/fixtures/input_trace/instrumented.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP input_trace_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    if fixture_url().is_none() {
        eprintln!(
            "SKIP input_trace_gate: fixture scripts/fixtures/input_trace/instrumented.html \
             absent. This is NOT a pass."
        );
        return true;
    }
    false
}

/// Run one NDJSON script and return the parsed envelope.
///
/// `profile` is passed as a global flag so the whole script runs under it.
fn run_script(profile: &str, lines: &[String]) -> Option<serde_json::Value> {
    let bin = binary()?;
    // Each invocation needs its own directory: these tests are threads of ONE
    // binary and a pid-keyed path is shared, so they would overwrite each
    // other's script.
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("input-trace-gate-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let script = dir.join("steps.jsonl");
    std::fs::write(&script, lines.join("\n")).ok()?;

    let out = Command::new(&bin)
        .args([
            "-q",
            "--timeout",
            "180",
            "--json",
            "--input-profile",
            profile,
            // Seeded so jitter is reproducible: the assertions below are about
            // the SHAPE of the trace, and a gate that flakes on randomness
            // teaches people to rerun it until it is green.
            "--input-seed",
            "424242",
            "run",
            "--script",
        ])
        .arg(&script)
        .output()
        .ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    serde_json::from_slice(&out.stdout).ok()
}

/// The steps every case shares: load the page, act, then read the trace.
///
/// `eval` comes last on purpose. It invalidates `@eN` refs, and reading the
/// trace before the gestures would measure an empty array.
fn trace_script(url: &str) -> Vec<String> {
    vec![
        format!(r#"{{"cmd":"goto","url":"{url}"}}"#),
        r##"{"cmd":"wait","selector":"#btn","wait_timeout_ms":8000}"##.into(),
        r#"{"cmd":"scroll","delta_y":400}"#.into(),
        r##"{"cmd":"type","target":"#field","text":"abcde"}"##.into(),
        r##"{"cmd":"press","target":"#btn"}"##.into(),
        r#"{"cmd":"eval","expression":"JSON.stringify(window.__trace)"}"#.into(),
    ]
}

/// Pull the recorded events out of the last `eval` step.
fn trace_of(env: &serde_json::Value) -> Vec<serde_json::Value> {
    let steps = env
        .pointer("/data/steps")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let raw = steps
        .iter()
        .rev()
        .find(|s| s.get("cmd").and_then(|c| c.as_str()) == Some("eval"))
        .and_then(|s| s.get("data"))
        .and_then(|d| d.get("result").or_else(|| d.get("value")))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let text = match raw {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };
    serde_json::from_str::<Vec<serde_json::Value>>(&text).unwrap_or_default()
}

fn kinds<'a>(trace: &'a [serde_json::Value], kind: &str) -> Vec<&'a serde_json::Value> {
    trace
        .iter()
        .filter(|e| e.get("k").and_then(|k| k.as_str()) == Some(kind))
        .collect()
}

/// Under `human`, the page must actually RECEIVE the gestures.
///
/// This is the assertion whose absence let `scroll` and `type` ship without
/// emitting anything. Each count is the product's own documented contract, not
/// a guess: one wheel tick per `input_scroll_tick_px`, one keydown/keyup pair
/// per character.
#[test]
fn human_profile_delivers_wheel_and_key_events_to_the_page() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture");
    let env = run_script("human", &trace_script(&url)).expect("run envelope");
    let trace = trace_of(&env);
    assert!(
        !trace.is_empty(),
        "the instrumented page recorded nothing at all -- either no gesture was \
         dispatched or the fixture stopped listening. envelope={env}"
    );

    let wheel = kinds(&trace, "wheel");
    assert!(
        wheel.len() >= 3,
        "a 400px scroll must reach the page as real wheel ticks; got {} \
         (this is the exact regression B7 exists to catch: the document can \
         scroll from JavaScript while the page receives zero wheel events, so \
         lazy-load and infinite-scroll stay dead). trace={trace:?}",
        wheel.len()
    );

    let down = kinds(&trace, "keydown");
    let up = kinds(&trace, "keyup");
    assert_eq!(
        (down.len(), up.len()),
        (5, 5),
        "typing 5 characters must produce 5 keydown and 5 keyup; got {} and {} \
         (bare Input.insertText fills the field and fires neither, which leaves \
         autocomplete, input masks and incremental validation dead). \
         trace={trace:?}",
        down.len(),
        up.len()
    );

    let moves = kinds(&trace, "mousemove");
    let downs = kinds(&trace, "mousedown");
    assert!(
        !downs.is_empty(),
        "press must reach the page as a mousedown. trace={trace:?}"
    );
    let first_down = trace
        .iter()
        .position(|e| e.get("k").and_then(|k| k.as_str()) == Some("mousedown"))
        .unwrap_or(trace.len());
    let approach: Vec<_> = trace[..first_down]
        .iter()
        .filter(|e| e.get("k").and_then(|k| k.as_str()) == Some("mousemove"))
        .filter_map(|e| Some((e.get("x")?.as_i64()?, e.get("y")?.as_i64()?)))
        .collect();
    let distinct: std::collections::HashSet<_> = approach.iter().collect();
    assert!(
        distinct.len() >= 2,
        "press must approach the target along a trajectory: at least 2 distinct \
         mousemove coordinates before mousedown, got {} distinct out of {} moves \
         ({} total mousemove in the trace). A single jump to the target is the \
         signature CDP-driven input leaves behind. trace={trace:?}",
        distinct.len(),
        approach.len(),
        moves.len()
    );
}

/// Consecutive intervals must not all be identical.
///
/// Perfectly uniform spacing is a signature on its own, and it is what a fixed
/// `sleep` between events produces. Asserting it here is what keeps a future
/// "simplification" from replacing the jitter with a constant.
#[test]
fn human_profile_spacing_is_not_perfectly_uniform() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture");
    let env = run_script("human", &trace_script(&url)).expect("run envelope");
    let trace = trace_of(&env);
    let times: Vec<f64> = trace
        .iter()
        .filter_map(|e| e.get("t").and_then(serde_json::Value::as_f64))
        .collect();
    assert!(
        times.len() >= 4,
        "not enough events to judge spacing: {} (envelope={env})",
        times.len()
    );
    let gaps: Vec<f64> = times.windows(2).map(|w| w[1] - w[0]).collect();
    // Sub-millisecond resolution from performance.now(), so "identical" means
    // identical, not "close". Two distinct gaps is the whole claim.
    let distinct = gaps
        .iter()
        .map(|g| format!("{g:.3}"))
        .collect::<std::collections::HashSet<_>>();
    assert!(
        distinct.len() >= 2,
        "every inter-event gap was identical ({distinct:?}), which is a fixed \
         cadence and a fingerprint by itself. gaps={gaps:?}"
    );
}

/// `direct` is the declared escape hatch, and it must stay bit-for-bit cheap.
///
/// The product documents `direct` as keeping the scripted `scrollBy` and the
/// bare `insertText`. That makes this the negative control: the same page and
/// the same steps must reach the page WITHOUT wheel or key events. Without this
/// case the two tests above could pass on a build that synthesizes events
/// unconditionally, and `direct` would have silently stopped being an opt-out.
#[test]
fn direct_profile_emits_no_synthetic_wheel_or_key_events() {
    if cannot_run() {
        return;
    }
    let url = fixture_url().expect("fixture");
    let env = run_script("direct", &trace_script(&url)).expect("run envelope");
    let trace = trace_of(&env);

    let wheel = kinds(&trace, "wheel");
    assert!(
        wheel.is_empty(),
        "--input-profile direct must keep the scripted scrollBy path and emit no \
         wheel; got {}. trace={trace:?}",
        wheel.len()
    );
    let down = kinds(&trace, "keydown");
    assert!(
        down.is_empty(),
        "--input-profile direct must keep bare Input.insertText and emit no \
         keydown; got {}. trace={trace:?}",
        down.len()
    );
}
