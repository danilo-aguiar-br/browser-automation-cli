//! One-shot filesystem path discovery (`find-paths`, fd-like UX; no Chrome).

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::RegexBuilder;
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// Options for `find-paths`.
#[derive(Debug, Clone)]
pub struct FindPathsOpts {
    /// Regex pattern on file name (empty = match all).
    pub pattern: String,
    /// Root paths to walk.
    pub roots: Vec<PathBuf>,
    /// Extension filter without dot (e.g. `rs`).
    pub extension: Option<String>,
    /// Include hidden entries.
    pub hidden: bool,
    /// Do not respect .gitignore/.ignore.
    pub no_ignore: bool,
    /// Max walk depth.
    pub max_depth: Option<usize>,
    /// Entry type: `f` file, `d` dir, empty = both.
    pub entry_type: Option<String>,
    /// Max results (0 = unlimited).
    pub limit: usize,
    /// Optional glob pattern (fd-like; GAP-A011 / §5AE). Matched against full path.
    pub glob: Option<String>,
}

impl Default for FindPathsOpts {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            roots: vec![PathBuf::from(".")],
            extension: None,
            hidden: false,
            no_ignore: false,
            max_depth: None,
            entry_type: None,
            limit: 10_000,
            glob: None,
        }
    }
}

/// Walk roots and return matching paths (one-shot).
pub fn find_paths(opts: &FindPathsOpts) -> Result<Value, CliError> {
    let roots = if opts.roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        opts.roots.clone()
    };
    let re = if opts.pattern.is_empty() {
        None
    } else {
        Some(
            RegexBuilder::new(&opts.pattern)
                .case_insensitive(true)
                .build()
                .map_err(|e| {
                    CliError::new(ErrorKind::Usage, format!("invalid find-paths pattern: {e}"))
                })?,
        )
    };
    let mut paths = Vec::new();
    for root in &roots {
        let mut builder = WalkBuilder::new(root);
        builder.hidden(!opts.hidden);
        builder.git_ignore(!opts.no_ignore);
        builder.git_global(!opts.no_ignore);
        builder.git_exclude(!opts.no_ignore);
        builder.ignore(!opts.no_ignore);
        if let Some(d) = opts.max_depth {
            builder.max_depth(Some(d));
        }
        builder.threads(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2),
        );
        for entry in builder.build() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let ft = entry.file_type();
            let is_file = ft.map(|t| t.is_file()).unwrap_or(false);
            let is_dir = ft.map(|t| t.is_dir()).unwrap_or(false);
            if let Some(ref t) = opts.entry_type {
                match t.as_str() {
                    "f" | "file" if !is_file => continue,
                    "d" | "dir" | "directory" if !is_dir => continue,
                    _ => {}
                }
            }
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if let Some(ref ext) = opts.extension {
                let e = path.extension().and_then(|x| x.to_str()).unwrap_or("");
                if !e.eq_ignore_ascii_case(ext.trim_start_matches('.')) {
                    continue;
                }
            }
            if let Some(ref re) = re {
                if !re.is_match(name) && !re.is_match(&path.to_string_lossy()) {
                    continue;
                }
            }
            if let Some(ref g) = opts.glob {
                if !glob_match(g, path) {
                    continue;
                }
            }
            paths.push(path.display().to_string());
            if opts.limit > 0 && paths.len() >= opts.limit {
                break;
            }
        }
        if opts.limit > 0 && paths.len() >= opts.limit {
            break;
        }
    }
    Ok(json!({
        "count": paths.len(),
        "paths": paths,
        "pattern": opts.pattern,
        "glob": opts.glob,
        "engine": "ignore",
        "chrome": false,
    }))
}

/// Match a path against a shell-style glob (GAP-A011). Uses `globset` when available via
/// a lightweight converter for `*`, `?`, and `**`.
fn glob_match(pattern: &str, path: &Path) -> bool {
    let path_s = path.to_string_lossy();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // Prefer full-path match; also try basename for patterns without `/`.
    if glob_match_str(pattern, &path_s) {
        return true;
    }
    if !pattern.contains('/') {
        return glob_match_str(pattern, name);
    }
    false
}

fn glob_match_str(pattern: &str, text: &str) -> bool {
    // Convert simple glob → regex (case-insensitive).
    let mut re = String::from("(?i)^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                re.push_str(".*");
                i += 2;
                if i < chars.len() && chars[i] == '/' {
                    i += 1;
                }
            }
            '*' => {
                re.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                re.push_str("[^/]");
                i += 1;
            }
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                re.push('\\');
                re.push(chars[i]);
                i += 1;
            }
            c => {
                re.push(c);
                i += 1;
            }
        }
    }
    re.push('$');
    RegexBuilder::new(&re)
        .case_insensitive(true)
        .build()
        .map(|r| r.is_match(text))
        .unwrap_or(false)
}

/// Normalize roots from CLI paths.
pub fn roots_from(paths: &[String]) -> Vec<PathBuf> {
    if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.iter().map(|p| Path::new(p).to_path_buf()).collect()
    }
}
