// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-platform locale detection + negotiation (single boot path).

use fluent_langneg::negotiate::NegotiationStrategy;
use fluent_langneg::negotiate_languages;
use unic_langid::LanguageIdentifier;

use super::idioma::Idioma;

/// Product env override: `BROWSER_AUTOMATION_CLI_LANG` (rules: `<CRATE_UPPER>_LANG`).
pub const LANG_ENV: &str = "BROWSER_AUTOMATION_CLI_LANG";

/// Where the effective UI locale came from (diagnostics / `locale` subcommand).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocaleSource {
    /// Global `--lang` argv flag.
    Flag,
    /// `BROWSER_AUTOMATION_CLI_LANG` process environment.
    Env,
    /// XDG config `lang` key.
    Xdg,
    /// OS locale via `sys-locale` + negotiation.
    System,
    /// Hard fallback (`en`) when nothing else matched.
    Default,
}

impl LocaleSource {
    /// Stable machine token for JSON diagnostics (`flag` / `env` / `xdg` / `system` / `default`).
    pub const fn as_str(self) -> &'static str {
        match self {
            LocaleSource::Flag => "flag",
            LocaleSource::Env => "env",
            LocaleSource::Xdg => "xdg",
            LocaleSource::System => "system",
            LocaleSource::Default => "default",
        }
    }
}

/// Result of the 5-layer resolution chain.
///
/// # Ownership
///
/// Owned fields only — never `Box::leak` / artificial `'static` for diagnostics
/// (rules_rust_ownership: `'static` only for true program-lifetime data).
/// `idioma` / `source` are `Copy`; `system_raw` is an owned `String` when present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLocale {
    /// Compiled UI pack selected for human suggestions.
    pub idioma: Idioma,
    /// Which precedence layer produced `idioma`.
    pub source: LocaleSource,
    /// Raw OS string from `sys-locale` when consulted (owned; not always set).
    pub system_raw: Option<String>,
}

/// Available language identifiers for negotiation (static MVP packs).
fn available_langids() -> Vec<LanguageIdentifier> {
    Idioma::DISPONIVEIS
        .iter()
        .map(|i| i.language_identifier())
        .collect()
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
pub fn negotiate(requested: &[LanguageIdentifier]) -> Idioma {
    let available = available_langids();
    let default = Idioma::En.language_identifier();
    let matched = negotiate_languages(
        requested,
        &available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );
    matched
        .first()
        .and_then(|id| Idioma::from_langid(id))
        .or_else(|| {
            // Language-only fallback (e.g. requested pt-PT → no pack → try language pt → None
            // then default en; requested pt-BR → pack).
            requested.iter().find_map(Idioma::from_langid)
        })
        .unwrap_or(Idioma::En)
}

/// Read OS locale once via `sys-locale` (never direct `LANG` reads in portable code).
pub fn detect_system_langid() -> Option<LanguageIdentifier> {
    let raw = sys_locale::get_locale()?;
    parse_langid(&raw)
}

/// Full 5-layer resolution:
/// 1. `--lang` flag
/// 2. `BROWSER_AUTOMATION_CLI_LANG`
/// 3. XDG `lang`
/// 4. OS via `sys-locale` + fluent-langneg
/// 5. default `en`
///
/// When the system layer is consulted, `system_raw` holds an **owned** copy of the
/// OS locale string for `locale` diagnostics (no process-lifetime leak).
pub fn resolve(
    cli_lang: Option<&str>,
    xdg_lang: Option<&str>,
) -> ResolvedLocale {
    // Layer 1 — flag
    if let Some(raw) = cli_lang.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(idioma) = Idioma::parse_token(raw) {
            return ResolvedLocale {
                idioma,
                source: LocaleSource::Flag,
                system_raw: None,
            };
        }
        // Invalid flag value: fall through but do not panic (clap may pre-validate).
        tracing::warn!(value = raw, "invalid --lang value; continuing resolution chain");
    }

    // Layer 2 — product lang env (explicitly allowed for i18n; not general config).
    if let Ok(raw) = std::env::var(LANG_ENV) {
        let t = raw.trim();
        if !t.is_empty() {
            if let Some(idioma) = Idioma::parse_token(t) {
                return ResolvedLocale {
                    idioma,
                    source: LocaleSource::Env,
                    system_raw: None,
                };
            }
            tracing::warn!(
                env = LANG_ENV,
                value = t,
                "invalid BROWSER_AUTOMATION_CLI_LANG; continuing resolution chain"
            );
        }
    }

    // Layer 3 — XDG persisted preference
    if let Some(raw) = xdg_lang.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(idioma) = Idioma::parse_token(raw) {
            return ResolvedLocale {
                idioma,
                source: LocaleSource::Xdg,
                system_raw: None,
            };
        }
        tracing::warn!(value = raw, "invalid XDG lang; continuing resolution chain");
    }

    // Layer 4 — OS locale (sys-locale abstracts LC_ALL/LC_MESSAGES/LANG / Win32 / CF)
    match sys_locale::get_locale() {
        Some(raw) => {
            // Own the OS string once; never Box::leak for Copy convenience.
            if let Some(id) = parse_langid(&raw) {
                let idioma = negotiate(std::slice::from_ref(&id));
                // If OS said something we could not map better than default and raw was C, still default.
                return ResolvedLocale {
                    idioma,
                    source: LocaleSource::System,
                    system_raw: Some(raw),
                };
            }
            tracing::debug!(
                raw = %raw,
                "OS locale unparsable; falling back to default en"
            );
            ResolvedLocale {
                idioma: Idioma::En,
                source: LocaleSource::Default,
                system_raw: Some(raw),
            }
        }
        None => {
            // Signal detection failure to local observability (no remote telemetry).
            tracing::debug!("sys-locale returned None; using default en");
            ResolvedLocale {
                idioma: Idioma::En,
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
        assert_eq!(negotiate(&[id]), Idioma::PtBr);
    }

    #[test]
    fn negotiate_unknown_falls_to_en() {
        let id: LanguageIdentifier = "ja-JP".parse().unwrap();
        assert_eq!(negotiate(&[id]), Idioma::En);
    }

    #[test]
    fn flag_layer_wins() {
        let r = resolve(Some("pt-BR"), Some("en"));
        assert_eq!(r.idioma, Idioma::PtBr);
        assert_eq!(r.source, LocaleSource::Flag);
    }
}
