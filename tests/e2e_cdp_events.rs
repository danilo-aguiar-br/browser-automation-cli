//! E2E: CDP event pipeline must deliver heap/trace/screencast bytes (not empty stubs).
//!
//! Skips when Chrome is not discoverable.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn chrome_discoverable() -> bool {
    for name in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ] {
        if Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn tmp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("ba-e2e-{prefix}-{stamp}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[test]
fn eval_accepts_already_invoked_iife() {
    if !chrome_discoverable() {
        eprintln!("skip: no Chrome for eval iife e2e");
        return;
    }
    let dir = tmp_dir("eval-iife");
    let script = dir.join("script.ndjson");
    // Already-invoked IIFE must not be re-wrapped as ((...)())().
    let body = r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"eval","expression":"(() => { const a=[]; for(let i=0;i<3;i++) a.push(i); return a.length; })()"}
"#;
    std::fs::write(&script, body).expect("write script");
    let output = Command::new(BIN)
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
        eprintln!("skip: no Chrome for heap e2e");
        return;
    }
    let dir = tmp_dir("heap");
    let script = dir.join("script.ndjson");
    let snap = dir.join("a.heapsnapshot");
    let body = format!(
        r#"{{"cmd":"goto","url":"https://example.com"}}
{{"cmd":"heap","action":"take","path":"{}"}}
{{"cmd":"heap","action":"summary","path":"{}"}}
"#,
        snap.display(),
        snap.display()
    );
    std::fs::write(&script, body).expect("write script");

    let output = Command::new(BIN)
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
        eprintln!("skip: no Chrome for perf e2e");
        return;
    }
    let dir = tmp_dir("perf");
    let script = dir.join("script.ndjson");
    let trace = dir.join("trace.ndjson");
    let body = format!(
        r#"{{"cmd":"goto","url":"https://example.com"}}
{{"cmd":"perf","action":"start"}}
{{"cmd":"wait","ms":600}}
{{"cmd":"perf","action":"stop","path":"{}"}}
"#,
        trace.display()
    );
    std::fs::write(&script, body).expect("write script");

    let output = Command::new(BIN)
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
        eprintln!("skip: no Chrome for screencast e2e");
        return;
    }
    let dir = tmp_dir("sc");
    let script = dir.join("script.ndjson");
    let sc_dir = dir.join("frames");
    let body = format!(
        r#"{{"cmd":"goto","url":"https://example.com"}}
{{"cmd":"screencast","action":"start","dir":"{}"}}
{{"cmd":"wait","ms":800}}
{{"cmd":"screencast","action":"stop"}}
"#,
        sc_dir.display()
    );
    std::fs::write(&script, body).expect("write script");

    let output = Command::new(BIN)
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
        eprintln!("skip: no Chrome for dialog e2e");
        return;
    }
    let dir = tmp_dir("dlg");
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

    let output = Command::new(BIN)
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
    let dir = tmp_dir("vision");
    let script = dir.join("script.ndjson");
    std::fs::write(
        &script,
        r#"{"cmd":"goto","url":"about:blank"}
{"cmd":"click-at","x":1,"y":1}
"#,
    )
    .unwrap();
    let output = Command::new(BIN)
        .args(["run", "--script", script.to_str().unwrap(), "--json"])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Fail-fast usage error expected
    assert!(
        !output.status.success() || stdout.contains("\"ok\":false"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("experimental-vision") || stdout.contains("click-at"),
        "stdout={stdout}"
    );
}
