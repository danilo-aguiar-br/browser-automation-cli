// SPDX-License-Identifier: MIT OR Apache-2.0
//! Universal data operations applied to the success envelope (agent CLEAN STDOUT).
//!
//! # Why this lives at the envelope and not in each command
//!
//! The product law is that the binary does the heavy work and the model receives
//! the minimum useful payload. Measured before this module existed:
//! `doctor --offline --quick` emitted 26_277 bytes with no way to narrow it, and
//! the product's own COOKBOOK told agents to pipe it through `jaq`. An agent that
//! needs a JSON processor in its prompt is an agent the CLI failed.
//!
//! Only 8 of 69 commands offered any of these operations, and they disagreed:
//! `crawl` grew eight of them locally, `scrape` grew one, and `doctor` — the most
//! invoked diagnostic — grew none. Implementing them once over `data` covers
//! every command, including the ones nobody thought to wire.
//!
//! # Order of operations
//!
//! `select` → resolve rows → `filter` → `sort` → `dedupe-by` → `limit` →
//! `truncate-content` → `count-only` → `max-output-bytes`.
//!
//! `select` runs FIRST on purpose. It is also the disambiguator: a `data` object
//! holding two arrays has no single obvious list to filter, and narrowing with
//! `--fields checks` leaves exactly one.

pub mod budget;
pub mod filter;
pub mod path;

use std::sync::Mutex;

use serde_json::{Map, Value};

use crate::error::{CliError, ErrorKind};
use crate::sync_util::lock_recover;
use filter::FilterExpr;

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

    /// True when any operation needs a list to work on.
    fn needs_rows(&self) -> bool {
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
    fn is_empty(&self) -> bool {
        self.total.is_none()
            && self.matched.is_none()
            && !self.truncated
            && self.omitted_rows.is_none()
            && self.unresolved_paths.is_empty()
            && self.expectation_unmet.is_empty()
    }
}

static AGENT_OPS: Mutex<Option<AgentOps>> = Mutex::new(None);

/// Install the operations for this process (call once after argv parse).
pub fn set_agent_ops(ops: Option<AgentOps>) {
    *lock_recover(&AGENT_OPS) = ops.filter(|o| !o.is_noop());
}

/// Operations installed for this process, if any.
#[must_use]
pub fn agent_ops() -> Option<AgentOps> {
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

/// Where the list to operate on lives inside `data`.
enum RowTarget {
    /// `data` is itself the list.
    Root,
    /// Exactly one field of `data` holds a list.
    Field(String),
}

/// Locate the single list inside `data`.
fn resolve_rows(data: &Value) -> Result<RowTarget, CliError> {
    match data {
        Value::Array(_) => Ok(RowTarget::Root),
        Value::Object(map) => {
            let arrays: Vec<&String> = map
                .iter()
                .filter(|(_, v)| v.is_array())
                .map(|(k, _)| k)
                .collect();
            match arrays.len() {
                1 => Ok(RowTarget::Field(arrays[0].clone())),
                0 => Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "this command's data has no list to filter, sort, dedupe or limit",
                    crate::i18n::suggestion_key("agent_ops_no_rows", None),
                )),
                _ => {
                    let mut names: Vec<&str> = arrays.iter().map(|s| s.as_str()).collect();
                    names.sort_unstable();
                    Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("data holds more than one list: {}", names.join(", ")),
                        crate::i18n::suggestion_key("agent_ops_many_rows", None),
                    ))
                }
            }
        }
        _ => Err(CliError::new(
            ErrorKind::Usage,
            "this command's data is a scalar and has no list to operate on",
        )),
    }
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

/// Detach the row list, leaving the surrounding object intact.
fn take_rows(data: &mut Value, target: &RowTarget) -> Vec<Value> {
    let slot = match target {
        RowTarget::Root => data,
        RowTarget::Field(key) => match data.get_mut(key) {
            Some(v) => v,
            None => return Vec::new(),
        },
    };
    match slot.take() {
        Value::Array(rows) => rows,
        other => {
            *slot = other;
            Vec::new()
        }
    }
}

/// Put the row list back where it came from.
fn put_rows(data: &mut Value, target: &RowTarget, rows: Vec<Value>) {
    match target {
        RowTarget::Root => *data = Value::Array(rows),
        RowTarget::Field(key) => {
            if let Some(slot) = data.get_mut(key) {
                *slot = Value::Array(rows);
            }
        }
    }
}

/// Apply the process-wide operations, if any, and return the report to echo.
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
fn unmet_expectations(data: &Value, ops: &AgentOps) -> Vec<String> {
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

/// Borrow the row list without detaching it.
fn rows_ref<'a>(data: &'a Value, target: &RowTarget) -> Option<&'a Vec<Value>> {
    match target {
        RowTarget::Root => data.as_array(),
        RowTarget::Field(key) => data.get(key).and_then(Value::as_array),
    }
}

#[cfg(test)]
mod tests;
