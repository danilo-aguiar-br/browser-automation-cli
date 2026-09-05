// SPDX-License-Identifier: MIT OR Apache-2.0
//! The fixed order in which the operations run, and the process-wide install.
//!
//! # Why the order is a module of its own
//!
//! The order is the contract. Documented in [`super`] and repeated in the
//! product docs, it is:
//!
//! `select` → resolve rows → `filter` → `sort` → `dedupe-by` → `limit` →
//! `truncate-content` → `count-only` → `max-output-bytes`.
//!
//! Every step is written elsewhere — projection in [`super::path`], predicates
//! and sorting in [`super::filter`], ceilings in [`super::budget`], row
//! location in [`super::rows`]. What was left in the facade was the sequencing,
//! and sequencing is the one thing a reader has to be able to check at a
//! glance: reorder two lines here and `--limit-rows` starts cutting before
//! `--sort-rows` ranks, which is a silently wrong answer, not an error.
//!
//! The process-wide install lives here too, because it exists only to feed
//! [`apply_process_ops`]: the flags are parsed once, stored once, and read by
//! this pipeline. Splitting the store from its only consumer would leave two
//! files that cannot be understood apart.

use std::sync::Mutex;

use serde_json::{Map, Value};

use crate::error::{CliError, ErrorKind};
use crate::sync_util::lock_recover;

use super::rows::{put_rows, resolve_rows, rows_ref, take_rows};
use super::types::{AgentOps, AgentOpsReport, UnresolvedPath};
use super::{budget, filter, path};

static AGENT_OPS: Mutex<Option<AgentOps>> = Mutex::new(None);

/// Install the operations for this process (call once after argv parse).
pub fn set_agent_ops(ops: Option<AgentOps>) {
    *lock_recover(&AGENT_OPS) = ops.filter(|o| !o.is_noop());
}

/// Operations installed for this process, if any.
///
/// Private on purpose: `apply_process_ops` below is the only reader, and it is
/// the entry point every caller already uses. Was `pub` with no reader outside
/// this file, which the phantom-flag gate reports as an orphan getter.
fn agent_ops() -> Option<AgentOps> {
    lock_recover(&AGENT_OPS).clone()
}

/// Set when `--expect-exit-code` is on and an expectation did not hold.
static EXPECTATION_UNMET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Record that an opted-in expectation failed.
///
/// A flag rather than an error return, because the caller must still receive
/// the payload: the whole value of `--expect` is seeing WHY the assertion
/// failed, and swallowing the envelope to raise an exit code would take that
/// away. The exit code is applied after the envelope is written.
fn set_expectation_unmet() {
    EXPECTATION_UNMET.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether this process must exit non-zero because of `--expect-exit-code`.
#[must_use]
pub fn expectation_unmet() -> bool {
    EXPECTATION_UNMET.load(std::sync::atomic::Ordering::Relaxed)
}

/// Apply every requested operation to `data`.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when a row operation is requested against data with no
/// single list, or when a byte ceiling cannot be met even with zero rows. Both
/// are agent mistakes with a concrete fix, so they are reported rather than
/// silently approximated.
pub fn apply(mut data: Value, ops: &AgentOps) -> Result<(Value, AgentOpsReport), CliError> {
    let mut report = AgentOpsReport::default();

    if !ops.select.is_empty() {
        let (projected, unresolved) = path::project(&data, &ops.select);
        data = projected;
        report
            .unresolved_paths
            .extend(unresolved.into_iter().map(|path| UnresolvedPath {
                flag: "fields",
                path,
            }));
    }

    if ops.needs_rows() {
        let target = resolve_rows(&data)?;
        let mut rows = take_rows(&mut data, &target);
        report.total = Some(rows.len());

        if !ops.filter.is_empty() {
            rows = filter::retain_matching(rows, &ops.filter);
        }
        // Probe before mutating: dedupe drops rows and sort reorders them, and
        // neither can report afterwards whether the key was ever there.
        if let Some(key) = &ops.dedupe_by {
            if filter::rows_with_key(&rows, key) == 0 && !rows.is_empty() {
                report.unresolved_paths.push(UnresolvedPath {
                    flag: "dedupe-by",
                    path: key.clone(),
                });
            }
            rows = filter::dedupe_rows(rows, key);
        }
        if let Some(key) = &ops.sort {
            if filter::rows_with_key(&rows, key) == 0 && !rows.is_empty() {
                report.unresolved_paths.push(UnresolvedPath {
                    flag: "sort-rows",
                    path: key.clone(),
                });
            }
            filter::sort_rows(&mut rows, key);
        }
        report.matched = Some(rows.len());

        if let Some(limit) = ops.limit {
            if rows.len() > limit {
                rows.truncate(limit);
                report.truncated = true;
            }
        }
        if ops.count_only {
            return Ok((
                Value::Object(count_object(report.matched.unwrap_or(0))),
                report,
            ));
        }
        put_rows(&mut data, &target, rows);
    } else if ops.count_only {
        return Err(CliError::new(
            ErrorKind::Usage,
            "--count-only needs a list, and this command's data has none",
        ));
    }

    if let Some(max) = ops.truncate_content {
        if budget::truncate_strings(&mut data, max) {
            report.truncated = true;
        }
    }

    if let Some(max_bytes) = ops.max_output_bytes {
        apply_byte_budget(&mut data, max_bytes, &mut report)?;
    }

    Ok((data, report))
}

/// Enforce `--max-output-bytes` by shedding rows from the end.
fn apply_byte_budget(
    data: &mut Value,
    max_bytes: usize,
    report: &mut AgentOpsReport,
) -> Result<(), CliError> {
    if budget::serialized_len(data) <= max_bytes {
        return Ok(());
    }
    let Ok(target) = resolve_rows(data) else {
        return Err(over_budget(max_bytes));
    };
    let mut rows = take_rows(data, &target);
    let overhead = budget::serialized_len(data);
    let outcome = budget::fit_rows_to_budget(&mut rows, max_bytes, overhead);
    put_rows(data, &target, rows);
    if outcome.truncated {
        report.truncated = true;
        report.omitted_rows = Some(outcome.omitted_rows);
    }
    if outcome.still_over {
        return Err(over_budget(max_bytes));
    }
    Ok(())
}

fn over_budget(max_bytes: usize) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!("payload cannot fit --max-output-bytes {max_bytes}"),
        crate::i18n::suggestion_key("agent_ops_over_budget", None),
    )
}

fn count_object(count: usize) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("count".to_string(), Value::from(count));
    map
}

/// Apply the operations installed for this process.
///
/// # Errors
///
/// Propagates the usage errors described on [`apply`].
pub fn apply_process_ops(data: Value) -> Result<(Value, Option<AgentOpsReport>), CliError> {
    let Some(ops) = agent_ops() else {
        return Ok((data, None));
    };
    let (data, mut report) = apply(data, &ops)?;
    // Evaluated here rather than inside `apply` because `apply` returns early
    // on `--count-only`. An assertion that skips a branch is worse than no
    // assertion: it reports "held" for a payload it never looked at.
    report.expectation_unmet = unmet_expectations(&data, &ops);
    if !report.expectation_unmet.is_empty() && ops.expect_exit_code {
        set_expectation_unmet();
    }
    Ok((
        data,
        if report.is_empty() {
            None
        } else {
            Some(report)
        },
    ))
}

/// Which `--expect` expressions the emitted payload fails.
///
/// # Where an expectation is evaluated
///
/// Against every row when the payload has a row list, and against the payload
/// object itself when it does not. An expectation holds when at least one row
/// satisfies it — `--expect status=200` asks "is there a 200 in here?", which
/// is the question an agent actually has. Requiring every row to match would
/// make the flag useless on any multi-row command without a `--filter-rows`
/// first, and the caller can already get the stricter reading by filtering.
pub(super) fn unmet_expectations(data: &Value, ops: &AgentOps) -> Vec<String> {
    if ops.expect.is_empty() {
        return Vec::new();
    }
    let rows: Vec<&Value> = match resolve_rows(data) {
        Ok(target) => match rows_ref(data, &target) {
            Some(list) => list.iter().collect(),
            None => vec![data],
        },
        // No list: the payload is the one thing there is to assert about.
        Err(_) => vec![data],
    };
    ops.expect
        .iter()
        .filter(|(_, pred)| !rows.iter().any(|row| pred.matches(row)))
        .map(|(raw, _)| raw.clone())
        .collect()
}
