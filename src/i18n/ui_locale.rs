// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed supported UI locales (`UiLocale`) — single source of truth.

use unic_langid::{langid, LanguageIdentifier};

/// Text direction for terminal/UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextDirection {
    /// Left-to-right (Latin, CJK layout LTR, etc.).
    Ltr,
    /// Right-to-left (Arabic, Hebrew) — only with `i18n-rtl` packs.
    Rtl,
}

/// Writing system tag used for documentation and future CJK/RTL packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WritingScript {
    /// Latin script (en, pt-BR, …).
    Latn,
    /// Simplified Chinese (scaffold; packs behind feature `i18n-cjk`).
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n-cjk")))]
    Hans,
    /// Traditional Chinese (scaffold; packs behind feature `i18n-cjk`).
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n-cjk")))]
    Hant,
    /// Japanese (scaffold; packs behind feature `i18n-cjk`).
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n-cjk")))]
    Jpan,
    /// Korean (scaffold; packs behind feature `i18n-cjk`).
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n-cjk")))]
    Kore,
    /// Arabic (scaffold; packs behind feature `i18n-rtl`).
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n-rtl")))]
    Arab,
    /// Hebrew (scaffold; packs behind feature `i18n-rtl`).
    #[cfg_attr(docsrs, doc(cfg(feature = "i18n-rtl")))]
    Hebr,
}

/// Supported UI locale for human-facing suggestions.
///
/// Machine JSON (`error.message`, envelopes) stays English regardless of [`UiLocale`].
/// Default build: [`UiLocale::En`] + [`UiLocale::PtBr`] only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UiLocale {
    /// Neutral English (`en`) — technical validation locale.
    En,
    /// Brazilian Portuguese (`pt-BR`) — development / accent validation locale.
    PtBr,
}

impl UiLocale {
    /// Locales compiled into this binary (MVP: en + pt-BR).
    pub const AVAILABLE: &'static [UiLocale] = &[UiLocale::En, UiLocale::PtBr];

    /// BCP 47 tag used in diagnostics and FTL paths.
    pub const fn bcp47(self) -> &'static str {
        match self {
            UiLocale::En => "en",
            UiLocale::PtBr => "pt-BR",
        }
    }

    /// Legacy two-letter token used by older call sites (`en` / `pt`).
    pub const fn legacy_token(self) -> &'static str {
        match self {
            UiLocale::En => "en",
            UiLocale::PtBr => "pt",
        }
    }

    /// Primary language subtag (ISO 639).
    pub const fn language(self) -> &'static str {
        match self {
            UiLocale::En => "en",
            UiLocale::PtBr => "pt",
        }
    }

    /// Regional fallback (pt-BR → still PtBr as base pack; en is neutral).
    pub const fn fallback(self) -> UiLocale {
        match self {
            UiLocale::En => UiLocale::En,
            UiLocale::PtBr => UiLocale::PtBr,
        }
    }

    /// Text direction for this locale.
    pub const fn direction(self) -> TextDirection {
        TextDirection::Ltr
    }

    /// Writing system.
    pub const fn script(self) -> WritingScript {
        WritingScript::Latn
    }

    /// Convert to `unic_langid::LanguageIdentifier`.
    pub fn language_identifier(self) -> LanguageIdentifier {
        match self {
            UiLocale::En => langid!("en"),
            UiLocale::PtBr => langid!("pt-BR"),
        }
    }

    /// Map a parsed language id onto a compiled pack (language + region aware).
    ///
    /// Rules: bare `pt` (no region) is **not** a substitute for `pt-BR`.
    /// `pt-PT` has no default pack → `None` (negotiator falls to `en`).
    pub fn from_langid(id: &LanguageIdentifier) -> Option<UiLocale> {
        let lang = id.language.as_str();
        match lang {
            "en" => Some(UiLocale::En),
            "pt" => match id.region.as_ref().map(|r| r.as_str()) {
                Some("BR") => Some(UiLocale::PtBr),
                // Bare `pt` / `pt-PT` / other regions: no MVP pack.
                _ => None,
            },
            _ => None,
        }
    }

    /// Parse a user/CLI/XDG token into a supported UI locale when unambiguous.
    ///
    /// Accepts `en`, `en-*`, `pt-BR` (and `pt_BR` / encoding suffixes after normalize).
    /// Rejects bare `pt` (must be `pt-BR` per multi-language rules).
    pub fn parse_token(raw: &str) -> Option<UiLocale> {
        let mut s = raw.trim().replace('_', "-");
        if s.is_empty() {
            return None;
        }
        // Drop encoding / modifier suffixes: `pt-BR.UTF-8@euro` → `pt-BR`
        if let Some(idx) = s.find(['.', '@']) {
            s.truncate(idx);
        }
        let lower = s.to_ascii_lowercase();
        // Bare `pt` is not a substitute for `pt-BR`.
        if lower == "pt" {
            return None;
        }
        if lower == "pt-br" {
            return Some(UiLocale::PtBr);
        }
        if lower == "en" || lower.starts_with("en-") {
            return Some(UiLocale::En);
        }
        let id: LanguageIdentifier = s.parse().ok()?;
        Self::from_langid(&id)
    }
}

impl std::fmt::Display for UiLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.bcp47())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tokens_accept_regional_reject_bare_pt() {
        assert_eq!(UiLocale::parse_token("pt-BR"), Some(UiLocale::PtBr));
        assert_eq!(UiLocale::parse_token("pt_BR.UTF-8"), Some(UiLocale::PtBr));
        assert_eq!(UiLocale::parse_token("pt"), None, "bare pt is not pt-BR");
        assert_eq!(UiLocale::parse_token("pt-PT"), None);
        assert_eq!(UiLocale::parse_token("EN-us"), Some(UiLocale::En));
        assert_eq!(UiLocale::parse_token("de-DE"), None);
    }

    #[test]
    fn available_is_mvp_bilingual() {
        assert_eq!(UiLocale::AVAILABLE, &[UiLocale::En, UiLocale::PtBr]);
    }
}
