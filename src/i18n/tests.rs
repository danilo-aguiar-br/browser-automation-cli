// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for locale resolution and the catalog.

use super::*;

#[test]
fn all_ui_message_non_empty_both_locales() {
    for m in UiMessage::ALL {
        let en = m.text(UiLocale::En);
        let pt = m.text(UiLocale::PtBr);
        assert!(!en.is_empty(), "empty en for {m:?}");
        assert!(!pt.is_empty(), "empty pt-BR for {m:?}");
        assert_ne!(en, "", "{m:?}");
    }
}

#[test]
fn pt_br_has_accents_on_critical_keys() {
    assert!(UiMessage::VisionRequired
        .text(UiLocale::PtBr)
        .contains("invocação"));
    assert!(UiMessage::RobotsDual
        .text(UiLocale::PtBr)
        .contains("propósito"));
    assert!(UiMessage::RunFailFast.text(UiLocale::PtBr).contains("não"));
    assert!(UiMessage::UsageSuggestion
        .text(UiLocale::PtBr)
        .contains("obrigatórios"));
}

#[test]
fn text_api_no_global_required() {
    // Tests must not depend on process OnceLock.
    assert_eq!(
        UiMessage::UsageSuggestion.text(UiLocale::En),
        "Check --help and required arguments"
    );
}

#[test]
fn truncate_respects_graphemes() {
    let s = "ação";
    assert_eq!(truncate_graphemes(s, 2), "aç");
    assert_eq!(truncate_graphemes(s, 10), "ação");
}
