// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bracket scanning and unquoting for the locator wire form.

/// Read a bracket body up to its unescaped `]`, returning the rest after it.
pub(super) fn read_bracket(input: &str) -> Option<(String, &str)> {
    let mut body = String::new();
    let mut chars = input.char_indices();
    let mut in_quotes = false;
    let mut escaped = false;
    while let Some((i, c)) = chars.next() {
        if escaped {
            body.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' => {
                body.push(c);
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                body.push(c);
            }
            ']' if !in_quotes => {
                let _ = chars;
                return Some((body, &input[i + 1..]));
            }
            _ => body.push(c),
        }
    }
    None
}

/// Strip surrounding quotes and unescape `\"` / `\\`.
pub(super) fn unquote(raw: &str) -> String {
    let t = raw.trim();
    let inner = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(t);
    inner.replace("\\\"", "\"").replace("\\\\", "\\")
}
