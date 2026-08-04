// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pure helpers for browser session (no session type dependency).

use std::path::Path;

use crate::error::{CliError, ErrorKind};

/// Confirm a file's magic bytes match `format`, reading only the header.
///
/// Reads `IMAGE_MAGIC_PROBE_BYTES` instead of the whole file: this checks the
/// first few bytes, and slurping a 500 MB screenshot into RAM to inspect twelve
/// of them is a cost with no upside. The constant existed for exactly this and
/// was not being used.
pub(crate) fn verify_image_magic(path: &Path, format: &str) -> bool {
    use std::io::Read;

    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = Vec::with_capacity(crate::constants::IMAGE_MAGIC_PROBE_BYTES);
    // `take` caps the read at the probe window; a shorter file simply yields
    // fewer bytes and the format check fails on its own terms.
    if file
        .take(crate::constants::IMAGE_MAGIC_PROBE_BYTES as u64)
        .read_to_end(&mut head)
        .is_err()
    {
        return false;
    }
    crate::image_local::verify_format_name(&head, format)
}

/// Rewrite native `[ref=eN]` markers to agent-facing `[@eN]`.
pub fn tree_to_at_refs(tree: &str) -> String {
    let mut out = String::with_capacity(tree.len());
    let bytes = tree.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"ref=") && i + 4 < bytes.len() && bytes[i + 4] == b'e' {
            out.push('@');
            i += 4;
            while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                out.push(bytes[i] as char);
                i += 1;
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Normalize JS for `Runtime.evaluate`.
///
/// - With `--args`, always call as `({expr})(arg0,…)`.
/// - Bare function / arrow: call once as `({expr})()`.
/// - Already-invoked IIFE ending in `)()`: leave as-is (never double-call).
/// - Plain expressions: leave as-is.
pub(crate) fn normalize_eval_expression(
    expression: &str,
    args_json: Option<&str>,
) -> Result<String, CliError> {
    if let Some(args_raw) = args_json {
        let uids: Vec<String> = crate::json_util::from_str(args_raw).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                format!("eval --args must be a JSON array of uids: {e}"),
                r#"Example: --args '["@e1","@e2"]'"#,
            )
        })?;
        let args_js: Vec<String> = uids
            .iter()
            .map(|u| {
                let cleaned = u.trim().trim_start_matches('@');
                format!("\"{cleaned}\"")
            })
            .collect();
        let joined = args_js.join(",");
        return Ok(format!("({expression})({joined})"));
    }

    let trimmed = expression.trim();
    // Strip a single trailing semicolon for IIFE detection only.
    let for_detect = trimmed.trim_end_matches(';').trim_end();
    // Already invoked: `(...)()` or `(async ...)()` — re-wrapping yields "is not a function".
    if for_detect.ends_with(")()") {
        return Ok(expression.to_string());
    }

    let head = trimmed.trim_start();
    let is_bare_callable = head.starts_with("function")
        || head.starts_with("async function")
        || (head.starts_with("async") && trimmed.contains("=>"))
        || (head.starts_with('(') && trimmed.contains("=>"));

    if is_bare_callable {
        // Bare function / arrow needs a single call site.
        return Ok(format!("({expression})()"));
    }

    Ok(expression.to_string())
}

#[cfg(test)]
mod eval_normalize_tests {
    use super::normalize_eval_expression;

    #[test]
    fn leaves_invoked_iife_alone() {
        let e = "(() => { return 9; })()";
        assert_eq!(normalize_eval_expression(e, None).unwrap(), e);
        let e2 = "(async () => 1)()";
        assert_eq!(normalize_eval_expression(e2, None).unwrap(), e2);
        let e3 = "(function(){ return 2; })()";
        assert_eq!(normalize_eval_expression(e3, None).unwrap(), e3);
    }

    #[test]
    fn wraps_bare_arrow_once() {
        assert_eq!(
            normalize_eval_expression("() => 7", None).unwrap(),
            "(() => 7)()"
        );
        assert_eq!(
            normalize_eval_expression("async () => 3", None).unwrap(),
            "(async () => 3)()"
        );
    }

    #[test]
    fn leaves_plain_expression() {
        assert_eq!(normalize_eval_expression("1+1", None).unwrap(), "1+1");
        assert_eq!(normalize_eval_expression("(1+1)", None).unwrap(), "(1+1)");
        assert_eq!(
            normalize_eval_expression("document.title", None).unwrap(),
            "document.title"
        );
    }

    #[test]
    fn args_force_call() {
        let out = normalize_eval_expression("(el) => el", Some(r#"["@e1"]"#)).unwrap();
        assert_eq!(out, r#"((el) => el)("e1")"#);
    }
}
