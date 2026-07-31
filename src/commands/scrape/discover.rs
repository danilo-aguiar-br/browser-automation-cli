// SPDX-License-Identifier: MIT OR Apache-2.0
//! Site map and local search handlers (HTTP only).

use crate::browser::block_on_browser_timeout;
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::robots::RobotsPolicy;

pub(crate) fn handle_map(
    url: &str,
    robots: RobotsPolicy,
    limit: usize,
    max_depth: usize,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        crate::scrape_local::map_http(url, robots, limit, max_depth),
        0,
    )?;
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
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(crate::scrape_local::search_http(query, robots, limit), 0)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok search count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
