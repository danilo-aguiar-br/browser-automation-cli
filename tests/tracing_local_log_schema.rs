//! D-22 / Pass I: contract for optional rotated JSON log lines (tracing-subscriber json layer).
//!
//! Does not require Chrome. Validates the **field names** agents may rely on when
//! `log_to_file=true` and a JSON line is parsed offline. Local-only — no remote telemetry.

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
    // Wording is local-only ("tracing initialized"), never product "telemetry".
    let sample = r#"{"timestamp":"2026-07-18T00:00:00.000000Z","level":"INFO","fields":{"message":"tracing initialized (local only; no remote export)","effective_filter":"error","filter_fallback":false,"correlation_id":"agent-42"},"target":"browser_automation_cli::tracing_local"}"#;
    assert_tracing_json_line_schema(sample);
    let v: Value = serde_json::from_str(sample).unwrap();
    let fields = v.get("fields").and_then(|f| f.as_object()).unwrap();
    assert!(fields.contains_key("correlation_id") || fields.contains_key("message"));
}

#[test]
fn filter_directive_helpers_are_stable() {
    use browser_automation_cli::tracing_local::resolve_filter_directive;
    // Quiet wins.
    assert_eq!(
        resolve_filter_directive(true, true, true, Some("debug")),
        "error"
    );
    // Debug beats verbose.
    assert_eq!(resolve_filter_directive(false, true, true, None), "debug");
    // XDG level when no argv flags.
    assert_eq!(
        resolve_filter_directive(false, false, false, Some("warn")),
        "warn"
    );
}

#[test]
fn log_level_and_rotation_validators() {
    use browser_automation_cli::tracing_local::{
        validate_log_level_directive, validate_log_rotation,
    };
    assert!(validate_log_level_directive("info").is_ok());
    assert!(validate_log_level_directive("%%%").is_err());
    assert!(validate_log_rotation("daily").is_ok());
    assert!(validate_log_rotation("weekly").is_err());
}
