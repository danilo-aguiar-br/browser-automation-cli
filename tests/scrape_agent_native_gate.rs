// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual scrape agent-native gates (offline).

use browser_automation_cli::scrape_local::{
    apply_max_text_chars, dedup_pages_envelope, finalize_scrape_value, page_matches_filter,
    parse_sitemap_xml, project_fields, sort_pages_envelope, PathFilter, ScrapeFormat,
};
use serde_json::json;

#[test]
fn select_omits_html() {
    let v = json!({
        "source_url": "https://example.com",
        "title": "T",
        "html": "<html></html>",
        "markdown": "# hi",
        "status_code": 200
    });
    let p = project_fields(v, Some("source_url,title,markdown,status_code"));
    assert!(p.get("html").is_none());
    assert_eq!(p["title"], "T");
    assert_eq!(p["status_code"], 200);
}

#[test]
fn max_text_sets_truncated() {
    let v = json!({"text": "abcdefghijklmnopqrstuvwxyz"});
    let p = apply_max_text_chars(v, 5);
    assert_eq!(p["truncated"], true);
    assert!(p["text"].as_str().unwrap().chars().count() <= 6); // 5 + ellipsis char
}

#[test]
fn path_filter_include_exclude() {
    let f = PathFilter::from_lists(&["/docs".into()], &["/docs/private".into()]);
    assert!(f.allows_url("https://ex.com/docs/a"));
    assert!(!f.allows_url("https://ex.com/docs/private/x"));
    assert!(!f.allows_url("https://ex.com/blog"));
}

#[test]
fn sitemap_urlset_and_index() {
    let (locs, nested) = parse_sitemap_xml(
        r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/a</loc></url>
        </urlset>"#,
    );
    assert_eq!(locs, vec!["https://example.com/a"]);
    assert!(nested.is_empty());

    let (locs2, nested2) = parse_sitemap_xml(
        r#"<?xml version="1.0"?>
        <sitemapindex>
          <sitemap><loc>https://example.com/s1.xml</loc></sitemap>
        </sitemapindex>"#,
    );
    assert!(locs2.is_empty());
    assert_eq!(nested2, vec!["https://example.com/s1.xml"]);
}

#[test]
fn finalize_filter_http_error() {
    let v = json!({
        "count": 2,
        "pages": [
            {"source_url":"ok","http_error":false,"status_code":200,"html":"<x>"},
            {"source_url":"bad","http_error":true,"status_code":404}
        ]
    });
    let p = finalize_scrape_value(
        v,
        Some("source_url,status_code"),
        Some("http_error=false"),
        None,
    );
    let pages = p["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0]["source_url"], "ok");
    assert!(pages[0].get("html").is_none());
}

#[test]
fn format_images_parses() {
    assert_eq!(ScrapeFormat::parse("images").unwrap(), ScrapeFormat::Images);
}

#[test]
fn filter_missing_http_error_keeps_ok() {
    let ok = json!({"source_url": "a", "status_code": 200, "text": "hi"});
    assert!(page_matches_filter(&ok, Some("http_error=false")));
    let bad = json!({"source_url": "b", "http_error": true});
    assert!(!page_matches_filter(&bad, Some("http_error=false")));
}

#[test]
fn multi_format_select_promotes_markdown() {
    let v = json!({
        "source_url": "https://example.com",
        "formats": {
            "markdown": {
                "source_url": "https://example.com",
                "markdown": "# Hi",
                "text": "Hi",
                "title": "T"
            },
            "jsonld": {"jsonld": [], "jsonld_count": 0}
        }
    });
    let p = project_fields(v, Some("source_url,markdown,title"));
    assert_eq!(p["source_url"], "https://example.com");
    assert_eq!(p["markdown"], "# Hi");
    assert_eq!(p["title"], "T");
}

#[test]
fn format_jsonld_and_json_parse() {
    assert_eq!(ScrapeFormat::parse("jsonld").unwrap(), ScrapeFormat::JsonLd);
    assert_eq!(ScrapeFormat::parse("json").unwrap(), ScrapeFormat::Json);
}

#[test]
fn sort_and_dedup_trailing_slash() {
    let v = json!({
        "urls": [
            "https://b.example/",
            "https://a.example",
            "https://a.example/"
        ]
    });
    let s = sort_pages_envelope(v, Some("url"));
    let d = dedup_pages_envelope(s, Some("url"));
    let urls = d["urls"].as_array().unwrap();
    assert_eq!(urls.len(), 2, "urls={urls:?}");
}
