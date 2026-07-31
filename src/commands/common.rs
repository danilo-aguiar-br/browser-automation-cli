// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared emit helpers and meta handlers for command dispatch.

use crate::envelope::{print_error_json, print_success_json};
use crate::error::{CliError, ErrorKind};

pub(crate) fn handle_version(json: bool) -> Result<(), CliError> {
    let data = crate::build_identity();
    emit_ok(data, json, |d| {
        let ver = d.get("version").and_then(|v| v.as_str()).unwrap_or("");
        let sha = d
            .get("git_sha")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if sha != "unknown" {
            crate::output::writeln_stdout(format!("{ver} ({sha})"))
        } else {
            crate::output::writeln_stdout(ver)
        }
    })
}

pub(crate) fn handle_locale(json: bool) -> Result<(), CliError> {
    let data = crate::i18n::locale_diagnostics();
    emit_ok(data, json, |d| {
        let resolved = d.get("resolved").and_then(|v| v.as_str()).unwrap_or("en");
        let source = d
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let system = d
            .get("system_locale")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        // Human labels follow resolved locale; values stay BCP47/English keys.
        let label_r =
            crate::i18n::UiMessage::LocaleResolved.text(crate::i18n::effective_ui_locale());
        let label_s = crate::i18n::UiMessage::LocaleSource.text(crate::i18n::effective_ui_locale());
        crate::output::writeln_stdout(format!(
            "{label_r}: {resolved}\n{label_s}: {source}\nsystem: {system}"
        ))
    })
}

pub(crate) fn emit_ok<F>(data: serde_json::Value, json: bool, text: F) -> Result<(), CliError>
where
    F: FnOnce(&serde_json::Value) -> Result<(), CliError>,
{
    if json {
        print_success_json(data)?;
    } else {
        text(&data)?;
    }
    crate::output::flush_stdout()?;
    Ok(())
}

pub(crate) fn emit_err(err: &CliError, json: bool) -> i32 {
    let localized = crate::i18n::localize_error_suggestion(err);
    if json {
        match print_error_json(&localized) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::BrokenPipe => return 141,
            Err(_) => {}
        }
    } else {
        let _ = crate::output::writeln_stderr(format!("error: {localized}"));
        if let Some(s) = localized.suggestion() {
            let _ = crate::output::writeln_stderr(format!("suggestion: {s}"));
        }
    }
    let _ = crate::output::flush_stdout();
    localized.exit_code() as i32
}

/// Peel known global flags mistakenly captured by `exec` trailing_var_arg.
pub(crate) fn peel_trailing_globals(args: &[String]) -> (Vec<String>, bool) {
    let mut json = false;
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        match a.as_str() {
            "--json" => json = true,
            other => out.push(other.to_string()),
        }
    }
    (out, json)
}
