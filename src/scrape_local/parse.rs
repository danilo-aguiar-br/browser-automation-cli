// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local file parse (html/md/pdf/docx/xlsx) one-shot, no Chrome.

use std::fs;
use std::path::Path;

use scraper::Html;
use serde_json::{json, Value};

use crate::cache::{self, HttpCache};
use crate::error::{CliError, ErrorKind};

use super::html::{redact_pii, visible_text};

/// Parse local file (html/md/txt/csv/json/xml/pdf/docx/xlsx) one-shot, no Chrome.
pub fn parse_file(path: &Path) -> Result<Value, CliError> {
    parse_file_opts(path, false)
}

/// Parse local file with optional PII redaction.
pub fn parse_file_opts(path: &Path, redact: bool) -> Result<Value, CliError> {
    let max_parse_bytes =
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::SCRAPE_MAX_PARSE_BYTES);
    let meta = fs::metadata(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("parse open {}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("not a file: {}", path.display()),
        ));
    }
    if meta.len() as usize > max_parse_bytes {
        return Err(CliError::new(
            ErrorKind::Data,
            format!(
                "file {} exceeds max parse size {}",
                path.display(),
                max_parse_bytes
            ),
        ));
    }
    let bytes = fs::read(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read {}: {e}", path.display())))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut extra = json!({});
    let (kind, mut text, engine) = match ext.as_str() {
        "html" | "htm" => {
            let s = String::from_utf8_lossy(&bytes);
            let doc = Html::parse_document(&s);
            ("html", visible_text(&doc), "local")
        }
        "md" | "markdown" | "txt" | "json" | "xml" => (
            "text",
            String::from_utf8_lossy(&bytes).into_owned(),
            "local",
        ),
        "csv" => {
            let s = String::from_utf8_lossy(&bytes).into_owned();
            extra["rows"] = json!(s.lines().count());
            ("csv", s, "local")
        }
        "pdf" => {
            let (kind, text, engine, pages, ocr_needed) = parse_pdf_bytes(&bytes)?;
            extra["pages"] = json!(pages);
            extra["ocr_needed"] = json!(ocr_needed);
            (kind, text, engine)
        }
        "docx" => parse_docx_bytes(&bytes)?,
        "xlsx" | "xlsm" | "xls" | "ods" => parse_spreadsheet(path)?,
        other => {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unsupported parse extension: {other}"),
                crate::i18n::suggestion_key("use_listed_value", None),
            ));
        }
    };
    let mut redacted = false;
    if redact {
        text = redact_pii(&text);
        redacted = true;
    }
    // GAP-023/011: store parse result under XDG HTTP/parse cache when available.
    let mut cache_hit = false;
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cache_key = cache::CacheKey::file_parse(path, bytes.len() as u64, mtime);
    if let Ok(c) = cache::default_cache() {
        if let Ok(Some(entry)) = HttpCache::get(c.as_ref(), &cache_key) {
            if let Ok(cached_text) = String::from_utf8(entry.body) {
                text = cached_text;
                cache_hit = true;
            }
        } else {
            let _ = HttpCache::put(
                c.as_ref(),
                &cache_key,
                cache::CacheEntry {
                    body: text.as_bytes().to_vec(),
                    content_type: Some(format!("text/{kind}")),
                    expires_unix: cache::expires_after(std::time::Duration::from_secs(
                        crate::xdg::policy::policy_u64(
                            crate::xdg::policy::key::FILE_PARSE_CACHE_TTL_SECS,
                        ),
                    )),
                },
            );
        }
    }
    let mut out = json!({
        "path": path.display().to_string(),
        "kind": kind,
        "bytes": bytes.len(),
        "text": text,
        "chars": text.chars().count(),
        "engine": engine,
        "redacted": redacted,
        "cache_hit": cache_hit,
    });
    if let Some(obj) = out.as_object_mut() {
        if let Some(extra_obj) = extra.as_object() {
            for (k, v) in extra_obj {
                obj.insert(k.clone(), v.clone());
            }
        }
    }
    Ok(out)
}

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
    let ocr_needed = text.trim().is_empty();
    Ok(("pdf", text, "lopdf", page_count, ocr_needed))
}

pub(crate) fn parse_docx_bytes(
    bytes: &[u8],
) -> Result<(&'static str, String, &'static str), CliError> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("docx zip open: {e}")))?;
    let mut file = archive.by_name("word/document.xml").map_err(|e| {
        CliError::new(
            ErrorKind::Data,
            format!("docx missing word/document.xml: {e}"),
        )
    })?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("docx read xml: {e}")))?;
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

pub(crate) fn parse_spreadsheet(
    path: &Path,
) -> Result<(&'static str, String, &'static str), CliError> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("spreadsheet open: {e}")))?;
    let mut lines = Vec::new();
    // PAR-59: multi-sheet is sequential — calamine `Reader` is not Sync; opening
    // once and ranging sheets in order is correct and cost ≪ coordination for
    // typical agent workbooks. Do not `par_iter` worksheet_range on &mut self.
    // calamine::Reader::sheet_names returns owned `Vec<String>` — do not re-clone.
    let sheets = workbook.sheet_names();
    for name in sheets {
        if let Ok(range) = workbook.worksheet_range(&name) {
            lines.push(format!("# sheet: {name}"));
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|c| match c {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(dt) => format!("{dt:?}"),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                        Data::Error(e) => format!("{e:?}"),
                    })
                    .collect();
                lines.push(cells.join("\t"));
            }
        }
    }
    Ok(("spreadsheet", lines.join("\n"), "calamine"))
}
