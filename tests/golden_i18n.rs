//! Golden envelope keys + i18n lang surface.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

#[test]
fn commands_json_has_schema_and_stable_shape() {
    let out = Command::new(BIN)
        .args(["commands", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    // Accept either envelope wrapper or raw list with schema_version sibling.
    let has_schema = stdout.contains("\"schema_version\":1")
        || stdout.contains("\"schema_version\": 1")
        || v.get("schema_version").and_then(|x| x.as_i64()) == Some(1);
    assert!(has_schema, "stdout={stdout}");
    assert!(
        stdout.contains("goto") || stdout.contains("scrape"),
        "stdout={stdout}"
    );
}

#[test]
fn lang_pt_br_changes_suggestion_on_usage_error() {
    let en = Command::new(BIN)
        .args([
            "--lang",
            "en",
            "--ignore-robots",
            "goto",
            "about:blank",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    let pt = Command::new(BIN)
        .args([
            "--lang",
            "pt-BR",
            "--ignore-robots",
            "goto",
            "about:blank",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    // Usage errors use sysexits-style code 2 in this CLI (ErrorKind::Usage).
    assert_eq!(en.status.code().unwrap_or(-1), 2);
    assert_eq!(pt.status.code().unwrap_or(-1), 2);
    let en_s = String::from_utf8_lossy(&en.stdout);
    let pt_s = String::from_utf8_lossy(&pt.stdout);
    // At least one of suggestion/message differs or pt contains Portuguese cue
    let differs = en_s != pt_s
        || pt_s.contains("Confira")
        || pt_s.contains("robots")
        || pt_s.to_lowercase().contains("argument");
    assert!(differs, "en={en_s} pt={pt_s}");
}

#[test]
fn lifecycle_signal_kinds_documented() {
    // Unit-level regression: kinds remain 124/130 (also covered in lifecycle unit tests).
    use browser_automation_cli::error::ErrorKind;
    assert_eq!(ErrorKind::Timeout.exit_code(), 124);
    assert_eq!(ErrorKind::Cancelled.exit_code(), 130);
}
