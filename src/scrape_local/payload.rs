// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scrape payload construction: HTML in, per-format agent envelope data out.

use scraper::Html;
use serde_json::{json, Map, Value};

use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::disclosure::insert_disguise_disclosure;
use super::html::{
    content_hash_hex, extract_all_json_ld, extract_branding_hints, extract_images,
    extract_json_ld_product, extract_links, extract_main_html, filter_html_by_selectors,
    html_to_markdown_simple, meta_content, redact_pii, text_of_first, visible_text,
};
use super::project::apply_max_text_chars;
use super::types::{ScrapeFormat, ScrapeOpts};

/// Run [`build_scrape_payload`] on Tokio's blocking pool (CPU-bound HTML parse).
///
/// Admission is gated by [`crate::concurrency::spawn_cpu_blocking`]: a crawl or
/// `batch-scrape` fan-out would otherwise admit one parser per blocking thread
/// (up to 16) regardless of how many cores the host actually has.
pub(super) async fn build_scrape_payload_blocking(
    source_url: String,
    status: u16,
    html: String,
    opts: ScrapeOpts,
    robots: RobotsPolicy,
) -> Result<Value, CliError> {
    crate::concurrency::spawn_cpu_blocking(move || {
        build_scrape_payload(&source_url, status, &html, &opts, robots)
    })
    .await
    .map_err(|e| {
        if e.is_panic() {
            CliError::new(ErrorKind::Software, "scrape HTML parse task panicked")
        } else {
            CliError::new(ErrorKind::Software, format!("scrape HTML parse join: {e}"))
        }
    })
}

/// Build agent envelope data from HTML (CLEAN: omit null keys).
pub fn build_scrape_payload(
    source_url: &str,
    status: u16,
    html: &str,
    opts: &ScrapeOpts,
    robots: RobotsPolicy,
) -> Value {
    let document = Html::parse_document(html);
    let title = text_of_first(&document, "title");
    let description = meta_content(&document, "description")
        .or_else(|| meta_content(&document, "og:description"))
        .unwrap_or_default();
    // `--only-main-content` is a HEURISTIC: the caller asked for "the article",
    // not for a named element, so answering with the whole document when the
    // page ships no <main> is defensible. Staying SILENT about which of the two
    // happened was not, so `main_content_found` now says it in the envelope.
    // Contrast with the include selector below, which names an exact target and
    // therefore fails closed instead of widening.
    let main_content = if opts.only_main_content {
        extract_main_html(&document)
    } else {
        None
    };
    let main_content_found = main_content.is_some();
    let mut body_html = main_content.unwrap_or_else(|| html.to_string());
    let selector_filter =
        filter_html_by_selectors(&body_html, &opts.include_selectors, &opts.exclude_selectors);
    body_html = selector_filter.html;
    let body_doc = Html::parse_document(&body_html);
    let mut text = visible_text(&body_doc);
    let mut markdown = html_to_markdown_simple(&body_html, &title);
    if opts.redact_pii {
        text = redact_pii(&text);
        markdown = redact_pii(&markdown);
    }
    // Selector-scoped when the caller asked for a subset, whole-document
    // otherwise. Until 0.1.9 this always read `document`, so
    // `--format links --include-selector <sel>` returned every link on the
    // page with `ok: true` and no warning: the caller asked for a subset,
    // received the whole set, and nothing in the envelope marked the
    // difference. Note the contrast with `ScrapeFormat::Attributes` below,
    // which reads the full document *on purpose* and says so — that one is a
    // decision, this one was an omission.
    //
    // Shared with `ScrapeFormat::Images`, which is BODY content under exactly
    // the same rule; hence `scoped_doc` and not a links-specific name.
    let scoped_doc = if opts.include_selectors.is_empty() && opts.exclude_selectors.is_empty() {
        &document
    } else {
        &body_doc
    };
    let links = extract_links(source_url, scoped_doc, opts.honor_nofollow);
    // Computed from the *full* document (not the selector-reduced body) because
    // `<link rel="next">` lives in <head>, which only_main_content strips.
    let rel_next = if opts.follow_rel_next {
        super::rel_next::extract_rel_next(source_url, &document)
    } else {
        Vec::new()
    };

    let mut map = Map::new();
    map.insert("source_url".into(), json!(source_url));
    map.insert("status_code".into(), json!(status));
    map.insert("title".into(), json!(title));
    map.insert("robots_policy".into(), json!(robots.as_str()));
    map.insert("engine".into(), json!(opts.engine));
    insert_disguise_disclosure(&mut map, opts.engine_kind());
    // Explicit false so `--filter http_error=false` keeps OK rows (agent CLEAN).
    map.insert("http_error".into(), json!(false));
    map.insert(
        "format".into(),
        json!(format!("{:?}", opts.format).to_ascii_lowercase()),
    );
    // Witnesses for the reductions the caller ASKED for, emitted only when the
    // caller asked (CLEAN: omit null keys). These are what let an agent tell an
    // empty result that means "no match" apart from an empty page.
    if opts.only_main_content {
        map.insert("main_content_found".into(), json!(main_content_found));
    }
    if !opts.include_selectors.is_empty() {
        map.insert("selector_matched".into(), json!(selector_filter.matched));
        map.insert(
            "selector_match_count".into(),
            json!(selector_filter.match_count),
        );
    }

    match opts.format {
        ScrapeFormat::Text => {
            map.insert("text".into(), json!(text));
        }
        ScrapeFormat::Markdown => {
            map.insert("markdown".into(), json!(markdown));
            map.insert("text".into(), json!(text));
        }
        ScrapeFormat::Html => {
            map.insert("html".into(), json!(body_html));
        }
        ScrapeFormat::RawHtml => {
            // `html` is the response body before main-content extraction and
            // before selector filtering; `body_html` is after both. Emitting it
            // under its own key is what makes "raw" checkable by the caller.
            map.insert("rawHtml".into(), json!(html));
        }
        ScrapeFormat::Links => {
            map.insert("links".into(), json!(links));
            map.insert("link_count".into(), json!(links.len()));
        }
        ScrapeFormat::Metadata => {
            // Start from what the document declares, then overlay the fields
            // only the fetch knows (status, resolved URL, link count). Emitting
            // five hardcoded keys while og/dc/article/canonical/favicon sat
            // unread in the same parsed document was the whole defect.
            //
            // Reads the FULL document *on purpose*, like `Attributes` and
            // `rel_next`: metadata lives in <head>, which selector reduction of
            // the body would delete outright. Scoping it would not narrow the
            // answer, it would empty it.
            let mut meta = super::html_meta::collect_metadata(&document);
            meta.insert("title".into(), json!(title));
            meta.insert("description".into(), json!(description));
            meta.insert("status_code".into(), json!(status));
            meta.insert("source_url".into(), json!(source_url));
            meta.insert("link_count".into(), json!(links.len()));
            map.insert("metadata".into(), Value::Object(meta));
        }
        ScrapeFormat::Screenshot => {
            map.insert("text".into(), json!(text));
            map.insert(
                "screenshot".into(),
                json!({
                    "note": "screenshot format requires --engine browser; use grab for explicit capture",
                }),
            );
        }
        ScrapeFormat::Summary => {
            let cap = crate::xdg::resolve_scrape_summary_chars();
            let summary = if text.chars().count() > cap {
                format!("{}…", text.chars().take(cap).collect::<String>())
            } else {
                text.clone()
            };
            map.insert("summary".into(), json!(summary));
            map.insert("text".into(), json!(text));
            map.insert("llm_required_for_full".into(), json!(true));
        }
        ScrapeFormat::Product => {
            // Reads the RAW body *on purpose*, like `Feed`: JSON-LD ships in
            // <script type="application/ld+json">, which selector reduction of
            // the body would drop. A caller narrowing the visible article is
            // not asking to hide the page's structured product data.
            let product = extract_json_ld_product(html);
            map.insert("product".into(), product);
            map.insert("text".into(), json!(text));
        }
        ScrapeFormat::Branding => {
            // Reads the RAW body *on purpose*: branding hints are favicon,
            // logo and theme colour, which live in <head> and outside whatever
            // subset the caller scoped the body to.
            map.insert("branding".into(), extract_branding_hints(html, &title));
            map.insert("text".into(), json!(text));
        }
        ScrapeFormat::Images => {
            // BODY content, so it follows the same rule as `links` above: an
            // `--include-selector 'article'` that still answered with every
            // image on the page would answer a different question than the one
            // asked.
            let images = extract_images(source_url, scoped_doc);
            map.insert("images".into(), json!(images));
            map.insert("image_count".into(), json!(images.len()));
        }
        ScrapeFormat::JsonLd => {
            // Reads the RAW body *on purpose*, for the same reason as
            // `Product` above: JSON-LD blocks are document-level and survive no
            // body reduction.
            let blocks = extract_all_json_ld(html);
            map.insert("jsonld".into(), json!(blocks));
            map.insert("jsonld_count".into(), json!(blocks.len()));
        }
        ScrapeFormat::Feed => {
            // Parses the *raw* body: selector/only-main reduction is an HTML
            // notion and would destroy an XML feed document.
            let feed =
                super::feed::extract_feed(html, crate::xdg::resolve_scrape_feed_max_entries());
            let found = feed.get("found").and_then(Value::as_bool).unwrap_or(false);
            map.insert("feed".into(), feed);
            map.insert("feed_found".into(), json!(found));
        }
        ScrapeFormat::Attributes => {
            // Read from the FULL document, not the selector-reduced body: the
            // caller already named the elements it wants, and letting
            // `--only-main-content` silently remove them would answer a
            // different question than the one asked.
            let extracted = super::attributes::extract_attributes(html, &opts.attribute_targets);
            let total: usize = extracted
                .iter()
                .filter_map(|e| e.get("values").and_then(Value::as_array))
                .map(Vec::len)
                .sum();
            map.insert("attributes".into(), json!(extracted));
            map.insert("attribute_count".into(), json!(total));
        }
        ScrapeFormat::Json => {
            // Placeholder: handler fills structured JSON via LLM when schema provided.
            map.insert(
                "json".into(),
                json!({
                    "note": "use --schema-json and/or --question with format json (LLM path)",
                    "text": text,
                    "markdown": markdown,
                }),
            );
        }
    }

    // Emitted for every format so crawl can continue a paginated series
    // regardless of what the operator asked the pages to render as.
    if !rel_next.is_empty() {
        map.insert("rel_next".into(), json!(rel_next));
    }

    if opts.with_content_hash {
        let basis = if !markdown.is_empty() {
            markdown.as_str()
        } else {
            text.as_str()
        };
        map.insert("content_hash".into(), json!(content_hash_hex(basis)));
    }

    let value = Value::Object(map);
    apply_max_text_chars(value, opts.max_text_chars)
}
