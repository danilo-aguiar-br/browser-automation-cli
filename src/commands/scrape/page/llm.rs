//! When the LLM extract branch is reachable, and what it does once it is.
//!
//! # Why this lives apart from the handler
//!
//! `page.rs` sat at exactly 300 production lines, the ceiling
//! `scripts/filesize-check.sh` enforces, so the next production line there would
//! have turned the gate red. The seam cut here is the one between the REQUEST
//! handler and the LLM extract POLICY: everything in this file answers "may this
//! argv use `--schema-json` / `--question`, and what happens when it does";
//! the parent answers "fetch the page, build the formats, emit the envelope".
//!
//! The tests travel with the predicate rather than staying behind in the parent.
//! A predicate whose tests live in another module is a predicate whose next
//! editor does not know it is covered.

use crate::error::CliError;

/// True when this argv actually reaches the LLM extract branch.
///
/// [`maybe_llm_json`] has exactly one call site, nested three conditions deep:
/// the `http` engine, a single `--format`, and that format being `json`. Any
/// argv outside that intersection carries `--schema-json` and `--question`
/// straight past the only code that reads them.
///
/// This predicate is the single source of truth for that intersection. The argv
/// guard and the branch both call it, so a caller can never be refused for a
/// combination that would have worked, nor accepted for one that silently
/// drops the flag.
pub(super) fn llm_extract_is_reachable(engine_l: &str, formats: &[&str]) -> bool {
    engine_l == "http"
        && formats.len() == 1
        && matches!(
            crate::scrape_local::ScrapeFormat::parse(formats[0]),
            Ok(crate::scrape_local::ScrapeFormat::Json)
        )
}

pub(super) fn maybe_llm_json(
    mut data: serde_json::Value,
    schema_json: Option<&std::path::Path>,
    question: Option<&str>,
) -> Result<serde_json::Value, CliError> {
    if schema_json.is_none() && question.map(|q| q.trim().is_empty()).unwrap_or(true) {
        return Ok(data);
    }
    let key = crate::xdg::openrouter_api_key();
    if key.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err(CliError::with_suggestion(
            crate::error::ErrorKind::Usage,
            "format json requires XDG openrouter_api_key via config set",
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    // Source text: prefer markdown/text from payload or nested json placeholder.
    let source_text = data
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("markdown").and_then(|v| v.as_str()))
        .or_else(|| {
            data.get("json")
                .and_then(|j| j.get("text").or_else(|| j.get("markdown")))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("")
        .to_string();
    if source_text.trim().is_empty() {
        return Err(CliError::new(
            crate::error::ErrorKind::Data,
            "format json: empty page text for LLM extract",
        ));
    }
    // Same ceiling as the other `--schema-json` reader; see
    // `commands::nav::capture::extract_llm`.
    //
    // GAP-026, read axis. The comment above claimed parity with that twin, and
    // the parity was FALSE: the twin guards the path against the allowed roots
    // and this one did not, so the same flag was bounded on one command and
    // unbounded on the other. MEASURED 2026-08-31: with the LLM knobs set,
    // `scrape --schema-json /dev/shm/<file>` read a 13 MiB file from outside the
    // roots and only failed later, at the LLM call, while `parse` refused that
    // same path with exit 64.
    //
    // Guarded at the read site rather than inside `read_text_file_limited`:
    // that shared reader also serves the product reading its own config, which
    // sits outside the roots by design. The guard belongs where an
    // operator-supplied path stops being trustworthy.
    let schema_body = match schema_json {
        Some(p) => Some(crate::json_util::read_text_file_limited(
            crate::fs_roots::ensure_read_allowed(p)?.as_path(),
            crate::xdg::resolve_max_json_file_bytes(),
        )?),
        None => None,
    };
    let llm = crate::llm_local::extract_with_llm(&source_text, question, schema_body.as_deref())?;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("json".into(), llm);
        obj.insert("llm_extracted".into(), serde_json::json!(true));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::llm_extract_is_reachable;

    /// The one argv shape that actually reaches the LLM extract branch.
    ///
    /// Asserting only the refusals would pass against a predicate that returned
    /// `false` unconditionally, which would refuse every caller including the
    /// ones the feature exists for. This direction is what makes the other one
    /// mean something.
    #[test]
    fn the_single_json_format_on_the_http_engine_reaches_the_branch() {
        assert!(llm_extract_is_reachable("http", &["json"]));
    }

    /// The three shapes measured on 2026-08-26 that dropped the flag silently.
    ///
    /// Each exited 0 with `ok: true` and no envelope field derived from the
    /// schema. They are asserted together because they are one defect with
    /// three faces, not three defects.
    #[test]
    fn every_argv_that_cannot_reach_the_branch_is_reported_as_unreachable() {
        // Wrong format: the branch tests `fmt`, so `text` never enters it.
        assert!(!llm_extract_is_reachable("http", &["text"]));
        // Multiple formats: the branch is guarded by `formats.len() == 1`, so
        // asking for json AND text drops the schema even though json was asked
        // for. This is the shape a caller is least likely to suspect.
        assert!(!llm_extract_is_reachable("http", &["json", "text"]));
        assert!(!llm_extract_is_reachable("http", &["text", "json"]));
        // Wrong engine: `maybe_llm_json` is only called inside the `http` arm.
        assert!(!llm_extract_is_reachable("browser", &["json"]));
    }

    /// An unparseable format is unreachable rather than a panic or a match.
    ///
    /// The predicate runs BEFORE the format is validated for real, so it has to
    /// answer for garbage without deciding that garbage is json.
    #[test]
    fn an_unparseable_format_is_not_mistaken_for_json() {
        assert!(!llm_extract_is_reachable("http", &["jsonn"]));
        assert!(!llm_extract_is_reachable("http", &[""]));
    }

    /// An empty format list cannot index `formats[0]`.
    ///
    /// The caller normalizes an empty `--format` to `["text"]` before this runs,
    /// but the predicate must not depend on that to avoid panicking.
    #[test]
    fn an_empty_format_list_does_not_panic() {
        assert!(!llm_extract_is_reachable("http", &[]));
    }
}
