// SPDX-License-Identifier: MIT OR Apache-2.0
//! Source audit of `CliError::with_suggestion` call sites (GAP-047 gate).
//!
//! # Why a source audit
//!
//! A catalog-coverage check that only inspects `suggestion_key("…")` call sites
//! is blind to the failure it exists to prevent: a hand-written English literal
//! passed straight to `with_suggestion`. This module parses the real argument
//! list so the gate can see literals, not just catalog keys.
//!
//! # Checks
//!
//! | Check | Rule |
//! |-------|------|
//! | duplicate | A literal byte-equal to an `en.ftl` value must use the key |
//! | ratchet | The literal count must not exceed [`LITERAL_BASELINE`] |
//!
//! The ratchet is a floor, not an endorsement: it stops the debt from growing
//! while the remaining literals are migrated family by family.

/// Maximum number of raw-literal suggestion sites tolerated in `src/`.
///
/// Lower this number when literals are migrated to catalog keys; never raise it.
#[cfg(test)]
pub(crate) const LITERAL_BASELINE: usize = 45;

/// One classified `with_suggestion` call site.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct SuggestionSite {
    /// Path relative to the crate root.
    pub file: String,
    /// 1-based line of the `with_suggestion(` token.
    pub line: usize,
    /// Third argument, source text with newlines collapsed.
    pub argument: String,
    /// `catalog` | `literal` | `format` | `expr`.
    pub kind: &'static str,
    /// Decoded text when `kind == "literal"`.
    pub literal: Option<String>,
}

#[cfg(test)]
mod scan {
    use super::SuggestionSite;

    /// Split a Rust argument list on top-level commas (string- and nest-aware).
    fn split_args(body: &str) -> Vec<String> {
        let mut args = Vec::new();
        let (mut depth, mut cur, mut in_str, mut esc) = (0i32, String::new(), false, false);
        for c in body.chars() {
            if in_str {
                cur.push(c);
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                }
                continue;
            }
            match c {
                '"' => {
                    in_str = true;
                    cur.push(c);
                }
                '(' | '[' | '{' => {
                    depth += 1;
                    cur.push(c);
                }
                ')' | ']' | '}' => {
                    depth -= 1;
                    cur.push(c);
                }
                ',' if depth == 0 => {
                    args.push(cur.trim().to_string());
                    cur.clear();
                }
                _ => cur.push(c),
            }
        }
        if !cur.trim().is_empty() {
            args.push(cur.trim().to_string());
        }
        args
    }

    /// Decode a Rust string literal token into its text value.
    fn decode_literal(token: &str) -> Option<String> {
        let inner = token.strip_prefix('"')?.strip_suffix('"')?;
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => break,
            }
        }
        Some(out)
    }

    /// Find the end of the call body starting just after `with_suggestion(`.
    fn call_body_end(text: &str, start: usize) -> usize {
        let bytes: Vec<char> = text.chars().collect();
        let (mut depth, mut i, mut in_str, mut esc) = (1i32, start, false, false);
        while i < bytes.len() && depth > 0 {
            let c = bytes[i];
            if in_str {
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                }
            } else if c == '"' {
                in_str = true;
            } else if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }
            i += 1;
        }
        i.saturating_sub(1)
    }

    /// Classify every `with_suggestion` call site under `src/`.
    pub(crate) fn scan_sources() -> Vec<SuggestionSite> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_rs(&root, &mut files);
        files.sort();
        let mut out = Vec::new();
        for path in files {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let chars: Vec<char> = text.chars().collect();
            let rel = path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap_or(&path)
                .display()
                .to_string();
            let needle: Vec<char> = "with_suggestion(".chars().collect();
            let mut i = 0usize;
            while i + needle.len() <= chars.len() {
                if chars[i..i + needle.len()] != needle[..] {
                    i += 1;
                    continue;
                }
                let start = i + needle.len();
                let end = call_body_end(&text, start);
                let body: String = chars[start..end.min(chars.len())].iter().collect();
                let args = split_args(&body);
                let line = chars[..i].iter().filter(|c| **c == '\n').count() + 1;
                let third = args.get(2).cloned().unwrap_or_default();
                let flat = third.replace('\n', " ");
                let trimmed = flat.trim();
                let (kind, literal) = if trimmed.contains("suggestion_key") {
                    ("catalog", None)
                } else if trimmed.starts_with("format!") || trimmed.starts_with("&format!") {
                    ("format", None)
                } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
                    ("literal", decode_literal(trimmed))
                } else {
                    ("expr", None)
                };
                out.push(SuggestionSite {
                    file: rel.clone(),
                    line,
                    argument: trimmed.chars().take(120).collect(),
                    kind,
                    literal,
                });
                i = end.max(i + 1);
            }
        }
        out
    }

    fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scan::scan_sources;
    use super::LITERAL_BASELINE;
    use crate::i18n::{ftl_source, UiLocale, UiMessage};

    /// English catalog values, used to spot hand-copied catalog text.
    fn english_catalog_values() -> Vec<String> {
        let mut values: Vec<String> = UiMessage::ALL
            .iter()
            .map(|m| m.text(UiLocale::En).to_string())
            .collect();
        for line in ftl_source(UiLocale::En).lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if let Some((_, value)) = t.split_once('=') {
                let value = value.trim();
                if !value.is_empty() {
                    values.push(value.to_string());
                }
            }
        }
        values
    }

    #[test]
    fn scan_finds_the_suggestion_call_sites() {
        let sites = scan_sources();
        assert!(
            sites.len() > 150,
            "scanner found only {} sites; parser is broken",
            sites.len()
        );
        assert!(sites.iter().any(|s| s.kind == "catalog"));
    }

    #[test]
    fn no_literal_suggestion_duplicates_the_catalog() {
        let catalog = english_catalog_values();
        let offenders: Vec<String> = scan_sources()
            .into_iter()
            .filter(|s| s.kind == "literal")
            .filter(|s| {
                s.literal
                    .as_ref()
                    .map(|l| catalog.iter().any(|c| c == l))
                    .unwrap_or(false)
            })
            .map(|s| format!("{}:{} {}", s.file, s.line, s.argument))
            .collect();
        assert!(
            offenders.is_empty(),
            "these sites hand-copied catalog text instead of calling suggestion_key:\n{}",
            offenders.join("\n")
        );
    }

    #[test]
    fn literal_suggestion_count_does_not_grow() {
        let literals: Vec<String> = scan_sources()
            .into_iter()
            .filter(|s| s.kind == "literal")
            .map(|s| format!("{}:{} {}", s.file, s.line, s.argument))
            .collect();
        assert!(
            literals.len() <= LITERAL_BASELINE,
            "raw-literal suggestions grew to {} (baseline {LITERAL_BASELINE}); \
             route new suggestions through crate::i18n::suggestion_key:\n{}",
            literals.len(),
            literals.join("\n")
        );
    }
}
