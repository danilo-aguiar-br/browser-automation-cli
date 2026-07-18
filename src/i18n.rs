//! Minimal en/pt-BR message helpers for critical errors.
//!
//! Technical `message` fields stay in English (agent-stable).
//! Human `suggestion` strings are localized via catalog keys or kind fallback.

use std::sync::OnceLock;

/// Process-wide effective language (`en` or `pt`), set once at CLI boot.
static EFFECTIVE_LANG: OnceLock<&'static str> = OnceLock::new();

/// Normalize lang token to "en" or "pt".
pub fn normalize_lang(lang: Option<&str>) -> &'static str {
    match lang.map(|s| s.trim().to_ascii_lowercase()) {
        Some(ref s) if s.starts_with("pt") => "pt",
        _ => "en",
    }
}

/// Resolve language from CLI flag, then XDG config, then OS locale hints.
pub fn resolve_lang(cli_lang: Option<&str>) -> &'static str {
    if let Some(l) = cli_lang {
        if !l.trim().is_empty() {
            return normalize_lang(Some(l));
        }
    }
    if let Ok(cfg) = crate::xdg::load_config() {
        if let Some(ref l) = cfg.lang {
            if !l.trim().is_empty() {
                return normalize_lang(Some(l));
            }
        }
    }
    // OS locale without product env vars: read glibc/setlocale-style via std only if available.
    // Prefer directories / LANG is OS concern — use libc setlocale is heavy; fall back to en.
    // On Unix, try reading /etc/locale.conf is overkill; use `sys_locale` free path:
    #[cfg(unix)]
    {
        // Minimal: inspect POSIX locale without treating as product config.
        // SAFETY: reading environment for OS locale is an OS concern (not product settings).
        // Plan forbids product env; OS locale detection is explicitly allowed as platform.
        if let Ok(v) = std::env::var("LANG") {
            if v.to_ascii_lowercase().starts_with("pt") {
                return "pt";
            }
        }
        if let Ok(v) = std::env::var("LC_ALL") {
            if v.to_ascii_lowercase().starts_with("pt") {
                return "pt";
            }
        }
        if let Ok(v) = std::env::var("LC_MESSAGES") {
            if v.to_ascii_lowercase().starts_with("pt") {
                return "pt";
            }
        }
    }
    "en"
}

/// Store effective language for the process (call once from `run()`).
pub fn set_effective_lang(lang: &'static str) {
    let _ = EFFECTIVE_LANG.set(lang);
}

/// Current effective language (defaults to `en` if unset).
pub fn effective_lang() -> &'static str {
    EFFECTIVE_LANG.get().copied().unwrap_or("en")
}

/// Localized suggestion for a known kind key.
pub fn suggestion_for(kind: &str, lang: Option<&str>) -> Option<&'static str> {
    let l = normalize_lang(lang.or(Some(effective_lang())));
    match (kind, l) {
        ("usage", "pt") => Some("Confira --help e os argumentos obrigatórios"),
        ("usage", _) => Some("Check --help and required arguments"),
        ("broken-pipe", "pt") => {
            Some("Nao pipe stdout para consumidor fechado; exit 141 e esperado")
        }
        ("broken-pipe", _) => Some("Do not pipe stdout to a closed consumer; exit 141 is expected"),
        ("unavailable", "pt") => {
            Some("Instale Chrome/Chromium no PATH ou use: browser-automation-cli config set chrome_path <path>")
        }
        ("unavailable", _) => {
            Some("Install Chrome/Chromium on PATH or: browser-automation-cli config set chrome_path <path>")
        }
        ("data", "pt") => Some("Verifique robots.txt ou o payload JSON/NDJSON"),
        ("data", _) => Some("Check robots.txt or the JSON/NDJSON payload"),
        ("browser", "pt") => Some("Verifique URL e se o Chrome ainda esta vivo no one-shot"),
        ("browser", _) => Some("Check the URL and whether Chrome stayed alive in this one-shot"),
        _ => None,
    }
}

/// Catalog of stable suggestion keys (preferred over hard-coded English).
pub fn suggestion_key(key: &str, lang: Option<&str>) -> &'static str {
    let l = normalize_lang(lang.or(Some(effective_lang())));
    match (key, l) {
        ("vision_required", "pt") => "Passe --experimental-vision na mesma invocação",
        ("vision_required", _) => "Pass --experimental-vision on the same invocation",
        ("robots_dual", "pt") => {
            "Passe as duas flags juntas quando ignorar robots.txt de propósito"
        }
        ("robots_dual", _) => "Pass both flags together when you intentionally skip robots.txt",
        ("category_memory", "pt") => {
            "Passe --category-memory (heap take/summary/close funcionam sem ops de grafo profundo)"
        }
        ("category_memory", _) => {
            "Pass --category-memory (heap take/summary/close work without deep graph ops)"
        }
        ("category_extensions", "pt") => "Passe --category-extensions na mesma invocação",
        ("category_extensions", _) => "Pass --category-extensions on the same invocation",
        ("screencast_flag", "pt") => "Passe --experimental-screencast na mesma invocação",
        ("screencast_flag", _) => "Pass --experimental-screencast on the same invocation",
        ("webmcp_flag", "pt") => "Passe --category-webmcp na mesma invocação",
        ("webmcp_flag", _) => "Pass --category-webmcp on the same invocation",
        ("third_party_flag", "pt") => "Passe --category-third-party na mesma invocação",
        ("third_party_flag", _) => "Pass --category-third-party on the same invocation",
        ("capture_network", "pt") => "Passe --capture-network antes de run/net",
        ("capture_network", _) => "Pass --capture-network before run/net",
        ("capture_console", "pt") => "Passe --capture-console antes de run/console",
        ("capture_console", _) => "Pass --capture-console before run/console",
        ("run_fail_fast", "pt") => {
            "Corrija o passo com falha; os passos seguintes não foram executados"
        }
        ("run_fail_fast", _) => "Fix the failing step; subsequent steps were not executed",
        ("lighthouse_missing", "pt") => {
            "Instale lighthouse ou: browser-automation-cli config set lighthouse_path <path>"
        }
        ("lighthouse_missing", _) => {
            "Install lighthouse or: browser-automation-cli config set lighthouse_path <path>"
        }
        (_, "pt") => "Confira --help e os argumentos obrigatórios",
        _ => "Check --help and required arguments",
    }
}

/// Apply kind-based localized suggestion when none is set, or re-map known EN strings.
pub fn localize_error_suggestion(err: &crate::error::CliError) -> crate::error::CliError {
    let lang = effective_lang();
    if lang == "en" {
        return err.clone();
    }
    // Prefer catalog by kind when suggestion is missing.
    if err.suggestion().is_none() {
        if let Some(s) = suggestion_for(err.kind().as_str(), Some(lang)) {
            let mut out = crate::error::CliError::with_suggestion(err.kind(), err.message(), s);
            if let Some(d) = err.data() {
                out = out.with_data(d.clone());
            }
            return out;
        }
        return err.clone();
    }
    let s = err.suggestion().unwrap_or("");
    // Map common English suggestions to Portuguese.
    let mapped = match s {
        "Pass --experimental-vision on the same invocation" => {
            suggestion_key("vision_required", Some(lang))
        }
        "Pass both flags together when you intentionally skip robots.txt" => {
            suggestion_key("robots_dual", Some(lang))
        }
        "Pass --category-memory (heap take/summary/close work without deep graph ops)" => {
            suggestion_key("category_memory", Some(lang))
        }
        "Pass --category-extensions on the same invocation" => {
            suggestion_key("category_extensions", Some(lang))
        }
        "Pass --experimental-screencast on the same invocation" => {
            suggestion_key("screencast_flag", Some(lang))
        }
        "Pass --category-webmcp on the same invocation" => {
            suggestion_key("webmcp_flag", Some(lang))
        }
        "Pass --category-third-party on the same invocation" => {
            suggestion_key("third_party_flag", Some(lang))
        }
        "Pass --capture-network before run/net" => suggestion_key("capture_network", Some(lang)),
        "Pass --capture-console before run/console" => {
            suggestion_key("capture_console", Some(lang))
        }
        "Fix the failing step; subsequent steps were not executed" => {
            suggestion_key("run_fail_fast", Some(lang))
        }
        other if other.contains("lighthouse") && other.contains("npm") => {
            suggestion_key("lighthouse_missing", Some(lang))
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
