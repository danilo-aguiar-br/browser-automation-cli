// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local file parse (html/md/pdf/docx/xlsx) one-shot, no Chrome.
//!
//! # Shape
//!
//! This file is DISPATCH. Each binary format that needs a real decoder lives in
//! its own submodule under `parse/`, because a defect in the PDF path should not
//! recompile the spreadsheet path and a reader looking for one decoder should
//! not scroll past the other three to find it.
//!
//! HTML has no submodule on purpose: it needs no decoder here. It reaches
//! [`super::html`], which already exists and is shared with the scrape path.
//! Creating an empty `parse/html.rs` to make the set look symmetric would be an
//! abstraction with one call site and no content.

mod docx;
mod pdf;
mod spreadsheet;

pub(crate) use docx::{parse_docx_bytes, parse_docx_bytes_with};
pub(crate) use pdf::parse_pdf_bytes;
pub(crate) use spreadsheet::parse_spreadsheet;

use std::fs;
use std::path::Path;

use scraper::Html;
use serde_json::{json, Value};

use crate::cache::{self, HttpCache};
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::html::{redact_pii, visible_text};

/// Parse local file (html/md/txt/csv/json/xml/pdf/docx/xlsx) one-shot, no Chrome.
///
/// # Errors
///
/// Propagates every condition of [`parse_file_opts`], which this call forwards
/// to with redaction off.
pub fn parse_file(path: &Path) -> Result<Value, CliError> {
    parse_file_opts(path, false)
}

/// Formats a non-HTML parse can honestly produce.
///
/// A PDF or a spreadsheet has no DOM, so `links`, `metadata`, `jsonld` and the
/// rest have nothing to read. Refusing them by name beats emitting an empty
/// array that an agent would read as "this document has no links".
const TEXT_ONLY_PARSE_FORMATS: &[&str] = &["text", "markdown", "md", "summary"];

/// Parse a local file and derive `scrape` formats from the result.
///
/// `parse` used to expose exactly one option, `--redact-pii`, while every
/// format the product knows how to derive sat one call away in
/// [`crate::scrape_local::build_formats_map`]. HTML input now takes the full
/// format surface; other kinds take the text-derived subset and reject the rest
/// with the reason.
///
/// # Errors
///
/// Returns [`ErrorKind::Usage`] when a format is unknown, or when a DOM-only
/// format is asked of a document that has no DOM.
pub fn parse_file_formats(
    path: &Path,
    redact: bool,
    formats: &[String],
) -> Result<Value, CliError> {
    let mut base = parse_file_opts(path, redact)?;
    if formats.is_empty() {
        return Ok(base);
    }
    let kind = base
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source = format!("file://{}", path.display());
    let refs: Vec<&str> = formats.iter().map(String::as_str).collect();

    let derived = if kind == "html" {
        // `build_formats_map` needs the raw HTML, which `parse_file_opts`
        // does not hand back — it returns extracted text — so the file is read
        // a second time here.
        //
        // That second read MUST carry its own ceiling. It used to be a bare
        // `fs::read` plus `from_utf8_lossy(..).into_owned()`: no size check at
        // all, and a third copy of the document in memory. The ceiling checked
        // in `parse_file_opts` said nothing about this read, so anything that
        // replaced the file in between — a symlink retarget, a rename over the
        // path, a build step writing the same name — was loaded whole. That is
        // a TOCTOU window, and the value it guards is an operator-set memory
        // limit, so re-checking is the point rather than an optimisation.
        //
        // Delegated to `read_text_file_limited_lossy`, and the LOSSY sibling is
        // load-bearing: the strict `read_text_file_limited` next to it finishes
        // with `read_to_string`, which errors on invalid bytes, while HTML
        // served as windows-1252 or latin-1 is ordinary. Same ceiling,
        // deliberately different decoder. The two are separate named functions
        // for exactly this reason — a caller reaching for the shared LIMIT
        // cannot pick up a stricter DECODER without spelling it out.
        //
        // The helper also owns the check-then-read pair, so the re-check above
        // and the read below can no longer drift apart.
        let max_parse_bytes =
            crate::xdg::policy::policy_usize(crate::xdg::policy::key::SCRAPE_MAX_PARSE_BYTES);
        let html = crate::json_util::read_text_file_limited_lossy(path, max_parse_bytes as u64)?;
        let opts = super::types::ScrapeOpts::default();
        super::build_formats_map(
            &source,
            200,
            &html,
            &refs,
            &opts,
            "local",
            RobotsPolicy::Ignore,
        )?
    } else {
        let text = base
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let mut out = serde_json::Map::new();
        for f in &refs {
            let norm = f.to_ascii_lowercase();
            if !TEXT_ONLY_PARSE_FORMATS.contains(&norm.as_str()) {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!(
                        "format `{f}` needs a DOM and `{}` has none; \
                         non-HTML parse accepts: {}",
                        kind,
                        TEXT_ONLY_PARSE_FORMATS.join(", ")
                    ),
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            let value = if norm == "summary" {
                let cap = crate::xdg::resolve_scrape_summary_chars();
                if text.chars().count() > cap {
                    format!("{}…", text.chars().take(cap).collect::<String>())
                } else {
                    text.clone()
                }
            } else {
                text.clone()
            };
            out.insert(norm.replace('-', "_"), json!(value));
        }
        out
    };

    if let Some(obj) = base.as_object_mut() {
        obj.insert("formats".into(), Value::Object(derived));
        obj.insert("format_list".into(), json!(formats));
    }
    Ok(base)
}

/// Parse local file with optional PII redaction.
///
/// # Errors
///
/// - [`ErrorKind::Io`] when the path cannot be stat'd or read
/// - [`ErrorKind::Usage`] when the path is not a regular file, or when the
///   extension is one this product has no decoder for
/// - [`ErrorKind::Data`] when the file exceeds the XDG `scrape_max_parse_bytes`
///   ceiling, and whatever the format backend reports for a malformed document
///
/// A cache miss is NOT an error: the entry is written best-effort and a failure
/// to write it leaves the parse result untouched.
pub fn parse_file_opts(path: &Path, redact: bool) -> Result<Value, CliError> {
    // GAP-026 on the READ side. `parse` returns the file's text in `data`, so an
    // unbounded path here is arbitrary file disclosure, not merely a read:
    // measured 2026-08-31, `parse /dev/shm/leak.txt` answered `ok: true` with the
    // contents inline while `run --script` refused that same directory with exit
    // 64. Same class as the write bypass in `image_local::atomic`, on the axis
    // nobody had checked.
    crate::fs_roots::ensure_read_allowed(path)?;
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
            let (kind, text, engine, pages, text_layer_empty) = parse_pdf_bytes(&bytes)?;
            extra["pages"] = json!(pages);
            extra["text_layer_empty"] = json!(text_layer_empty);
            (kind, text, engine)
        }
        // The ceiling is already resolved above; pass it rather than letting the
        // facade read the same knob a second time.
        "docx" => parse_docx_bytes_with(&bytes, max_parse_bytes)?,
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
                    // A local file has no redirect and no final URL to record.
                    final_url: None,
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
