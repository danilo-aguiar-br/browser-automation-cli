// SPDX-License-Identifier: MIT OR Apache-2.0
//! Embedded Fluent (FTL) catalogs — parity source for translators + optional runtime check.

use fluent::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use super::ui_locale::UiLocale;
use super::ui_message::{ftl_id, UiMessage};

const EN_FTL: &str = include_str!("../../locales/en.ftl");
const PT_BR_FTL: &str = include_str!("../../locales/pt-BR.ftl");

/// Embedded FTL source for a compiled UI locale.
pub fn ftl_source(ui_locale: UiLocale) -> &'static str {
    match ui_locale {
        UiLocale::En => EN_FTL,
        UiLocale::PtBr => PT_BR_FTL,
    }
}

/// Build a Fluent bundle for a UI locale from the embedded FTL (for tests / diagnostics).
pub fn bundle_for(ui_locale: UiLocale) -> Result<FluentBundle<FluentResource>, String> {
    let lang: LanguageIdentifier = ui_locale.language_identifier();
    let mut bundle = FluentBundle::new(vec![lang]);
    // One-shot CLI: no need for concurrent memoizer (single-threaded format at boot/tests).
    bundle.set_use_isolating(false);
    let res = FluentResource::try_new(ftl_source(ui_locale).to_string())
        .map_err(|(_, errs)| format!("FTL parse errors for {}: {errs:?}", ui_locale.bcp47()))?;
    bundle
        .add_resource(res)
        .map_err(|errs| format!("FTL add_resource {}: {errs:?}", ui_locale.bcp47()))?;
    Ok(bundle)
}

/// Format a message id from the embedded FTL; falls back to enum catalog on miss.
pub fn format_ftl(ui_locale: UiLocale, msg: UiMessage) -> String {
    let id = ftl_id(msg);
    match bundle_for(ui_locale) {
        Ok(bundle) => {
            if let Some(message) = bundle.get_message(id) {
                if let Some(pattern) = message.value() {
                    let mut errors = vec![];
                    let s = bundle.format_pattern(pattern, None, &mut errors);
                    if errors.is_empty() && !s.is_empty() {
                        return s.to_string();
                    }
                }
            }
            msg.text(ui_locale).to_string()
        }
        Err(_) => msg.text(ui_locale).to_string(),
    }
}

/// Extract bare message ids from an FTL source (lines `key = value`, skip comments/blank).
pub fn ftl_keys(source: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = t.split_once('=') {
            let key = key.trim();
            if !key.is_empty() && !key.starts_with('-') {
                keys.push(key.to_string());
            }
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftl_en_pt_key_parity() {
        let en = ftl_keys(EN_FTL);
        let pt = ftl_keys(PT_BR_FTL);
        assert_eq!(en, pt, "en.ftl and pt-BR.ftl key sets must match");
        assert!(!en.is_empty());
    }

    #[test]
    fn ftl_keys_cover_all_ui_message() {
        let en = ftl_keys(EN_FTL);
        for m in UiMessage::ALL {
            let id = ftl_id(*m);
            assert!(en.iter().any(|k| k == id), "missing FTL key {id} for {m:?}");
        }
    }

    #[test]
    fn fluent_parses_both_packs() {
        bundle_for(UiLocale::En).expect("en FTL");
        bundle_for(UiLocale::PtBr).expect("pt-BR FTL");
    }

    #[test]
    fn ftl_format_matches_enum_for_usage() {
        let en = format_ftl(UiLocale::En, UiMessage::UsageSuggestion);
        assert_eq!(en, UiMessage::UsageSuggestion.text(UiLocale::En));
        let pt = format_ftl(UiLocale::PtBr, UiMessage::VisionRequired);
        assert!(pt.contains("invocação"), "{pt}");
    }

    #[test]
    fn ftl_matches_enum_catalog_all_messages_both_locales() {
        for m in UiMessage::ALL {
            for loc in [UiLocale::En, UiLocale::PtBr] {
                let ftl = format_ftl(loc, *m);
                let enum_t = m.text(loc);
                assert_eq!(
                    ftl,
                    enum_t,
                    "FTL/enum drift for {m:?} locale {}",
                    loc.bcp47()
                );
            }
        }
    }
}
