//! GAP-017: residual zero for CLI marker profiles after one-shot DIE.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

#[test]
fn goto_leaves_zero_cli_chrome_marker_dirs() {
    let before = browser_automation_cli::residual::list_cli_chrome_marker_dirs();
    let output = Command::new(BIN)
        .args(["--json", "goto", "about:blank"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn goto");
    assert!(
        output.status.success(),
        "goto failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = browser_automation_cli::residual::list_cli_chrome_marker_dirs();
    // Any dirs that appeared during the run must be gone after DIE.
    let leaked: Vec<_> = after.iter().filter(|p| !before.contains(p)).collect();
    assert!(
        leaked.is_empty(),
        "leaked CLI chrome marker dirs after one-shot: {leaked:?}"
    );
}
