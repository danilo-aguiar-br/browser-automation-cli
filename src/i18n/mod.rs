// SPDX-License-Identifier: MIT OR Apache-2.0
//! Automatic multi-language UI for human-facing suggestions.
//!
//! # Language isolation (crates.io / agent contract)
//!
//! - **All identifiers, comments, and technical `message` fields are English.**
//! - **Portuguese appears only as catalog string literals** for human `suggestion`
//!   UI text when locale resolves to `pt-BR` (not in identifiers or logs).
//! - Agent-stable JSON `error.message` remains English regardless of locale.
//! - Prefer [`Mensagem`] / [`suggestion_key`] over hardcoding UI strings at call sites.
//! - Stdout JSON envelopes are **not** translated (machine contract).
//!
//! # Boot order (rules multi-idioma)
//!
//! 1. Windows console UTF-8 ([`configure_console_utf8`])
//! 2. TTY / plain / screen-reader hints (see [`crate::color`])
//! 3. OS locale via `sys-locale` inside [`resolve_locale`]
//! 4. Parse → `LanguageIdentifier` (`unic-langid`)
//! 5. Negotiate against compiled packs (`fluent-langneg`)
//! 6. Publish in [`OnceLock`] via [`set_effective_idioma`]
//!
//! # Precedence (5 layers)
//!
//! `--lang` → `BROWSER_AUTOMATION_CLI_LANG` → XDG `lang` → system → default `en`

mod detect;
mod en;
mod ftl;
mod idioma;
mod mensagem;
mod pt_br;

pub use detect::{
    detect_system_langid, negotiate, parse_langid, resolve, LocaleSource, ResolvedLocale, LANG_ENV,
};
pub use ftl::{format_ftl, ftl_keys, ftl_source};
pub use idioma::{Direcao, Idioma, ScriptEscrita};
pub use mensagem::{ftl_id, Mensagem};

use std::sync::OnceLock;

/// Process-wide effective UI locale, set once at CLI boot.
///
/// # Concurrency
///
/// `OnceLock` is `Sync`; first successful `set` wins. Concurrent readers see
/// either the initialized value or the default [`Idioma::En`] via [`effective_idioma`].
static EFFECTIVE: OnceLock<ResolvedLocale> = OnceLock::new();

/// Configure Windows console to UTF-8 (and VT) before any user-facing I/O.
///
/// Delegates to [`crate::platform::configure_console`] (single multiplatform entry).
pub fn configure_console_utf8() {
    crate::platform::configure_console();
}

/// Resolve language from CLI flag, then env, XDG, OS locale (see [`resolve`]).
pub fn resolve_locale(cli_lang: Option<&str>) -> ResolvedLocale {
    let xdg = crate::xdg::load_config()
        .ok()
        .and_then(|c| c.lang)
        .filter(|s| !s.trim().is_empty());
    resolve(cli_lang, xdg.as_deref())
}

/// Store effective locale for the process (call once from `run()`).
pub fn set_effective_idioma(resolved: ResolvedLocale) {
    // Clone fields needed for tracing before move into OnceLock (owned system_raw).
    let idioma = resolved.idioma;
    let source = resolved.source;
    let system = resolved.system_raw.clone();
    let _ = EFFECTIVE.set(resolved);
    if source == LocaleSource::Default && system.is_none() {
        // Detection failed or empty chain — local observability only.
        tracing::debug!(
            idioma = idioma.bcp47(),
            source = source.as_str(),
            "UI locale defaulted to en"
        );
    } else {
        tracing::debug!(
            idioma = idioma.bcp47(),
            source = source.as_str(),
            system = system.as_deref().unwrap_or(""),
            "UI locale resolved"
        );
    }
}

/// Current effective idioma (defaults to `en` if unset).
pub fn effective_idioma() -> Idioma {
    EFFECTIVE
        .get()
        .map(|r| r.idioma)
        .unwrap_or(Idioma::En)
}

/// Full resolved snapshot (for `locale` subcommand).
pub fn effective_resolved() -> ResolvedLocale {
    EFFECTIVE.get().cloned().unwrap_or(ResolvedLocale {
        idioma: Idioma::En,
        source: LocaleSource::Default,
        system_raw: None,
    })
}

// ── Compatibility API (legacy `&'static str` tokens) ─────────────────────

/// Normalize lang token to legacy `"en"` or `"pt"`.
pub fn normalize_lang(lang: Option<&str>) -> &'static str {
    match lang.and_then(Idioma::parse_token) {
        Some(Idioma::PtBr) => "pt",
        _ => "en",
    }
}

/// Resolve language from CLI flag, then XDG config, then OS locale hints.
///
/// Returns legacy `"en"` / `"pt"` tokens for older call sites.
pub fn resolve_lang(cli_lang: Option<&str>) -> &'static str {
    resolve_locale(cli_lang).idioma.legacy_token()
}

/// Store effective language for the process (call once from `run()`).
///
/// Accepts legacy `"en"` / `"pt"` / BCP47 tokens.
pub fn set_effective_lang(lang: &'static str) {
    let idioma = Idioma::parse_token(lang).unwrap_or(Idioma::En);
    set_effective_idioma(ResolvedLocale {
        idioma,
        source: LocaleSource::Flag,
        system_raw: None,
    });
}

/// Current effective language legacy token (defaults to `en` if unset).
pub fn effective_lang() -> &'static str {
    effective_idioma().legacy_token()
}

/// Localized suggestion for a known kind key.
pub fn suggestion_for(kind: &str, lang: Option<&str>) -> Option<&'static str> {
    let idioma = lang
        .and_then(Idioma::parse_token)
        .unwrap_or_else(effective_idioma);
    Mensagem::from_error_kind(kind).map(|m| m.texto(idioma))
}

/// Catalog of stable suggestion keys (preferred over hard-coded English).
pub fn suggestion_key(key: &str, lang: Option<&str>) -> &'static str {
    let idioma = lang
        .and_then(Idioma::parse_token)
        .unwrap_or_else(effective_idioma);
    Mensagem::from_suggestion_key(key).texto(idioma)
}

/// Apply kind-based localized suggestion when none is set, or re-map known EN strings.
pub fn localize_error_suggestion(err: &crate::error::CliError) -> crate::error::CliError {
    let idioma = effective_idioma();
    if idioma == Idioma::En {
        return err.clone();
    }
    // Prefer catalog by kind when suggestion is missing.
    if err.suggestion().is_none() {
        if let Some(s) = suggestion_for(err.kind().as_str(), Some(idioma.legacy_token())) {
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
            Mensagem::VisionRequired.texto(idioma)
        }
        "Pass both flags together when you intentionally skip robots.txt" => {
            Mensagem::RobotsDual.texto(idioma)
        }
        "Pass --category-memory (heap take/summary/close work without deep graph ops)" => {
            Mensagem::CategoryMemory.texto(idioma)
        }
        "Pass --category-extensions on the same invocation" => {
            Mensagem::CategoryExtensions.texto(idioma)
        }
        "Pass --experimental-screencast on the same invocation" => {
            Mensagem::ScreencastFlag.texto(idioma)
        }
        "Pass --category-webmcp on the same invocation" => Mensagem::WebmcpFlag.texto(idioma),
        "Pass --category-third-party on the same invocation" => {
            Mensagem::ThirdPartyFlag.texto(idioma)
        }
        "Pass --capture-network before run/net" => Mensagem::CaptureNetwork.texto(idioma),
        "Pass --capture-console before run/console" => Mensagem::CaptureConsole.texto(idioma),
        "Fix the failing step; subsequent steps were not executed" => {
            Mensagem::RunFailFast.texto(idioma)
        }
        other if other.contains("lighthouse") && other.contains("npm") => {
            Mensagem::LighthouseMissing.texto(idioma)
        }
        other if other.contains("lighthouse") && other.contains("config set") => {
            Mensagem::LighthouseMissing.texto(idioma)
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

/// JSON-ready diagnostics for `locale` subcommand (machine keys English).
pub fn locale_diagnostics() -> serde_json::Value {
    let r = effective_resolved();
    let sys = sys_locale::get_locale();
    serde_json::json!({
        "resolved": r.idioma.bcp47(),
        "legacy": r.idioma.legacy_token(),
        "source": r.source.as_str(),
        "direction": match r.idioma.direcao() {
            Direcao::Ltr => "ltr",
            Direcao::Rtl => "rtl",
        },
        "script": format!("{:?}", r.idioma.script()).to_ascii_lowercase(),
        "available": Idioma::DISPONIVEIS.iter().map(|i| i.bcp47()).collect::<Vec<_>>(),
        "system_locale": sys,
        "env_override": std::env::var(LANG_ENV).ok(),
        "lang_env_key": LANG_ENV,
        "product_note": "error.message and stdout JSON stay English; suggestions localize",
    })
}

/// Grapheme-aware truncation for terminal width (CJK-safe boundary).
pub fn truncate_graphemes(s: &str, max: usize) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    for (i, g) in s.graphemes(true).enumerate() {
        if i >= max {
            break;
        }
        out.push_str(g);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_mensagem_non_empty_both_locales() {
        for m in Mensagem::ALL {
            let en = m.texto(Idioma::En);
            let pt = m.texto(Idioma::PtBr);
            assert!(!en.is_empty(), "empty en for {m:?}");
            assert!(!pt.is_empty(), "empty pt-BR for {m:?}");
            assert_ne!(en, "", "{m:?}");
        }
    }

    #[test]
    fn pt_br_has_accents_on_critical_keys() {
        assert!(Mensagem::VisionRequired.texto(Idioma::PtBr).contains("invocação"));
        assert!(Mensagem::RobotsDual.texto(Idioma::PtBr).contains("propósito"));
        assert!(Mensagem::RunFailFast.texto(Idioma::PtBr).contains("não"));
        assert!(Mensagem::UsageSuggestion.texto(Idioma::PtBr).contains("obrigatórios"));
    }

    #[test]
    fn texto_api_no_global_required() {
        // Tests must not depend on process OnceLock.
        assert_eq!(
            Mensagem::UsageSuggestion.texto(Idioma::En),
            "Check --help and required arguments"
        );
    }

    #[test]
    fn truncate_respects_graphemes() {
        let s = "ação";
        assert_eq!(truncate_graphemes(s, 2), "aç");
        assert_eq!(truncate_graphemes(s, 10), "ação");
    }
}
