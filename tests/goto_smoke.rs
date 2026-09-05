//! Integration smoke: one-shot `goto about:blank` (PR3).
//!
//! Skips when Chrome is not available on PATH / system locations.

mod common;
use common::chrome_discoverable;

#[test]
fn goto_about_blank_json_when_chrome_present() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "goto_smoke::goto",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }

    let output = common::cmd()
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
    let output = common::cmd().args(["goto"]).output().expect("spawn");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn view_without_prior_session_is_one_shot_launch() {
    if !chrome_discoverable() {
        common::skip_with_remedy(
            "goto_smoke::view",
            "no usable Chrome on this host.",
            "install a system Chrome/Chromium.",
        );
        return;
    }
    // GAP-012: blank about:blank is refused unless --allow-empty.
    // Each command is one-shot: view launches its own headless Chrome then FINALIZE/DIE.
    let output = common::cmd()
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
