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
    // The English suggestion is returned untouched, and that is now correct.
    //
    // # Why the string-keyed remap is gone
    //
    // This function used to end in a `match` over twelve LITERAL English
    // suggestion strings, remapping each to a `UiMessage`. Measured 2026-08-31:
    // every one of those twelve strings occurred exactly TWICE in the tree, and
    // both occurrences were inside `src/i18n/` — the arm itself and the English
    // text table. ZERO sites in the product emitted any of them, because all
    // 346 call sites had already migrated to `suggestion_key`, which resolves
    // by stable key and localizes at CONSTRUCTION time. The two
    // `contains("lighthouse")` guards were dead for the same reason:
    // `commands/ops/lighthouse/` passes `suggestion_key("lighthouse_missing")`.
    //
    // Dead code that appears to work is worse than an obvious gap: it reads as
    // proof that passing a raw English string is safe, when such a string would
    // in fact fall through the `_` arm and reach the user untranslated. Keeping
    // the arm would also have coupled every translation to the exact wording of
    // an English sentence, so editing one word silently dropped the locale.
    //
    // If a raw English suggestion ever needs localizing again, the fix is to
    // give that site a `suggestion_key`, not to re-add a string match here.
    err.clone()
}
