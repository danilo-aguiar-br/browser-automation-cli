// SPDX-License-Identifier: MIT OR Apache-2.0
//! Filesystem discovery and structural lint/rewrite handlers (no Chrome).

use crate::commands::common::emit_ok;
use crate::error::CliError;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_find_paths(
    pattern: Option<&str>,
    paths: &[String],
    extension: Option<&str>,
    hidden: bool,
    no_ignore: bool,
    max_depth: Option<usize>,
    entry_type: Option<&str>,
    limit: usize,
    glob: Option<&str>,
    json: bool,
) -> Result<(), CliError> {
    let opts = crate::find_paths::FindPathsOpts {
        pattern: pattern.unwrap_or("").to_string(),
        roots: crate::find_paths::roots_from(paths),
        extension: extension.map(|s| s.to_string()),
        hidden,
        no_ignore,
        max_depth,
        entry_type: entry_type.map(|s| s.to_string()),
        limit,
        glob: glob.map(|s| s.to_string()),
    };
    let data = crate::find_paths::find_paths(&opts)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok find-paths count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}

pub(crate) fn handle_sg_scan(paths: &[String], limit: usize, json: bool) -> Result<(), CliError> {
    let roots: Vec<std::path::PathBuf> = if paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        paths.iter().map(std::path::PathBuf::from).collect()
    };
    let data = crate::sg_local::sg_scan(&roots, limit)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok sg-scan count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}

pub(crate) fn handle_sg_rewrite(paths: &[String], apply: bool, json: bool) -> Result<(), CliError> {
    let roots: Vec<std::path::PathBuf> = if paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        paths.iter().map(std::path::PathBuf::from).collect()
    };
    let data = crate::sg_local::sg_rewrite(&roots, apply)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok sg-rewrite apply={} planned={}",
            d.get("apply").and_then(|v| v.as_bool()).unwrap_or(false),
            d.get("planned").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
