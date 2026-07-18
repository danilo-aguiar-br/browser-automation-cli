//! Local scrape/crawl/map/search/parse (one-shot HTTP and file extract; no SaaS).
//!
//! Engines:
//! - `http` — reqwest + scraper HTML (no Chrome)
//! - `browser` — chromiumoxide via [`crate::browser::OneShotSession`]

use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::Path;
use std::time::Duration;

use scraper::{Html, Selector};
use serde_json::{json, Value};
use url::Url;

use crate::cache::{self, HttpCache};
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

/// Identifiable product User-Agent for HTTP scrapes (PRD politeness).
pub const HTTP_USER_AGENT: &str =
    "browser-automation-cli/0.1.3 (+https://github.com/danilo-aguiar-br/browser-automation-cli; local-scrape)";

/// Reject `file://`, bare paths, and other non-HTTP(S) targets for the HTTP engine (GAP-A004).
pub fn reject_non_http_scheme_for_http_engine(url: &str) -> Result<(), CliError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "empty URL for scrape --engine http",
            "Pass an absolute http(s) URL",
        ));
    }
    // Bare local path (not a URL).
    if !trimmed.contains("://") {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("HTTP engine cannot fetch local path: {trimmed}"),
            "Use: browser-automation-cli parse <path>   or   scrape file:///… --engine browser",
        ));
    }
    match Url::parse(trimmed) {
        Ok(u) => match u.scheme() {
            "http" | "https" => Ok(()),
            "file" => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("HTTP engine cannot fetch file:// URL: {trimmed}"),
                "Use: browser-automation-cli scrape <url> --engine browser   or   parse <path>",
            )),
            other => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("HTTP engine does not support scheme `{other}`"),
                "Pass an absolute http(s) URL, or use --engine browser / parse for local files",
            )),
        },
        Err(e) => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid URL for HTTP scrape: {e}"),
            "Pass an absolute http(s) URL",
        )),
    }
}

/// Output formats for local scrape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrapeFormat {
    /// Visible/plain text.
    Text,
    /// Simplified markdown.
    Markdown,
    /// Raw HTML body.
    Html,
    /// Extracted anchor links.
    Links,
    /// Title / description / status metadata.
    Metadata,
    /// Screenshot path placeholder (browser engine fills via CDP grab).
    Screenshot,
    /// LLM-oriented short summary (requires --llm path or offline stub from title/text).
    Summary,
    /// Product fields from JSON-LD Product schema when present.
    Product,
    /// Branding colors/fonts heuristics from HTML.
    Branding,
}

impl ScrapeFormat {
    /// Parse from CLI flag (comma-separated first token).
    pub fn parse(s: &str) -> Result<Self, CliError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" | "body" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" | "raw-html" | "rawhtml" | "raw_html" | "rawHtml" => Ok(Self::Html),
            "links" => Ok(Self::Links),
            "metadata" | "meta" => Ok(Self::Metadata),
            "screenshot" | "shot" | "image" => Ok(Self::Screenshot),
            "summary" => Ok(Self::Summary),
            "product" => Ok(Self::Product),
            "branding" => Ok(Self::Branding),
            other => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown scrape format: {other}"),
                "Use text|markdown|html|raw-html|links|metadata|screenshot|summary|product|branding",
            )),
        }
    }
}

/// Shared scrape options.
#[derive(Debug, Clone)]
pub struct ScrapeOpts {
    /// Output format.
    pub format: ScrapeFormat,
    /// Prefer only main content heuristics.
    pub only_main_content: bool,
    /// Engine: "http" or "browser".
    pub engine: String,
    /// Max body bytes for HTTP.
    pub max_body_bytes: usize,
}

impl Default for ScrapeOpts {
    fn default() -> Self {
        Self {
            format: ScrapeFormat::Text,
            only_main_content: false,
            engine: "browser".into(),
            max_body_bytes: 5_000_000,
        }
    }
}

/// HTTP static scrape (no Chrome).
pub async fn scrape_http(
    url: &str,
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
) -> Result<Value, CliError> {
    // GAP-A004: reject non-HTTP(S) schemes early with an agent-usable suggestion.
    reject_non_http_scheme_for_http_engine(url)?;

    crate::robots::enforce_robots(url, robots, HTTP_USER_AGENT).await?;

    // GAP-011: layered XDG cache for GET scrape (hit skips network).
    let cache_key = cache::CacheKey::http_get(url);
    if let Ok(cache) = cache::default_cache() {
        if let Ok(Some(entry)) = HttpCache::get(cache.as_ref(), &cache_key) {
            if let Ok(html) = String::from_utf8(entry.body) {
                let mut payload = build_scrape_payload(url, 200, &html, opts, robots);
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("cache_hit".into(), json!(true));
                }
                return Ok(payload);
            }
        }
    }

    let client = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("http client: {e}")))?;

    // GAP-013: retry transient HTTP failures with named policy.
    let cfg = crate::retry::RetryConfig::http();
    let mut attempt = 0u32;
    let (status, final_url, bytes) = loop {
        attempt += 1;
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let final_url = resp.url().to_string();
                match resp.bytes().await {
                    Ok(bytes) => break (status, final_url, bytes),
                    Err(e) => {
                        let err = format!("read body: {e}");
                        if attempt >= cfg.max_attempts || !crate::retry::is_retryable_message(&err)
                        {
                            return Err(CliError::new(ErrorKind::Io, err));
                        }
                    }
                }
            }
            Err(e) => {
                let err = format!("GET {url}: {e}");
                if attempt >= cfg.max_attempts || !crate::retry::is_retryable_message(&err) {
                    return Err(CliError::new(ErrorKind::Unavailable, err));
                }
            }
        }
        tokio::time::sleep(cfg.delay_for_attempt(attempt.saturating_sub(1))).await;
    };
    if bytes.len() > opts.max_body_bytes {
        return Err(CliError::new(
            ErrorKind::Data,
            format!(
                "body {} exceeds max_body_bytes {}",
                bytes.len(),
                opts.max_body_bytes
            ),
        ));
    }
    let html = String::from_utf8_lossy(&bytes).into_owned();
    if let Ok(cache) = cache::default_cache() {
        let _ = HttpCache::put(
            cache.as_ref(),
            &cache_key,
            cache::CacheEntry {
                body: html.as_bytes().to_vec(),
                content_type: Some("text/html".into()),
                expires_unix: cache::expires_after(Duration::from_secs(3600)),
            },
        );
    }
    let mut payload = build_scrape_payload(&final_url, status, &html, opts, robots);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("cache_hit".into(), json!(false));
        obj.insert("http_attempts".into(), json!(attempt));
    }
    Ok(payload)
}

/// Build agent envelope data from HTML.
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
    let body_html = if opts.only_main_content {
        extract_main_html(&document).unwrap_or_else(|| html.to_string())
    } else {
        html.to_string()
    };
    let body_doc = Html::parse_document(&body_html);
    let text = visible_text(&body_doc);
    let markdown = html_to_markdown_simple(&body_html, &title);
    let links = extract_links(source_url, &document);

    let mut data = json!({
        "source_url": source_url,
        "status_code": status,
        "title": title,
        "robots_policy": robots.as_str(),
        "engine": opts.engine,
        "format": format!("{:?}", opts.format).to_ascii_lowercase(),
    });

    match opts.format {
        ScrapeFormat::Text => {
            data["text"] = json!(text);
        }
        ScrapeFormat::Markdown => {
            data["markdown"] = json!(markdown);
            data["text"] = json!(text);
        }
        ScrapeFormat::Html => {
            data["html"] = json!(body_html);
        }
        ScrapeFormat::Links => {
            data["links"] = json!(links);
        }
        ScrapeFormat::Metadata => {
            data["metadata"] = json!({
                "title": title,
                "description": description,
                "status_code": status,
                "source_url": source_url,
                "link_count": links.len(),
            });
        }
        ScrapeFormat::Screenshot => {
            // Browser path attaches path after grab; HTTP engine notes unsupported.
            data["text"] = json!(text);
            data["screenshot"] = json!({
                "note": "screenshot format requires --engine browser; use grab for explicit capture",
                "path": null,
            });
        }
        ScrapeFormat::Summary => {
            let summary = if text.len() > 400 {
                format!("{}…", text.chars().take(400).collect::<String>())
            } else {
                text.clone()
            };
            data["summary"] = json!(summary);
            data["text"] = json!(text);
            data["llm_required_for_full"] = json!(true);
        }
        ScrapeFormat::Product => {
            data["product"] = extract_json_ld_product(html);
            data["text"] = json!(text);
        }
        ScrapeFormat::Branding => {
            data["branding"] = extract_branding_hints(html, &title);
            data["text"] = json!(text);
        }
    }
    data
}

fn extract_json_ld_product(html: &str) -> Value {
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse("script[type=\"application/ld+json\"]") else {
        return json!({ "found": false });
    };
    for el in doc.select(&sel) {
        let raw = el.text().collect::<String>();
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            if is_product_ld(&v) {
                return json!({ "found": true, "json_ld": v });
            }
            if let Some(arr) = v.as_array() {
                for item in arr {
                    if is_product_ld(item) {
                        return json!({ "found": true, "json_ld": item });
                    }
                }
            }
            if let Some(graph) = v.get("@graph").and_then(|g| g.as_array()) {
                for item in graph {
                    if is_product_ld(item) {
                        return json!({ "found": true, "json_ld": item });
                    }
                }
            }
        }
    }
    json!({ "found": false, "json_ld": null })
}

fn is_product_ld(v: &Value) -> bool {
    match v.get("@type") {
        Some(Value::String(s)) => s.eq_ignore_ascii_case("Product"),
        Some(Value::Array(a)) => a.iter().any(|x| {
            x.as_str()
                .map(|s| s.eq_ignore_ascii_case("Product"))
                .unwrap_or(false)
        }),
        _ => false,
    }
}

fn extract_branding_hints(html: &str, title: &str) -> Value {
    let mut colors = BTreeSet::new();
    let re = regex::Regex::new(r"#[0-9A-Fa-f]{3,8}\b").ok();
    if let Some(re) = re {
        for m in re.find_iter(html).take(32) {
            colors.insert(m.as_str().to_string());
        }
    }
    json!({
        "title": title,
        "color_samples": colors.into_iter().collect::<Vec<_>>(),
        "note": "heuristic branding; not a full brand kit",
    })
}

/// Redact common PII patterns in text (email, phone, card-like digits).
pub fn redact_pii(text: &str) -> String {
    let email = regex::Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").ok();
    let phone = regex::Regex::new(
        r"\b(?:\+?\d{1,3}[-.\s]?)?(?:\(?\d{2,4}\)?[-.\s]?)?\d{3,4}[-.\s]?\d{4}\b",
    )
    .ok();
    let card = regex::Regex::new(r"\b(?:\d[ -]*?){13,19}\b").ok();
    let mut out = text.to_string();
    if let Some(re) = email {
        out = re.replace_all(&out, "[REDACTED_EMAIL]").into_owned();
    }
    if let Some(re) = phone {
        out = re.replace_all(&out, "[REDACTED_PHONE]").into_owned();
    }
    if let Some(re) = card {
        out = re.replace_all(&out, "[REDACTED_CARD]").into_owned();
    }
    out
}

fn text_of_first(doc: &Html, sel: &str) -> String {
    let Ok(selector) = Selector::parse(sel) else {
        return String::new();
    };
    doc.select(&selector)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default()
}

fn meta_content(doc: &Html, name: &str) -> Option<String> {
    let sel =
        format!("meta[name=\"{name}\"], meta[property=\"{name}\"], meta[property=\"og:{name}\"]");
    let Ok(selector) = Selector::parse(&sel) else {
        return None;
    };
    doc.select(&selector)
        .find_map(|e| e.value().attr("content").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

fn extract_main_html(doc: &Html) -> Option<String> {
    for sel in ["main", "article", "[role=main]", "#content", ".content"] {
        if let Ok(selector) = Selector::parse(sel) {
            if let Some(el) = doc.select(&selector).next() {
                return Some(el.html());
            }
        }
    }
    None
}

fn visible_text(doc: &Html) -> String {
    let Ok(selector) = Selector::parse("body") else {
        return String::new();
    };
    let text = doc
        .select(&selector)
        .next()
        .map(|e| e.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn html_to_markdown_simple(html: &str, title: &str) -> String {
    let doc = Html::parse_document(html);
    let mut out = String::new();
    if !title.is_empty() {
        out.push_str("# ");
        out.push_str(title);
        out.push_str("\n\n");
    }
    // Headings (static selectors avoid SelectorErrorKind lifetime on dynamic strings).
    const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];
    for (idx, sel) in HEADINGS.iter().enumerate() {
        let level = idx + 1;
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };
        for el in doc.select(&selector) {
            let t = el.text().collect::<String>().trim().to_string();
            if !t.is_empty() {
                out.push_str(&"#".repeat(level));
                out.push(' ');
                out.push_str(&t);
                out.push_str("\n\n");
            }
        }
    }
    // Paragraphs
    if let Ok(selector) = Selector::parse("p") {
        for el in doc.select(&selector) {
            let t = el.text().collect::<String>();
            let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
            if !t.is_empty() {
                out.push_str(&t);
                out.push_str("\n\n");
            }
        }
    }
    if out.trim().is_empty() {
        out = visible_text(&doc);
    }
    out
}

fn extract_links(base: &str, doc: &Html) -> Vec<Value> {
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let base_url = Url::parse(base).ok();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for el in doc.select(&selector) {
        let href = el.value().attr("href").unwrap_or("").trim();
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            continue;
        }
        let abs = match (&base_url, Url::parse(href)) {
            (_, Ok(u)) if u.scheme() == "http" || u.scheme() == "https" || u.scheme() == "file" => {
                u.to_string()
            }
            (Some(b), _) => b
                .join(href)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| href.to_string()),
            _ => href.to_string(),
        };
        if seen.insert(abs.clone()) {
            let text = el.text().collect::<String>();
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            out.push(json!({ "url": abs, "text": text }));
        }
    }
    out
}

/// Batch scrape N URLs (HTTP engine, sequential by default).
pub async fn batch_scrape_http(
    urls: &[String],
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
    concurrency: usize,
) -> Result<Value, CliError> {
    let concurrency = concurrency.clamp(1, 16);
    let mut results = Vec::new();
    let mut errors = Vec::new();
    // Bounded concurrency via JoinSet (shutdown-friendly).
    use tokio::task::JoinSet;
    let mut set: JoinSet<Result<Value, CliError>> = JoinSet::new();
    let mut in_flight = 0usize;
    let mut idx = 0usize;
    while idx < urls.len() || in_flight > 0 {
        while in_flight < concurrency && idx < urls.len() {
            let u = urls[idx].clone();
            idx += 1;
            let opts = opts.clone();
            set.spawn(async move { scrape_http(&u, robots, &opts).await });
            in_flight += 1;
        }
        if let Some(joined) = set.join_next().await {
            in_flight = in_flight.saturating_sub(1);
            match joined {
                Ok(Ok(v)) => results.push(v),
                Ok(Err(e)) => errors.push(json!({ "error": e.to_string() })),
                Err(e) => errors.push(json!({ "error": format!("join: {e}") })),
            }
        }
    }
    Ok(json!({
        "ok": errors.is_empty(),
        "count": results.len(),
        "error_count": errors.len(),
        "results": results,
        "errors": errors,
        "engine": "http",
        "robots_policy": robots.as_str(),
    }))
}

/// BFS crawl from seed URL (HTTP), limited by `limit` and `max_depth`.
pub async fn crawl_http(
    seed: &str,
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
    limit: usize,
    max_depth: usize,
    same_host: bool,
) -> Result<Value, CliError> {
    let limit = limit.clamp(1, 500);
    let max_depth = max_depth.min(10);
    let seed_url = Url::parse(seed)
        .map_err(|e| CliError::new(ErrorKind::Usage, format!("invalid seed URL: {e}")))?;
    let seed_host = seed_url.host_str().map(|s| s.to_string());

    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    queue.push_back((seed.to_string(), 0));
    seen.insert(seed.to_string());

    let mut pages = Vec::new();
    while let Some((url, depth)) = queue.pop_front() {
        if pages.len() >= limit {
            break;
        }
        match scrape_http(&url, robots, opts).await {
            Ok(mut page) => {
                page["depth"] = json!(depth);
                if let Some(links) = page.get("links").and_then(|v| v.as_array()).cloned() {
                    if depth < max_depth {
                        for link in links {
                            let Some(href) = link.get("url").and_then(|v| v.as_str()) else {
                                continue;
                            };
                            if !seen.insert(href.to_string()) {
                                continue;
                            }
                            if same_host {
                                if let (Some(ref sh), Ok(u)) = (&seed_host, Url::parse(href)) {
                                    if u.host_str() != Some(sh.as_str()) {
                                        continue;
                                    }
                                }
                            }
                            queue.push_back((href.to_string(), depth + 1));
                        }
                    }
                } else if depth < max_depth {
                    // Re-scrape as links format for discovery when current format has no links.
                    let mut link_opts = opts.clone();
                    link_opts.format = ScrapeFormat::Links;
                    if let Ok(lp) = scrape_http(&url, robots, &link_opts).await {
                        if let Some(links) = lp.get("links").and_then(|v| v.as_array()) {
                            for link in links {
                                let Some(href) = link.get("url").and_then(|v| v.as_str()) else {
                                    continue;
                                };
                                if !seen.insert(href.to_string()) {
                                    continue;
                                }
                                if same_host {
                                    if let (Some(ref sh), Ok(u)) = (&seed_host, Url::parse(href)) {
                                        if u.host_str() != Some(sh.as_str()) {
                                            continue;
                                        }
                                    }
                                }
                                queue.push_back((href.to_string(), depth + 1));
                            }
                        }
                    }
                }
                pages.push(page);
            }
            Err(e) => {
                pages.push(json!({
                    "source_url": url,
                    "depth": depth,
                    "error": e.to_string(),
                }));
            }
        }
    }

    Ok(json!({
        "seed": seed,
        "count": pages.len(),
        "limit": limit,
        "max_depth": max_depth,
        "same_host": same_host,
        "pages": pages,
        "robots_policy": robots.as_str(),
        "engine": "http",
    }))
}

/// Map site: collect unique URLs via BFS without full content (links format).
pub async fn map_http(
    seed: &str,
    robots: RobotsPolicy,
    limit: usize,
    max_depth: usize,
) -> Result<Value, CliError> {
    let mut opts = ScrapeOpts {
        format: ScrapeFormat::Links,
        engine: "http".into(),
        ..ScrapeOpts::default()
    };
    opts.only_main_content = false;
    let crawl = crawl_http(seed, robots, &opts, limit, max_depth, true).await?;
    let mut urls: BTreeSet<String> = BTreeSet::new();
    urls.insert(seed.to_string());
    if let Some(pages) = crawl.get("pages").and_then(|p| p.as_array()) {
        for p in pages {
            if let Some(u) = p.get("source_url").and_then(|v| v.as_str()) {
                urls.insert(u.to_string());
            }
            if let Some(links) = p.get("links").and_then(|v| v.as_array()) {
                for l in links {
                    if let Some(u) = l.get("url").and_then(|v| v.as_str()) {
                        urls.insert(u.to_string());
                    }
                }
            }
        }
    }
    let list: Vec<String> = urls.into_iter().take(limit.max(1)).collect();
    Ok(json!({
        "seed": seed,
        "count": list.len(),
        "urls": list,
        "robots_policy": robots.as_str(),
        "engine": "http",
    }))
}

/// Local search: fetch a public HTML search page or treat query as URL list seed.
/// MVP: if query looks like URL, map it; else use DuckDuckGo HTML (optional network).
pub async fn search_http(
    query: &str,
    robots: RobotsPolicy,
    limit: usize,
) -> Result<Value, CliError> {
    let limit = limit.clamp(1, 50);
    let q = query.trim();
    if q.starts_with("http://") || q.starts_with("https://") {
        return map_http(q, robots, limit, 1).await;
    }
    let search_url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(q)
    );
    let opts = ScrapeOpts {
        format: ScrapeFormat::Links,
        engine: "http".into(),
        ..ScrapeOpts::default()
    };
    let page = scrape_http(&search_url, robots, &opts).await?;
    let mut results = Vec::new();
    if let Some(links) = page.get("links").and_then(|v| v.as_array()) {
        for l in links {
            let raw = l
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let text = l
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let clean = clean_serp_url(&raw);
            // Drop same-host SERP chrome and empty destinations.
            if clean.is_empty() {
                continue;
            }
            if clean.contains("duckduckgo.com/html") || clean.ends_with("duckduckgo.com/") {
                continue;
            }
            results.push(json!({ "text": text, "url": clean }));
            if results.len() >= limit {
                break;
            }
        }
    }
    Ok(json!({
        "query": q,
        "count": results.len(),
        "results": results,
        "source_url": search_url,
        "robots_policy": robots.as_str(),
        "engine": "http",
        "note": "local HTTP search via public HTML SERP; no SaaS API key",
    }))
}

/// Unwrap SERP redirect wrappers (e.g. uddg=) into destination URLs.
fn clean_serp_url(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    if let Ok(u) = Url::parse(raw) {
        // duckduckgo /l/?uddg=https%3A%2F%2F...
        for (k, v) in u.query_pairs() {
            if k == "uddg" || k == "u" || k == "url" {
                let decoded = urlencoding::decode(&v).unwrap_or_else(|_| v.clone());
                let s = decoded.into_owned();
                if s.starts_with("http://") || s.starts_with("https://") {
                    return s;
                }
            }
        }
        // already clean
        if u.host_str()
            .map(|h| !h.contains("duckduckgo.com"))
            .unwrap_or(true)
        {
            return raw.to_string();
        }
    }
    raw.to_string()
}

/// Parse local file (html/md/txt/csv/json/xml/pdf/docx/xlsx) one-shot, no Chrome.
pub fn parse_file(path: &Path) -> Result<Value, CliError> {
    parse_file_opts(path, false)
}

/// Parse local file with optional PII redaction.
pub fn parse_file_opts(path: &Path, redact: bool) -> Result<Value, CliError> {
    const MAX_PARSE_BYTES: usize = 50_000_000;
    let meta = fs::metadata(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("parse open {}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("not a file: {}", path.display()),
        ));
    }
    if meta.len() as usize > MAX_PARSE_BYTES {
        return Err(CliError::new(
            ErrorKind::Data,
            format!(
                "file {} exceeds max parse size {}",
                path.display(),
                MAX_PARSE_BYTES
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
                "Supported: html htm md markdown txt csv json xml pdf docx xlsx xls ods",
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
                    expires_unix: cache::expires_after(std::time::Duration::from_secs(86_400)),
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

fn parse_pdf_bytes(
    bytes: &[u8],
) -> Result<(&'static str, String, &'static str, usize, bool), CliError> {
    if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "invalid PDF magic: expected %PDF- header",
            "Provide a real PDF file; generate with print-pdf if needed",
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

fn parse_docx_bytes(bytes: &[u8]) -> Result<(&'static str, String, &'static str), CliError> {
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

fn parse_spreadsheet(path: &Path) -> Result<(&'static str, String, &'static str), CliError> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("spreadsheet open: {e}")))?;
    let mut lines = Vec::new();
    let sheets = workbook.sheet_names().to_vec();
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

/// Read URLs file (one URL per line, # comments).
pub fn read_urls_file(path: &Path) -> Result<Vec<String>, CliError> {
    let raw = fs::read_to_string(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("read urls file {}: {e}", path.display()),
        )
    })?;
    let mut urls = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        urls.push(line.to_string());
    }
    if urls.is_empty() {
        return Err(CliError::new(ErrorKind::Usage, "urls file has no URLs"));
    }
    Ok(urls)
}

/// Stable sorted map helper for tests.
#[allow(dead_code)]
pub fn sorted_keys(v: &Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_parse() {
        assert!(matches!(
            ScrapeFormat::parse("md").unwrap(),
            ScrapeFormat::Markdown
        ));
    }

    #[test]
    fn build_payload_links() {
        let html = r#"<html><head><title>T</title></head><body><a href="/a">A</a></body></html>"#;
        let opts = ScrapeOpts {
            format: ScrapeFormat::Links,
            engine: "http".into(),
            ..Default::default()
        };
        let v = build_scrape_payload(
            "https://example.com/",
            200,
            html,
            &opts,
            RobotsPolicy::Ignore,
        );
        assert_eq!(v["title"], "T");
        assert!(!v["links"].as_array().unwrap().is_empty());
    }
}
