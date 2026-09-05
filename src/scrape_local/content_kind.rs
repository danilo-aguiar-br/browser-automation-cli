// SPDX-License-Identifier: MIT OR Apache-2.0
//! Route a fetched body by what it IS, not by the format the caller asked for.
//!
//! # Why this module exists (G12)
//!
//! The scrape pipeline decided every body was HTML because the requested
//! FORMAT was an HTML format. Nothing ever consulted the type the server
//! actually sent. `Content-Type` was read one scope away, in
//! [`super::http`], handed to the charset decoder, and dropped.
//!
//! The observable result was the most expensive failure shape an agent can
//! meet: `scrape --format text` against a PDF returned `exit 0`, `ok: true`,
//! and a `text` field beginning `%PDF-1.5` followed by FlateDecode streams.
//! Every transport field reported success because transport HAD succeeded.
//! `--only-main-content` was a silent no-op, because it filters by DOM
//! selector and a PDF has no DOM. There was no field to branch on, so a
//! caller could not tell "I got the document" from "I got its bytes".
//!
//! This is the same family as the bot-check gap ([`super::block_detect`]):
//! the envelope measured the transport and never the content.
//!
//! # Why magic bytes outrank `Content-Type`
//!
//! Servers mislabel downloads constantly -- `application/octet-stream` for
//! PDFs is routine, and some CDNs answer `text/html` for anything they did
//! not generate. The first five bytes of a PDF cannot be wrong in the same
//! way. The header is consulted only when the bytes are inconclusive.

use serde_json::{json, Map, Value};

use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::types::{ScrapeFormat, ScrapeOpts};

/// What a fetched body actually is, decided from bytes first and header second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    /// Markup the HTML pipeline can parse. The only kind that keeps the old path.
    Html,
    /// Plain text with no markup: served as-is, never parsed as a document.
    Text,
    /// PDF, extracted through the same `lopdf` path the `parse` command uses.
    Pdf,
    /// OOXML word processing document, extracted like `parse` does.
    Docx,
    /// Recognised binary with no in-process extractor reachable from a fetch.
    ///
    /// Spreadsheets land here: `calamine` opens a `Path`, and materialising a
    /// temp file mid-scrape would trade a content bug for a residual-disk one.
    Opaque(&'static str),
}

impl ContentKind {
    /// Stable machine token emitted as `data.content_kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Text => "text",
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Opaque(name) => name,
        }
    }

    /// True when the HTML pipeline is the right consumer for this body.
    pub fn is_html(self) -> bool {
        matches!(self, Self::Html)
    }
}

/// Leading bytes of a ZIP container, shared by every OOXML format.
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";

/// Decide what the body is.
///
/// Order is deliberate: unambiguous magic, then the declared MIME, then a
/// sniff of the leading text. Defaulting to [`ContentKind::Html`] is safe only
/// as the LAST step, because that default is exactly what produced G12.
pub fn classify(content_type: Option<&str>, bytes: &[u8]) -> ContentKind {
    if bytes.starts_with(b"%PDF-") {
        return ContentKind::Pdf;
    }
    if bytes.starts_with(ZIP_MAGIC) {
        // Every OOXML format shares the ZIP magic, so the member list is what
        // separates them. Only the MIME is cheap enough to consult here.
        return match mime_of(content_type) {
            Some(m) if m.contains("wordprocessingml") => ContentKind::Docx,
            Some(m) if m.contains("spreadsheetml") || m.contains("ms-excel") => {
                ContentKind::Opaque("spreadsheet")
            }
            Some(m) if m.contains("presentationml") => ContentKind::Opaque("presentation"),
            _ => ContentKind::Opaque("archive"),
        };
    }
    if let Some(mime) = mime_of(content_type) {
        if mime.contains("html") || mime.contains("xhtml") {
            return ContentKind::Html;
        }
        if mime.contains("pdf") {
            return ContentKind::Pdf;
        }
        if mime.starts_with("image/") {
            return ContentKind::Opaque("image");
        }
        if mime.starts_with("audio/") {
            return ContentKind::Opaque("audio");
        }
        if mime.starts_with("video/") {
            return ContentKind::Opaque("video");
        }
        // Structured text the EXISTING pipeline already understands stays on the
        // default path. Diverting it here would not be routing, it would be
        // deleting a feature: `--format feed` parses RSS, Atom AND JSON Feed
        // downstream, and `--format json` reads JSON bodies the same way.
        //
        // Measured: routing `application/json` to `Text` made
        // `json_feed_is_parsed_and_projected` report `feed_found: null` for a
        // `/feed.json` fixture that had worked. The point of this module is to
        // stop bodies reaching a parser that cannot read them — not to take work
        // away from parsers that can.
        if mime.contains("xml") || mime.contains("json") {
            return ContentKind::Html;
        }
        if mime.starts_with("text/") {
            return ContentKind::Text;
        }
    }
    // No usable header. A body that opens with markup is markup; anything else
    // that is not valid UTF-8 is binary of an unknown shape.
    let head = &bytes[..bytes.len().min(1024)];
    let looks_markup = std::str::from_utf8(head)
        .map(|s| {
            let t = s.trim_start().to_ascii_lowercase();
            t.starts_with("<!doctype") || t.starts_with("<html") || t.starts_with("<?xml")
        })
        .unwrap_or(false);
    if looks_markup {
        return ContentKind::Html;
    }
    if std::str::from_utf8(head).is_err() {
        return ContentKind::Opaque("binary");
    }
    ContentKind::Html
}

/// Lowercased MIME type with parameters stripped (`; charset=utf-8`).
fn mime_of(content_type: Option<&str>) -> Option<String> {
    let raw = content_type?;
    let base = raw.split(';').next()?.trim();
    if base.is_empty() {
        return None;
    }
    Some(base.to_ascii_lowercase())
}

/// Text extracted from a non-HTML body, plus the page count when meaningful.
#[derive(Debug)]
pub struct Extracted {
    /// Human-readable text, ready to serve as `text` / `markdown`.
    pub text: String,
    /// Page count for paginated documents; `None` for everything else.
    pub pages: Option<usize>,
    /// True when the document parsed but carries no text layer.
    ///
    /// A scanned PDF is the case: extraction succeeds and returns nothing.
    /// Reporting that as empty content would repeat G12 one level down.
    pub text_layer_empty: bool,
}

/// Extract text for a kind the pipeline can handle in process.
///
/// [`ContentKind::Opaque`] never reaches here: it is refused earlier with a
/// message that names the command which CAN read it. Returning raw bytes under
/// a text key is the one outcome this module exists to prevent.
pub fn extract(kind: ContentKind, bytes: &[u8]) -> Result<Extracted, CliError> {
    match kind {
        ContentKind::Pdf => {
            let (_, text, _, pages, empty) = super::parse::parse_pdf_bytes(bytes)?;
            Ok(Extracted {
                text,
                pages: Some(pages),
                text_layer_empty: empty,
            })
        }
        ContentKind::Docx => {
            let (_, text, _) = super::parse::parse_docx_bytes(bytes)?;
            let empty = text.trim().is_empty();
            Ok(Extracted {
                text,
                pages: None,
                text_layer_empty: empty,
            })
        }
        ContentKind::Text => {
            let text = String::from_utf8_lossy(bytes).into_owned();
            let empty = text.trim().is_empty();
            Ok(Extracted {
                text,
                pages: None,
                text_layer_empty: empty,
            })
        }
        ContentKind::Html => Err(CliError::new(
            ErrorKind::Software,
            "extract called for HTML; the HTML pipeline owns that path",
        )),
        ContentKind::Opaque(name) => Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!("cannot extract text from {name} content in a scrape"),
            crate::i18n::suggestion_key("scrape_opaque_content", None),
        )),
    }
}

/// Formats that only exist for markup, and therefore cannot be served here.
///
/// Named rather than silently dropped: an absent key is indistinguishable from
/// an empty result, which is the ambiguity this whole module removes.
fn html_only(format: ScrapeFormat) -> Option<&'static str> {
    match format {
        ScrapeFormat::Html => Some("html"),
        ScrapeFormat::RawHtml => Some("rawHtml"),
        ScrapeFormat::Links => Some("links"),
        ScrapeFormat::Images => Some("images"),
        ScrapeFormat::JsonLd => Some("jsonld"),
        ScrapeFormat::Product => Some("product"),
        ScrapeFormat::Branding => Some("branding"),
        ScrapeFormat::Feed => Some("feed"),
        ScrapeFormat::Screenshot => Some("screenshot"),
        _ => None,
    }
}

/// Build the envelope payload for a body that is not markup.
///
/// Keeps the transport keys identical to the HTML path so an agent can read
/// `source_url`, `status_code` and `http_error` without branching, and adds
/// `content_kind` so it CAN branch when it needs to.
pub fn build_payload(
    source_url: &str,
    status: u16,
    kind: ContentKind,
    content_type: Option<&str>,
    extracted: &Extracted,
    opts: &ScrapeOpts,
    robots: RobotsPolicy,
) -> Value {
    // Borrow when nothing rewrites the body. `redact_pii` is off by default, so
    // the old `extracted.text.clone()` copied the whole page on the common path
    // just to hand it to the branch below, which copies again into the JSON.
    let text: std::borrow::Cow<'_, str> = if opts.redact_pii {
        std::borrow::Cow::Owned(super::html::redact_pii(&extracted.text))
    } else {
        std::borrow::Cow::Borrowed(extracted.text.as_str())
    };

    let mut map = Map::new();
    map.insert("source_url".into(), json!(source_url));
    map.insert("status_code".into(), json!(status));
    map.insert("robots_policy".into(), json!(robots.as_str()));
    map.insert("engine".into(), json!(opts.engine));
    // Non-HTML content came down the same transport as HTML, so it carries the
    // same disguise coverage. Omitting the disclosure here made a PDF scrape
    // look like a route with no stated limits at all.
    super::disclosure::insert_disguise_disclosure(&mut map, opts.engine_kind());
    map.insert("http_error".into(), json!(false));
    map.insert(
        "format".into(),
        json!(format!("{:?}", opts.format).to_ascii_lowercase()),
    );
    map.insert("content_kind".into(), json!(kind.as_str()));
    if let Some(ct) = content_type {
        map.insert("content_type".into(), json!(ct));
    }
    if let Some(pages) = extracted.pages {
        map.insert("page_count".into(), json!(pages));
    }
    if extracted.text_layer_empty {
        // Extraction succeeded and produced nothing. A scanned PDF reaches this
        // branch, and the caller must not read the empty string as "no content
        // on the page".
        map.insert("text_layer_empty".into(), json!(true));
    }

    if let Some(name) = html_only(opts.format) {
        map.insert("unsupported_format".into(), json!(name));
        map.insert("text".into(), json!(text));
        return Value::Object(map);
    }

    match opts.format {
        ScrapeFormat::Markdown => {
            // Extracted text carries no markup, so markdown and text are the
            // same string. Emitting both keeps the key contract stable.
            map.insert("markdown".into(), json!(&text));
            map.insert("text".into(), json!(text));
        }
        ScrapeFormat::Metadata => {
            let mut meta = Map::new();
            meta.insert("status_code".into(), json!(status));
            meta.insert("source_url".into(), json!(source_url));
            meta.insert("content_kind".into(), json!(kind.as_str()));
            if let Some(ct) = content_type {
                meta.insert("content_type".into(), json!(ct));
            }
            if let Some(pages) = extracted.pages {
                meta.insert("page_count".into(), json!(pages));
            }
            map.insert("metadata".into(), Value::Object(meta));
        }
        ScrapeFormat::Summary => {
            let cap = crate::xdg::resolve_scrape_summary_chars();
            let summary = if text.chars().count() > cap {
                format!("{}…", text.chars().take(cap).collect::<String>())
            } else {
                text.clone().into_owned()
            };
            map.insert("summary".into(), json!(summary));
            map.insert("text".into(), json!(text));
        }
        _ => {
            map.insert("text".into(), json!(text));
        }
    }

    super::project::apply_max_text_chars(Value::Object(map), opts.max_text_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_magic_beats_a_lying_content_type() {
        // The exact shape G12 shipped: server says HTML, bytes say PDF.
        let kind = classify(Some("text/html; charset=utf-8"), b"%PDF-1.5\n1 0 obj");
        assert_eq!(kind, ContentKind::Pdf);
    }

    #[test]
    fn pdf_recognised_from_mime_without_magic() {
        assert_eq!(
            classify(Some("application/pdf"), b"garbage"),
            ContentKind::Pdf
        );
    }

    #[test]
    fn html_stays_html() {
        assert!(classify(Some("text/html"), b"<!doctype html><html>").is_html());
        assert!(classify(None, b"<!DOCTYPE HTML>\n<html>").is_html());
    }

    #[test]
    fn plain_text_is_not_parsed_as_markup() {
        assert_eq!(classify(Some("text/plain"), b"hello"), ContentKind::Text);
    }

    #[test]
    fn json_stays_on_the_default_path_like_xml() {
        // This module routes AWAY from parsers that cannot read a body. JSON is
        // the opposite case: `--format feed` parses JSON Feed and `--format
        // json` parses JSON, both downstream on the default path.
        //
        // An earlier revision classified `application/json` as `Text`, which
        // made `--format feed` return `feed_found: null` for a `/feed.json`
        // fixture. Routing is not the same as removing.
        assert!(classify(Some("application/json"), b"{}").is_html());
        assert!(classify(Some("application/feed+json"), b"{}").is_html());
        assert!(classify(Some("application/rss+xml"), b"<rss/>").is_html());
    }

    #[test]
    fn spreadsheet_is_opaque_not_docx() {
        let ct = Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
        assert_eq!(
            classify(ct, b"PK\x03\x04rest"),
            ContentKind::Opaque("spreadsheet")
        );
    }

    #[test]
    fn docx_recognised_by_mime_over_zip_magic() {
        let ct = Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
        assert_eq!(classify(ct, b"PK\x03\x04rest"), ContentKind::Docx);
    }

    #[test]
    fn xml_feeds_keep_the_html_path() {
        assert!(classify(Some("application/rss+xml"), b"<rss>").is_html());
    }

    #[test]
    fn non_utf8_without_a_header_is_binary_not_html() {
        // The old default assumed HTML here, which is how raw bytes reached the
        // `text` key with exit 0.
        assert_eq!(
            classify(None, &[0xff, 0xfe, 0x00, 0x01]),
            ContentKind::Opaque("binary")
        );
    }

    #[test]
    fn mime_parameters_are_stripped() {
        assert_eq!(
            mime_of(Some("text/HTML; charset=UTF-8")),
            Some("text/html".into())
        );
        assert_eq!(mime_of(Some("  ")), None);
        assert_eq!(mime_of(None), None);
    }

    #[test]
    fn opaque_refuses_extraction_and_names_a_way_out() {
        let err = extract(ContentKind::Opaque("image"), b"\x89PNG").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Data);
        assert!(err.message().contains("image"));
    }

    #[test]
    fn text_extraction_is_lossless_for_utf8() {
        let got = extract(ContentKind::Text, "olá mundo".as_bytes()).unwrap();
        assert_eq!(got.text, "olá mundo");
        assert!(!got.text_layer_empty);
    }
}
