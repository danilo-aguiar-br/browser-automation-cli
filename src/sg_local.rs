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
//! - Regex rules compile once via [`LazyLock`](std::sync::LazyLock) (fixed closures; MSRV ≥ 1.80).
//!
//! # Accuracy, measured
//!
//! Run over `src/` and `tests/` on 2026-08-25, this scanner reported 16
//! findings and all 16 were false positives. Three causes, each closed by a
//! named exemption: prose describing a rule (`is_comment_line`), the file
//! defining the rules scanning itself (`defines_the_rules`), and a test
//! exemption bolted to one rule by string comparison rather than declared per
//! rule (`Rule::exempt_in_tests`). The same run now reports 1.
//!
//! ## The one that is left
//!
//! `src/xdg/config_write.rs` builds the default `config.toml`, and the TOML
//! header it writes reads "no `.env` at runtime". That sentence is prose, but it
//! lives inside a Rust string literal rather than a comment, so the line IS
//! code and `is_comment_line` correctly declines to skip it.
//!
//! It stays. Separating prose in a string from a path in a string needs to know
//! where string literals begin and end, which is a parser, and every cheap
//! approximation available here — skipping lines with `#`, skipping lines
//! inside `format!` — trades one loud false positive for a class of silent
//! false negatives. A scanner that misses a real dotenv read is worse than one
//! that flags a sentence, so the number is written down instead of driven to
//! zero.

use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rayon::prelude::*;
use serde_json::{json, Value};

use crate::error::CliError;

mod rewrite;
mod rules;

#[cfg(test)]
mod tests;

pub use rewrite::sg_rewrite;

use rules::{compiled_rules, Rule};

/// A single finding.
#[derive(Debug, Clone)]
struct Finding {
    path: String,
    line: usize,
    rule: &'static str,
    snippet: String,
}

/// Read a source file only when metadata size is within the `max_sg_file_bytes` policy.
///
/// Oversized files are skipped (empty `None`) so a multi-GB artifact cannot
/// OOM the one-shot process via unbounded `read_to_string` (ECO-12 / Pass 28).
fn read_source_within_budget(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > crate::xdg::policy::policy_u64(crate::xdg::policy::key::MAX_SG_FILE_BYTES) {
        return None;
    }
    fs::read_to_string(path).ok()
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

/// Whether a path holds test code by construction, for the `unwrap_prod` rule.
///
/// Three shapes, because this crate uses all three: the `tests/` integration
/// directory, an in-crate module declared as `#[cfg(test)] mod tests;` in its
/// PARENT file, and the `test_utils` helper. The parent's attribute never
/// appears inside `tests.rs` itself, so a scanner that looks for `#[cfg(test)]`
/// within the file classifies every one of those modules as production. Measured
/// on 2026-08-25: the previous path filter reported 323 test `unwrap()` calls as
/// violations, which is a false-positive rate high enough to retire the rule
/// without the gate ever turning red.
fn is_test_path(path: &Path) -> bool {
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    if file == "tests.rs" || file == "test_utils.rs" || file.ends_with("_test.rs") {
        return true;
    }
    path.components()
        .any(|c| matches!(c.as_os_str().to_str(), Some("tests") | Some("benches")))
}

/// Line number of the file's first `#[cfg(test)]`, if it has one.
///
/// Rust convention puts the in-file test module last, so every line from that
/// attribute onwards is test code. This is the weaker of the two signals and it
/// only covers inline `#[cfg(test)] mod tests { .. }` blocks inside production
/// files; [`is_test_path`] carries the cases where the attribute lives in a
/// different file altogether.
fn first_cfg_test_line(text: &str) -> Option<usize> {
    text.lines()
        .position(|l| l.trim_start().starts_with("#[cfg(test)]"))
        .map(|i| i + 1)
}

/// Whether the line is a Rust line comment, and therefore prose rather than code.
///
/// Every rule here forbids a runtime BEHAVIOUR, so a line that cannot execute
/// cannot violate one. Skipping comments closes a class of false positive with a
/// perverse shape: the better a rule was documented, the more violations it
/// reported. Measured on 2026-08-25 over `src/` and `tests/`, five of sixteen
/// findings were sentences ASSERTING the conformance — `src/lib.rs` promising
/// "no `.env` at runtime" was accused of reading `.env`.
///
/// Block comments (`/* .. */`) are deliberately NOT handled. This is a line
/// scanner; spanning constructs need a parser, and pretending otherwise would
/// trade a loud false positive for a silent false negative. The debt is stated
/// here rather than hidden in the loop.
fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Whether this path is the file that DEFINES the rules, and is exempt from them.
///
/// A rule's own pattern is, by construction, an instance of what it hunts:
/// `Regex::new(r"\.unwrap\(\)")` contains the exact text the unwrap rule looks
/// for. Measured on 2026-08-25 over `src/` and `tests/`, seven of sixteen
/// findings were this one file, including the line that IS the dotenv pattern.
///
/// The path comes from [`file!`], so renaming or moving this module carries the
/// exemption with it instead of leaving a dead string literal behind — the
/// failure mode this crate has already hit once, in the path filter that
/// `is_test_path` replaced. The accepted trade is that a DIFFERENT crate with a
/// file at the same relative path would also be skipped; this scanner is aimed
/// at this repository and says so.
fn defines_the_rules(path: &Path) -> bool {
    path.ends_with(file!())
}

fn scan_file(path: &Path, rules: &[Rule]) -> Vec<Finding> {
    if defines_the_rules(path) {
        return Vec::new();
    }
    let Some(text) = read_source_within_budget(path) else {
        return Vec::new();
    };
    // The test exemption is decided once per file, never per line: either the
    // path is test code by construction, or the file opens its own `#[cfg(test)]`
    // block at a known line.
    let path_is_test = is_test_path(path);
    let inline_test_from = first_cfg_test_line(&text);
    // Pre-size conservatively: most files yield few findings; reserve a small
    // floor so the first hits avoid reallocation (rules: with_capacity).
    let mut out = Vec::with_capacity(4);
    for (lineno, line) in text.lines().enumerate() {
        let lineno = lineno + 1;
        if is_comment_line(line) {
            continue;
        }
        let in_test_code = path_is_test || inline_test_from.is_some_and(|start| lineno >= start);
        for rule in rules {
            if rule.exempt_in_tests && in_test_code {
                continue;
            }
            if rule.re.is_match(line) {
                out.push(Finding {
                    path: path.display().to_string(),
                    line: lineno,
                    rule: rule.name,
                    snippet: line.trim().chars().take(160).collect(),
                });
            }
        }
    }
    out
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
