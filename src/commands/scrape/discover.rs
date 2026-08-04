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
) -> Result<(), CliError> {
    let path_filter = PathFilter::from_lists(include_path, exclude_path);
    let use_sm = resolve_use_sitemap(use_sitemap);
    let data = block_on_browser_timeout(
        crate::scrape_local::map_http(url, robots, limit, max_depth, &path_filter, use_sm, search),
        0,
    )?;
    let data = finalize_scrape_value_ex(data, select, None, None, sort, dedup_key);
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok map count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}

pub(crate) fn handle_search(
    query: &str,
    robots: RobotsPolicy,
    limit: usize,
    json: bool,
    select: Option<&str>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(crate::scrape_local::search_http(query, robots, limit), 0)?;
    let data = finalize_scrape_value_ex(data, select, None, None, sort, dedup_key);
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok search count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
