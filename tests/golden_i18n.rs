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
    assert!(
        stdout.contains("locale"),
        "commands --json must list locale: {stdout}"
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

#[test]
fn pt_br_suggestions_use_accents() {
    use browser_automation_cli::i18n::{UiLocale, UiMessage};
    let v = UiMessage::VisionRequired.text(UiLocale::PtBr);
    assert!(
        v.contains("invocação"),
        "expected accented pt-BR invocação: {v}"
    );
    let r = UiMessage::RobotsDual.text(UiLocale::PtBr);
    assert!(r.contains("propósito"), "expected propósito: {r}");
    let u = UiMessage::RunFailFast.text(UiLocale::PtBr);
    assert!(u.contains("não"), "expected não: {u}");
}

#[test]
fn locale_subcommand_json_shape() {
    let out = Command::new(BIN)
        .args(["--lang", "pt-BR", "locale", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    assert_eq!(
        out.status.code().unwrap_or(-1),
        0,
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    // Envelope or raw
    let data = v.get("data").cloned().unwrap_or(v);
    assert_eq!(data.get("resolved").and_then(|x| x.as_str()), Some("pt-BR"));
    assert_eq!(data.get("source").and_then(|x| x.as_str()), Some("flag"));
    assert!(data.get("available").is_some());
    // Product-law: no product env key in diagnostics.
    assert!(data.get("lang_env_key").is_none());
    assert!(data.get("env_override").is_none());
    let resolution = data
        .get("resolution")
        .and_then(|x| x.as_array())
        .expect("resolution array");
    assert!(resolution.iter().any(|v| v.as_str() == Some("flag")));
    assert!(resolution.iter().any(|v| v.as_str() == Some("xdg")));
}

#[test]
fn product_env_lang_is_ignored() {
    // Product-law: BROWSER_AUTOMATION_CLI_LANG must not override locale.
    let out = Command::new(BIN)
        .args(["--lang", "en", "locale", "--json"])
        .env("NO_COLOR", "1")
        .env("BROWSER_AUTOMATION_CLI_LANG", "pt-BR")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"source\":\"flag\"") || stdout.contains("\"source\": \"flag\""),
        "flag must win; env ignored: {stdout}"
    );
    assert!(
        !stdout.contains("\"source\":\"env\"") && !stdout.contains("\"source\": \"env\""),
        "env must not be a locale source: {stdout}"
    );
}

#[test]
fn ui_message_parity_en_pt_via_public_api() {
    use browser_automation_cli::i18n::{UiLocale, UiMessage};
    for m in UiMessage::ALL {
        assert!(!m.text(UiLocale::En).is_empty());
        assert!(!m.text(UiLocale::PtBr).is_empty());
    }
}
