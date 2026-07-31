// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-platform locale detection + negotiation (single boot path).
//!
//! Product-law: **no product environment variables**. Locale comes from
//! `--lang` → XDG `lang` → OS (`sys-locale`) → default `en`.

use std::sync::OnceLock;

use fluent_langneg::negotiate::NegotiationStrategy;
use fluent_langneg::negotiate_languages;
use unic_langid::LanguageIdentifier;

use super::ui_locale::UiLocale;

/// Where the effective UI locale came from (diagnostics / `locale` subcommand).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocaleSource {
    /// Global `--lang` argv flag.
    Flag,
    /// XDG config `lang` key.
    Xdg,
    /// OS locale via `sys-locale` + negotiation.
    System,
    /// Hard fallback (`en`) when nothing else matched.
    Default,
}

impl LocaleSource {
    /// Stable machine token for JSON diagnostics (`flag` / `xdg` / `system` / `default`).
    pub const fn as_str(self) -> &'static str {
        match self {
            LocaleSource::Flag => "flag",
            LocaleSource::Xdg => "xdg",
            LocaleSource::System => "system",
            LocaleSource::Default => "default",
        }
    }
}

/// Result of the 4-layer resolution chain.
///
/// # Ownership
///
/// Owned fields only — never `Box::leak` / artificial `'static` for diagnostics
/// (rules_rust_ownership: `'static` only for true program-lifetime data).
/// `ui_locale` / `source` are `Copy`; `system_raw` is an owned `String` when present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLocale {
    /// Compiled UI pack selected for human suggestions.
    pub ui_locale: UiLocale,
    /// Which precedence layer produced `ui_locale`.
    pub source: LocaleSource,
    /// Raw OS string from `sys-locale` when consulted (owned; not always set).
    pub system_raw: Option<String>,
}

/// Available language identifiers for negotiation (static MVP packs).
///
/// Cached once — negotiate runs at boot / tests only (memory: no per-call `Vec` rebuild).
fn available_langids() -> &'static [LanguageIdentifier] {
    static AVAIL: OnceLock<Vec<LanguageIdentifier>> = OnceLock::new();
    AVAIL
        .get_or_init(|| {
            UiLocale::AVAILABLE
                .iter()
                .map(|i| i.language_identifier())
                .collect()
        })
        .as_slice()
}

/// Normalize raw OS / user locale strings into a [`LanguageIdentifier`].
///
/// Accepts `pt_BR.UTF-8`, `pt-BR`, `en_US.utf8`, etc.
pub fn parse_langid(raw: &str) -> Option<LanguageIdentifier> {
    let mut s = raw.trim().replace('_', "-");
    if s.is_empty() {
        return None;
    }
    // Drop encoding / modifier suffixes: `pt-BR.UTF-8@euro` → `pt-BR`
    if let Some(idx) = s.find(['.', '@']) {
        s.truncate(idx);
    }
    // Reject C / POSIX as user preference (rules: never treat as en-US synonym).
    let lower = s.to_ascii_lowercase();
    if lower == "c" || lower == "posix" {
        return None;
    }
    s.parse().ok()
}

/// Negotiate requested identifiers against compiled packs; always returns a pack.
///
/// `fluent-langneg` may map bare `pt` → available `pt-BR`. Product rules forbid
/// treating bare `pt` (or non-BR Portuguese regions) as a `pt-BR` substitute, so
/// we only keep the PtBr pack when a request explicitly carries region `BR`.
pub fn negotiate(requested: &[LanguageIdentifier]) -> UiLocale {
    let available = available_langids();
    let default = UiLocale::En.language_identifier();
    let matched = negotiate_languages(
        requested,
        available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );
    let ui = matched
        .first()
        .and_then(|id| UiLocale::from_langid(id))
        .or_else(|| requested.iter().find_map(UiLocale::from_langid))
        .unwrap_or(UiLocale::En);

    if ui == UiLocale::PtBr {
        let explicit_pt_br = requested.iter().any(|id| {
            id.language.as_str() == "pt" && id.region.as_ref().map(|r| r.as_str()) == Some("BR")
        });
        if !explicit_pt_br {
            return UiLocale::En;
        }
    }
    ui
}

/// Read OS locale once via `sys-locale` (never direct `LANG` reads in portable code).
pub fn detect_system_langid() -> Option<LanguageIdentifier> {
    let raw = sys_locale::get_locale()?;
    parse_langid(&raw)
}

/// Full 4-layer resolution (product-law: no product env vars):
/// 1. `--lang` flag
/// 2. XDG `lang` (`config set lang …`)
/// 3. OS via `sys-locale` + fluent-langneg
/// 4. default `en`
///
/// When the system layer is consulted, `system_raw` holds an **owned** copy of the
/// OS locale string for `locale` diagnostics (no process-lifetime leak).
pub fn resolve(cli_lang: Option<&str>, xdg_lang: Option<&str>) -> ResolvedLocale {
    // Layer 1 — flag
    if let Some(raw) = cli_lang.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(ui_locale) = UiLocale::parse_token(raw) {
            return ResolvedLocale {
                ui_locale,
                source: LocaleSource::Flag,
                system_raw: None,
            };
        }
        // Invalid flag value: fall through but do not panic (clap may pre-validate).
        tracing::warn!(
            value = raw,
            "invalid --lang value; continuing resolution chain"
        );
    }

    // Layer 2 — XDG persisted preference
    if let Some(raw) = xdg_lang.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(ui_locale) = UiLocale::parse_token(raw) {
            return ResolvedLocale {
                ui_locale,
                source: LocaleSource::Xdg,
                system_raw: None,
            };
        }
        tracing::warn!(value = raw, "invalid XDG lang; continuing resolution chain");
    }

    // Layer 3 — OS locale (sys-locale abstracts LC_ALL/LC_MESSAGES/LANG / Win32 / CF)
    match sys_locale::get_locale() {
        Some(raw) => {
            // Own the OS string once; never Box::leak for Copy convenience.
            if let Some(id) = parse_langid(&raw) {
                let ui_locale = negotiate(std::slice::from_ref(&id));
                return ResolvedLocale {
                    ui_locale,
                    source: LocaleSource::System,
                    system_raw: Some(raw),
                };
            }
            tracing::debug!(
                raw = %raw,
                "OS locale unparsable; falling back to default en"
            );
            ResolvedLocale {
                ui_locale: UiLocale::En,
                source: LocaleSource::Default,
                system_raw: Some(raw),
            }
        }
        None => {
            // Signal detection failure to local observability (no remote telemetry).
            tracing::debug!("sys-locale returned None; using default en");
            ResolvedLocale {
                ui_locale: UiLocale::En,
                source: LocaleSource::Default,
                system_raw: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strips_encoding_and_underscore() {
        let id = parse_langid("pt_BR.UTF-8").expect("parse");
        assert_eq!(id.language.as_str(), "pt");
        assert_eq!(id.region.as_ref().map(|r| r.as_str()), Some("BR"));
    }

    #[test]
    fn reject_c_locale() {
        assert!(parse_langid("C").is_none());
        assert!(parse_langid("POSIX").is_none());
    }

    #[test]
    fn negotiate_pt_br_prefers_pack() {
        let id: LanguageIdentifier = "pt-BR".parse().unwrap();
        assert_eq!(negotiate(&[id]), UiLocale::PtBr);
    }

    #[test]
    fn negotiate_unknown_falls_to_en() {
        let id: LanguageIdentifier = "ja-JP".parse().unwrap();
        assert_eq!(negotiate(&[id]), UiLocale::En);
    }

    #[test]
    fn bare_pt_system_does_not_map_to_pt_br_pack() {
        let id: LanguageIdentifier = "pt".parse().unwrap();
        assert!(UiLocale::from_langid(&id).is_none());
        assert_eq!(negotiate(&[id]), UiLocale::En);
    }

    #[test]
    fn flag_bare_pt_falls_through() {
        // Invalid bare `pt` is not Flag layer — continues chain (here XDG en).
        let r = resolve(Some("pt"), Some("en"));
        assert_eq!(r.ui_locale, UiLocale::En);
        assert_eq!(r.source, LocaleSource::Xdg);
    }

    #[test]
    fn flag_layer_wins() {
        let r = resolve(Some("pt-BR"), Some("en"));
        assert_eq!(r.ui_locale, UiLocale::PtBr);
        assert_eq!(r.source, LocaleSource::Flag);
    }

    #[test]
    fn xdg_layer_wins_without_flag() {
        let r = resolve(None, Some("pt-BR"));
        assert_eq!(r.ui_locale, UiLocale::PtBr);
        assert_eq!(r.source, LocaleSource::Xdg);
    }
}
