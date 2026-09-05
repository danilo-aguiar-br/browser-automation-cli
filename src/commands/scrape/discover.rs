// SPDX-License-Identifier: MIT OR Apache-2.0
//! Site map and local search handlers (HTTP only).

use crate::browser::block_on_browser_timeout;
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::robots::RobotsPolicy;
use crate::scrape_local::{finalize_scrape_value_ex, PathFilter};

fn resolve_use_sitemap(cli: Option<bool>) -> bool {
    cli.unwrap_or_else(crate::xdg::resolve_scrape_use_sitemap)
}

/// Host of `url`, lowercased, or `None` when the URL has no host to speak of.
fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
}

/// Does `host` belong to `root`, counting subdomains only when asked?
///
/// Subdomain matching is textual against the seed host (`docs.example.com` under
/// `example.com`) rather than against a public-suffix registrable domain: the
/// product ships no PSL table, and inventing one would make `co.uk` behave
/// differently from `com` for reasons no caller could see in the argv.
fn host_in_scope(host: &str, root: &str, include_subdomains: bool) -> bool {
    host == root || (include_subdomains && host.ends_with(&format!(".{root}")))
}

/// Rewrite `data.urls` in place: host scoping, then query-param collapsing.
///
/// Both run after `map_http` because the URL set it returns is the only place
/// where every discovery path — sitemap, BFS pages and the links harvested from
/// them — has already been merged. Filtering earlier would have to be repeated
/// once per path.
fn scope_map_urls(
    mut data: serde_json::Value,
    seed: &str,
    include_subdomains: bool,
    ignore_query_params: bool,
) -> serde_json::Value {
    let root = host_of(seed);
    let Some(urls) = data.get("urls").and_then(|v| v.as_array()) else {
        return data;
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut kept: Vec<serde_json::Value> = Vec::with_capacity(urls.len());
    for entry in urls {
        let Some(raw) = entry.as_str() else { continue };
        if let Some(root) = root.as_deref() {
            match host_of(raw) {
                Some(h) if host_in_scope(&h, root, include_subdomains) => {}
                _ => continue,
            }
        }
        let normalized = if ignore_query_params {
            crate::scrape_local::normalize_url_for_dedup_ex(raw, true)
        } else {
            raw.to_string()
        };
        if !seen.insert(normalized.clone()) {
            continue;
        }
        kept.push(serde_json::Value::String(normalized));
    }
    if let Some(obj) = data.as_object_mut() {
        obj.insert("count".into(), serde_json::json!(kept.len()));
        obj.insert("urls".into(), serde_json::Value::Array(kept));
        obj.insert(
            "include_subdomains".into(),
            serde_json::json!(include_subdomains),
        );
        obj.insert(
            "ignore_query_params".into(),
            serde_json::json!(ignore_query_params),
        );
    }
    data
}

/// Expand a seed URL into the site's URL inventory.
///
/// # Errors
///
/// Returns [`CliError`] when the seed cannot be fetched under the active robots
/// policy, or when the envelope cannot be written to stdout.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_map(
    url: &str,
    robots: RobotsPolicy,
    limit: usize,
    max_depth: usize,
    json: bool,
    select: Option<&str>,
    include_path: &[String],
    exclude_path: &[String],
    use_sitemap: Option<bool>,
    search: Option<&str>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
    sitemap_only: bool,
    include_subdomains: bool,
    ignore_query_params: bool,
) -> Result<(), CliError> {
    let path_filter = PathFilter::from_lists(include_path, exclude_path);
    // `--sitemap-only` is the two knobs below said once: take the sitemap
    // frontier, and never let HTML link discovery descend from it. Resolved
    // here, next to the knobs it overrides, rather than in the dispatcher.
    let max_depth = if sitemap_only { 0 } else { max_depth };
    let use_sm = if sitemap_only {
        true
    } else {
        resolve_use_sitemap(use_sitemap)
    };
    let data = block_on_browser_timeout(
        crate::scrape_local::map_http(url, robots, limit, max_depth, &path_filter, use_sm, search),
        0,
    )?;
    let data = scope_map_urls(data, url, include_subdomains, ignore_query_params);
    let data = finalize_scrape_value_ex(data, select, None, None, sort, dedup_key);
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok map count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}

/// Is this URL inside the caller's domain scope?
///
/// A URL with no parseable host is dropped whenever either list is non-empty:
/// the caller asked for a specific set of sites, and "we could not tell" is not
/// a member of it.
fn url_in_domains(url: &str, include: &[String], exclude: &[String]) -> bool {
    let Some(host) = host_of(url) else {
        return false;
    };
    if !include.is_empty() && !include.iter().any(|d| host_in_scope(&host, d, true)) {
        return false;
    }
    !exclude.iter().any(|d| host_in_scope(&host, d, true))
}

/// Keep or drop search rows by result host.
///
/// Applied locally, on the fetched SERP: the public HTML endpoint exposes no
/// site-restriction parameter this product can depend on, and a `site:` operator
/// smuggled into the query would silently change the caller's search terms.
/// Include runs first, exclude second, so a domain named in both is dropped.
///
/// Handles both shapes `search` can return, because a query that is itself a URL
/// is answered by the site mapper and comes back as `urls`. Filtering only
/// `results` would leave the flag parsed and inert on exactly that path.
fn filter_search_domains(
    mut data: serde_json::Value,
    include: &[String],
    exclude: &[String],
) -> serde_json::Value {
    if include.is_empty() && exclude.is_empty() {
        return data;
    }
    let kept: Vec<serde_json::Value> =
        if let Some(rows) = data.get("results").and_then(|v| v.as_array()) {
            rows.iter()
                .filter(|r| {
                    r.get("url")
                        .and_then(|v| v.as_str())
                        .is_some_and(|u| url_in_domains(u, include, exclude))
                })
                .cloned()
                .collect()
        } else if let Some(rows) = data.get("urls").and_then(|v| v.as_array()) {
            rows.iter()
                .filter(|u| {
                    u.as_str()
                        .is_some_and(|u| url_in_domains(u, include, exclude))
                })
                .cloned()
                .collect()
        } else {
            return data;
        };
    let key = if data.get("results").is_some() {
        "results"
    } else {
        "urls"
    };
    if let Some(obj) = data.as_object_mut() {
        obj.insert("count".into(), serde_json::json!(kept.len()));
        obj.insert(key.into(), serde_json::Value::Array(kept));
        obj.insert("include_domains".into(), serde_json::json!(include));
        obj.insert("exclude_domains".into(), serde_json::json!(exclude));
    }
    data
}

/// Run a local HTTP search and emit the result collection.
///
/// # Errors
///
/// Returns [`CliError`] when the SERP endpoint cannot be fetched under the
/// active robots policy, or when the envelope cannot be written to stdout.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_search(
    query: &str,
    robots: RobotsPolicy,
    limit: usize,
    json: bool,
    select: Option<&str>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
    include_domains: Option<&str>,
    exclude_domains: Option<&str>,
    country: Option<&str>,
    search_lang: Option<&str>,
    time_filter: Option<&str>,
) -> Result<(), CliError> {
    let include = include_domains
        .map(crate::agent_ops::path::split_csv_lower)
        .unwrap_or_default();
    let exclude = exclude_domains
        .map(crate::agent_ops::path::split_csv_lower)
        .unwrap_or_default();
    let data = block_on_browser_timeout(
        crate::scrape_local::search_http(query, robots, limit, country, search_lang, time_filter),
        0,
    )?;
    let data = filter_search_domains(data, &include, &exclude);
    let data = finalize_scrape_value_ex(data, select, None, None, sort, dedup_key);
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok search count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
