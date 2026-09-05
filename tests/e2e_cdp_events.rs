//! E2E: CDP event pipeline must deliver heap/trace/screencast bytes (not empty stubs).
//!
//! Skips when Chrome is not discoverable.

mod common;
use common::chrome_discoverable;

/// A scratch directory that is removed when the test that owns it ends.
///
/// # Why the millisecond stamp had to go
///
/// This used to build `ba-e2e-{prefix}-{millis}` under `temp_dir()` and never
/// remove it. The stamp made every run pick a fresh name, so the directories
/// accumulated one set per invocation, forever — and because a screencast test
/// writes frames and a heap test writes a snapshot, they were not empty.
///
/// The leak had a consumer, which is how it stayed invisible: the offline
/// `heap` gate in `devtools_envelope_behavior.rs` used to SCAN `/tmp` for
/// `ba-e2e-52-*/a.heapsnapshot` and feed the newest one to the CLI. Producer
/// and consumer masked each other — the litter looked like a fixture, and the
/// fixture looked like it had a source. Both sides are now hermetic.
///
/// The caller binds the returned handle and reads `.path()`, because dropping
/// it deletes the directory; a bare `tmp_dir(..).path()` would delete the
/// directory before the CLI ever saw it.
fn tmp_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("ba-e2e-{prefix}-"))
        .tempdir()
        .expect("create e2e scratch dir")
}

#[test]
fn eval_accepts_already_invoked_iife() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "e2e_cdp_events::eval_iife",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let tmp = tmp_dir("eval-iife");
    let dir = tmp.path();
    let script = dir.join("script.ndjson");
    // Already-invoked IIFE must not be re-wrapped as ((...)())().
    let body = r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"eval","expression":"(() => { const a=[]; for(let i=0;i<3;i++) a.push(i); return a.length; })()"}
"#;
    std::fs::write(&script, body).expect("write script");
    let output = common::cmd()
        .args([
            "run",
            "--script",
            script.to_str().unwrap(),
            "--json",
            "--ignore-robots",
            "--i-accept-robots-risk",
        ])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("\"result\":3") || stdout.contains("\"result\": 3"),
        "expected IIFE result 3; stdout={stdout}"
    );
}

#[test]
fn heap_take_writes_nonzero_snapshot() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "e2e_cdp_events::heap",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let tmp = tmp_dir("heap");
    let dir = tmp.path();
    let script = dir.join("script.ndjson");
    let snap = dir.join("a.heapsnapshot");
    let body = format!(
        r#"{{"cmd":"goto","url":"data:text/html,<title>fx</title><h1>e2e</h1><script>window.__f=new Array(20000).fill(7)</script>"}}
{{"cmd":"heap","action":"take","path":"{}"}}
{{"cmd":"heap","action":"summary","path":"{}"}}
"#,
        snap.display(),
        snap.display()
    );
    std::fs::write(&script, body).expect("write script");

    let output = common::cmd()
        .args([
            "run",
            "--script",
            script.to_str().unwrap(),
            "--json",
            "--category-memory",
        ])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    );
    let meta = std::fs::metadata(&snap).expect("snapshot exists");
    assert!(
        meta.len() > 1000,
        "heap snapshot too small: {} bytes; stdout={stdout}",
        meta.len()
    );
    assert!(
        stdout.contains("\"bytes\"") || stdout.contains("heap"),
        "stdout={stdout}"
    );
}

#[test]
fn perf_stop_records_events() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "e2e_cdp_events::perf",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let tmp = tmp_dir("perf");
    let dir = tmp.path();
    let script = dir.join("script.ndjson");
    let trace = dir.join("trace.ndjson");
    let body = format!(
        r#"{{"cmd":"goto","url":"data:text/html,<title>fx</title><h1>e2e</h1><script>window.__f=new Array(20000).fill(7)</script>"}}
{{"cmd":"perf","action":"start"}}
{{"cmd":"wait","ms":600}}
{{"cmd":"perf","action":"stop","path":"{}"}}
"#,
        trace.display()
    );
    std::fs::write(&script, body).expect("write script");

    let output = common::cmd()
        .args(["run", "--script", script.to_str().unwrap(), "--json"])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    );
    // Prefer non-zero events; allow soft fail only if Chrome tracing is unavailable
    // but still surface empty as test failure — this is the regression gate.
    assert!(
        stdout.contains("\"events\":") && !stdout.contains("\"events\":0"),
        "expected non-zero perf events; stdout={stdout}"
    );
}

#[test]
fn screencast_writes_frames_with_experimental_flag() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "e2e_cdp_events::screencast",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let tmp = tmp_dir("sc");
    let dir = tmp.path();
    let script = dir.join("script.ndjson");
    let sc_dir = dir.join("frames");
    // The page must keep PAINTING for the whole window.
    //
    // Chrome emits a screencast frame per composited frame, so a static page
    // hands over nothing: it painted once, during `goto`, before the screencast
    // was even started. Measured 2026-09-04, the previous static fixture
    // returned `"frame_count":0` with `ok:true` and exit 0, and this gate only
    // ever passed when something incidental forced a repaint. The
    // `requestAnimationFrame` loop below moves a node every frame, so a
    // non-zero count is a property of the fixture rather than of the host's
    // luck.
    let body = format!(
        r#"{{"cmd":"goto","url":"data:text/html,<title>fx</title><h1 id=h>e2e</h1><script>window.__f=new Array(20000).fill(7);var n=0;(function tick(){{n++;document.getElementById('h').style.transform='translateX('+(n%%20)+'px)';requestAnimationFrame(tick);}})();</script>"}}
{{"cmd":"screencast","action":"start","dir":"{}"}}
{{"cmd":"wait","ms":800}}
{{"cmd":"screencast","action":"stop"}}
"#,
        sc_dir.display()
    );
    std::fs::write(&script, body).expect("write script");

    let output = common::cmd()
        .args([
            "run",
            "--script",
            script.to_str().unwrap(),
            "--json",
            "--experimental-screencast",
        ])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("\"frame_count\":") && !stdout.contains("\"frame_count\":0"),
        "expected non-zero screencast frames; stdout={stdout}"
    );
}

#[test]
fn eval_auto_accepts_alert_dialog() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "e2e_cdp_events::dialog",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    let tmp = tmp_dir("dlg");
    let dir = tmp.path();
    let html = dir.join("d.html");
    std::fs::write(
        &html,
        r#"<!doctype html><html><body><script>/* empty */</script></body></html>"#,
    )
    .unwrap();
    let script = dir.join("script.ndjson");
    let url = format!("file://{}", html.display());
    let body = format!(
        r#"{{"cmd":"goto","url":"{url}"}}
{{"cmd":"eval","expression":"window.alert('hi'); 'ok'"}}
"#
    );
    std::fs::write(&script, body).unwrap();

    let output = common::cmd()
        .args(["run", "--script", script.to_str().unwrap(), "--json"])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("\"result\":\"ok\"") || stdout.contains("\"result\": \"ok\""),
        "stdout={stdout}"
    );
}

#[test]
fn click_at_requires_experimental_vision_in_run() {
    let tmp = tmp_dir("vision");
    let dir = tmp.path();
    let script = dir.join("script.ndjson");
    std::fs::write(
        &script,
        r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"click-at","x":1,"y":1}
"#,
    )
    .unwrap();
    let output = common::cmd()
        .args(["run", "--script", script.to_str().unwrap(), "--json"])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // MEASURED 2026-08-24: exit 64, `ok:false`, kind `capability-disabled`,
    // and a message naming `--experimental-vision`. Asserted exactly.
    //
    // What this replaces was `!status.success() || stdout.contains("ok\":false")`
    // followed by `contains("experimental-vision") || contains("click-at")`.
    // The first was a disjunction wide enough never to fail: it passed on ANY
    // non-zero exit — a missing binary, an unreadable config, a panic — none of
    // which says anything about the vision gate.
    //
    // MEASURED: the envelope carries exactly `error`, `ok` and `schema_version`,
    // and the word "click-at" appears in NEITHER. So the second assertion was
    // load-bearing after all, resting entirely on its `experimental-vision`
    // branch — but a reader could not know that, and the `|| contains("click-at")`
    // half was one envelope change away from silently absorbing the failure.
    // Stating the flag exactly removes that trapdoor.
    assert_eq!(
        output.status.code(),
        Some(64),
        "capability-disabled is exit 64, not a generic failure: {stdout}"
    );
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must be a JSON envelope: {e}; raw={stdout}"));
    assert_eq!(v["ok"], false, "{stdout}");
    assert_eq!(
        v["error"]["kind"], "capability-disabled",
        "the gate must report capability-disabled, never a usage error: {stdout}"
    );
    assert!(
        stdout.contains("--experimental-vision"),
        "the envelope must name the flag that would enable the step: {stdout}"
    );
}
