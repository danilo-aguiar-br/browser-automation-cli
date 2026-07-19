//! D-22: contract for optional rotated JSON log lines (tracing-subscriber json layer).
//!
//! Does not require Chrome. Validates the **field names** agents may rely on when
//! `log_to_file=true` and a JSON line is parsed offline.

use serde_json::Value;

/// Minimal schema expected from `tracing_subscriber::fmt::layer().json()`.
fn assert_tracing_json_line_schema(line: &str) {
    let v: Value = serde_json::from_str(line).expect("json log line");
    let obj = v.as_object().expect("object");
    // Core fields produced by the json formatter (stable across minor versions).
    assert!(
        obj.contains_key("timestamp") || obj.contains_key("time"),
        "timestamp field: {obj:?}"
    );
    assert!(obj.contains_key("level"), "level: {obj:?}");
    assert!(
        obj.contains_key("fields") || obj.contains_key("message") || obj.contains_key("target"),
        "payload fields: {obj:?}"
    );
}

#[test]
fn sample_tracing_json_line_matches_agent_contract() {
    // Representative line shape (mirrors tracing-subscriber json layer output).
    let sample = r#"{"timestamp":"2026-07-18T00:00:00.000000Z","level":"INFO","fields":{"message":"telemetry ready","filter":"error"},"target":"browser_automation_cli::telemetry"}"#;
    assert_tracing_json_line_schema(sample);
}

#[test]
fn filter_directive_helpers_are_stable() {
    use browser_automation_cli::telemetry::resolve_filter_directive;
    // Quiet wins.
    assert_eq!(
        resolve_filter_directive(true, true, true, Some("debug")),
        "error"
    );
    // Debug beats verbose.
    assert_eq!(
        resolve_filter_directive(false, true, true, None),
        "debug"
    );
    // XDG level when no argv flags.
    assert_eq!(
        resolve_filter_directive(false, false, false, Some("warn")),
        "warn"
    );
}
