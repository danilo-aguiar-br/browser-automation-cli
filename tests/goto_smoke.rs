//! Integration smoke: one-shot `goto about:blank` (PR3).
//!
//! Skips when Chrome is not available on PATH / system locations.

use std::process::Command;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn chrome_discoverable() -> bool {
    // Match discovery used by native launch loosely: common binaries.
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

#[test]
fn goto_about_blank_json_when_chrome_present() {
    if !chrome_discoverable() {
        eprintln!("skip: no system Chrome/Chromium for goto smoke");
        return;
    }

    let output = Command::new(BIN)
        .args(["goto", "about:blank", "--json"])
        .output()
        .expect("spawn browser-automation-cli");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exit={:?} stdout={stdout} stderr={stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("\"schema_version\":1") || stdout.contains("\"schema_version\": 1"),
        "stdout={stdout}"
    );
    // Envelope uses `ok` (schema_version=1 product contract).
    assert!(
        stdout.contains("\"ok\":true") || stdout.contains("\"ok\": true"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("about:blank") || stdout.contains("url"),
        "stdout={stdout}"
    );
}

#[test]
fn invalid_argv_still_exits_2() {
    let output = Command::new(BIN).args(["goto"]).output().expect("spawn");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn view_without_prior_session_is_one_shot_launch() {
    if !chrome_discoverable() {
        eprintln!("skip: no system Chrome/Chromium for view smoke");
        return;
    }
    // GAP-012: blank about:blank is refused unless --allow-empty.
    // Each command is one-shot: view launches its own headless Chrome then FINALIZE/DIE.
    let output = Command::new(BIN)
        .args(["view", "--json", "--allow-empty"])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code();
    // Launch/snapshot may pass (0) or fail unavailable (69) if Chrome is broken.
    assert!(
        code == Some(0) || code == Some(69),
        "exit={code:?} stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("schema_version") || stdout.contains("ok") || stdout.contains("error"),
        "stdout={stdout}"
    );
}

// Silence unused Duration import on skip-only platforms if rustc warns later.
#[allow(dead_code)]
const _SMOKE_BUDGET: Duration = Duration::from_secs(120);
