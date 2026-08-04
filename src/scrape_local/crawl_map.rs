// SPDX-License-Identifier: MIT OR Apache-2.0
//! Site map: URL discovery without full page content.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::error::CliError;
use crate::robots::RobotsPolicy;

use super::crawl::crawl_http;
use super::path_filter::{normalize_url_for_dedup, PathFilter};
use super::types::{ScrapeFormat, ScrapeOpts};

/// Map site: sitemap seed (optional) + BFS link discovery without full content.
pub async fn map_http(
    seed: &str,
    robots: RobotsPolicy,
    limit: usize,
    max_depth: usize,
    path_filter: &PathFilter,
    use_sitemap: bool,
    search: Option<&str>,
) -> Result<Value, CliError> {
    let mut urls: BTreeSet<String> = BTreeSet::new();
    let seed_norm = normalize_url_for_dedup(seed);
    if path_filter.allows_url(&seed_norm) {
        urls.insert(seed_norm.clone());
    }

    let mut sitemap_url_count = 0usize;
    if use_sitemap {
        if let Ok(sm) =
            super::sitemap::discover_sitemap_urls(seed, robots, limit, path_filter).await
        {
            sitemap_url_count = sm.len();
            for u in sm {
                if urls.len() >= limit {
                    break;
                }
                urls.insert(normalize_url_for_dedup(&u));
            }
        }
    }

    // BFS link discovery for remaining slots.
    if urls.len() < limit && max_depth > 0 {
        let mut opts = ScrapeOpts {
            format: ScrapeFormat::Links,
            engine: "http".into(),
            ..ScrapeOpts::default()
        };
        opts.only_main_content = false;
        let crawl = crawl_http(
            seed,
            robots,
            &opts,
            limit,
            max_depth,
            true,
            path_filter,
            false, // sitemap already applied above
            false,
        )
        .await?;
        if let Some(pages) = crawl.get("pages").and_then(|p| p.as_array()) {
            for p in pages {
                if let Some(u) = p.get("source_url").and_then(|v| v.as_str()) {
                    if path_filter.allows_url(u) {
                        urls.insert(normalize_url_for_dedup(u));
                    }
                }
                if let Some(links) = p.get("links").and_then(|v| v.as_array()) {
                    for l in links {
                        if let Some(u) = l.get("url").and_then(|v| v.as_str()) {
                            if path_filter.allows_url(u) {
                                urls.insert(normalize_url_for_dedup(u));
                            }
                        }
                    }
                }
                if urls.len() >= limit {
                    break;
                }
            }
        }
    }

    let search_l = search
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());
    let list: Vec<String> = urls
        .into_iter()
        .filter(|u| {
            search_l
                .as_ref()
                .map(|q| u.to_ascii_lowercase().contains(q.as_str()))
                .unwrap_or(true)
        })
        .take(limit.max(1))
        .collect();
    Ok(json!({
        "seed": seed,
        "count": list.len(),
        "urls": list,
        "robots_policy": robots.as_str(),
        "engine": "http",
        "use_sitemap": use_sitemap,
        "sitemap_url_count": sitemap_url_count,
        "path_filter_empty": path_filter.is_empty(),
        "search": search.unwrap_or(""),
    }))
}
