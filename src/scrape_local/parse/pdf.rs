// SPDX-License-Identifier: MIT OR Apache-2.0
//! PDF text extraction backend for `parse`.
//!
//! Split out of `parse.rs` so a change to PDF handling does not recompile the
//! spreadsheet or DOCX path, and so the orchestration in the parent reads as
//! dispatch rather than as four inlined decoders.

use crate::error::{CliError, ErrorKind};

/// Extract the text layer of a PDF, with its page count.
///
/// Returns `(kind, text, engine, page_count, text_layer_empty)`.
///
/// `text_layer_empty` is reported rather than treated as failure: a scanned
/// document legitimately carries no text layer, and the caller — not this
/// function — decides what to do about it. This product does not recognise text
/// in images, so an empty layer is a fact to publish, never a fallback to run.
///
/// # Errors
///
/// - [`ErrorKind::Data`] when the first five bytes are not the `%PDF-` magic,
///   when `lopdf` cannot load the document, or when text extraction fails
pub(crate) fn parse_pdf_bytes(
    bytes: &[u8],
) -> Result<(&'static str, String, &'static str, usize, bool), CliError> {
    if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "invalid PDF magic: expected %PDF- header",
            crate::i18n::suggestion_key("pdf_input_invalid", None),
        ));
    }
    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("pdf load failed: {e}")))?;
    let pages = doc.get_pages();
    let page_numbers: Vec<u32> = pages.keys().copied().collect();
    let page_count = page_numbers.len();
    let text = doc
        .extract_text(&page_numbers)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("pdf extract_text: {e}")))?;
    let text_layer_empty = text.trim().is_empty();
    Ok(("pdf", text, "lopdf", page_count, text_layer_empty))
}
