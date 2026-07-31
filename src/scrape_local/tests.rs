// SPDX-License-Identifier: MIT OR Apache-2.0
//! scrape_local unit tests.

use super::*;
use crate::robots::RobotsPolicy;

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

#[tokio::test]
async fn batch_pre_cancelled_returns_cancelled_flag() {
    let lc = crate::lifecycle::Lifecycle::new();
    lc.cancel.cancel();
    let opts = ScrapeOpts {
        format: ScrapeFormat::Text,
        engine: "http".into(),
        ..Default::default()
    };
    let urls = vec!["https://example.com/".into()];
    let v = batch_scrape_http(&urls, RobotsPolicy::Ignore, &opts, 1)
        .await
        .expect("batch should complete with cancelled diagnostics");
    assert_eq!(v["cancelled"], true);
}
