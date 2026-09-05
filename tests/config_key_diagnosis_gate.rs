// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config set` must diagnose the KEY independently of the VALUE.
//!
//! # The defect this gate closes
//!
//! `policy_set` used to parse the value before deciding whether the key
//! belonged to the promoted-knob table. An unparseable value short-circuited
//! with `<key> must be a positive integer`, so one missing key produced two
//! different diagnoses:
//!
//! ```text
//! config set nonexistent_key abc  ->  "nonexistent_key must be a positive integer"
//! config set nonexistent_key 42   ->  "unknown config key: nonexistent_key"
//! ```
//!
//! The first message asserts the key exists. An agent that mistypes a key is
//! told to send an integer, retries with a number, and only then learns the
//! key was never real — two turns spent on a claim the code could not make.
//!
//! # Why a separate file
//!
//! The property under test is a cross-cutting contract of the `config set`
//! dispatcher, not of any one key family, and it must keep holding as keys are
//! added and removed. It also pins the OCR excision: `ocr_engine`, `ocr_lang`
//! and `tesseract_path` are gone and must report as unknown, never as invalid.

mod common;

/// Run `config set KEY VALUE` in an isolated config home and return the message.
fn set_key_message(home: &std::path::Path, key: &str, value: &str) -> String {
    let out = common::cmd()
        .args(["--json", "config", "set", key, value])
        // `HOME` is what isolates config on macOS: `directories` resolves to
        // ~/Library/Application Support and never reads `XDG_CONFIG_HOME`.
        // Full measurement (2026-09-04) lives in `tests/scrape_wave6_gate.rs`.
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .output()
        .expect("spawn config set");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("config set emits a JSON envelope on stdout");
    assert_eq!(
        v.get("ok").and_then(serde_json::Value::as_bool),
        Some(false),
        "expected a failure envelope for {key}={value}, got {v}"
    );
    v.pointer("/error/message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[test]
fn an_unknown_key_reads_the_same_whatever_the_value_looks_like() {
    let guard = tempfile::Builder::new()
        .prefix("bac-config-key-diagnosis-")
        .tempdir()
        .expect("temp config home");
    let tmp = guard.path().to_path_buf();

    let numeric = set_key_message(&tmp, "totally_invented_key", "42");
    let textual = set_key_message(&tmp, "totally_invented_key", "abc");

    assert!(
        numeric.contains("unknown config key"),
        "numeric value must still name the key as unknown: {numeric}"
    );
    assert_eq!(
        numeric, textual,
        "the diagnosis of a missing key must not depend on the value"
    );
}

#[test]
fn the_excised_ocr_keys_report_as_unknown() {
    let guard = tempfile::Builder::new()
        .prefix("bac-config-key-diagnosis-")
        .tempdir()
        .expect("temp config home");
    let tmp = guard.path().to_path_buf();

    for key in ["ocr_engine", "ocr_lang", "tesseract_path"] {
        let msg = set_key_message(&tmp, key, "tesseract");
        assert!(
            msg.contains("unknown config key"),
            "{key} must be gone from the settable surface, got: {msg}"
        );
    }
}

#[test]
fn a_real_knob_still_rejects_a_bad_value_by_naming_the_value() {
    let guard = tempfile::Builder::new()
        .prefix("bac-config-key-diagnosis-")
        .tempdir()
        .expect("temp config home");
    let tmp = guard.path().to_path_buf();

    let msg = set_key_message(&tmp, "redis_io_timeout_secs", "not-a-number");
    assert!(
        msg.contains("integer"),
        "a real policy knob must still validate its value: {msg}"
    );
    assert!(
        !msg.contains("unknown config key"),
        "a real policy knob must not be reported as unknown: {msg}"
    );
}

#[test]
fn the_ocr_keys_are_absent_from_list_keys() {
    let out = common::cmd()
        .args(["--json", "config", "list-keys"])
        .output()
        .expect("spawn config list-keys");
    assert!(out.status.success(), "config list-keys must succeed");
    let body = String::from_utf8(out.stdout).expect("utf-8 envelope");
    for needle in ["ocr_engine", "ocr_lang", "tesseract_path"] {
        assert!(
            !body.contains(needle),
            "{needle} must not appear in config list-keys"
        );
    }
}
