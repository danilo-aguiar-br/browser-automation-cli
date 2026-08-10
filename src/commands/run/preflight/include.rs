// SPDX-License-Identifier: MIT OR Apache-2.0
//! `include` step expansion with cycle and depth protection.

use super::super::execute::reject_unknown_step_fields;
use super::super::parse::parse_run_script;
use super::{max_include_depth, INCLUDE_CMD};
use crate::error::{CliError, ErrorKind};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Read one script file and splice in every `include` it declares.
pub(super) fn load_expanded(
    path: &Path,
    stack: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<Vec<Value>, CliError> {
    let max_depth = max_include_depth();
    if depth > max_depth {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "include nesting deeper than {max_depth} levels at {}",
                path.display()
            ),
            crate::i18n::suggestion_key("include_depth", None),
        ));
    }

    // Canonicalize so two spellings of the same file still close a cycle.
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if stack.contains(&key) {
        let chain = stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!("include cycle: {chain} -> {}", key.display()),
            crate::i18n::suggestion_key("include_cycle", None),
        ));
    }

    // GAP-034 pillar 3: an include names a path the caller controls, so it goes
    // through the same allowed-roots check as any other read. Without this an
    // include could pull a script from outside the workspace.
    crate::fs_roots::ensure_read_allowed(path)?;

    let text =
        crate::json_util::read_text_file_limited(path, crate::xdg::resolve_max_json_file_bytes())
            .map_err(|e| {
            if e.kind() == ErrorKind::Io || e.kind() == ErrorKind::NoInput {
                CliError::with_suggestion(
                    ErrorKind::NoInput,
                    format!("cannot read script {}: {}", path.display(), e.message()),
                    crate::i18n::suggestion_key("run_script_file", None),
                )
            } else {
                e
            }
        })?;

    let parsed = parse_run_script(&text)?;
    stack.push(key);

    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut out = Vec::with_capacity(parsed.len());
    for step in parsed {
        if step_cmd(&step) != INCLUDE_CMD {
            out.push(step);
            continue;
        }
        // Reject typo fields here: an include never reaches dispatch, so this is
        // its only chance to be checked.
        reject_unknown_step_fields(INCLUDE_CMD, &step)?;
        let target = include_target(&step).ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Data,
                format!("include without a path in {}", path.display()),
                crate::i18n::suggestion_key("include_path_required", None),
            )
        })?;
        let nested = base.join(target);
        out.extend(load_expanded(&nested, stack, depth + 1)?);
    }

    stack.pop();
    Ok(out)
}

/// The path an `include` step points at, if this step is one.
fn include_target(step: &Value) -> Option<&str> {
    let cmd = step_cmd(step);
    if cmd != INCLUDE_CMD {
        return None;
    }
    step.get("path")
        .or_else(|| step.get("script"))
        .or_else(|| step.get("file"))
        .and_then(|v| v.as_str())
}

/// The command token of a step (`cmd`, falling back to `action`).
pub(super) fn step_cmd(step: &Value) -> &str {
    step.get("cmd")
        .or_else(|| step.get("action"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
}
