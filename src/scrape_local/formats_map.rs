// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared multi-format payload derivation for scrape engines.

use crate::error::CliError;
use crate::robots::RobotsPolicy;

/// Derive one payload per requested format from a single fetched HTML body.
///
/// Single source of truth for the HTTP and browser engine multi-format paths;
/// keys are the format names with `-` normalised to `_`.
pub fn build_formats_map(
    source: &str,
    status: u16,
    html: &str,
    formats: &[&str],
    only_main_content: bool,
    engine: &str,
    robots: RobotsPolicy,
) -> Result<serde_json::Map<String, serde_json::Value>, CliError> {
    let mut formats_out = serde_json::Map::new();
    for f in formats {
        let fmt = crate::scrape_local::ScrapeFormat::parse(f)?;
        let opts = crate::scrape_local::ScrapeOpts {
            format: fmt,
            only_main_content,
            engine: engine.into(),
            ..Default::default()
        };
        let part = crate::scrape_local::build_scrape_payload(source, status, html, &opts, robots);
        formats_out.insert(f.replace('-', "_"), part);
    }
    Ok(formats_out)
}
