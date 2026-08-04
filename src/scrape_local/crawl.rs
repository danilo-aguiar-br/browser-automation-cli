// SPDX-License-Identifier: MIT OR Apache-2.0
//! BFS crawl engine (bounded parallel frontier).

use std::collections::{BTreeSet, VecDeque};

use serde_json::{json, Value};
use url::Url;

use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::crawl_frontier::crawl_enqueue_link;
use super::error_page::http_error_page;
use super::http::scrape_http;
use super::path_filter::{normalize_url_for_dedup_ex, PathFilter};
use super::types::{ScrapeFormat, ScrapeOpts};

/// One crawl task result: `(url, depth, page_result, discovered_hrefs)`.
///
/// Hrefs are collected under the same semaphore permit as the fetch, so a task
/// never holds a permit while waiting to enqueue its children.
type CrawlTaskOutput = (String, usize, Result<Value, CliError>, Vec<String>);

/// BFS crawl from seed URL (HTTP) with **bounded parallel frontier**.
///
/// # Workload
///
/// **I/O-bound.** Ready URLs are fetched concurrently up to
/// [`crate::concurrency::effective_limit`] via `JoinSet` + `Arc<Semaphore>`
/// (`acquire_owned` moved into each task; never unbounded spawn).
#[allow(clippy::too_many_arguments)]
pub async fn crawl_http(
    seed: &str,
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
    limit: usize,
    max_depth: usize,
    same_host: bool,
    path_filter: &PathFilter,
    use_sitemap: bool,
    ignore_query_params: bool,
) -> Result<Value, CliError> {
    let limit = limit.clamp(
        1,
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::SCRAPE_CRAWL_LIMIT_MAX),
    );
    let max_depth = max_depth.min(crate::xdg::policy::policy_usize(
        crate::xdg::policy::key::SCRAPE_CRAWL_MAX_DEPTH,
    ));
    let concurrency = crate::concurrency::effective_limit();
    let seed_url = Url::parse(seed)
        .map_err(|e| CliError::new(ErrorKind::Usage, format!("invalid seed URL: {e}")))?;
    let seed_host = seed_url.host_str().map(|s| s.to_string());

    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let seed_norm = normalize_url_for_dedup_ex(seed, ignore_query_params);
    queue.push_back((seed_norm.clone(), 0));
    seen.insert(seed_norm);

    let mut sitemap_seed_count = 0usize;
    if use_sitemap {
        if let Ok(urls) =
            super::sitemap::discover_sitemap_urls(seed, robots, limit, path_filter).await
        {
            sitemap_seed_count = urls.len();
            for u in urls {
                crawl_enqueue_link(
                    &u,
                    0,
                    same_host,
                    seed_host.as_deref(),
                    path_filter,
                    &mut seen,
                    &mut queue,
                    ignore_query_params,
                );
            }
        }
    }

    let mut pages = Vec::new();
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;
    let sem = Arc::new(Semaphore::new(concurrency));
    let mut set: JoinSet<CrawlTaskOutput> = JoinSet::new();
    let mut in_flight = 0usize;
    let cancel = crate::lifecycle::current_cancel();

    while pages.len() < limit && (!queue.is_empty() || in_flight > 0) {
        if cancel.is_cancelled() {
            set.abort_all();
            break;
        }
        while in_flight < concurrency && pages.len() + in_flight < limit {
            if cancel.is_cancelled() {
                break;
            }
            let Some((url, depth)) = queue.pop_front() else {
                break;
            };
            // try_acquire_owned: if no permit, wait for a join_next (backpressure).
            let permit = match Arc::clone(&sem).try_acquire_owned() {
                Ok(p) => p,
                Err(_) => break,
            };
            let opts = opts.clone();
            let need_discovery = depth < max_depth && opts.format != ScrapeFormat::Links;
            set.spawn(async move {
                let _permit = permit;
                let result = scrape_http(&url, robots, &opts).await;
                // Link discovery under the **same** permit (never re-fetch on the
                // collector without admission control — PAR-31).
                let mut discovered = Vec::new();
                if let Ok(ref page) = result {
                    if depth < max_depth {
                        // rel=next continuation is admitted through the *same*
                        // frontier as ordinary links, so limit / depth / host /
                        // path filters and robots all still bind it.
                        if let Some(next) = page.get("rel_next").and_then(|v| v.as_array()) {
                            for href in next.iter().filter_map(|v| v.as_str()) {
                                discovered.push(href.to_string());
                            }
                        }
                        if let Some(links) = page.get("links").and_then(|v| v.as_array()) {
                            for link in links {
                                if let Some(href) = link.get("url").and_then(|v| v.as_str()) {
                                    discovered.push(href.to_string());
                                }
                            }
                        } else if need_discovery {
                            let mut link_opts = opts.clone();
                            link_opts.format = ScrapeFormat::Links;
                            if let Ok(lp) = scrape_http(&url, robots, &link_opts).await {
                                if let Some(links) = lp.get("links").and_then(|v| v.as_array()) {
                                    for link in links {
                                        if let Some(href) = link.get("url").and_then(|v| v.as_str())
                                        {
                                            discovered.push(href.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                (url, depth, result, discovered)
            });
            in_flight += 1;
        }

        // Race join with cancel so a mid-crawl SIGINT does not wait for the
        // slowest HTTP task before abort_all (outer block_on also drops fut).
        let joined = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                set.abort_all();
                break;
            }
            j = set.join_next() => j,
        };
        let Some(joined) = joined else {
            break;
        };
        in_flight = in_flight.saturating_sub(1);
        let (url, depth, result, discovered) = match joined {
            Ok(t) => t,
            Err(e) => {
                pages.push(json!({
                    "error": format!("join: {e}"),
                    "http_error": true,
                    "join_panic": e.is_panic(),
                    "join_cancelled": e.is_cancelled(),
                }));
                continue;
            }
        };

        match result {
            Ok(mut page) => {
                page["depth"] = json!(depth);
                if depth < max_depth {
                    for href in discovered {
                        crawl_enqueue_link(
                            &href,
                            depth,
                            same_host,
                            seed_host.as_deref(),
                            path_filter,
                            &mut seen,
                            &mut queue,
                            ignore_query_params,
                        );
                    }
                }
                pages.push(page);
            }
            Err(e) => {
                pages.push(http_error_page(&url, &e.to_string(), Some(depth)));
            }
        }
    }
    // At limit or cancel: abort remaining work (PAR-36); then drain JoinError.
    if pages.len() >= limit || cancel.is_cancelled() {
        set.abort_all();
    }
    while let Some(joined) = set.join_next().await {
        if pages.len() >= limit {
            continue;
        }
        match joined {
            Ok((url, depth, result, discovered)) => match result {
                Ok(mut page) => {
                    page["depth"] = json!(depth);
                    if depth < max_depth {
                        for href in discovered {
                            crawl_enqueue_link(
                                &href,
                                depth,
                                same_host,
                                seed_host.as_deref(),
                                path_filter,
                                &mut seen,
                                &mut queue,
                                ignore_query_params,
                            );
                        }
                    }
                    pages.push(page);
                }
                Err(e) => pages.push(http_error_page(&url, &e.to_string(), Some(depth))),
            },
            Err(e) if e.is_cancelled() => {}
            Err(e) => pages.push(json!({
                "error": format!("join: {e}"),
                "http_error": true,
                "join_panic": e.is_panic(),
            })),
        }
    }

    Ok(json!({
        "seed": seed,
        "count": pages.len(),
        "limit": limit,
        "max_depth": max_depth,
        "same_host": same_host,
        "concurrency": concurrency,
        "gate": "Arc<Semaphore>::try_acquire_owned",
        "pages": pages,
        "robots_policy": robots.as_str(),
        "engine": "http",
        "cancelled": cancel.is_cancelled(),
        "use_sitemap": use_sitemap,
        "sitemap_seed_count": sitemap_seed_count,
        "path_filter_empty": path_filter.is_empty(),
        "follow_rel_next": opts.follow_rel_next,
    }))
}
