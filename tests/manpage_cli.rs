//! Man page generation (clap_mangen) integration.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

#[test]
fn man_stdout_is_roff() {
    let out = Command::new(BIN)
        .args(["man"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn man");
    assert!(
        out.status.success(),
        "man failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains(".TH") || text.contains("browser-automation-cli"),
        "expected roff man content, got head: {}",
        text.chars().take(200).collect::<String>()
    );
}

#[test]
fn man_out_writes_file_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("browser-automation-cli.1");
    let out = Command::new(BIN)
        .args(["man", "--out"])
        .arg(&path)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn man --out");
    assert!(
        out.status.success(),
        "man --out failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = std::fs::read_to_string(&path).expect("read man file");
    assert!(body.contains("browser-automation-cli") || body.contains(".TH"));
}

#[test]
fn man_rejects_path_traversal() {
    let out = Command::new(BIN)
        .args(["man", "--out", "../evil.1"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2), "expected usage exit 2");
}
