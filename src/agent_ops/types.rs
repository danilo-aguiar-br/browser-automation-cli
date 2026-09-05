// SPDX-License-Identifier: MIT OR Apache-2.0
//! What was asked for, and what was done: the vocabulary of the operations.
//!
//! # Why this is separate from [`super::pipeline`]
//!
//! Two different audiences read these two files. The argv parser
//! (`cli::agent_ops_args`) and every envelope consumer only need the shapes:
//! which flags exist, what each field means, when a reduction is refused. The
//! pipeline needs none of that prose — it needs the order in which the
//! operations run.
//!
//! Mixing them meant a change to the report's serialization sat in the same
//! file as the loop that sheds rows to fit a byte budget, and neither could be
//! read without the other. The types are also the stable half: they are named
//! in the JSON envelope and cannot drift, while the pipeline is free to be
//! reorganised as long as the documented order holds.

use crate::error::{CliError, ErrorKind};

use super::filter::FilterExpr;

/// Parsed, validated agent operations for this process.
#[derive(Debug, Default, Clone)]
pub struct AgentOps {
    /// Dotted paths to project.
    pub select: Vec<String>,
    /// Row predicates, ANDed.
    pub filter: Vec<FilterExpr>,
    /// Maximum rows to emit.
    pub limit: Option<usize>,
    /// Dotted path to sort rows by.
    pub sort: Option<String>,
    /// Dotted path to deduplicate rows by.
    pub dedupe_by: Option<String>,
    /// Emit only a count.
    pub count_only: bool,
    /// Character ceiling for every string in the payload.
    pub truncate_content: Option<usize>,
    /// Byte ceiling for the emitted payload.
    pub max_output_bytes: Option<usize>,
    /// Assertions over the emitted payload, as `(original text, predicate)`.
    ///
    /// The original text is kept so the report can echo the caller's own
    /// argument back. Reconstructing it from the parsed predicate would
    /// normalise spacing and leave the agent matching a string it never wrote.
    pub expect: Vec<(String, FilterExpr)>,
    /// Turn an unmet expectation into a non-zero exit.
    pub expect_exit_code: bool,
}

impl AgentOps {
    /// True when nothing was requested, so the envelope is emitted untouched.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.select.is_empty()
            && self.filter.is_empty()
            && self.limit.is_none()
            && self.sort.is_none()
            && self.dedupe_by.is_none()
            && !self.count_only
            && self.truncate_content.is_none()
            && self.max_output_bytes.is_none()
            && self.expect.is_empty()
    }

    /// Names of the reduction operations the caller asked for.
    ///
    /// Only the ones that reshape the payload: `--expect` is a gate, not a cut,
    /// and it works the same on any output mode.
    fn reshaping_ops(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if !self.select.is_empty() {
            names.push("--fields");
        }
        if !self.filter.is_empty() {
            names.push("--filter-rows");
        }
        if self.limit.is_some() {
            names.push("--limit-rows");
        }
        if self.sort.is_some() {
            names.push("--sort-rows");
        }
        if self.dedupe_by.is_some() {
            names.push("--dedupe-by");
        }
        if self.count_only {
            names.push("--count-only");
        }
        if self.truncate_content.is_some() {
            names.push("--truncate-content");
        }
        if self.max_output_bytes.is_some() {
            names.push("--max-output-bytes");
        }
        names
    }

    /// Refuse reduction operations on an output mode that cannot honour them.
    ///
    /// These operations reshape a JSON payload. The human text renderer reads
    /// named fields it expects to be there, so applying a cut before it would
    /// blank the output instead of shrinking it — and applying nothing at all
    /// is worse, because the flag is then accepted with exit 0 and no effect.
    ///
    /// Measured before this refusal existed: `--count-only config list-keys`
    /// without `--json` printed 23102 bytes, while the same call with `--json`
    /// printed 92. The operator had no way to tell the flag was inert.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Usage`] naming every reduction flag that was ignored, so
    /// the fix is visible without reading this source.
    pub fn refuse_unless_json(&self, json: bool) -> Result<(), CliError> {
        if json || self.is_noop() {
            return Ok(());
        }
        let asked = self.reshaping_ops();
        if asked.is_empty() {
            return Ok(());
        }
        Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "payload reduction ({}) needs --json; \
                 the human renderer cannot honour it",
                asked.join(", ")
            ),
            crate::i18n::suggestion_key("agent-ops-needs-json", None),
        ))
    }

    /// True when any operation needs a list to work on.
    pub(super) fn needs_rows(&self) -> bool {
        !self.filter.is_empty()
            || self.limit.is_some()
            || self.sort.is_some()
            || self.dedupe_by.is_some()
            || self.count_only
    }
}

/// What the operations did, echoed in the envelope.
///
/// Emitted only when something actually happened, so an untouched envelope keeps
/// its exact previous shape and no existing consumer has to learn a new field.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct AgentOpsReport {
    /// Rows before filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    /// Rows after filtering and deduplication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<usize>,
    /// True when a limit, a byte ceiling or string truncation cut the payload.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// Rows dropped to satisfy `--max-output-bytes`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omitted_rows: Option<usize>,
    /// Dotted paths that did not resolve, with the flag that asked for each.
    ///
    /// Without this an agent cannot tell "the key was there and this is the
    /// answer" from "the key was never there and I did nothing". `--fields
    /// typo.path` used to return `data:{}` with exit 0, and `--sort-rows
    /// typo.path` returned the rows in their original order with
    /// `matched == total` — both indistinguishable from success. Naming the
    /// paths is what makes the report actionable: a count of one tells the
    /// agent that something failed, not which of its keys was misspelled.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unresolved_paths: Vec<UnresolvedPath>,
    /// `--expect` expressions the emitted payload did not satisfy.
    ///
    /// Echoed verbatim so the agent matches the argument it passed. Absent
    /// when every expectation held, which keeps "asserted and passed"
    /// distinguishable from "asserted nothing" only through the exit code and
    /// the caller's own argv — the agent already knows what it asked for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expectation_unmet: Vec<String>,
}

/// One requested dotted path that no row (or the payload) actually carried.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UnresolvedPath {
    /// The flag that asked for it: `fields`, `sort-rows` or `dedupe-by`.
    pub flag: &'static str,
    /// The dotted path exactly as the agent typed it.
    pub path: String,
}

impl AgentOpsReport {
    /// True when nothing is worth telling the caller, so the field is omitted.
    pub(crate) fn is_empty(&self) -> bool {
        self.total.is_none()
            && self.matched.is_none()
            && !self.truncated
            && self.omitted_rows.is_none()
            && self.unresolved_paths.is_empty()
            && self.expectation_unmet.is_empty()
    }
}
