// SPDX-License-Identifier: MIT OR Apache-2.0
//! Embedded Fluent (FTL) catalogs — parity source for translators + optional runtime check.

use fluent::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use super::idioma::Idioma;
use super::mensagem::{ftl_id, Mensagem};

const EN_FTL: &str = include_str!("../../locales/en.ftl");
const PT_BR_FTL: &str = include_str!("../../locales/pt-BR.ftl");

/// Embedded FTL source for a compiled idioma.
pub fn ftl_source(idioma: Idioma) -> &'static str {
    match idioma {
        Idioma::En => EN_FTL,
        Idioma::PtBr => PT_BR_FTL,
    }
}

/// Build a Fluent bundle for `idioma` from the embedded FTL (for tests / diagnostics).
pub fn bundle_for(idioma: Idioma) -> Result<FluentBundle<FluentResource>, String> {
    let lang: LanguageIdentifier = idioma.language_identifier();
    let mut bundle = FluentBundle::new(vec![lang]);
    // One-shot CLI: no need for concurrent memoizer (single-threaded format at boot/tests).
    bundle.set_use_isolating(false);
    let res = FluentResource::try_new(ftl_source(idioma).to_string())
        .map_err(|(_, errs)| format!("FTL parse errors for {}: {errs:?}", idioma.bcp47()))?;
    bundle
        .add_resource(res)
        .map_err(|errs| format!("FTL add_resource {}: {errs:?}", idioma.bcp47()))?;
    Ok(bundle)
}

/// Format a message id from the embedded FTL; falls back to enum catalog on miss.
pub fn format_ftl(idioma: Idioma, msg: Mensagem) -> String {
    let id = ftl_id(msg);
    match bundle_for(idioma) {
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
            msg.texto(idioma).to_string()
        }
        Err(_) => msg.texto(idioma).to_string(),
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
    fn ftl_keys_cover_all_mensagem() {
        let en = ftl_keys(EN_FTL);
        for m in Mensagem::ALL {
            let id = ftl_id(*m);
            assert!(
                en.iter().any(|k| k == id),
                "missing FTL key {id} for {m:?}"
            );
        }
    }

    #[test]
    fn fluent_parses_both_packs() {
        bundle_for(Idioma::En).expect("en FTL");
        bundle_for(Idioma::PtBr).expect("pt-BR FTL");
    }

    #[test]
    fn ftl_format_matches_enum_for_usage() {
        let en = format_ftl(Idioma::En, Mensagem::UsageSuggestion);
        assert_eq!(en, Mensagem::UsageSuggestion.texto(Idioma::En));
        let pt = format_ftl(Idioma::PtBr, Mensagem::VisionRequired);
        assert!(pt.contains("invocação"), "{pt}");
    }
}
