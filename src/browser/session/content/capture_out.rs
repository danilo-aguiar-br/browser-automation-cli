// SPDX-License-Identifier: MIT OR Apache-2.0
//! print_pdf, grab

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;
use crate::browser::helpers::verify_image_magic;

impl OneShotSession {
    /// Render the current page to PDF.
    pub async fn print_pdf(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        use base64::Engine as _;
        self.drain_events();
        let session_id = self.session_id()?;
        let result: Value = self
            .manager
            .client
            .send_command(
                "Page.printToPDF",
                Some(json!({
                    "printBackground": true,
                    "preferCSSPageSize": true,
                })),
                Some(&session_id),
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("print-pdf failed: {e}"),
                    crate::i18n::suggestion_key("navigate_first", None),
                )
            })?;
        let b64 = result
            .get("data")
            .or_else(|| result.pointer("/result/data"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CliError::new(
                    ErrorKind::Browser,
                    "print-pdf: missing base64 data in CDP result",
                )
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("print-pdf base64: {e}")))?;
        if bytes.len() < 5 || &bytes[0..4] != b"%PDF" {
            return Err(CliError::new(
                ErrorKind::Data,
                "print-pdf: result is not a valid PDF",
            ));
        }
        let out = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            std::path::PathBuf::from(format!("print-{stamp}.pdf"))
        });
        // PAR-82: unify on write_bytes_blocking (docsrs: never pin async with std::fs).
        let byte_len = bytes.len();
        crate::concurrency::write_bytes_blocking(out.clone(), bytes)
            .await
            .map_err(|e| {
                CliError::new(ErrorKind::Io, format!("write pdf {}: {e}", out.display()))
            })?;
        Ok(json!({
            "path": out.display().to_string(),
            "bytes": byte_len,
            "format": "pdf",
        }))
    }

    /// Screenshot the page or one element.
    pub async fn grab(
        &mut self,
        path: Option<&Path>,
        format: &str,
        full_page: bool,
        quality: Option<i32>,
        element: Option<&str>,
        include_base64: bool,
    ) -> Result<Value, CliError> {
        use crate::native::screenshot::{
            screenshot_ext_for_format, take_screenshot, ScreenshotOptions,
        };

        let session_id = self.session_id()?;
        let ext = screenshot_ext_for_format(format);

        let out_path = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            std::path::PathBuf::from(format!("grab-{stamp}.{ext}"))
        });

        let options = ScreenshotOptions {
            path: Some(out_path.to_string_lossy().into_owned()),
            format: format.to_string(),
            full_page,
            quality,
            selector: element.map(|s| s.to_string()),
            include_base64,
            ..ScreenshotOptions::default()
        };

        let result = take_screenshot(
            &self.manager.client,
            &session_id,
            &self.ref_map,
            &options,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("grab failed: {e}"),
                crate::i18n::suggestion_key("navigate_first", None),
            )
        })?;

        let path_str = if result.path.is_empty() {
            out_path.to_string_lossy().into_owned()
        } else {
            result.path
        };
        let path_buf = std::path::PathBuf::from(&path_str);
        let path_for_meta = path_buf.clone();
        let format_owned = format.to_string();
        // Dimensions are read here, on the blocking pool, alongside the size
        // and magic probe that already stat the file.
        //
        // Emitting them is the difference between one invocation and two: the
        // caller almost always needs to know how tall a `--full-page` capture
        // came out, and without these fields the only way to learn it is a
        // second process running `image info` on a file this command just
        // wrote. Reading a PNG/JPEG/WebP header costs bytes, not a round trip.
        let (written, magic_ok, byte_size, dims) = tokio::task::spawn_blocking(move || {
            let written = path_for_meta.exists();
            let magic_ok = written && verify_image_magic(&path_for_meta, &format_owned);
            let byte_size = std::fs::metadata(&path_for_meta)
                .map(|m| m.len())
                .unwrap_or(0);
            let dims = written
                .then(|| image::image_dimensions(&path_for_meta).ok())
                .flatten();
            (written, magic_ok, byte_size, dims)
        })
        .await
        .unwrap_or((false, false, 0, None));

        // Agent-native: omit base64 key unless explicitly requested.
        let mut data = json!({
            "path": path_str,
            "format": format,
            "written": written,
            "magic_ok": magic_ok,
            "byte_size": byte_size,
            "width": dims.map(|(w, _)| w),
            "height": dims.map(|(_, h)| h),
            "full_page": full_page,
            "quality": quality,
            "element": element,
        });
        if let Some(b64) = result.base64 {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("base64".into(), json!(b64));
            }
        }
        Ok(data)
    }
}
