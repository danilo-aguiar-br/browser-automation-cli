// SPDX-License-Identifier: MIT OR Apache-2.0
//! Turning "the hash moved" into "here is what moved".
//!
//! # Why a hash alone is not an answer
//!
//! `monitor check` reported `changed: true` and nothing else. That tells a
//! caller a page is different; it does not tell them whether the difference is
//! the price they are watching or the timestamp in the footer. Every run that
//! flipped the bit forced a human to go and look, which is the work the
//! command exists to remove.
//!
//! # Where the previous content comes from
//!
//! The baseline file holds a hash, and that contract predates this module. So
//! the content is kept beside it in `<baseline>.content` rather than changing
//! what the baseline file means — an old baseline keeps working, and a caller
//! who never asks for a diff never pays for the extra file.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// How much of a change to report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffMode {
    /// Hash only, the behaviour that predates this module.
    None,
    /// A unified diff, as text.
    Git,
    /// Added and removed lines as structured lists.
    Json,
}

impl DiffMode {
    /// Parse the `--diff-mode` value.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Usage`] for anything outside the three names. Falling back
    /// to `None` on a typo would silently answer the smaller question.
    pub(super) fn parse(raw: &str) -> Result<Self, CliError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "none" | "" => Ok(Self::None),
            "git" | "git-diff" | "unified" => Ok(Self::Git),
            "json" => Ok(Self::Json),
            other => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unknown --diff-mode: {other}"),
                crate::i18n::suggestion_key("use_listed_value", None),
            )),
        }
    }

    /// Whether the caller asked for anything beyond the hash.
    pub(super) fn wants_content(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// `<baseline>.content`, where the previous body is kept.
pub(super) fn content_path(baseline: &Path) -> PathBuf {
    let mut name = baseline.as_os_str().to_os_string();
    name.push(".content");
    PathBuf::from(name)
}

/// Build the diff payload, or explain why there is none yet.
///
/// Never fails the run: a missing or unreadable sidecar means the caller gets
/// the hash answer they always got, plus `diff_available: false` saying why.
/// Failing here would turn a first run with a new flag into an error.
///
/// # Why there is no root check here
///
/// GAP-026 asked for one and the answer is that it would be a second copy of a
/// check that already ran. `baseline` reaches this function from exactly one
/// place, `handle_monitor`, which bounds it with `ensure_read_allowed` before
/// the fetch; `content_path` derives the sidecar from that same bounded value,
/// so a refused `--baseline` never produces a path to read here.
///
/// Adding a check anyway would also break the contract stated above: this
/// function has no error channel, so a refusal would have to be laundered into
/// `diff_available: false`, reporting "no previous content recorded" for a path
/// that was rejected on policy. A wrong reason is worse than no reason.
pub(super) fn build_diff(mode: DiffMode, baseline: &Path, current: &str) -> Value {
    if !mode.wants_content() {
        return Value::Null;
    }
    let path = content_path(baseline);
    let Ok(previous) = std::fs::read_to_string(&path) else {
        return json!({
            "diff_available": false,
            "reason": "no previous content recorded; run again to compare against this one",
        });
    };
    let max = crate::xdg::resolve_monitor_diff_max_bytes();
    match mode {
        DiffMode::None => Value::Null,
        DiffMode::Git => {
            let text = unified_diff(&previous, current);
            let (text, truncated) = clamp(text, max);
            json!({
                "diff_available": true,
                "mode": "git",
                "diff": text,
                "diff_truncated": truncated,
            })
        }
        DiffMode::Json => {
            let (added, removed) = line_changes(&previous, current);
            // Counts are computed BEFORE the byte clamp, so a caller reading
            // `added_count` learns the real size of the change even when the
            // lists themselves were cut to fit.
            let (added_count, removed_count) = (added.len(), removed.len());
            let (added, removed) = clamp_lists(added, removed, max);
            json!({
                "diff_available": true,
                "mode": "json",
                "added": added,
                "removed": removed,
                "added_count": added_count,
                "removed_count": removed_count,
                "diff_truncated": added.len() < added_count || removed.len() < removed_count,
            })
        }
    }
}

/// Record the current content for the next run to compare against.
///
/// Best-effort by design: a scrape that succeeded must not be reported as
/// failed because a convenience sidecar could not be written.
pub(super) fn record_content(baseline: &Path, current: &str) {
    let path = content_path(baseline);
    if let Err(e) = crate::concurrency::write_bytes_sync(&path, current.as_bytes()) {
        tracing::warn!(
            target: "browser_automation_cli::monitor",
            path = %path.display(),
            error = %e,
            "could not record content for the next diff; the hash check is unaffected"
        );
    }
}

/// A unified diff between two texts, using the crate the product already has.
fn unified_diff(previous: &str, current: &str) -> String {
    similar::TextDiff::from_lines(previous, current)
        .unified_diff()
        .header("baseline", "current")
        .to_string()
}

/// Added and removed lines, without the surrounding context a diff carries.
fn line_changes(previous: &str, current: &str) -> (Vec<String>, Vec<String>) {
    let diff = similar::TextDiff::from_lines(previous, current);
    let mut added = Vec::new();
    let mut removed = Vec::new();
    for change in diff.iter_all_changes() {
        let line = change.value().trim_end_matches('\n').to_string();
        match change.tag() {
            similar::ChangeTag::Insert => added.push(line),
            similar::ChangeTag::Delete => removed.push(line),
            similar::ChangeTag::Equal => {}
        }
    }
    (added, removed)
}

/// Cut a string to a byte ceiling without splitting a UTF-8 sequence.
fn clamp(text: String, max: usize) -> (String, bool) {
    if max == 0 || text.len() <= max {
        return (text, false);
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

/// Drop whole lines from the end until both lists fit the budget.
///
/// Whole lines, never partial ones: half a line in a structured list is a
/// value a consumer would treat as real.
fn clamp_lists(
    mut added: Vec<String>,
    mut removed: Vec<String>,
    max: usize,
) -> (Vec<String>, Vec<String>) {
    if max == 0 {
        return (added, removed);
    }
    let size = |v: &[String]| v.iter().map(|s| s.len() + 3).sum::<usize>();
    while size(&added) + size(&removed) > max {
        if added.len() >= removed.len() {
            if added.pop().is_none() {
                break;
            }
        } else if removed.pop().is_none() {
            break;
        }
    }
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_modes_parse_and_nothing_else_does() {
        assert_eq!(DiffMode::parse("none").unwrap(), DiffMode::None);
        assert_eq!(DiffMode::parse("git").unwrap(), DiffMode::Git);
        assert_eq!(DiffMode::parse("JSON").unwrap(), DiffMode::Json);
        // A typo must not quietly answer the smaller question.
        assert!(DiffMode::parse("gti").is_err());
    }

    #[test]
    fn the_sidecar_sits_beside_the_baseline() {
        let p = content_path(Path::new("/tmp/site.baseline"));
        assert_eq!(p, Path::new("/tmp/site.baseline.content"));
    }

    #[test]
    fn line_changes_report_only_what_moved() {
        let (added, removed) = line_changes("a\nb\nc\n", "a\nB\nc\n");
        assert_eq!(added, vec!["B".to_string()]);
        assert_eq!(removed, vec!["b".to_string()]);
    }

    #[test]
    fn an_unchanged_page_produces_no_lines() {
        let (added, removed) = line_changes("same\n", "same\n");
        assert!(added.is_empty() && removed.is_empty());
    }

    #[test]
    fn clamping_never_splits_a_utf8_sequence() {
        // A byte-exact cut through "ç" would produce invalid UTF-8, which is a
        // panic in the making rather than a truncated string.
        let (out, truncated) = clamp("preço".to_string(), 4);
        assert!(truncated);
        assert!(out.chars().all(|c| c != '\u{fffd}'), "{out}");
    }

    #[test]
    fn clamping_lists_drops_whole_lines() {
        let added = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let (kept, _) = clamp_lists(added, Vec::new(), 12);
        assert!(kept.len() < 3);
        assert!(kept
            .iter()
            .all(|l| ["one", "two", "three"].contains(&l.as_str())));
    }

    #[test]
    fn a_zero_budget_means_no_clamp_rather_than_no_output() {
        let (out, truncated) = clamp("anything".to_string(), 0);
        assert_eq!(out, "anything");
        assert!(!truncated);
    }

    #[test]
    fn mode_none_produces_no_diff_object_at_all() {
        assert!(build_diff(DiffMode::None, Path::new("/tmp/x"), "body").is_null());
    }

    #[test]
    fn a_first_run_says_why_there_is_no_diff() {
        let missing = std::env::temp_dir().join(format!("bac-nodiff-{}", std::process::id()));
        let out = build_diff(DiffMode::Git, &missing, "body");
        assert_eq!(out["diff_available"], json!(false));
        assert!(out.get("reason").is_some());
    }
}
