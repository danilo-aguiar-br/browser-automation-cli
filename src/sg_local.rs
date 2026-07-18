//! One-shot structural lint / rewrite for product-forbidden patterns (§5AC / GAP-A011).
//!
//! Scans Rust sources under given roots for patterns that violate agent-first / one-shot
//! product rules (remote telemetry strings, product secret env reads, naked `unwrap()` in
//! non-test production modules). Default rewrite is dry-run; `--apply` writes in place.

use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::Regex;
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// A single finding.
#[derive(Debug, Clone)]
struct Finding {
    path: String,
    line: usize,
    rule: &'static str,
    snippet: String,
}

/// Scan roots for forbidden structural patterns (one-shot).
pub fn sg_scan(roots: &[PathBuf], limit: usize) -> Result<Value, CliError> {
    let roots = if roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        roots.to_vec()
    };
    let rules = compile_rules();
    let mut findings = Vec::new();
    for root in &roots {
        let mut builder = WalkBuilder::new(root);
        builder.hidden(false);
        builder.git_ignore(true);
        for entry in builder.build() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_none_or(|e| e != "rs")
            {
                continue;
            }
            // Skip tests and generated/target.
            let s = path.to_string_lossy();
            if s.contains("/target/") || s.contains("\\target\\") {
                continue;
            }
            let Ok(text) = fs::read_to_string(path) else {
                continue;
            };
            for (lineno, line) in text.lines().enumerate() {
                let lineno = lineno + 1;
                // Skip pure test modules lightly: lines under #[cfg(test)] blocks still scanned
                // but test files under tests/ are informational only for unwrap.
                for (rule, re) in &rules {
                    if re.is_match(line) {
                        if *rule == "unwrap_prod"
                            && (s.contains("/tests/")
                                || s.contains("\\tests\\")
                                || path.ends_with("test_utils.rs"))
                        {
                            continue;
                        }
                        findings.push(Finding {
                            path: path.display().to_string(),
                            line: lineno,
                            rule,
                            snippet: line.trim().chars().take(160).collect(),
                        });
                        if limit > 0 && findings.len() >= limit {
                            return Ok(findings_to_json(&findings, false));
                        }
                    }
                }
            }
        }
    }
    Ok(findings_to_json(&findings, false))
}

/// Dry-run or apply safe rewrites (GAP-A011). Only applies trivial safe fixes.
pub fn sg_rewrite(roots: &[PathBuf], apply: bool) -> Result<Value, CliError> {
    let roots = if roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        roots.to_vec()
    };
    // Safe rewrite: strip `RUST_LOG` product-env comments that reintroduce env secrets guidance.
    // Real rewrites of unwrap are intentionally NOT automatic (would be blind rewrite — forbidden).
    let re_env_hint = Regex::new(r#"(?i)export\s+RUST_LOG\s*="#)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("regex: {e}")))?;
    let mut changed = Vec::new();
    let mut planned = 0usize;
    for root in &roots {
        let mut builder = WalkBuilder::new(root);
        builder.git_ignore(true);
        for entry in builder.build() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_none_or(|e| e != "md" && e != "txt" && e != "rs")
            {
                continue;
            }
            let s = path.to_string_lossy();
            if s.contains("/target/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(path) else {
                continue;
            };
            if !re_env_hint.is_match(&text) {
                continue;
            }
            planned += 1;
            if apply {
                let new_text = re_env_hint.replace_all(
                    &text,
                    "# (removed product RUST_LOG export — use XDG log_level) ",
                );
                atomic_write(path, new_text.as_ref())?;
                changed.push(path.display().to_string());
            } else {
                changed.push(path.display().to_string());
            }
        }
    }
    Ok(json!({
        "ok": true,
        "apply": apply,
        "planned": planned,
        "changed": changed,
        "note": "Blind AST rewrite is forbidden; only safe RUST_LOG export hints are rewritten",
        "chrome": false,
    }))
}

fn compile_rules() -> Vec<(&'static str, Regex)> {
    vec![
        (
            "telemetry_string",
            Regex::new(r"(?i)\b(opentelemetry|sentry\.io|telemetry\.|posthog|datadog)\b").unwrap(),
        ),
        (
            "product_env_secret",
            Regex::new(r#"std::env::var\(\s*"(API_KEY|OPENAI_API_KEY|SECRET|TOKEN|PASSWORD)""#)
                .unwrap(),
        ),
        ("unwrap_prod", Regex::new(r"\.unwrap\(\)").unwrap()),
        ("dotenv", Regex::new(r"(?i)\bdotenv\b|\.env\b").unwrap()),
    ]
}

fn findings_to_json(findings: &[Finding], apply: bool) -> Value {
    let items: Vec<Value> = findings
        .iter()
        .map(|f| {
            json!({
                "path": f.path,
                "line": f.line,
                "rule": f.rule,
                "snippet": f.snippet,
            })
        })
        .collect();
    json!({
        "ok": true,
        "count": items.len(),
        "findings": items,
        "apply": apply,
        "chrome": false,
    })
}

fn atomic_write(path: &Path, body: &str) -> Result<(), CliError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(
        ".browser-automation-cli-sg-{}.tmp",
        std::process::id()
    ));
    fs::write(&tmp, body)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("write temp {}: {e}", tmp.display())))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        CliError::new(
            ErrorKind::Io,
            format!("rename {} → {}: {e}", tmp.display(), path.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scan_finds_unwrap_in_temp_rs() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("prod.rs");
        let mut file = fs::File::create(&f).unwrap();
        writeln!(file, "fn x() {{ let _ = \"a\".parse::<u32>().unwrap(); }}").unwrap();
        let v = sg_scan(&[dir.path().to_path_buf()], 50).unwrap();
        assert!(v.get("count").and_then(|c| c.as_u64()).unwrap_or(0) >= 1);
    }
}
