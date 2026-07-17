//! Local Firecrawl-parity scrape/crawl/map/search/parse (one-shot, no SaaS).
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

use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;
use crate::xdg;

/// Identifiable product User-Agent for HTTP scrapes (PRD politeness).
pub const HTTP_USER_AGENT: &str =
    "browser-automation-cli/0.1 (+https://github.com/danilo-aguiar-br/browser-automation-cli; local-scrape)";

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
}

impl ScrapeFormat {
    /// Parse from CLI flag (comma-separated first token).
    pub fn parse(s: &str) -> Result<Self, CliError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" | "body" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            "links" => Ok(Self::Links),
            "metadata" | "meta" => Ok(Self::Metadata),
            other => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown scrape format: {other}"),
                "Use text|markdown|html|links|metadata",
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
pub async fn scrape_http(url: &str, robots: RobotsPolicy, opts: &ScrapeOpts) -> Result<Value, CliError> {
    crate::robots::enforce_robots(url, robots, HTTP_USER_AGENT).await?;
    let client = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("http client: {e}")))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| CliError::new(ErrorKind::Unavailable, format!("GET {url}: {e}")))?;
    let status = resp.status().as_u16();
    let final_url = resp.url().to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read body: {e}")))?;
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
    Ok(build_scrape_payload(&final_url, status, &html, opts, robots))
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
    }
    data
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
    let sel = format!(
        "meta[name=\"{name}\"], meta[property=\"{name}\"], meta[property=\"og:{name}\"]"
    );
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
            (Some(b), _) => b.join(href).map(|u| u.to_string()).unwrap_or_else(|_| href.to_string()),
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
    let concurrency = concurrency.max(1).min(16);
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
            let robots = robots;
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
    let limit = limit.max(1).min(500);
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
pub async fn search_http(query: &str, robots: RobotsPolicy, limit: usize) -> Result<Value, CliError> {
    let limit = limit.max(1).min(50);
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
        for l in links.iter().take(limit) {
            results.push(l.clone());
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

/// Parse local file (text/html/markdown/pdf bytes as text extraction best-effort).
pub fn parse_file(path: &Path) -> Result<Value, CliError> {
    let meta = fs::metadata(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("parse open {}: {e}", path.display()),
        )
    })?;
    if !meta.is_file() {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("not a file: {}", path.display()),
        ));
    }
    let bytes = fs::read(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read {}: {e}", path.display())))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (kind, text) = match ext.as_str() {
        "html" | "htm" => {
            let s = String::from_utf8_lossy(&bytes);
            let doc = Html::parse_document(&s);
            ("html", visible_text(&doc))
        }
        "md" | "markdown" | "txt" | "csv" | "json" | "xml" => {
            ("text", String::from_utf8_lossy(&bytes).into_owned())
        }
        "pdf" => {
            // Best-effort: extract printable ASCII runs (no full PDF engine in MVP path).
            let mut out = String::new();
            let mut run = String::new();
            for &b in &bytes {
                if (32..127).contains(&b) || b == b'\n' || b == b'\t' {
                    run.push(b as char);
                } else if run.len() >= 4 {
                    out.push_str(&run);
                    out.push('\n');
                    run.clear();
                } else {
                    run.clear();
                }
            }
            if run.len() >= 4 {
                out.push_str(&run);
            }
            ("pdf-text-extract", out)
        }
        other => {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unsupported parse extension: {other}"),
                "Supported: html htm md markdown txt csv json xml pdf",
            ));
        }
    };
    // Optional cache note under XDG.
    let _ = xdg::cache_dir().map(|d| d.join("parse"));
    Ok(json!({
        "path": path.display().to_string(),
        "kind": kind,
        "bytes": bytes.len(),
        "text": text,
        "engine": "local",
    }))
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
        return Err(CliError::new(
            ErrorKind::Usage,
            "urls file has no URLs",
        ));
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
        assert!(matches!(ScrapeFormat::parse("md").unwrap(), ScrapeFormat::Markdown));
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
        assert!(v["links"].as_array().unwrap().len() >= 1);
    }
}
