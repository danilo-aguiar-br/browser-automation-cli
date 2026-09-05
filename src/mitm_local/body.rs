// SPDX-License-Identifier: MIT OR Apache-2.0
//! Body buffering, retention and rendering policy for the capture proxy.
//!
//! Split out of `handler.rs` on 2026-08-28, when that file crossed the 300-line
//! production ceiling enforced by `scripts/filesize-check.sh`. The seam is the
//! one that already existed: everything here decides what happens to BYTES,
//! and knows nothing about hosts, allowlists or the handler's state machine.

use super::handler::BodyPolicy;

/// Hard ceiling on buffering, independent of what the operator wants retained.
///
/// Buffering means holding the whole body before forwarding a byte, so this is
/// the point past which observing a response would cost more than the response
/// is worth. Retention (`--mitm-max-body-bytes`) decides how much is WRITTEN;
/// this decides how much is ever held.
pub(super) const BUFFER_CEILING_BYTES: usize = 8 * 1024 * 1024;

/// Whether this body can be buffered without risking the page.
///
/// # Why `Content-Length` alone was not enough
///
/// The first cut refused every body without a declared length. That is safe and
/// nearly useless: measured against a real site, the responses came back
/// `transfer-encoding: chunked`, which is the norm on HTTP/1.1 and HTTP/2, so
/// the capture stayed empty for the ordinary case.
///
/// # What is refused now
///
/// A declared length above the ceiling, and any endless-by-design content type.
/// An event stream has no last byte: buffering one does not finish late, it
/// never finishes, and the page waits on a proxy that waits on the server.
/// Everything else is buffered up to the ceiling.
pub(super) fn body_is_bufferable(
    headers: &hudsucker::hyper::HeaderMap,
    content_type: &Option<String>,
) -> bool {
    if content_type
        .as_deref()
        .is_some_and(|ct| ct == "text/event-stream")
    {
        return false;
    }
    match headers
        .get(hudsucker::hyper::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(len) => len <= BUFFER_CEILING_BYTES,
        // Undeclared length is the common case, and it is admitted here because
        // the readers wrap the body in `Limited::new(.., BUFFER_CEILING_BYTES)`.
        // That wrapping is what makes this bounded — this `true` on its own does
        // not bound anything, and until 2026-08-28 nothing else did: the comment
        // claimed "the ceiling still applies while reading" while the readers
        // called `body.collect()` unwrapped, so a chunked peer chose how much
        // memory this process allocated. A comment asserting a guarantee the
        // code does not implement is worse than no comment: it answers, wrongly,
        // the exact question a reviewer came to ask.
        None => true,
    }
}

/// Cut retained bytes to the budget, reporting whether anything was dropped.
pub(super) fn clip(bytes: &[u8], budget: usize) -> (&[u8], bool) {
    if bytes.len() <= budget {
        return (bytes, false);
    }
    // Never split a UTF-8 sequence: the retained text is rendered for an agent,
    // and half a codepoint is worse than one fewer character.
    let mut end = budget;
    while end > 0 && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
        end -= 1;
    }
    (&bytes[..end], true)
}

/// Read `Content-Type`, lowercased, without the parameters after `;`.
pub(super) fn content_type_of(headers: &hudsucker::hyper::HeaderMap) -> Option<String> {
    headers
        .get(hudsucker::hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase())
}

/// Whether this content type is media the operator asked to skip.
pub(super) fn is_media(content_type: &Option<String>) -> bool {
    content_type.as_deref().is_some_and(|ct| {
        ct.starts_with("image/") || ct.starts_with("video/") || ct.starts_with("audio/")
    })
}

/// Decide how many bytes of this body to keep.
pub(super) fn retain_budget(policy: BodyPolicy, content_type: &Option<String>) -> usize {
    if policy.skip_media && is_media(content_type) {
        return 0;
    }
    policy.max_bytes
}

/// Redact the WHOLE body, then cut it to the budget.
///
/// # Why the order is the fix
///
/// Until 2026-09-04 the pipeline clipped first and redacted later, in
/// `types.rs`. A JSON payload past the 64 KiB retain budget therefore reached
/// `redact_body` as a FRAGMENT, `serde_json` refused to parse it, and a
/// credential inside that fragment was written to disk in the clear while the
/// capture presented itself as redacted.
///
/// The bytes are already fully resident here — `Limited::new(..,
/// BUFFER_CEILING_BYTES)` in the readers bounded the READ at 8 MiB, which is
/// two orders of magnitude above the retain budget — so parsing the intact
/// document costs nothing that has not already been paid.
///
/// # Why this is not the text-scanning fallback that was refused
///
/// The rejected idea was a heuristic scanner for free text, which trades a real
/// risk of mangling captures for a speculative gain. This is the opposite: the
/// SAME structural parser, given the input it was always meant to receive.
///
/// # Why redaction is checked here rather than assumed
///
/// `redact_secrets()` is the operator's switch. Redacting unconditionally would
/// make `--mitm-no-redact-secrets` a flag that changes nothing, which is the
/// defect class this file already carries a fix for.
pub(super) fn redact_then_clip(
    bytes: &[u8],
    content_type: &Option<String>,
    budget: usize,
) -> Option<String> {
    if !super::policy::redact_secrets() {
        let (kept, truncated) = clip(bytes, budget);
        return render_body(kept, content_type, truncated);
    }
    // Non-text and media never reach the structural redactor, so there is
    // nothing to gain by parsing them whole; take the cheap path.
    let Ok(text) = std::str::from_utf8(bytes) else {
        let (kept, truncated) = clip(bytes, budget);
        return render_body(kept, content_type, truncated);
    };
    if is_media(content_type) {
        let (kept, truncated) = clip(bytes, budget);
        return render_body(kept, content_type, truncated);
    }
    let mut whole = Some(text.to_string());
    super::redact::redact_body(&mut whole);
    let redacted = whole.unwrap_or_default();
    // Clip the REDACTED text. Masking can change the length in either
    // direction, so the budget is applied to what will actually be written.
    let (kept, truncated) = clip(redacted.as_bytes(), budget);
    render_body(kept, content_type, truncated)
}

/// Render retained bytes for an agent: text as text, anything else as a note.
///
/// Binary is never emitted raw. A capture is read by a model over stdout, and
/// pasting a PNG into that stream costs tokens and tells the reader nothing.
pub(super) fn render_body(
    bytes: &[u8],
    content_type: &Option<String>,
    truncated: bool,
) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(s) if !is_media(content_type) => Some(if truncated {
            format!("{s}…[truncated]")
        } else {
            s.to_string()
        }),
        _ => Some(format!(
            "<{} bytes {}>",
            bytes.len(),
            content_type.as_deref().unwrap_or("binary")
        )),
    }
}

#[cfg(test)]
mod redact_order_tests {
    use super::*;

    /// The property the whole ordering fix exists for.
    ///
    /// A secret sitting PAST the retain budget used to survive: the body was cut
    /// first, `serde_json` refused the fragment, and `redact_body` returned it
    /// untouched while the capture presented itself as redacted. Parsing the
    /// intact document is what makes the mask reach it.
    ///
    /// The padding is deliberately larger than the budget passed here, so the
    /// secret cannot be inside the kept window by accident — a test that only
    /// passes because the value happened to fit asserts nothing.
    #[test]
    fn a_secret_past_the_budget_is_masked_not_leaked() {
        super::super::policy::set_redact_secrets_for_test(true);
        let padding = "x".repeat(4096);
        let body = format!(r#"{{"padding":"{padding}","authorization":"Bearer topsecret"}}"#);
        let out = redact_then_clip(body.as_bytes(), &Some("application/json".into()), 512)
            .expect("body must render");
        assert!(
            !out.contains("topsecret"),
            "a secret beyond the retain budget must not reach the capture: {out}"
        );
    }

    /// The operator's switch still switches.
    ///
    /// Redacting unconditionally would turn `--mitm-no-redact-secrets` into a
    /// flag that changes nothing, which is a defect class this module already
    /// carries a fix for.
    #[test]
    fn redaction_off_leaves_the_body_alone() {
        super::super::policy::set_redact_secrets_for_test(false);
        let body = r#"{"authorization":"Bearer topsecret"}"#;
        let out = redact_then_clip(body.as_bytes(), &Some("application/json".into()), 4096)
            .expect("body must render");
        assert!(
            out.contains("topsecret"),
            "with redaction off the body must survive byte for byte: {out}"
        );
        super::super::policy::set_redact_secrets_for_test(true);
    }
}
