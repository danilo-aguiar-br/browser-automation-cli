// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTTP audio download with SSRF policy, body cap, and magic verification.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::magic::{detect_container, DetectedAudio};
use crate::error::{CliError, ErrorKind};
use crate::robots::shared_http_client;
use crate::xdg;

/// Download a media URL to disk (one-shot; no Chrome).
pub async fn download_audio(
    url: &str,
    out: Option<&Path>,
    max_bytes: Option<usize>,
    require_audio: bool,
) -> Result<Value, CliError> {
    crate::net::assert_safe_http_url(url)?;
    let max = max_bytes.unwrap_or_else(xdg::resolve_audio_download_max_bytes);
    let client = shared_http_client()?;
    let cfg = crate::retry::RetryConfig::http();
    let mut attempt = 0u32;
    let (final_url, bytes) = loop {
        attempt += 1;
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let final_url = resp.url().to_string();
                crate::net::assert_safe_http_url(&final_url)?;
                if !status.is_success() {
                    let err = format!("audio download HTTP {status} for {url}");
                    if attempt >= cfg.max_attempts || !crate::retry::is_retryable_message(&err) {
                        return Err(CliError::new(ErrorKind::Io, err));
                    }
                } else {
                    match crate::net::read_body_limited(resp, max).await {
                        Ok(bytes) => break (final_url, bytes),
                        Err(e) if e.kind() == ErrorKind::Data => return Err(e),
                        Err(e) => {
                            let err = e.message().to_string();
                            if attempt >= cfg.max_attempts
                                || !crate::retry::is_retryable_message(&err)
                            {
                                return Err(CliError::new(ErrorKind::Io, err));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let err = format!("GET {url}: {e}");
                if attempt >= cfg.max_attempts || !crate::retry::is_retryable_message(&err) {
                    return Err(CliError::new(ErrorKind::Unavailable, err));
                }
            }
        }
        tokio::time::sleep(cfg.delay_for_attempt(attempt.saturating_sub(1))).await;
    };

    let detected = match detect_container(&bytes) {
        Ok(f) => Some(f),
        Err(e) if require_audio => return Err(e),
        Err(_) => None,
    };
    if require_audio {
        let f = detected.ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Data,
                "downloaded body is not a supported audio container",
                crate::i18n::suggestion_key("audio_magic_invalid", None),
            )
        })?;
        if !f.is_audio_container() {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("downloaded container {} is unsupported", f.as_str()),
                crate::i18n::suggestion_key("audio_format_unsupported", None),
            ));
        }
    }

    let ext = detected.map(DetectedAudio::as_str).unwrap_or("bin");
    let out_path = match out {
        Some(p) => p.to_path_buf(),
        None => {
            let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            dir.join(format!("audio-download-{stamp}.{ext}"))
        }
    };
    crate::image_local::write_bytes_atomic(&out_path, &bytes)?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha = format!("{:x}", hasher.finalize());

    Ok(json!({
        "action": "download",
        "url": url,
        "final_url": final_url,
        "path": out_path.display().to_string(),
        "bytes": bytes.len(),
        "sha256": sha,
        "container": detected.map(|d| d.as_str()),
        "magic_ok": detected.is_some(),
        "engine": "http",
    }))
}
