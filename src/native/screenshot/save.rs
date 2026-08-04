// SPDX-License-Identifier: MIT OR Apache-2.0
//! Decode base64 and persist screenshot bytes (blocking / spawn_blocking).
//!
//! # Workload
//!
//! Decode+write is CPU/IO bound; callers must use [`save_screenshot_async`]
//! which runs on Tokio `spawn_blocking` (docsrs: never pin async workers).
use std::path::PathBuf;

pub(crate) fn save_screenshot(
    base64_data: &str,
    explicit_path: Option<&str>,
    ext: &str,
    output_dir: Option<&str>,
) -> Result<String, String> {
    let save_path = match explicit_path {
        Some(path) => path.to_string(),
        None => {
            let dir = match output_dir {
                Some(d) => PathBuf::from(d),
                None => get_screenshot_dir()?,
            };
            let _ = std::fs::create_dir_all(&dir);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let name = format!("screenshot-{timestamp}.{ext}");
            dir.join(name).to_string_lossy().to_string()
        }
    };

    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
        .map_err(|e| format!("Failed to decode screenshot: {e}"))?;

    // BUG-IMG-003: atomic write (tmp + fsync + rename) so kill mid-write cannot
    // leave a truncated artifact. Runs on spawn_blocking / one-shot paths only.
    let path = std::path::Path::new(&save_path);
    crate::image_local::write_bytes_atomic(path, &bytes)
        .map_err(|e| format!("Failed to save screenshot to {save_path}: {e}"))?;

    Ok(save_path)
}

/// Async-safe screenshot save: decode+write on Tokio blocking pool.
pub async fn save_screenshot_async(
    base64_data: String,
    explicit_path: Option<String>,
    ext: String,
    output_dir: Option<String>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        save_screenshot(
            &base64_data,
            explicit_path.as_deref(),
            &ext,
            output_dir.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("screenshot save join: {e}"))?
}

pub(crate) fn get_screenshot_dir() -> Result<PathBuf, String> {
    crate::xdg::cache_dir()
        .map(|d| d.join("screenshots"))
        .map_err(|e| format!("screenshot cache dir (XDG): {e}"))
}
