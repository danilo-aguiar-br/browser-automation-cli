//! Minimal en/pt-BR message helpers for critical errors.

/// Normalize lang token to "en" or "pt".
pub fn normalize_lang(lang: Option<&str>) -> &'static str {
    match lang.map(|s| s.trim().to_ascii_lowercase()) {
        Some(ref s) if s.starts_with("pt") => "pt",
        _ => "en",
    }
}

/// Localized suggestion for a known kind key.
pub fn suggestion_for(kind: &str, lang: Option<&str>) -> Option<&'static str> {
    let l = normalize_lang(lang);
    match (kind, l) {
        ("usage", "pt") => Some("Confira --help e os argumentos obrigatorios"),
        ("usage", _) => Some("Check --help and required arguments"),
        ("broken-pipe", "pt") => {
            Some("Nao pipe stdout para consumidor fechado; exit 141 e esperado")
        }
        ("broken-pipe", _) => Some("Do not pipe stdout to a closed consumer; exit 141 is expected"),
        ("unavailable", "pt") => Some("Instale Chrome/Chromium no PATH ou configure o executavel"),
        ("unavailable", _) => Some("Install Chrome/Chromium on PATH or set the executable"),
        ("data", "pt") => Some("Verifique robots.txt ou o payload JSON/NDJSON"),
        ("data", _) => Some("Check robots.txt or the JSON/NDJSON payload"),
        ("browser", "pt") => Some("Verifique URL e se o Chrome ainda esta vivo no one-shot"),
        ("browser", _) => Some("Check the URL and whether Chrome stayed alive in this one-shot"),
        _ => None,
    }
}
