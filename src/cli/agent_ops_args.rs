// SPDX-License-Identifier: MIT OR Apache-2.0
//! Universal data-operation flags shared by every command (agent CLEAN STDOUT).
//!
//! Flattened into [`super::global::GlobalOpts`] rather than declared there, because
//! `global.rs` is already near the 300-line ceiling `scripts/filesize-check.sh`
//! enforces, and these eight flags are one responsibility of their own.
//!
//! Semantics live in [`crate::agent_ops`]; this file is argv only.

use clap::{ArgAction, Args};

use crate::agent_ops::{filter::FilterExpr, path::split_csv, AgentOps};
use crate::error::CliError;

/// Reduce the payload in the binary so the model never receives what it will discard.
///
/// # Why four of these carry a `-rows` suffix
///
/// `--select`, `--filter`, `--limit` and `--sort` already exist as PER-COMMAND
/// flags across 32 declarations (`scrape`, `crawl`, `map`, `search`,
/// `batch-scrape`, `audio`, `video`, `image`, `find-paths`, `sg-scan`). A global
/// flag of the same name does not replace those: clap hands the value to both, so
/// the local projection runs and then the global one runs again over its result.
/// Measured — `audio info --select format,path,bytes` lost the `action` key that
/// the local flag deliberately keeps.
///
/// Collapsing those 32 declarations into this module is the correct end state and
/// is tracked as open debt. Until then the universal set uses names that cannot
/// collide, which keeps every documented per-command behaviour intact instead of
/// silently changing ten commands as a side effect of adding a global.
#[derive(Debug, Clone, Args)]
pub struct AgentOpsArgs {
    /// Project data down to these dotted paths (CSV): `--fields residual.ghost_marker_processes`
    ///
    /// Rebuilds the nesting each path implies, so the shape stays the one the
    /// command documents. Also disambiguates commands whose data holds more than
    /// one list.
    #[arg(
        long,
        global = true,
        value_name = "PATHS",
        help_heading = "Agent output"
    )]
    pub fields: Option<String>,

    /// Keep rows matching `key=value`, `key!=value` or `key~substring` (repeatable, ANDed)
    ///
    /// A missing field never matches, including under `!=`: absence is not
    /// difference.
    #[arg(
        long = "filter-rows",
        global = true,
        value_name = "EXPR",
        help_heading = "Agent output"
    )]
    pub filter_rows: Vec<String>,

    /// Emit at most N rows (applied after filter, dedupe and sort)
    #[arg(
        long = "limit-rows",
        global = true,
        value_name = "N",
        help_heading = "Agent output"
    )]
    pub limit_rows: Option<usize>,

    /// Sort rows by a dotted path (numbers compare numerically)
    #[arg(
        long = "sort-rows",
        global = true,
        value_name = "PATH",
        help_heading = "Agent output"
    )]
    pub sort_rows: Option<String>,

    /// Drop rows whose dotted-path value repeats, keeping the first
    #[arg(
        long = "dedupe-by",
        global = true,
        value_name = "PATH",
        help_heading = "Agent output"
    )]
    pub dedupe_by: Option<String>,

    /// Emit only `{"count": N}` instead of the rows
    #[arg(
        long = "count-only",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Agent output"
    )]
    pub count_only: bool,

    /// Cut every string in the payload to N characters, marking `truncated`
    #[arg(
        long = "truncate-content",
        global = true,
        value_name = "CHARS",
        help_heading = "Agent output"
    )]
    pub truncate_content: Option<usize>,

    /// Hard ceiling on emitted bytes; sheds rows from the end and marks `truncated`
    #[arg(
        long = "max-output-bytes",
        global = true,
        value_name = "BYTES",
        help_heading = "Agent output"
    )]
    pub max_output_bytes: Option<usize>,
}

impl AgentOpsArgs {
    /// Validate and convert to the runtime shape.
    ///
    /// Parsing happens once here rather than per envelope, so a malformed
    /// `--filter` fails before the command does any work instead of after.
    ///
    /// # Errors
    ///
    /// [`crate::error::ErrorKind::Usage`] when a `--filter` term has no operator
    /// or an empty left-hand side.
    pub fn to_ops(&self) -> Result<AgentOps, CliError> {
        let mut filter = Vec::with_capacity(self.filter_rows.len());
        for raw in &self.filter_rows {
            filter.push(FilterExpr::parse(raw)?);
        }
        Ok(AgentOps {
            select: self.fields.as_deref().map(split_csv).unwrap_or_default(),
            filter,
            limit: self.limit_rows,
            sort: self.sort_rows.clone(),
            dedupe_by: self.dedupe_by.clone(),
            count_only: self.count_only,
            truncate_content: self.truncate_content,
            max_output_bytes: self.max_output_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> AgentOpsArgs {
        AgentOpsArgs {
            fields: None,
            filter_rows: Vec::new(),
            limit_rows: None,
            sort_rows: None,
            dedupe_by: None,
            count_only: false,
            truncate_content: None,
            max_output_bytes: None,
        }
    }

    #[test]
    fn empty_args_convert_to_a_noop() {
        assert!(args().to_ops().expect("convert").is_noop());
    }

    #[test]
    fn select_splits_on_commas() {
        let mut a = args();
        a.fields = Some("x, y ,".into());
        let ops = a.to_ops().expect("convert");
        assert_eq!(ops.select, vec!["x".to_string(), "y".to_string()]);
    }

    /// A bad filter must fail at argv time, not after the command did its work.
    #[test]
    fn a_malformed_filter_fails_conversion() {
        let mut a = args();
        a.filter_rows = vec!["nooperator".into()];
        assert!(a.to_ops().is_err());
    }

    #[test]
    fn repeated_filters_are_all_kept() {
        let mut a = args();
        a.filter_rows = vec!["a=1".into(), "b~z".into()];
        assert_eq!(a.to_ops().expect("convert").filter.len(), 2);
    }
}
