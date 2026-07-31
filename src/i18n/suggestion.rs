// SPDX-License-Identifier: MIT OR Apache-2.0
//! Localized human suggestions: kind lookup, catalog keys, remapping.
//!
//! Machine `error.message` stays English; only the human hint localizes.

use super::{effective_ui_locale, UiLocale, UiMessage};

/// Localized suggestion for a known kind key.
pub fn suggestion_for(kind: &str, lang: Option<&str>) -> Option<&'static str> {
    let ui_locale = lang
        .and_then(UiLocale::parse_token)
        .unwrap_or_else(effective_ui_locale);
    UiMessage::from_error_kind(kind).map(|m| m.text(ui_locale))
}

/// Catalog of stable suggestion keys (preferred over hard-coded English).
pub fn suggestion_key(key: &str, lang: Option<&str>) -> &'static str {
    let ui_locale = lang
        .and_then(UiLocale::parse_token)
        .unwrap_or_else(effective_ui_locale);
    UiMessage::from_suggestion_key(key).text(ui_locale)
}

/// Apply kind-based localized suggestion when none is set, or re-map known EN strings.
pub fn localize_error_suggestion(err: &crate::error::CliError) -> crate::error::CliError {
    let ui_locale = effective_ui_locale();
    if ui_locale == UiLocale::En {
        return err.clone();
    }
    // Prefer catalog by kind when suggestion is missing.
    if err.suggestion().is_none() {
        if let Some(s) = suggestion_for(err.kind().as_str(), Some(ui_locale.legacy_token())) {
            let mut out = crate::error::CliError::with_suggestion(err.kind(), err.message(), s);
            if let Some(d) = err.data() {
                out = out.with_data(d.clone());
            }
            return out;
        }
        return err.clone();
    }
    let s = err.suggestion().unwrap_or("");
    // Map common English suggestions to Portuguese via enum catalog.
    let mapped = match s {
        "Pass --experimental-vision on the same invocation" => {
            UiMessage::VisionRequired.text(ui_locale)
        }
        "Pass both flags together when you intentionally skip robots.txt" => {
            UiMessage::RobotsDual.text(ui_locale)
        }
        "Pass --category-memory (heap take/summary/close work without deep graph ops)" => {
            UiMessage::CategoryMemory.text(ui_locale)
        }
        "Pass --category-extensions on the same invocation" => {
            UiMessage::CategoryExtensions.text(ui_locale)
        }
        "Pass --experimental-screencast on the same invocation" => {
            UiMessage::ScreencastFlag.text(ui_locale)
        }
        "Pass --category-webmcp on the same invocation" => UiMessage::WebmcpFlag.text(ui_locale),
        "Pass --category-third-party on the same invocation" => {
            UiMessage::ThirdPartyFlag.text(ui_locale)
        }
        "Pass --capture-network before run/net" => UiMessage::CaptureNetwork.text(ui_locale),
        "Pass --capture-console before run/console" => UiMessage::CaptureConsole.text(ui_locale),
        "Fix the failing step; subsequent steps were not executed" => {
            UiMessage::RunFailFast.text(ui_locale)
        }
        other if other.contains("lighthouse") && other.contains("npm") => {
            UiMessage::LighthouseMissing.text(ui_locale)
        }
        other if other.contains("lighthouse") && other.contains("config set") => {
            UiMessage::LighthouseMissing.text(ui_locale)
        }
        _ => {
            return err.clone();
        }
    };
    let mut out = crate::error::CliError::with_suggestion(err.kind(), err.message(), mapped);
    if let Some(d) = err.data() {
        out = out.with_data(d.clone());
    }
    out
}
