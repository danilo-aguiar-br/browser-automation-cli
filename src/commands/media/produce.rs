// SPDX-License-Identifier: MIT OR Apache-2.0
//! Batch support for the media actions that WRITE a file.
//!
//! A read action needs no output contract: N inputs give N envelopes. An action
//! that produces a file does, because a single `--out` cannot name N
//! destinations. This module is that contract, and it is deliberately narrow:
//! derive one output beside each input, and refuse rather than overwrite.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::batch::{envelope, err_item, ok_item, MediaInputs};
use crate::error::{CliError, ErrorKind};

/// Run a file-producing operation over the resolved inputs.
///
/// `ext_for` gives the target extension for one input, because the target
/// format is per-action: fixed for `to-mp3`, flag-driven for `convert`, and
/// inherited from the input for `trim`.
///
/// `f` receives the source and the destination this run decided on. In the
/// single-input case that destination is whatever `--out` said, including
/// `None`, so nothing about the existing behaviour moves.
///
/// # Why `--out` is refused alongside `--paths-file`
///
/// One destination cannot serve N inputs. Accepting both would mean either
/// overwriting the same file N-1 times or silently ignoring the flag, and both
/// readings destroy work the operator asked for.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when `--out` is combined with `--paths-file`.
/// [`ErrorKind::Data`] (exit 65) when every item of a batch failed, since a run
/// that wrote nothing must not answer `ok: true`. A PARTIAL batch succeeds: a
/// bad item must not discard the outputs already produced, so `error_count` —
/// not `ok` — is what says whether anything went wrong.
/// Propagates a single-input failure unchanged.
pub(crate) fn run_producing<S>(
    label: &str,
    inputs: MediaInputs<S>,
    out: Option<&Path>,
    ext_for: impl Fn(&Path) -> String,
    f: impl Fn(&S, Option<&Path>) -> Result<Value, CliError>,
) -> Result<Value, CliError> {
    let items = match inputs {
        MediaInputs::Single(src) => return f(&src, out),
        MediaInputs::Batch(items) => items,
    };
    if out.is_some() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "{label}: --out cannot be combined with --paths-file; one destination \
                 cannot serve {} inputs. Each output is derived beside its input.",
                items.len()
            ),
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }

    let mut report = Vec::with_capacity(items.len());
    for (path, src) in &items {
        let derived = match derive_out(path, &ext_for(path)) {
            Ok(p) => p,
            Err(e) => {
                report.push(err_item(path, None, &e));
                continue;
            }
        };
        report.push(match f(src, Some(&derived)) {
            Ok(data) => ok_item(path, Some(&derived), data),
            Err(e) => err_item(path, Some(&derived), &e),
        });
    }
    envelope(label, report)
}

/// Destination for one batch item: the input path with the target extension.
///
/// # Why not the single-input default
///
/// Without `--out` a single run writes `<action>-<millis>.<ext>` under the XDG
/// cache. Measured, that stamp is in MILLISECONDS, so two items of a fast batch
/// can land on the same name and the second silently destroys the first. A name
/// derived from the input is unique exactly when the inputs are.
///
/// # Errors
///
/// [`ErrorKind::Data`] when the derived path is the input itself (converting a
/// format to itself would truncate the source mid-read) or when it already
/// exists. Neither is a reason to stop the batch — the caller records the item
/// and continues.
fn derive_out(input: &Path, ext: &str) -> Result<PathBuf, CliError> {
    let derived = input.with_extension(ext);
    if derived == input {
        return Err(CliError::new(
            ErrorKind::Data,
            format!(
                "derived output equals the input {}; refusing to write a file over its own source",
                input.display()
            ),
        ));
    }
    if derived.exists() {
        return Err(CliError::new(
            ErrorKind::Data,
            format!(
                "derived output {} already exists; refusing to overwrite",
                derived.display()
            ),
        ));
    }
    Ok(derived)
}

/// Extension of `input`, lowercased, for actions whose format follows the input.
///
/// `trim` keeps the container it was given, so its target extension is not a
/// flag but a property of each item.
pub(crate) fn input_ext(input: &Path, fallback: &str) -> String {
    input
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| fallback.to_string())
}
