// SPDX-License-Identifier: MIT OR Apache-2.0
//! SVG source sanitisation performed *before* any parser sees the bytes
//! (GAP-IMG-092).
//!
//! An SVG is a program, not a pixel buffer. A few hundred bytes can encode
//! unbounded work or an outbound request, so this pass runs first and fails
//! closed. Every ceiling is an XDG knob, never a literal:
//!
//! | Threat | Guard | Knob |
//! |--------|-------|------|
//! | Oversized source | byte cap | `svg_max_bytes` |
//! | Billion laughs / quadratic blowup | `<!ENTITY>` count | `svg_max_entities` |
//! | Parser stack exhaustion | nesting depth | `svg_max_depth` |
//! | SSRF / local file read | external reference reject | — (unconditional) |
//! | Stored XSS on re-serialise | `<script>` + `on*` reject | — (unconditional) |
//!
//! The renderer downstream neither executes scripts nor fetches URLs, so the
//! last two rules are defence in depth: they stop a hostile SVG from surviving
//! a round-trip through an agent that later hands the source to a browser.

use crate::error::{CliError, ErrorKind};

/// Outcome of a successful sanitisation pass, reported in the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgReport {
    /// Source length in bytes.
    pub bytes: usize,
    /// Deepest element nesting observed.
    pub max_depth: u32,
    /// `<!ENTITY>` declarations found (always within the configured cap).
    pub entities: u32,
}

fn reject(msg: impl Into<String>, key: &str) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Data,
        msg.into(),
        crate::i18n::suggestion_key(key, None),
    )
}

/// Attribute prefixes that carry script in every SVG-hosting engine.
const EVENT_HANDLER_PREFIX: &str = "on";

/// Schemes rejected inside `href` / `xlink:href` values.
///
/// `data:` is allowed because it is self-contained and cannot reach the network
/// or the filesystem; everything that can is refused.
const FORBIDDEN_SCHEMES: &[&str] = &["http:", "https:", "file:", "ftp:", "jar:", "javascript:"];

/// Find the first `href` / `xlink:href` value carrying a forbidden scheme.
///
/// The check inspects attribute *values*, never the raw document. Scanning the
/// whole text for `http:` would reject every conformant SVG, because the
/// mandatory `xmlns="http://www.w3.org/2000/svg"` declaration contains it — a
/// namespace URI is an identifier, not a fetch.
fn scan_forbidden_href(lower: &str) -> Option<&'static str> {
    for (at, _) in lower.match_indices("href") {
        // The attribute name must be `href` or `<prefix>:href`, and must start
        // at a name boundary so `xmlns:xlink` never matches.
        let before = lower[..at].chars().next_back();
        match before {
            Some(c) if c.is_whitespace() || c == ':' => {}
            _ => continue,
        }
        let after = &lower[at + "href".len()..];
        let Some((gap, value_part)) = after.split_once('=') else {
            continue;
        };
        if !gap.chars().all(char::is_whitespace) {
            continue;
        }
        let value = value_part.trim_start();
        let quote = match value.chars().next() {
            Some(q @ ('"' | '\'')) => q,
            // Unquoted attribute values are not valid XML; treat as no match.
            _ => continue,
        };
        let Some(end) = value[1..].find(quote) else {
            continue;
        };
        let target = value[1..1 + end].trim_start();
        if let Some(scheme) = FORBIDDEN_SCHEMES
            .iter()
            .copied()
            .find(|s| target.starts_with(s))
        {
            return Some(scheme);
        }
    }
    None
}

/// Measure element nesting depth without building a tree.
///
/// Counts `<tag …>` opens and `</tag>` closes, ignoring self-closing tags,
/// comments, CDATA, and processing instructions.
fn measure_depth(src: &str, max_depth: u32) -> Result<u32, CliError> {
    let bytes = src.as_bytes();
    let mut depth: u32 = 0;
    let mut deepest: u32 = 0;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &src[i..];
        if rest.starts_with("<!--") {
            i += rest.find("-->").map_or(bytes.len(), |p| p + 3);
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            i += rest.find("]]>").map_or(bytes.len(), |p| p + 3);
            continue;
        }
        if rest.starts_with("<?") || rest.starts_with("<!") {
            i += rest.find('>').map_or(bytes.len(), |p| p + 1);
            continue;
        }
        let Some(close_at) = rest.find('>') else {
            break;
        };
        let tag = &rest[..close_at + 1];
        if tag.starts_with("</") {
            depth = depth.saturating_sub(1);
        } else if !tag.ends_with("/>") {
            depth = depth.saturating_add(1);
            deepest = deepest.max(depth);
            if deepest > max_depth {
                return Err(reject(
                    format!("svg nesting depth exceeds svg_max_depth {max_depth}"),
                    "svg_rejected",
                ));
            }
        }
        i += close_at + 1;
    }
    Ok(deepest)
}

/// Validate an SVG source against every configured ceiling.
///
/// Returns the measured [`SvgReport`] on success. The input is never rewritten:
/// a rejected document is refused outright rather than silently edited, so an
/// agent is never handed a picture that differs from the bytes it supplied.
pub fn sanitize(src: &[u8]) -> Result<SvgReport, CliError> {
    let max_bytes = crate::xdg::resolve_svg_max_bytes();
    if src.len() > max_bytes {
        return Err(reject(
            format!(
                "svg source {} bytes exceeds svg_max_bytes {max_bytes}",
                src.len()
            ),
            "image_too_large",
        ));
    }
    let text = std::str::from_utf8(src).map_err(|e| {
        reject(
            format!("svg source is not valid UTF-8: {e}"),
            "svg_rejected",
        )
    })?;
    let lower = text.to_ascii_lowercase();

    let max_entities = crate::xdg::resolve_svg_max_entities();
    let entities = u32::try_from(lower.matches("<!entity").count()).unwrap_or(u32::MAX);
    if entities > max_entities {
        return Err(reject(
            format!(
                "svg declares {entities} XML entities, over svg_max_entities {max_entities} \
                 (entity expansion is the billion-laughs vector)"
            ),
            "svg_rejected",
        ));
    }
    // A DOCTYPE without entity declarations is still the only way to reach the
    // external-subset resolver, so refuse it whenever entities are disallowed.
    if max_entities == 0 && lower.contains("<!doctype") {
        return Err(reject(
            "svg carries a DOCTYPE; set svg_max_entities > 0 to allow one",
            "svg_rejected",
        ));
    }
    if lower.contains("<script") {
        return Err(reject("svg contains a <script> element", "svg_rejected"));
    }
    if lower.contains("<foreignobject") {
        return Err(reject(
            "svg contains a <foreignObject> element (arbitrary embedded markup)",
            "svg_rejected",
        ));
    }
    if let Some(scheme) = scan_forbidden_href(&lower) {
        return Err(reject(
            format!("svg references the '{scheme}' scheme in an href (SSRF / local file read)"),
            "svg_rejected",
        ));
    }
    if has_event_handler(&lower) {
        return Err(reject(
            "svg carries an on* event-handler attribute",
            "svg_rejected",
        ));
    }

    let max_depth = crate::xdg::resolve_svg_max_depth();
    let depth = measure_depth(text, max_depth)?;
    Ok(SvgReport {
        bytes: src.len(),
        max_depth: depth,
        entities,
    })
}

/// Detect an `on…=` attribute without flagging ordinary words containing "on".
fn has_event_handler(lower: &str) -> bool {
    lower.match_indices(EVENT_HANDLER_PREFIX).any(|(at, _)| {
        // Must start an attribute name: preceded by whitespace inside a tag.
        let starts_attr = at > 0
            && lower.as_bytes()[at - 1].is_ascii_whitespace()
            && lower[..at].rfind('<') > lower[..at].rfind('>');
        if !starts_attr {
            return false;
        }
        // …and be followed by identifier chars then `=`.
        lower[at + EVENT_HANDLER_PREFIX.len()..]
            .split_once('=')
            .is_some_and(|(name, _)| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic())
            })
    })
}
