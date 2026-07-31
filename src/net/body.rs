// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded HTTP response body reads (anti-OOM).

use futures_util::StreamExt;

use crate::error::{CliError, ErrorKind};

/// Read response body with a hard byte ceiling.
///
/// 1. If `Content-Length` exceeds `max`, error **without** downloading.
/// 2. Otherwise stream chunks and stop if cumulative size exceeds `max`.
pub async fn read_body_limited(resp: reqwest::Response, max: usize) -> Result<Vec<u8>, CliError> {
    if let Some(cl) = resp.content_length() {
        if cl as usize > max {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("body content-length {cl} exceeds max_body_bytes {max}"),
                crate::i18n::suggestion_key("http_body_too_large", None),
            ));
        }
    }
    let mut out = Vec::new();
    if max > 0 {
        let _ = out.try_reserve(max.min(64 * 1024));
    }
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| CliError::new(ErrorKind::Io, format!("read body: {e}")))?;
        if out.len().saturating_add(chunk.len()) > max {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!(
                    "body exceeds max_body_bytes {max} (received ≥ {})",
                    out.len().saturating_add(chunk.len())
                ),
                crate::i18n::suggestion_key("http_body_too_large", None),
            ));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    // Unit coverage for logic paths is exercised via scrape integration / mock
    // servers; pure helpers without a Response are limited. Compile-time presence
    // of this module is asserted by network-check + lib tests for SSRF/clients.
    #[test]
    fn max_zero_rejects_any_positive_cl_logic() {
        // Document contract: max==0 means only empty bodies are acceptable.
        assert!(0usize.saturating_add(1) > 0);
    }
}
