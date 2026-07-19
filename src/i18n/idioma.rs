// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed supported UI locales (`Idioma`) — single source of truth.

use unic_langid::{langid, LanguageIdentifier};

/// Text direction for terminal/UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direcao {
    /// Left-to-right (Latin, CJK layout LTR, etc.).
    Ltr,
    /// Right-to-left (Arabic, Hebrew) — only with `i18n-rtl` packs.
    Rtl,
}

/// Writing system tag used for documentation and future CJK/RTL packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScriptEscrita {
    /// Latin script (en, pt-BR, …).
    Latn,
    /// Simplified Chinese (feature `i18n-cjk`).
    Hans,
    /// Traditional Chinese (feature `i18n-cjk`).
    Hant,
    /// Japanese (feature `i18n-cjk`).
    Jpan,
    /// Korean (feature `i18n-cjk`).
    Kore,
    /// Arabic (feature `i18n-rtl`).
    Arab,
    /// Hebrew (feature `i18n-rtl`).
    Hebr,
}

/// Supported UI locale for human-facing suggestions.
///
/// Machine JSON (`error.message`, envelopes) stays English regardless of [`Idioma`].
/// Default build: [`Idioma::En`] + [`Idioma::PtBr`] only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Idioma {
    /// Neutral English (`en`) — technical validation locale.
    En,
    /// Brazilian Portuguese (`pt-BR`) — development / accent validation locale.
    PtBr,
}

impl Idioma {
    /// Locales compiled into this binary (MVP: en + pt-BR).
    pub const DISPONIVEIS: &'static [Idioma] = &[Idioma::En, Idioma::PtBr];

    /// BCP 47 tag used in diagnostics and FTL paths.
    pub const fn bcp47(self) -> &'static str {
        match self {
            Idioma::En => "en",
            Idioma::PtBr => "pt-BR",
        }
    }

    /// Legacy two-letter token used by older call sites (`en` / `pt`).
    pub const fn legacy_token(self) -> &'static str {
        match self {
            Idioma::En => "en",
            Idioma::PtBr => "pt",
        }
    }

    /// Primary language subtag (ISO 639).
    pub const fn language(self) -> &'static str {
        match self {
            Idioma::En => "en",
            Idioma::PtBr => "pt",
        }
    }

    /// Regional fallback (pt-BR → still PtBr as base pack; en is neutral).
    pub const fn fallback(self) -> Idioma {
        match self {
            Idioma::En => Idioma::En,
            Idioma::PtBr => Idioma::PtBr,
        }
    }

    /// Text direction for this locale.
    pub const fn direcao(self) -> Direcao {
        Direcao::Ltr
    }

    /// Writing system.
    pub const fn script(self) -> ScriptEscrita {
        ScriptEscrita::Latn
    }

    /// Convert to `unic_langid::LanguageIdentifier`.
    pub fn language_identifier(self) -> LanguageIdentifier {
        match self {
            Idioma::En => langid!("en"),
            Idioma::PtBr => langid!("pt-BR"),
        }
    }

    /// Map a parsed language id onto a compiled pack (language + region aware).
    pub fn from_langid(id: &LanguageIdentifier) -> Option<Idioma> {
        let lang = id.language.as_str();
        match lang {
            "en" => Some(Idioma::En),
            "pt" => {
                // MVP pack is pt-BR only. Bare `pt` and `pt-BR` → PtBr.
                // `pt-PT` has no pack in default binary → None (negotiator falls to en).
                match id.region.as_ref().map(|r| r.as_str()) {
                    Some("PT") => None,
                    Some("BR") | None => Some(Idioma::PtBr),
                    _ => Some(Idioma::PtBr),
                }
            }
            _ => None,
        }
    }

    /// Parse a user/CLI/env token into a supported idioma when unambiguous.
    pub fn parse_token(raw: &str) -> Option<Idioma> {
        let s = raw.trim().replace('_', "-");
        if s.is_empty() {
            return None;
        }
        // Accept legacy `pt` as pt-BR (MVP rule: no bare-pt pack, CLI accepts pt → pt-BR).
        let lower = s.to_ascii_lowercase();
        if lower == "pt" || lower.starts_with("pt-br") || lower == "pt-br" {
            return Some(Idioma::PtBr);
        }
        if lower == "en" || lower.starts_with("en-") {
            return Some(Idioma::En);
        }
        let id: LanguageIdentifier = s.parse().ok()?;
        Self::from_langid(&id)
    }
}

impl std::fmt::Display for Idioma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.bcp47())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tokens_accept_regional_and_legacy() {
        assert_eq!(Idioma::parse_token("pt-BR"), Some(Idioma::PtBr));
        assert_eq!(Idioma::parse_token("pt_BR.UTF-8"), Some(Idioma::PtBr));
        assert_eq!(Idioma::parse_token("pt"), Some(Idioma::PtBr));
        assert_eq!(Idioma::parse_token("EN-us"), Some(Idioma::En));
        assert_eq!(Idioma::parse_token("de-DE"), None);
    }

    #[test]
    fn disponiveis_is_mvp_bilingual() {
        assert_eq!(Idioma::DISPONIVEIS, &[Idioma::En, Idioma::PtBr]);
    }
}
