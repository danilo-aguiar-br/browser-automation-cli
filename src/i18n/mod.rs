// SPDX-License-Identifier: MIT OR Apache-2.0
//! Automatic multi-language UI for human-facing suggestions.
//!
//! # Language isolation (crates.io / agent contract)
//!
//! - **All identifiers, comments, and technical `message` fields are English.**
//! - **Portuguese appears only as catalog string literals** for human `suggestion`
//!   UI text when locale resolves to `pt-BR` (not in identifiers or logs).
//! - Agent-stable JSON `error.message` remains English regardless of locale.
//! - Prefer [`UiMessage`](crate::i18n::UiMessage) / [`suggestion_key`](crate::i18n::suggestion_key) over hardcoding UI strings at call sites.
//! - Stdout JSON envelopes are **not** translated (machine contract).
//!
//! # Boot order (multi-language UI rules)
//!
//! 1. Windows console UTF-8 ([`configure_console_utf8`](crate::i18n::configure_console_utf8))
//! 2. TTY / plain / screen-reader hints (see [`crate::color`])
//! 3. OS locale via `sys-locale` inside [`resolve_locale`](crate::i18n::resolve_locale)
//! 4. Parse → `LanguageIdentifier` (`unic-langid`)
//! 5. Negotiate against compiled packs (`fluent-langneg`)
//! 6. Publish in [`OnceLock`](std::sync::OnceLock) via [`set_effective_ui_locale`](crate::i18n::set_effective_ui_locale)
//!
//! # Precedence (4 layers — product-law: no product env vars)
//!
//! `--lang` → XDG `lang` (`config set lang`) → system (`sys-locale`) → default `en`

mod catalog_audit;
mod detect;
mod diagnostics;
mod en;
mod ftl;
mod pt_br;
mod suggestion;
mod ui_locale;
mod ui_message;

pub use detect::{
    detect_system_langid, negotiate, parse_langid, resolve, LocaleSource, ResolvedLocale,
};
pub use diagnostics::{locale_diagnostics, truncate_graphemes};
pub use ftl::{format_ftl, ftl_keys, ftl_source};
pub use suggestion::{localize_error_suggestion, suggestion_for, suggestion_key};
pub use ui_locale::{TextDirection, UiLocale, WritingScript};
pub use ui_message::{ftl_id, UiMessage};

use std::sync::OnceLock;

/// Process-wide effective UI locale, set once at CLI boot.
///
/// # Concurrency
///
/// `OnceLock` is `Sync`; first successful `set` wins. Concurrent readers see
/// either the initialized value or the default [`UiLocale::En`] via [`effective_ui_locale`].
static EFFECTIVE: OnceLock<ResolvedLocale> = OnceLock::new();

/// Configure Windows console to UTF-8 (and VT) before any user-facing I/O.
///
/// Delegates to [`crate::platform::configure_console`] (single multiplatform entry).
pub fn configure_console_utf8() {
    crate::platform::configure_console();
}

/// Resolve language from CLI flag, then XDG, OS locale (see [`resolve`]).
pub fn resolve_locale(cli_lang: Option<&str>) -> ResolvedLocale {
    let xdg = crate::xdg::load_config()
        .ok()
        .and_then(|c| c.lang)
        .filter(|s| !s.trim().is_empty());
    resolve(cli_lang, xdg.as_deref())
}

/// Validate a `config set lang` / flag token (`en` | `pt-BR` and regional `en-*`).
///
/// Rejects bare `pt` and unknown tags with a machine-English usage error.
pub fn validate_lang_token(raw: &str) -> Result<UiLocale, crate::error::CliError> {
    UiLocale::parse_token(raw).ok_or_else(|| {
        crate::error::CliError::with_suggestion(
            crate::error::ErrorKind::Usage,
            format!("invalid lang {raw:?}; expected en or pt-BR (bare pt is not accepted)"),
            "Use: config set lang en   or   config set lang pt-BR   or   --lang pt-BR",
        )
    })
}

/// Scan raw argv for `--lang <token>` or `--lang=<token>` (pre-clap parse path).
///
/// Used so clap usage errors can still localize human suggestions after OS/XDG resolve.
pub fn scan_lang_flag_from_argv(args: &[impl AsRef<std::ffi::OsStr>]) -> Option<String> {
    let mut iter = args.iter().map(|a| a.as_ref());
    while let Some(arg) = iter.next() {
        let s = arg.to_string_lossy();
        if s == "--lang" {
            return iter.next().map(|v| v.to_string_lossy().into_owned());
        }
        if let Some(rest) = s.strip_prefix("--lang=") {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Store effective locale for the process (call once from `run()`).
pub fn set_effective_ui_locale(resolved: ResolvedLocale) {
    // Clone fields needed for tracing before move into OnceLock (owned system_raw).
    let ui_locale = resolved.ui_locale;
    let source = resolved.source;
    let system = resolved.system_raw.clone();
    let _ = EFFECTIVE.set(resolved);
    if source == LocaleSource::Default && system.is_none() {
        // Detection failed or empty chain — local observability only.
        tracing::debug!(
            ui_locale = ui_locale.bcp47(),
            source = source.as_str(),
            "UI locale defaulted to en"
        );
    } else {
        tracing::debug!(
            ui_locale = ui_locale.bcp47(),
            source = source.as_str(),
            system = system.as_deref().unwrap_or(""),
            "UI locale resolved"
        );
    }
}

/// Current effective UI locale (defaults to `en` if unset).
pub fn effective_ui_locale() -> UiLocale {
    EFFECTIVE.get().map(|r| r.ui_locale).unwrap_or(UiLocale::En)
}

/// Full resolved snapshot (for `locale` subcommand).
pub fn effective_resolved() -> ResolvedLocale {
    EFFECTIVE.get().cloned().unwrap_or(ResolvedLocale {
        ui_locale: UiLocale::En,
        source: LocaleSource::Default,
        system_raw: None,
    })
}

// ── Compatibility API (legacy `&'static str` tokens) ─────────────────────

/// Normalize lang token to legacy `"en"` or `"pt"`.
pub fn normalize_lang(lang: Option<&str>) -> &'static str {
    match lang.and_then(UiLocale::parse_token) {
        Some(UiLocale::PtBr) => "pt",
        _ => "en",
    }
}

/// Resolve language from CLI flag, then XDG config, then OS locale hints.
///
/// Returns legacy `"en"` / `"pt"` tokens for older call sites.
pub fn resolve_lang(cli_lang: Option<&str>) -> &'static str {
    resolve_locale(cli_lang).ui_locale.legacy_token()
}

/// Store effective language for the process (call once from `run()`).
///
/// Accepts `"en"` / `"pt-BR"` / BCP47 tokens (`parse_token`). Bare `"pt"` is rejected → `en`.
pub fn set_effective_lang(lang: &'static str) {
    let ui_locale = UiLocale::parse_token(lang).unwrap_or(UiLocale::En);
    set_effective_ui_locale(ResolvedLocale {
        ui_locale,
        source: LocaleSource::Flag,
        system_raw: None,
    });
}

/// Current effective language legacy token (defaults to `en` if unset).
pub fn effective_lang() -> &'static str {
    effective_ui_locale().legacy_token()
}

#[cfg(test)]
mod tests;
