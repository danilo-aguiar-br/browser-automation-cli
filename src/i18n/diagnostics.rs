// SPDX-License-Identifier: MIT OR Apache-2.0
//! `locale` subcommand payload and terminal-safe truncation.

use super::{effective_resolved, TextDirection, UiLocale};

/// JSON-ready diagnostics for `locale` subcommand (machine keys English).
pub fn locale_diagnostics() -> serde_json::Value {
    let r = effective_resolved();
    // Prefer the system string captured at resolve (one-shot once); fall back to a
    // live probe only when resolve never consulted the OS layer.
    let sys = r.system_raw.clone().or_else(sys_locale::get_locale);
    serde_json::json!({
        "resolved": r.ui_locale.bcp47(),
        "legacy": r.ui_locale.legacy_token(),
        "source": r.source.as_str(),
        "direction": match r.ui_locale.direction() {
            TextDirection::Ltr => "ltr",
            TextDirection::Rtl => "rtl",
        },
        "script": format!("{:?}", r.ui_locale.script()).to_ascii_lowercase(),
        "available": UiLocale::AVAILABLE.iter().map(|i| i.bcp47()).collect::<Vec<_>>(),
        "system_locale": sys,
        "resolution": ["flag", "xdg", "system", "default"],
        "product_note": "error.message and stdout JSON stay English; suggestions localize; no product env vars (use --lang or config set lang pt-BR)",
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
