// SPDX-License-Identifier: MIT OR Apache-2.0
//! Filesystem discovery and structural lint/rewrite handlers (no Chrome).

use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};

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

/// Bound every operator-named scan root to the allowed roots.
///
/// # Errors
///
/// [`crate::fs_roots::ensure_read_allowed`] when any named root falls outside
/// the allowed roots (GAP-026).
///
/// `sg-scan` and `sg-rewrite` take their roots as `Vec<String>` and not as
/// `PathBuf`, which is why they outlived three sweeps of this class: those
/// sweeps enumerated the `PathBuf` fields under `src/cli/`, and a path typed as
/// a string is invisible to that search. The same wrong-identifier failure has
/// now produced a false "all clear" at every level it could — the internal
/// function instead of the facade, the helper instead of the command, and the
/// type instead of the role.
///
/// MEASURED 2026-08-31: `sg-scan /dev/shm/x` exited 0 and walked that
/// directory, and `sg-rewrite` reported `target_resolved` pointing into it,
/// while `parse` refused the identical path with exit 64.
///
/// Bounded HERE, at the command boundary, rather than inside the walker: the
/// walk descends into paths it derived itself, and the `.` default — used when
/// argv names no root — has to keep working.
fn guarded_roots(paths: &[String]) -> Result<Vec<std::path::PathBuf>, CliError> {
    if paths.is_empty() {
        return Ok(vec![std::path::PathBuf::from(".")]);
    }
    paths
        .iter()
        .map(|p| crate::fs_roots::ensure_read_allowed(std::path::Path::new(p)))
        .collect()
}

pub(crate) fn handle_sg_scan(paths: &[String], limit: usize, json: bool) -> Result<(), CliError> {
    let roots = guarded_roots(paths)?;
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
    // Defaulting to `.` is fine while this only reports. Under `--apply` it
    // rewrites source files, and then the root has to be named: the target of a
    // destructive verb comes from argv, never from wherever the process happens
    // to have been started.
    if apply && paths.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "sg-rewrite --apply needs an explicit root; it will not default to the current directory",
            crate::i18n::suggestion_key("sg-rewrite-needs-root", None),
        ));
    }
    let roots = guarded_roots(paths)?;
    // Under `--apply` the roots came from argv, because the branch above refuses
    // otherwise. The reporting path may still have defaulted to `.`, and it says
    // so rather than passing a guess off as an explicit root.
    let resolved = roots
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = if paths.is_empty() {
        crate::etd::TargetSource::Ambient
    } else {
        crate::etd::TargetSource::Argv
    };
    let data = crate::etd::with_target(
        crate::sg_local::sg_rewrite(&roots, apply)?,
        &resolved,
        source,
    );
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok sg-rewrite apply={} planned={}",
            d.get("apply").and_then(|v| v.as_bool()).unwrap_or(false),
            d.get("planned").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
