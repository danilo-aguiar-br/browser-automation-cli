// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lighthouse JSON report to envelope payload.

use crate::error::{CliError, ErrorKind};
use serde_json::Value;
use std::path::Path;

use super::resolve::resolve_lighthouse_binary;

/// Extract category scores and audit pass/fail counts from a Lighthouse LHR JSON.
///
/// Pure parse path so unit tests can pin the contract without spawning the
/// external binary (GAP-021: mock e2e must not be the only evidence).
pub(crate) fn scores_from_lhr(lhr: &Value) -> (Vec<Value>, u64, u64) {
    let mut scores = Vec::new();
    let mut passed_audits = 0u64;
    let mut failed_audits = 0u64;
    if let Some(cats) = lhr.get("categories").and_then(|c| c.as_object()) {
        for (id, cat) in cats {
            scores.push(serde_json::json!({
                "id": id,
                "title": cat.get("title").and_then(|t| t.as_str()).unwrap_or(id),
                "score": cat.get("score"),
            }));
        }
    }
    if let Some(audits) = lhr.get("audits").and_then(|a| a.as_object()) {
        for a in audits.values() {
            if let Some(sc) = a.get("score").and_then(|s| s.as_f64()) {
                if sc < 1.0 {
                    failed_audits += 1;
                } else {
                    passed_audits += 1;
                }
            }
        }
    }
    (scores, passed_audits, failed_audits)
}

/// Run lighthouse binary and return envelope data (shared by CLI and `run` scripts).
pub(crate) fn lighthouse_to_value(
    url: &str,
    out_dir: Option<&Path>,
    device: &str,
    mode: &str,
    lighthouse_path: Option<&Path>,
) -> Result<serde_json::Value, CliError> {
    let (bin_path, binary_source) = resolve_lighthouse_binary(lighthouse_path)?;
    let bin = bin_path.display().to_string();
    let out = match out_dir {
        Some(p) => p.to_path_buf(),
        None => crate::xdg::cache_dir()
            .map_err(|e| {
                CliError::new(
                    ErrorKind::Io,
                    format!("lighthouse out-dir: XDG cache unavailable: {e}"),
                )
            })?
            .join("lighthouse"),
    };
    crate::fs_roots::ensure_write_allowed(&out)?;
    std::fs::create_dir_all(&out)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("lighthouse out-dir: {e}")))?;
    let form_factor = if device.eq_ignore_ascii_case("mobile") {
        "mobile"
    } else {
        "desktop"
    };
    let mode_norm = if mode.eq_ignore_ascii_case("snapshot") {
        "snapshot"
    } else if mode.eq_ignore_ascii_case("navigation") || mode.is_empty() {
        "navigation"
    } else {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unsupported lighthouse mode: {mode}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    };
    // Reject NUL in user-controlled argv (Unix truncation).
    if crate::platform::arg_contains_nul(url) || crate::platform::arg_contains_nul(out.as_os_str())
    {
        return Err(CliError::new(
            ErrorKind::Usage,
            "lighthouse url/out-dir must not contain NUL bytes",
        ));
    }
    // Map mode to real Lighthouse CLI args (GAP-006). Snapshot uses gather-mode.
    let html_path = out.join("report.html");
    let json_path = out.join("report.json");
    let report_base = out.join("report");
    let mut cmd = std::process::Command::new(&bin_path);
    // One argv per logical flag; `--flag=value` is a single token (not shell).
    cmd.arg(url)
        .arg("--quiet")
        .arg("--output=html")
        .arg("--output=json")
        .arg(format!("--output-path={}", report_base.display()))
        .arg(format!("--form-factor={form_factor}"))
        .arg(format!(
            "--chrome-flags={}",
            crate::constants::LIGHTHOUSE_CHROME_FLAGS
        ))
        .arg(format!(
            "--only-categories={}",
            crate::constants::LIGHTHOUSE_ONLY_CATEGORIES
        ));
    if mode_norm == "snapshot" {
        // Lighthouse user-flows / gather-mode snapshot (when supported by binary).
        cmd.arg("--gather-mode=snapshot");
    }
    let timeout = std::time::Duration::from_secs(crate::xdg::resolve_lighthouse_timeout_secs());
    let output =
        crate::platform::run_capture_with_timeout(&mut cmd, timeout).map_err(|e| match e {
            crate::platform::ProcessCaptureError::Timeout => CliError::with_suggestion(
                ErrorKind::Unavailable,
                format!("lighthouse timed out after {}s", timeout.as_secs()),
                crate::i18n::suggestion_key("lighthouse_timeout", None),
            ),
            crate::platform::ProcessCaptureError::Spawn(err) => CliError::with_suggestion(
                ErrorKind::Unavailable,
                format!("lighthouse spawn failed: {err}"),
                crate::i18n::suggestion_key("lighthouse_missing", None),
            ),
            crate::platform::ProcessCaptureError::Wait(err) => CliError::with_suggestion(
                ErrorKind::Software,
                format!("lighthouse wait failed: {err}"),
                crate::i18n::suggestion_key("lighthouse_missing", None),
            ),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::with_suggestion(
            ErrorKind::Software,
            format!("lighthouse exited non-zero: {stderr}"),
            "Check URL and lighthouse install",
        ));
    }
    // Lighthouse may write report.report.html / report.report.json depending on version.
    // Move paths when the preferred name exists; otherwise build alternate PathBufs.
    let report_html = if html_path.exists() {
        html_path
    } else if out.join("report.report.html").exists() {
        out.join("report.report.html")
    } else {
        html_path
    };
    let report_json = if json_path.exists() {
        json_path
    } else if out.join("report.report.json").exists() {
        out.join("report.report.json")
    } else {
        // Some builds write plain report.json next to html
        out.join("report.json")
    };

    let (scores, passed_audits, failed_audits) = if report_json.exists() {
        if let Ok(raw) = crate::json_util::read_text_file_limited(
            &report_json,
            crate::xdg::resolve_max_json_file_bytes(),
        ) {
            if let Ok(lhr) = crate::json_util::value_from_str(&raw) {
                scores_from_lhr(&lhr)
            } else {
                (Vec::new(), 0, 0)
            }
        } else {
            (Vec::new(), 0, 0)
        }
    } else {
        (Vec::new(), 0, 0)
    };

    Ok(serde_json::json!({
        "lighthouse": true,
        "url": url,
        "device": form_factor,
        "mode": mode_norm,
        "binary": bin,
        "binary_source": binary_source.as_str(),
        "binary_present": true,
        "out_dir": out.to_string_lossy(),
        "reports": {
            "html": report_html.to_string_lossy(),
            "json": report_json.to_string_lossy(),
        },
        "scores": scores,
        "passed_audits": passed_audits,
        "failed_audits": failed_audits,
    }))
}

#[cfg(test)]
mod tests {
    use super::scores_from_lhr;
    use serde_json::json;

    /// LHR-shaped fixture on disk (GAP-021): not a full Chrome capture, but the
    /// real field layout (`lighthouseVersion`, `categories`, `audits`) used by
    /// the product parser. Agent-native stdout still never dumps this body.
    const MINIMAL_LHR_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/scripts/fixtures/lighthouse/minimal_lhr.json"
    ));

    #[test]
    fn scores_from_file_lhr_fixture() {
        let lhr: serde_json::Value =
            serde_json::from_str(MINIMAL_LHR_JSON).expect("minimal_lhr.json parses");
        assert_eq!(lhr["lighthouseVersion"], "12.0.0");
        let (scores, passed, failed) = scores_from_lhr(&lhr);
        assert_eq!(scores.len(), 4);
        assert_eq!(passed, 3);
        assert_eq!(failed, 1);
        let perf = scores
            .iter()
            .find(|s| s["id"] == "performance")
            .expect("performance category");
        assert_eq!(perf["score"], 0.9);
        assert!(scores.iter().any(|s| s["id"] == "accessibility"));
        assert!(scores.iter().any(|s| s["id"] == "seo"));
    }

    #[test]
    fn scores_from_minimal_inline_edge_null_audit() {
        // Edge: null audit scores must not count as pass or fail.
        let lhr = json!({
            "categories": {
                "performance": { "title": "Performance", "score": 0.9 }
            },
            "audits": {
                "first-contentful-paint": { "score": 1.0 },
                "unscored": { "score": null }
            }
        });
        let (scores, passed, failed) = scores_from_lhr(&lhr);
        assert_eq!(scores.len(), 1);
        assert_eq!(passed, 1);
        assert_eq!(failed, 0);
    }

    /// Sanitized subset captured from real `npx lighthouse` + Chrome headless
    /// (GAP-021). Full artifacts stripped so the fixture stays agent-sized.
    const CHROME_CAPTURED_LHR_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/scripts/fixtures/lighthouse/chrome_captured_lhr.json"
    ));

    #[test]
    fn scores_from_chrome_captured_lhr_fixture() {
        let lhr: serde_json::Value =
            serde_json::from_str(CHROME_CAPTURED_LHR_JSON).expect("chrome_captured_lhr.json parses");
        assert!(
            lhr["lighthouseVersion"]
                .as_str()
                .is_some_and(|v| v.starts_with("13.")),
            "expected real Lighthouse 13.x capture, got {}",
            lhr["lighthouseVersion"]
        );
        let (scores, passed, failed) = scores_from_lhr(&lhr);
        assert!(
            scores.len() >= 3,
            "real capture must expose multiple categories: {scores:?}"
        );
        assert!(
            passed + failed > 0,
            "expected scored audits in sanitized capture (passed={passed} failed={failed})"
        );
        assert!(scores.iter().any(|s| s["id"] == "performance"));
        assert!(scores.iter().any(|s| s["id"] == "accessibility"));
    }
}
