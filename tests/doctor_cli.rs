//! Integration tests for `browser-automation-cli doctor`.
//!
//! These tests spawn the real CLI binary via `env!("CARGO_BIN_EXE_*")` and
//! verify the doctor command produces sane output. They override
//! `HOME` / `USERPROFILE` so the doctor inspects a throwaway directory
//! and never touches the user's real state.

use std::process::Command;
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn build_doctor_cmd(tmp: &TempDir, args: &[&str]) -> Command {
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("NO_COLOR", "1");
    cmd
}

#[test]
fn doctor_offline_quick_json_emits_valid_payload() {
    let tmp = TempDir::new().unwrap();

    let output = build_doctor_cmd(&tmp, &["doctor", "--offline", "--quick", "--json"])
        .output()
        .expect("failed to invoke browser-automation-cli doctor");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // Exit code 0 (all pass) or 1 (one or more fails) are both valid outcomes.
    assert!(
        code == 0 || code == 1,
        "unexpected exit code {}\nstdout:\n{}\nstderr:\n{}",
        code,
        stdout,
        stderr,
    );

    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not JSON: {}\n---\n{}", e, stdout));

    assert_eq!(payload["schema_version"], 1);
    assert!(payload.get("ok").is_some(), "missing ok field");
    let data = payload.get("data").expect("missing data envelope");
    assert!(data.get("ok").is_some(), "missing data.ok");
    assert_eq!(data["schema_version"], 1);

    let checks = data["checks"]
        .as_array()
        .expect("checks should be an array");
    assert!(!checks.is_empty(), "checks array should not be empty");

    for c in checks {
        assert!(
            c["id"].as_str().is_some_and(|s| !s.is_empty()),
            "check missing id: {}",
            c
        );
        let status = c["status"].as_str().expect("status should be string");
        assert!(
            ["pass", "warn", "fail", "info"].contains(&status),
            "unexpected status {:?}",
            status
        );
        assert!(
            c["message"].as_str().is_some_and(|s| !s.is_empty()),
            "check missing message: {}",
            c
        );
    }

    let mut seen = std::collections::HashSet::new();
    for c in checks {
        let id = c["id"].as_str().unwrap();
        assert!(
            seen.insert(id.to_string()),
            "duplicate check id in JSON output: {}\nfull payload:\n{}",
            id,
            stdout
        );
    }

    // GAP-004: never recommend npm in product UX.
    assert!(
        !stdout.contains("npm i") && !stdout.contains("npm install"),
        "doctor must not suggest npm:\n{stdout}"
    );
}

#[test]
fn doctor_fix_json_never_suggests_npm() {
    let tmp = TempDir::new().unwrap();
    let output = build_doctor_cmd(&tmp, &["doctor", "--offline", "--quick", "--fix", "--json"])
        .output()
        .expect("failed to invoke doctor --fix");
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        !stdout.contains("npm"),
        "doctor --fix must not mention npm:\n{stdout}"
    );
}

#[test]
fn doctor_help_describes_flags_and_examples() {
    let tmp = TempDir::new().unwrap();

    let output = build_doctor_cmd(&tmp, &["doctor", "--help"])
        .output()
        .expect("failed to invoke browser-automation-cli doctor --help");

    assert!(
        output.status.success(),
        "doctor --help should exit 0; got {:?}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    for needle in [
        "browser-automation-cli doctor",
        "--offline",
        "--quick",
        "--fix",
        "--json",
    ] {
        assert!(
            stdout.contains(needle),
            "doctor --help output missing {:?}\n---\n{}",
            needle,
            stdout
        );
    }
}
