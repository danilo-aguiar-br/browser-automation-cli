// SPDX-License-Identifier: MIT OR Apache-2.0
//! Sitemap.xml / sitemapindex discovery for map/crawl (quick-xml).

use std::collections::BTreeSet;

use quick_xml::events::Event;
use quick_xml::Reader;
use url::Url;

use crate::error::{CliError, ErrorKind};
use crate::robots::{shared_http_client, RobotsPolicy};

use super::path_filter::PathFilter;

/// Fetch and parse `{origin}/robots.txt` Sitemap: hints + `{origin}/sitemap.xml`.
pub async fn discover_sitemap_urls(
    seed: &str,
    robots: RobotsPolicy,
    limit: usize,
    filter: &PathFilter,
) -> Result<Vec<String>, CliError> {
    let seed_url = Url::parse(seed)
        .map_err(|e| CliError::new(ErrorKind::Usage, format!("invalid seed URL: {e}")))?;
    let origin = seed_url.origin().ascii_serialization();
    crate::net::assert_safe_http_url(seed)?;

    let mut candidates: Vec<String> = Vec::new();
    // robots Sitemap: lines (best-effort)
    let robots_url = format!("{origin}/robots.txt");
    if let Ok(client) = shared_http_client() {
        if let Ok(resp) = client.get(&robots_url).send().await {
            if resp.status().is_success() {
                let max_sm = crate::xdg::resolve_scrape_sitemap_max_bytes();
                if let Ok(bytes) = crate::net::read_body_limited(resp, max_sm).await {
                    let body = String::from_utf8_lossy(&bytes);
                    for line in body.lines() {
                        let line = line.trim();
                        if line.to_ascii_lowercase().starts_with("sitemap:") {
                            if let Some(rest) = line.split_once(':') {
                                let u = rest.1.trim();
                                if !u.is_empty() {
                                    candidates.push(u.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    candidates.push(format!("{origin}/sitemap.xml"));
    candidates.push(format!("{origin}/sitemap_index.xml"));

    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut seen_sitemaps: BTreeSet<String> = BTreeSet::new();
    let mut queue = candidates;

    while let Some(sm_url) = queue.pop() {
        if out.len() >= limit {
            break;
        }
        if !seen_sitemaps.insert(sm_url.clone()) {
            continue;
        }
        if crate::net::assert_safe_http_url(&sm_url).is_err() {
            continue;
        }
        // Honor robots for sitemap URL itself when policy is honor.
        if matches!(robots, RobotsPolicy::Honor)
            && crate::robots::enforce_robots(&sm_url, robots, &crate::robots::robots_user_agent())
                .await
                .is_err()
        {
            continue;
        }
        let Ok(client) = shared_http_client() else {
            break;
        };
        let Ok(resp) = client.get(&sm_url).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(bytes) = crate::net::read_body_limited(resp, 2_000_000).await else {
            continue;
        };
        let xml = String::from_utf8_lossy(&bytes);
        let (locs, nested) = parse_sitemap_xml(&xml);
        for n in nested {
            if seen_sitemaps.len() + queue.len() < 32 {
                queue.push(n);
            }
        }
        for loc in locs {
            if out.len() >= limit {
                break;
            }
            if filter.allows_url(&loc) {
                out.insert(loc);
            }
        }
    }

    Ok(out.into_iter().take(limit).collect())
}

/// Parse a sitemap or sitemapindex body. Returns (page urls, nested sitemap urls).
pub fn parse_sitemap_xml(xml: &str) -> (Vec<String>, Vec<String>) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut locs = Vec::new();
    let mut nested = Vec::new();
    let mut in_loc = false;
    let mut in_sitemap = false;
    let mut depth_sitemap = 0i32;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_ascii_lowercase();
                let local = name.rsplit(':').next().unwrap_or(&name);
                if local == "sitemap" {
                    in_sitemap = true;
                    depth_sitemap += 1;
                }
                if local == "loc" {
                    in_loc = true;
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_ascii_lowercase();
                let local = name.rsplit(':').next().unwrap_or(&name);
                if local == "loc" {
                    in_loc = false;
                }
                if local == "sitemap" {
                    depth_sitemap -= 1;
                    if depth_sitemap <= 0 {
                        in_sitemap = false;
                        depth_sitemap = 0;
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if in_loc {
                    let raw = t.as_ref();
                    let s = String::from_utf8_lossy(raw);
                    let u = s.trim().to_string();
                    if u.starts_with("http://") || u.starts_with("https://") {
                        if in_sitemap {
                            nested.push(u);
                        } else {
                            locs.push(u);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    (locs, nested)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_urlset() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/a</loc></url>
          <url><loc>https://example.com/b</loc></url>
        </urlset>"#;
        let (locs, nested) = parse_sitemap_xml(xml);
        assert_eq!(locs.len(), 2);
        assert!(nested.is_empty());
    }

    #[test]
    fn parse_index() {
        let xml = r#"<?xml version="1.0"?>
        <sitemapindex>
          <sitemap><loc>https://example.com/s1.xml</loc></sitemap>
        </sitemapindex>"#;
        let (locs, nested) = parse_sitemap_xml(xml);
        assert!(locs.is_empty());
        assert_eq!(nested.len(), 1);
    }
}
