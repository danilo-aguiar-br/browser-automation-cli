// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for local tracing configuration.

use super::*;

#[test]
fn filter_priority_quiet_wins() {
    assert_eq!(
        resolve_filter_directive(true, true, true, Some("trace")),
        DEFAULT_LOG_LEVEL
    );
}

#[test]
fn filter_debug_before_verbose() {
    assert_eq!(resolve_filter_directive(false, true, true, None), "debug");
}

#[test]
fn filter_verbose_info() {
    assert_eq!(resolve_filter_directive(false, true, false, None), "info");
}

#[test]
fn filter_xdg_when_no_flags() {
    assert_eq!(
        resolve_filter_directive(false, false, false, Some("warn")),
        "warn"
    );
}

#[test]
fn filter_default_error() {
    assert_eq!(
        resolve_filter_directive(false, false, false, Some("  ")),
        DEFAULT_LOG_LEVEL
    );
    assert_eq!(
        resolve_filter_directive(false, false, false, None),
        DEFAULT_LOG_LEVEL
    );
}

#[test]
fn max_log_files_clamp() {
    assert_eq!(clamp_max_log_files(None), DEFAULT_MAX_LOG_FILES as usize);
    assert_eq!(clamp_max_log_files(Some(0)), MAX_LOG_FILES_MIN as usize);
    assert_eq!(clamp_max_log_files(Some(999)), MAX_LOG_FILES_CAP as usize);
    assert_eq!(clamp_max_log_files(Some(14)), 14);
}

#[test]
fn validate_log_level_accepts_common() {
    assert!(validate_log_level_directive("error").is_ok());
    assert!(validate_log_level_directive("info").is_ok());
    assert!(validate_log_level_directive("debug").is_ok());
    assert!(validate_log_level_directive("browser_automation_cli=debug").is_ok());
}

#[test]
fn validate_log_level_rejects_garbage() {
    assert!(validate_log_level_directive("").is_err());
    assert!(validate_log_level_directive("   ").is_err());
    assert!(validate_log_level_directive("%%%not-a-filter").is_err());
}

#[test]
fn validate_rotation_values() {
    assert!(validate_log_rotation("daily").is_ok());
    assert!(validate_log_rotation("HOURLY").is_ok());
    assert!(validate_log_rotation("never").is_ok());
    assert!(validate_log_rotation("weekly").is_err());
}

#[test]
fn parse_rotation_maps() {
    assert!(matches!(
        parse_log_rotation(Some("hourly")),
        Rotation::HOURLY
    ));
    assert!(matches!(parse_log_rotation(Some("never")), Rotation::NEVER));
    assert!(matches!(parse_log_rotation(None), Rotation::DAILY));
    assert!(matches!(parse_log_rotation(Some("weird")), Rotation::DAILY));
}

#[test]
fn log_file_prefix_matches_package() {
    assert_eq!(LOG_FILE_PREFIX, env!("CARGO_PKG_NAME"));
}
