// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTTP image download with SSRF policy, body cap, and magic verification.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::atomic::write_bytes_atomic;
use super::magic::{detect_format, DetectedFormat};
use crate::error::{CliError, ErrorKind};
use crate::retry::Attempt;
use crate::robots::shared_http_client;
use crate::xdg;

/// Download an image URL to disk (one-shot; no Chrome).
pub async fn download_image(
    url: &str,
    out: Option<&Path>,
    max_bytes: Option<usize>,
    require_image: bool,
) -> Result<Value, CliError> {
    crate::net::assert_safe_http_url(url)?;
    let max = max_bytes.unwrap_or_else(xdg::resolve_image_download_max_bytes);
    let client = shared_http_client()?;
    // The retry loop itself lives in `crate::retry`: image, video and audio all
    // download the same way, and three copies of one loop drift apart silently.
    let (final_url, bytes) =
        crate::retry::retry_http_async(crate::retry::RetryConfig::http(), || async {
            match client.get(url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let final_url = resp.url().to_string();
                    if let Err(e) = crate::net::assert_safe_http_url(&final_url) {
                        return Attempt::Fatal(e);
                    }
                    if !status.is_success() {
                        return Attempt::Failed(CliError::new(
                            ErrorKind::Io,
                            format!("image download HTTP {status} for {url}"),
                        ));
                    }
                    match crate::net::read_body_limited(resp, max).await {
                        Ok(bytes) => Attempt::Done((final_url, bytes)),
                        // Over the body ceiling: another round trip downloads the
                        // same oversized body to the same refusal.
                        Err(e) if e.kind() == ErrorKind::Data => Attempt::Fatal(e),
                        Err(e) => {
                            Attempt::Failed(CliError::new(ErrorKind::Io, e.message().to_string()))
                        }
                    }
                }
                Err(e) => Attempt::Failed(CliError::new(
                    ErrorKind::Unavailable,
                    format!("GET {url}: {e}"),
                )),
            }
        })
        .await?;

    let detected = match detect_format(&bytes) {
        Ok(f) => Some(f),
        Err(e) if require_image => return Err(e),
        Err(_) => None,
    };
    if require_image {
        let f = detected.ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Data,
                "downloaded body is not a supported image",
                crate::i18n::suggestion_key("image_magic_invalid", None),
            )
        })?;
        if !f.is_supported() {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("downloaded format {} is unsupported", f.as_str()),
                crate::i18n::suggestion_key("image_format_unsupported", None),
            ));
        }
    }

    let ext = detected.map(DetectedFormat::as_str).unwrap_or("bin");
    let out_path = match out {
        Some(p) => p.to_path_buf(),
        None => {
            let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            dir.join(format!("download-{stamp}.{ext}"))
        }
    };
    write_bytes_atomic(&out_path, &bytes)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha = format!("{:x}", hasher.finalize());
    Ok(json!({
        "action": "download",
        "url": url,
        "final_url": final_url,
        "path": out_path.display().to_string(),
        "bytes": bytes.len(),
        "format": detected.map(|f| f.as_str()),
        "magic_ok": detected.map(|f| f.is_supported()).unwrap_or(false),
        "sha256": sha,
        "engine": "image",
    }))
}
