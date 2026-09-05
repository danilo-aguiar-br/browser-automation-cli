// SPDX-License-Identifier: MIT OR Apache-2.0
//! Locating and moving the row list inside a command payload.
//!
//! Every row operation (`--filter-rows`, `--sort-rows`, `--dedupe-by`,
//! `--limit-rows`, `--count-only`) needs one question answered first: which
//! list did the caller mean? That question is this module's whole
//! responsibility, kept apart from the pipeline that consumes the answer.

use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Where the list to operate on lives inside `data`.
pub(crate) enum RowTarget {
    /// `data` is itself the list.
    Root,
    /// One field of `data` holds the list.
    Field(String),
}

/// Conventional names of the result list, most preferred first.
///
/// Consulted only when a payload holds MORE than one array, to pick the list a
/// caller means by "the rows". Refusing every such payload was the old
/// behaviour, and it made the product's own largest envelopes unreachable:
/// measured, `commands` carries six arrays, `locale` two, `batch-scrape`
/// `pages` plus `errors`, `storage export` `cookies` plus `origins`.
///
/// The first group is the generic result carriers, the second is the names this
/// product actually emits (measured, not guessed), and `data` is last because
/// it is the likeliest to be a nested envelope rather than a row set. A payload
/// whose arrays match none of these still errors, naming them all — that is a
/// genuinely ambiguous shape and a guess would be worse than a question.
const PRIMARY_ROW_KEYS: &[&str] = &[
    // Generic result carriers.
    "results",
    "items",
    "hits",
    "rows",
    "matches",
    // Names this product emits.
    "pages",
    "entities",
    "commands",
    "checks",
    "keys",
    "steps",
    "available",
    "cookies",
    "schemas",
    "endpoints",
    "domains",
    // Last: most likely to be a nested envelope.
    "data",
];

/// Locate the list inside `data`.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the payload holds no list at all, holds several
/// and none of them is a conventional result list, or is a bare scalar.
pub(crate) fn resolve_rows(data: &Value) -> Result<RowTarget, CliError> {
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
                    // More than one list is the COMMON shape, not the exotic
                    // one: `commands` carries six, `batch-scrape` carries
                    // `pages` plus `errors`, `storage export` carries `cookies`
                    // plus `origins`. Refusing all of them made those payloads
                    // — the largest the product emits — unreachable without a
                    // `--fields` first, which is the opposite of a reduction
                    // flag's job. Prefer the conventional result list by name.
                    if let Some(primary) = PRIMARY_ROW_KEYS
                        .iter()
                        .find(|k| arrays.iter().any(|a| a.as_str() == **k))
                    {
                        return Ok(RowTarget::Field((*primary).to_string()));
                    }
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

/// Detach the row list, leaving the surrounding object intact.
pub(crate) fn take_rows(data: &mut Value, target: &RowTarget) -> Vec<Value> {
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
pub(crate) fn put_rows(data: &mut Value, target: &RowTarget, rows: Vec<Value>) {
    match target {
        RowTarget::Root => *data = Value::Array(rows),
        RowTarget::Field(key) => {
            if let Some(slot) = data.get_mut(key) {
                *slot = Value::Array(rows);
            }
        }
    }
}

/// Borrow the row list without detaching it.
pub(crate) fn rows_ref<'a>(data: &'a Value, target: &RowTarget) -> Option<&'a Vec<Value>> {
    match target {
        RowTarget::Root => data.as_array(),
        RowTarget::Field(key) => data.get(key).and_then(Value::as_array),
    }
}
