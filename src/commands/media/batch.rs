// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resolve a media input list and run one operation over it.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// What a media action was pointed at.
///
/// `Single` is the pre-existing shape and emits the operation envelope
/// unchanged, so adding `--paths-file` costs nothing to a caller that does not
/// use it.
pub(crate) enum MediaInputs<S> {
    /// One source, from `--path` or `--stdin`.
    Single(S),
    /// Many sources, from `--paths-file`, each carrying its own path for the
    /// per-item report.
    Batch(Vec<(PathBuf, S)>),
}

/// Resolve `--path` / `--stdin` / `--paths-file` into a source list.
///
/// # Why the three are mutually exclusive
///
/// Each names the input by a different mechanism. Accepting two would mean
/// picking a winner, and a silent winner is how an operator ends up measuring
/// a file they did not pass. Every conflicting combination is
/// [`ErrorKind::Usage`], which the process reports as exit 2.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when zero or more than one input mechanism is given, or
/// when the list file is over the `max_urls_file_bytes` ceiling or empty.
pub(crate) fn resolve<S>(
    label: &str,
    path: Option<PathBuf>,
    stdin: bool,
    paths_file: Option<PathBuf>,
    from_path: impl Fn(PathBuf) -> S,
    stdin_source: impl FnOnce() -> S,
) -> Result<MediaInputs<S>, CliError> {
    match (path, stdin, paths_file) {
        (None, false, Some(list)) => {
            let paths = read_paths_file(label, &list)?;
            Ok(MediaInputs::Batch(
                paths
                    .into_iter()
                    .map(|p| (p.clone(), from_path(p)))
                    .collect(),
            ))
        }
        (Some(p), false, None) => Ok(MediaInputs::Single(from_path(p))),
        (None, true, None) => Ok(MediaInputs::Single(stdin_source())),
        (None, false, None) => Err(CliError::new(
            ErrorKind::Usage,
            format!("{label}: require --path, --stdin, or --paths-file"),
        )),
        _ => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("{label}: pass exactly one of --path, --stdin, or --paths-file"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )),
    }
}

/// Read one path per line, `#` comments and blanks skipped.
///
/// Reuses [`crate::scrape_local::read_urls_file`] rather than opening the file
/// a second way: that reader is the one place the `max_urls_file_bytes` ceiling
/// is enforced, and an unbounded `read_to_string` on operator input is how a
/// one-shot process is OOM-killed by a file it was only asked to look at. Only
/// the wording is restated, because "urls file" and "no URLs" would misname
/// what the operator actually passed.
fn read_paths_file(label: &str, list: &Path) -> Result<Vec<PathBuf>, CliError> {
    match crate::scrape_local::read_urls_file(list) {
        Ok(lines) => Ok(lines.into_iter().map(PathBuf::from).collect()),
        Err(e) => Err(CliError::new(
            e.kind(),
            format!(
                "{label} --paths-file {}: {}",
                list.display(),
                e.message()
                    .replace("urls file", "paths file")
                    .replace("no URLs", "no paths")
            ),
        )),
    }
}

/// Run `f` over the resolved inputs and build the envelope.
///
/// # Why a failing item does not end the run
///
/// A batch exists to spare N processes. Aborting on the first bad file would
/// make the batch strictly worse than the loop it replaces: the operator would
/// still have to re-run, and would have lost the results already computed. Each
/// item reports its own outcome and the run reports the counts.
///
/// A batch that produced ANYTHING is `ok: true`; `error_count` is the signal
/// that some items failed, and it must be read rather than assumed zero. A
/// batch that produced NOTHING is a failure — see [`envelope`].
///
/// # Errors
///
/// Propagates a single-source failure unchanged. For a batch, [`ErrorKind::Data`]
/// (exit 65) when every item failed; the per-item detail rides along in `data`.
///
/// # Why the items run concurrently
///
/// The items are independent by construction — each reads its own file and
/// reports its own outcome — and each one is a whole media operation: a decode,
/// a probe, a parse. A serial loop over N of them costs N times the slowest
/// item for no reason.
///
/// The pool is the product's own: [`install_rayon_pool_once`] sizes the global
/// Rayon pool with `rayon_threads()`, which is the process concurrency budget
/// and therefore already honours `--max-concurrency`. No ceiling is invented
/// here. `par_iter().map().collect()` keeps the input order, so the report still
/// lists items in the order the operator wrote them.
///
/// [`crate::concurrency::map_cpu`] is deliberately not used: its
/// `CPU_MAP_THRESHOLD` keeps collections under 32 sequential, which is right for
/// the in-memory filters it was written for and wrong here, where five items can
/// be five seconds of work.
///
/// [`install_rayon_pool_once`]: crate::concurrency::install_rayon_pool_once
pub(crate) fn run<S>(
    label: &str,
    inputs: MediaInputs<S>,
    f: impl Fn(&S) -> Result<Value, CliError> + Sync + Send,
) -> Result<Value, CliError>
where
    S: Sync,
{
    let items = match inputs {
        MediaInputs::Single(src) => return f(&src),
        MediaInputs::Batch(items) => items,
    };
    crate::concurrency::install_rayon_pool_once();
    use rayon::prelude::*;
    let out: Vec<Value> = items
        .par_iter()
        .map(|(path, src)| match f(src) {
            Ok(data) => ok_item(path, None, data),
            Err(e) => err_item(path, None, &e),
        })
        .collect();
    envelope(label, out)
}

/// One successful item of a batch report.
pub(super) fn ok_item(path: &Path, path_out: Option<&Path>, data: Value) -> Value {
    // Built by hand rather than with `json!` so `data` is MOVED into the report
    // instead of re-serialized: an operation envelope can carry a full EXIF map
    // or a variant ladder, and copying it once per item is pure waste.
    let mut obj = serde_json::Map::new();
    obj.insert("path".into(), json!(path.display().to_string()));
    obj.insert(
        "path_out".into(),
        json!(path_out.map(|p| p.display().to_string())),
    );
    obj.insert("ok".into(), Value::Bool(true));
    obj.insert("data".into(), data);
    Value::Object(obj)
}

/// One failed item of a batch report.
///
/// The input path is always named. Without it the operator is told that
/// something in a list of N failed, which is the one thing a batch report
/// exists to avoid.
pub(super) fn err_item(path: &Path, path_out: Option<&Path>, e: &CliError) -> Value {
    json!({
        "path": path.display().to_string(),
        "path_out": path_out.map(|p| p.display().to_string()),
        "ok": false,
        "error": {
            "kind": e.kind().as_str(),
            "message": e.message(),
        },
    })
}

/// Wrap per-item reports with the counts an agent branches on.
///
/// # Why a fully failed batch is an error, not a report
///
/// The envelope's `ok` is the first field the contract tells an agent to read.
/// A batch where NOTHING succeeded answering `ok: true` with exit 0 is a green
/// that means nothing: the caller concludes the work was done and moves on.
/// Zero successes out of N is a failed run, so it is reported as one — with the
/// per-item detail still attached under `data`, because the caller needs to
/// know WHICH inputs failed and why.
///
/// A partial batch stays successful: it genuinely produced. `error_count` is
/// the signal there, which is why every `--paths-file` doc comment says to read
/// it rather than treating `ok: true` as "all passed".
///
/// # Errors
///
/// [`ErrorKind::Data`] (exit 65) when `count > 0` and `ok_count == 0`.
pub(super) fn envelope(label: &str, items: Vec<Value>) -> Result<Value, CliError> {
    let ok_count = items
        .iter()
        .filter(|i| i.get("ok").and_then(Value::as_bool) == Some(true))
        .count();
    let count = items.len();
    // Same reason as `ok_item`: the item list is moved, never copied.
    let mut obj = serde_json::Map::new();
    obj.insert("action".into(), json!(format!("{label}-batch")));
    obj.insert("count".into(), json!(count));
    obj.insert("ok_count".into(), json!(ok_count));
    obj.insert("error_count".into(), json!(count - ok_count));
    obj.insert("items".into(), Value::Array(items));
    let data = Value::Object(obj);

    if count > 0 && ok_count == 0 {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!("{label}: none of the {count} inputs produced a result"),
            crate::i18n::suggestion_key("file_path_invalid", None),
        )
        .with_data(data));
    }
    Ok(data)
}
