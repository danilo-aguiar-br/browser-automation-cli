// SPDX-License-Identifier: MIT OR Apache-2.0
//! DOCX text extraction backend for `parse`.
//!
//! Split out of `parse.rs` for the same reason as the PDF backend: one format's
//! decoder should not sit in the same compilation unit as the others.

use crate::error::{CliError, ErrorKind};

/// Extract visible text from the main document part of a `.docx`.
///
/// Returns `(kind, text, engine)`.
///
/// The tag stripper is deliberate rather than a full XML parse: `word/document.xml`
/// carries the runs in document order, so dropping markup and collapsing runs of
/// whitespace reproduces the reading order without pulling a DOM in for a value
/// that is discarded immediately afterwards. A space is inserted at every tag
/// close so adjacent runs do not fuse into one word.
///
/// # Errors
///
/// - [`ErrorKind::Data`] when the bytes are not a readable ZIP archive, or when
///   the archive has no `word/document.xml` member
/// - [`ErrorKind::Io`] when that member exists but cannot be read to a string
pub(crate) fn parse_docx_bytes(
    bytes: &[u8],
) -> Result<(&'static str, String, &'static str), CliError> {
    parse_docx_bytes_with(
        bytes,
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::SCRAPE_MAX_PARSE_BYTES),
    )
}

/// Parameterized core: the same parse against an explicit ceiling.
///
/// # Why the caller's ceiling does not reach here on its own
///
/// `parse_file_opts` rejects a file whose `metadata().len()` exceeds
/// `scrape_max_parse_bytes`, but a `.docx` is a ZIP: that ceiling measures the
/// COMPRESSED archive on disk, while the member read below is DECOMPRESSED.
/// Repetitive XML compresses on the order of 1000:1, so an archive comfortably
/// inside the ceiling can expand to gigabytes. That is a zip bomb, and the
/// outer check cannot see it.
///
/// # Why the declared size is checked AND the read is capped
///
/// `ZipFile::size()` is read from the archive's own header — data supplied by
/// whoever produced the file. Trusting it alone defends only against honest
/// archives. So the declared size rejects early (cheap, and gives an accurate
/// message), and [`std::io::Read::take`] bounds what is actually read, for the
/// case where the header understates the truth.
pub(crate) fn parse_docx_bytes_with(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<(&'static str, String, &'static str), CliError> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("docx zip open: {e}")))?;
    let file = archive.by_name("word/document.xml").map_err(|e| {
        CliError::new(
            ErrorKind::Data,
            format!("docx missing word/document.xml: {e}"),
        )
    })?;
    let declared = file.size();
    if declared > max_bytes as u64 {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "docx word/document.xml declares {declared} bytes uncompressed, over the \
                 {max_bytes} byte ceiling"
            ),
            crate::i18n::suggestion_key("split_input_or_raise_limit", None),
        ));
    }
    let mut xml = String::new();
    // `+ 1` so a member that lies LOW in its header still trips the check
    // below instead of being silently truncated into valid-looking XML.
    let mut capped = file.take(max_bytes as u64 + 1);
    capped
        .read_to_string(&mut xml)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("docx read xml: {e}")))?;
    if xml.len() > max_bytes {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "docx word/document.xml expands past the {max_bytes} byte ceiling; its \
                 header declared {declared}"
            ),
            crate::i18n::suggestion_key("split_input_or_raise_limit", None),
        ));
    }
    // Strip tags; insert space between tags for word boundaries.
    let mut text = String::with_capacity(xml.len() / 4);
    let mut in_tag = false;
    let mut last_space = true;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !last_space {
                    text.push(' ');
                    last_space = true;
                }
            }
            _ if !in_tag => {
                if ch.is_whitespace() {
                    if !last_space {
                        text.push(' ');
                        last_space = true;
                    }
                } else {
                    text.push(ch);
                    last_space = false;
                }
            }
            _ => {}
        }
    }
    Ok(("docx", text.trim().to_string(), "local-docx"))
}
