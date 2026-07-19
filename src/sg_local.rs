// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot structural lint / rewrite for product-forbidden patterns (§5AC / GAP-A011).
//!
//! Scans Rust sources under given roots for patterns that violate agent-first / one-shot
//! product rules (remote telemetry strings, product secret env reads, naked `unwrap()` in
//! non-test production modules). Default rewrite is dry-run; `--apply` writes in place.
//!
//! # Workload
//!
//! **Mista (I/O + CPU):**
//! - Walk is disk I/O (`ignore::WalkBuilder` with parallel threads).
//! - Per-file line scan is **CPU-bound** → Rayon `par_iter` over collected paths.
//! - Rewrite path stays sequential when `--apply` (deterministic atomic writes; no
//!   concurrent writers on the same tree).
//! - Regex rules compile once via [`OnceLock`].

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ignore::WalkBuilder;
use rayon::prelude::*;
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

/// Scan roots for forbidden structural patterns (one-shot, parallel CPU).
pub fn sg_scan(roots: &[PathBuf], limit: usize) -> Result<Value, CliError> {
    crate::concurrency::install_rayon_pool_once();
    let roots = if roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        roots.to_vec()
    };
    let rules = compiled_rules();

    // Stage 1: collect candidate paths (parallel walk threads via ignore).
    // Multi-root: independent walks under Rayon (same pattern as find_paths).
    let walk_threads = crate::concurrency::walk_threads();
    let collect_root = |root: &PathBuf| -> Vec<PathBuf> {
        let mut local = Vec::new();
        let mut builder = WalkBuilder::new(root);
        builder.hidden(false);
        builder.git_ignore(true);
        builder.threads(walk_threads);
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
            let s = path.to_string_lossy();
            if s.contains("/target/") || s.contains("\\target\\") {
                continue;
            }
            local.push(path.to_path_buf());
        }
        local
    };
    let paths: Vec<PathBuf> = if roots.len() <= 1 {
        roots.iter().flat_map(collect_root).collect()
    } else {
        roots.par_iter().flat_map(collect_root).collect()
    };

    // Stage 2: CPU-bound line scan in parallel (Rayon).
    let mut findings: Vec<Finding> = paths
        .par_iter()
        .flat_map_iter(|path| scan_file(path, rules))
        .collect();

    // Deterministic order for agents (path, line, rule). PAR-94/104: sort_cpu.
    crate::concurrency::sort_by_cpu(&mut findings, |a, b| {
        (&a.path, a.line, a.rule).cmp(&(&b.path, b.line, b.rule))
    });
    if limit > 0 && findings.len() > limit {
        findings.truncate(limit);
    }

    Ok(findings_to_json(&findings, false))
}

fn scan_file(path: &Path, rules: &[(&'static str, Regex)]) -> Vec<Finding> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let s = path.to_string_lossy();
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let lineno = lineno + 1;
        for (rule, re) in rules {
            if re.is_match(line) {
                if *rule == "unwrap_prod"
                    && (s.contains("/tests/")
                        || s.contains("\\tests\\")
                        || path.ends_with("test_utils.rs"))
                {
                    continue;
                }
                out.push(Finding {
                    path: path.display().to_string(),
                    line: lineno,
                    rule,
                    snippet: line.trim().chars().take(160).collect(),
                });
            }
        }
    }
    out
}

/// Dry-run or apply safe rewrites (GAP-A011). Only applies trivial safe fixes.
///
/// # Parallelism
///
/// - **Collect paths:** multi-threaded `ignore` walk.
/// - **Dry-run (`apply=false`):** Rayon `par_iter` over paths (CPU + disk read).
/// - **Apply (`apply=true`):** **sequential** by design — concurrent writers would
///   race on the same tree; atomic rename is per-file but ordering stays deterministic.
pub fn sg_rewrite(roots: &[PathBuf], apply: bool) -> Result<Value, CliError> {
    crate::concurrency::install_rayon_pool_once();
    let roots = if roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        roots.to_vec()
    };
    // Safe rewrite: strip `RUST_LOG` product-env comments that reintroduce env secrets guidance.
    // Real rewrites of unwrap are intentionally NOT automatic (would be blind rewrite — forbidden).
    let re_env_hint = re_rust_log_export();
    let walk_threads = crate::concurrency::walk_threads();
    let collect_root = |root: &PathBuf| -> Vec<PathBuf> {
        let mut local = Vec::new();
        let mut builder = WalkBuilder::new(root);
        builder.git_ignore(true);
        builder.threads(walk_threads);
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
            local.push(path.to_path_buf());
        }
        local
    };
    let mut paths: Vec<PathBuf> = if roots.len() <= 1 {
        roots.iter().flat_map(collect_root).collect()
    } else {
        roots.par_iter().flat_map(collect_root).collect()
    };

    let replacement = "# (removed product RUST_LOG export — use XDG log_level) ";
    let mut changed = Vec::new();
    let planned;

    if apply {
        // Sequential apply: deterministic order + no concurrent writers (N-136).
        // PAR-94: large path lists use par_sort before sequential apply.
        crate::concurrency::sort_cpu(&mut paths);
        let mut n = 0usize;
        for path in &paths {
            let Ok(text) = fs::read_to_string(path) else {
                continue;
            };
            if !re_env_hint.is_match(&text) {
                continue;
            }
            n += 1;
            let new_text = re_env_hint.replace_all(&text, replacement);
            atomic_write(path, new_text.as_ref())?;
            changed.push(path.display().to_string());
        }
        planned = n;
    } else {
        // Dry-run: parallel CPU match over collected paths.
        let mut hits: Vec<String> = paths
            .par_iter()
            .filter_map(|path| {
                let text = fs::read_to_string(path).ok()?;
                if re_env_hint.is_match(&text) {
                    Some(path.display().to_string())
                } else {
                    None
                }
            })
            .collect();
        crate::concurrency::sort_cpu(&mut hits);
        planned = hits.len();
        changed = hits;
    }

    Ok(json!({
        "ok": true,
        "apply": apply,
        "planned": planned,
        "changed": changed,
        "note": "Blind AST rewrite is forbidden; only safe RUST_LOG export hints are rewritten",
        "chrome": false,
        "parallel_walk": true,
        "dry_run_rayon": !apply,
        "apply_sequential": apply,
        "concurrency": crate::concurrency::effective_limit(),
    }))
}

fn re_rust_log_export() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)export\s+RUST_LOG\s*="#).expect("RUST_LOG export regex")
    })
}

fn compiled_rules() -> &'static [(&'static str, Regex)] {
    static RULES: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            vec![
                (
                    "telemetry_string",
                    Regex::new(
                        r"(?i)\b(opentelemetry|sentry\.io|telemetry\.|posthog|datadog)\b",
                    )
                    .expect("telemetry regex"),
                ),
                (
                    "product_env_secret",
                    Regex::new(
                        r#"std::env::var\(\s*"(API_KEY|OPENAI_API_KEY|SECRET|TOKEN|PASSWORD)""#,
                    )
                    .expect("env secret regex"),
                ),
                (
                    "unwrap_prod",
                    Regex::new(r"\.unwrap\(\)").expect("unwrap regex"),
                ),
                (
                    "dotenv",
                    Regex::new(r"(?i)\bdotenv\b|\.env\b").expect("dotenv regex"),
                ),
            ]
        })
        .as_slice()
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
        "engine": "sg-local",
        "parallel": true,
        "concurrency": crate::concurrency::effective_limit(),
        "chrome": false,
    })
}

fn atomic_write(path: &Path, body: &str) -> Result<(), CliError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(
        ".browser-automation-cli-sg-{}.tmp",
        std::process::id()
    ));
    fs::write(&tmp, body).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("write temp {}: {e}", tmp.display()),
        )
    })?;
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
        writeln!(
            file,
            "fn x() {{ let _ = \"a\".parse::<u32>().unwrap(); }}"
        )
        .unwrap();
        let v = sg_scan(&[dir.path().to_path_buf()], 50).unwrap();
        assert!(v.get("count").and_then(|c| c.as_u64()).unwrap_or(0) >= 1);
    }
}
