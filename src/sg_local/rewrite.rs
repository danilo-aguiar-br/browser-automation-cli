// SPDX-License-Identifier: MIT OR Apache-2.0
//! The side that WRITES, and the one rewrite this scanner is allowed to make.
//!
//! # Why the writer is not next to the scanner
//!
//! [`super`] reads files and reports; nothing it does can damage a tree, so it
//! runs fully parallel and needs no permission check. This file mutates source
//! it found by walking, which changes three things at once: the apply pass must
//! be SEQUENTIAL, because concurrent writers on one tree race; it has to pass
//! [`crate::fs_roots::ensure_write_allowed`], because a symlink inside an
//! allowed root can point outside it; and every rewrite it performs has to be
//! provably safe without understanding the code.
//!
//! That last constraint is the real seam. Only ONE rewrite qualifies today —
//! stripping a `RUST_LOG` export hint — and every rule the scanner reports has
//! deliberately no automatic fix, because a blind AST rewrite of `unwrap()` is
//! forbidden by this product. Keeping the writer in its own file makes that
//! asymmetry visible instead of leaving it as an absence a reader has to notice.

use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rayon::prelude::*;
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::read_source_within_budget;
use super::rules::re_rust_log_export;

/// Dry-run or apply safe rewrites (GAP-A011). Only applies trivial safe fixes.
///
/// # Parallelism
///
/// - **Collect paths:** multi-threaded `ignore` walk.
/// - **Dry-run (`apply=false`):** Rayon `par_iter` over paths (CPU + disk read).
/// - **Apply (`apply=true`):** **sequential** by design — concurrent writers would
///   race on the same tree; atomic rename is per-file but ordering stays deterministic.
///
/// # Errors
///
/// Propagates `atomic_write` when `apply` is set: a path outside every
/// allowed root, or an I/O failure writing or renaming the temp file.
///
/// Cited without a link on purpose: `atomic_write` is crate-internal, and
/// rustdoc treats a link from public documentation to a private item as an
/// ERROR, not a warning. Naming the function still tells the reader where the
/// failure comes from, which is all this sentence ever needed to do.
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
            let Some(text) = read_source_within_budget(path) else {
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
                let text = read_source_within_budget(path)?;
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

fn atomic_write(path: &Path, body: &str) -> Result<(), CliError> {
    // GAP-026, write axis. This is `sg-rewrite --apply` rewriting a source file
    // it found by walking. The root it walked is bounded at the command
    // boundary by `commands::scrape::local_fs::guarded_roots`, so this second
    // check is not the primary one — it covers the case the root check cannot,
    // which is a symlink inside an allowed root pointing out of it.
    crate::fs_roots::ensure_write_allowed(path)?;
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
