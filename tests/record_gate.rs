//! Permanent gate for `record`: the NDJSON it writes must be a script `run` runs.
//!
//! # Why this file exists
//!
//! A recorder is only worth having if its output needs no translation. A gate
//! that asserted "the command exits 0" or "the file is non-empty" would stay
//! green the day the step vocabulary drifts away from what `run --script`
//! dispatches — and the drift would surface as a replay failure blamed on `run`.
//!
//! So the assertions here are about the FILE being executable: every line is fed
//! back through `run --script` and the replay has to succeed.
//!
//! # How the fixture removes the human
//!
//! `scripts/fixtures/record/interactions.html` drives itself after load: it
//! fills a field, dispatches `change`, then clicks a button. Those are real DOM
//! events reaching the capturing listeners, so the gate runs offline with nobody
//! at the keyboard.
//!
//! # What this file does NOT cover
//!
//! - It does not cover gestures inside an iframe; the recorder only captures the
//!   top frame, which is a declared limitation of the capture script.
//! - It does not cover the `seconds` ceiling firing before the events do — that
//!   would need a page that fires nothing, and would pay its whole budget in
//!   wall-clock on every run.
//! - It says nothing about selector quality on a page with no ids; the fallback
//!   `nth-of-type` path is covered by the unit tests next to the script.
//!
//! # Skip policy
//!
//! No binary, no fixture or no Chrome means SKIP LOUDLY, and one case in this
//! file fails the ENVIRONMENT so a host that can never run the gate is visible.

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
    let p = root().join("scripts/fixtures/record/interactions.html");
    p.exists().then(|| format!("file://{}", p.display()))
}

/// A per-case scratch directory: the cases run as threads of ONE test binary,
/// so a path keyed only by pid would be shared and the files would collide.
fn scratch(tag: &str) -> Option<PathBuf> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("record-gate-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Run `record` against the fixture and return `(envelope, recorded lines)`.
fn record(out: &PathBuf, max_events: u32) -> Option<(serde_json::Value, Vec<String>)> {
    let bin = binary()?;
    let url = fixture_url()?;
    let output = Command::new(&bin)
        .args(["-q", "--timeout", "120", "--json", "record", "--url"])
        .arg(&url)
        .arg("--path")
        .arg(out)
        .args(["--seconds", "6", "--max-events", &max_events.to_string()])
        .output()
        .ok()?;
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let body = std::fs::read_to_string(out).unwrap_or_default();
    let lines = body.lines().map(str::to_string).collect();
    Some((envelope, lines))
}

/// Replay a recorded NDJSON file through `run --script` and return the envelope.
fn replay(script: &PathBuf) -> Option<serde_json::Value> {
    let bin = binary()?;
    let output = Command::new(&bin)
        .args(["-q", "--timeout", "120", "--json", "run", "--script"])
        .arg(script)
        .output()
        .ok()?;
    serde_json::from_slice(&output.stdout).ok()
}

/// True when the host cannot run the gate. Prints why; never silently passes.
fn cannot_run() -> bool {
    if binary().is_none() {
        eprintln!(
            "SKIP record_gate: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return true;
    }
    if fixture_url().is_none() {
        eprintln!(
            "SKIP record_gate: fixture scripts/fixtures/record/interactions.html absent. \
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
            "SKIP record_gate: doctor reports the host is not ready for Chrome. \
             This is NOT a pass."
        );
        return true;
    }
    false
}

/// POSITIVE CONTROL: one line per captured gesture, and the file replays.
///
/// The replay is the load-bearing assertion. Counting lines proves the recorder
/// wrote something; running them proves it wrote the vocabulary `run` speaks.
#[test]
fn recorded_ndjson_is_one_line_per_event_and_replays_through_run() {
    if cannot_run() {
        return;
    }
    let dir = scratch("replay").expect("scratch dir");
    let out = dir.join("steps.jsonl");
    let (envelope, lines) = record(&out, 50).expect("record envelope");

    assert_eq!(
        envelope.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "recording the fixture must succeed: {envelope}"
    );
    let data = envelope.get("data").expect("data");
    assert_eq!(data.get("action").and_then(|v| v.as_str()), Some("record"));
    assert_eq!(
        data.get("truncated").and_then(|v| v.as_bool()),
        Some(false),
        "fifty is well above what the fixture fires; nothing should truncate: {data}"
    );

    let events = data.get("events").and_then(|v| v.as_u64()).unwrap_or(0);
    assert_eq!(
        lines.len() as u64,
        events,
        "the envelope count and the file must agree: events={events} lines={}",
        lines.len()
    );
    assert!(
        events >= 3,
        "the fixture fires a navigation, a change and a click; got {events} \
         line(s): {lines:?}. Fewer means the capture script did not see the page's \
         own events, which is the whole mechanism this gate covers."
    );

    let mut kinds = Vec::new();
    for line in &lines {
        let step: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line is not JSON: {line} ({e})"));
        let cmd = step
            .get("cmd")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        assert!(
            matches!(cmd.as_str(), "goto" | "press" | "write" | "submit"),
            "recorded an unreplayable step `{cmd}`: {line}"
        );
        kinds.push(cmd);
    }
    assert_eq!(
        kinds.first().map(String::as_str),
        Some("goto"),
        "the recording opens with the navigation it caused: {kinds:?}"
    );
    assert!(
        kinds.iter().any(|k| k == "write"),
        "the `change` on the field must land as a write step: {kinds:?}"
    );
    assert!(
        kinds.iter().any(|k| k == "press"),
        "the click must land as a press step: {kinds:?}"
    );

    let replayed = replay(&out).expect("replay envelope");
    assert_eq!(
        replayed.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "the recorded file must run unmodified through `run --script`: {replayed}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// CEILING: `--max-events 1` stops after one step and SAYS it stopped early.
///
/// Reporting `truncated` is what keeps a partial recording from reading as a
/// complete one — the exact silent-success shape the rest of this file rejects.
#[test]
fn the_event_ceiling_truncates_and_is_reported() {
    if cannot_run() {
        return;
    }
    let dir = scratch("truncate").expect("scratch dir");
    let out = dir.join("steps.jsonl");
    let (envelope, lines) = record(&out, 1).expect("record envelope");

    assert_eq!(
        envelope.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "a truncated recording is a known outcome, not a failure: {envelope}"
    );
    let data = envelope.get("data").expect("data");
    assert_eq!(
        data.get("truncated").and_then(|v| v.as_bool()),
        Some(true),
        "one event with a ceiling of one must report truncation: {data}"
    );
    assert_eq!(data.get("events").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        lines.len(),
        1,
        "the file must honour the ceiling: {lines:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// ENVIRONMENT GUARD: this one never skips.
///
/// The behavioural cases return early on a host without Chrome, and a test that
/// returns counts as a PASS. Keeping the environment check in its own case lets
/// someone develop without a browser while an unusable host still turns exactly
/// one test RED, in one place, instead of turning the file green in silence.
#[test]
fn the_host_can_actually_run_this_gate() {
    assert!(
        !cannot_run(),
        "host cannot run this gate: every other case in this file skipped, and a \
         skip is NOT a pass. The SKIP line on stderr names the missing \
         precondition (binary, fixture, or Chrome)."
    );
}
