// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP screenshot capture (Page.captureScreenshot).
use serde_json::Value;

use super::annotate::{
    collect_annotations, filter_annotations, get_rect_for_selector, get_scroll_offsets,
    inject_annotation_overlay, project_annotations, remove_annotation_overlay,
};
use super::save::save_screenshot_async;
use super::types::{ScreenshotOptions, ScreenshotResult};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::RefMap;

/// Capture the page, or one element, and write it to disk.
///
/// Honours [`ScreenshotOptions`] for clipping,
/// encoding and annotation; the returned result carries both the path and the
/// image inline.
///
/// # Errors
///
/// Under `options.annotate`, fails when the target rect of
/// `options.selector` cannot be read, when annotations cannot be collected,
/// when the overlay cannot be injected, or when the scroll offsets needed to
/// project a full-page capture cannot be read.
///
/// Then fails with the CDP error raised by `Page.captureScreenshot` — an
/// element clip that resolves to nothing, or an unsupported format — and with
/// the save error from
/// [`save_screenshot_async`]:
/// undecodable base64, or an unwritable destination.
///
/// The capture error is deliberately held until the overlay has been removed,
/// so a failed screenshot never leaves the annotation overlay on the page.
pub async fn take_screenshot(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    options: &ScreenshotOptions,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<ScreenshotResult, String> {
    let target_rect = if options.annotate {
        match options.selector.as_deref() {
            Some(selector) => {
                get_rect_for_selector(client, session_id, ref_map, selector, iframe_sessions)
                    .await?
            }
            None => None,
        }
    } else {
        None
    };

    let raw_annotations = if options.annotate {
        collect_annotations(client, session_id, ref_map).await?
    } else {
        Vec::new()
    };

    let overlay_items = filter_annotations(raw_annotations, target_rect.as_ref());
    let overlay_injected = if options.annotate && !overlay_items.is_empty() {
        inject_annotation_overlay(client, session_id, &overlay_items).await?;
        true
    } else {
        false
    };

    let base64 =
        capture_screenshot_base64(client, session_id, ref_map, options, iframe_sessions).await;

    if overlay_injected {
        let _ = remove_annotation_overlay(client, session_id).await;
    }

    let base64 = base64?;
    let annotations = if options.annotate {
        let scroll = if options.full_page {
            Some(get_scroll_offsets(client, session_id).await?)
        } else {
            None
        };
        project_annotations(&overlay_items, target_rect.as_ref(), scroll)
    } else {
        Vec::new()
    };

    // BUG-IMG-001: honor webp extension (was forced to png).
    let ext = super::types::screenshot_ext_for_format(options.format.as_str());
    // path/output_dir are small Options — clone is deliberate for the blocking task.
    // BUG-IMG-004: retain base64 only when `--include-base64` opts in (agent-native default off).
    let keep_b64 = options.include_base64;
    let path = save_screenshot_async(
        base64.clone(),
        options.path.clone(),
        ext.to_string(),
        options.output_dir.clone(),
    )
    .await?;

    Ok(ScreenshotResult {
        path,
        base64: if keep_b64 { Some(base64) } else { None },
        annotations,
    })
}

async fn capture_screenshot_base64(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    options: &ScreenshotOptions,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<String, String> {
    let mut params = CaptureScreenshotParams {
        format: Some(options.format.clone()),
        quality: if matches!(options.format.as_str(), "jpeg" | "jpg" | "webp") {
            options
                .quality
                .or(Some(crate::xdg::resolve_default_jpeg_quality()))
        } else {
            None
        },
        clip: None,
        from_surface: Some(true),
        capture_beyond_viewport: if options.full_page { Some(true) } else { None },
    };

    if options.full_page {
        let metrics: Value = client
            .send_command_no_params("Page.getLayoutMetrics", Some(session_id))
            .await?;

        let content_size = metrics
            .get("contentSize")
            .or_else(|| metrics.get("cssContentSize"));
        if let Some(size) = content_size {
            let width = size
                .get("width")
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::from(crate::xdg::policy::policy_u32(
                    crate::xdg::policy::key::DEFAULT_VIEWPORT_WIDTH,
                )));
            let height = size
                .get("height")
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::from(crate::xdg::policy::policy_u32(
                    crate::xdg::policy::key::DEFAULT_VIEWPORT_HEIGHT,
                )));

            params.clip = Some(Viewport {
                x: 0.0,
                y: 0.0,
                width,
                height,
                scale: 1.0,
            });
        }
    } else if let Some(ref selector) = options.selector {
        if let Some(rect) =
            get_rect_for_selector(client, session_id, ref_map, selector, iframe_sessions).await?
        {
            params.clip = Some(Viewport {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                scale: 1.0,
            });
        }
    }

    let result: CaptureScreenshotResult = client
        .send_command_typed("Page.captureScreenshot", &params, Some(session_id))
        .await?;

    Ok(result.data)
}
