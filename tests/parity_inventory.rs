//! Inventory gate: official tool-reference tools must appear in the parity matrix
//! and map to a CLI surface (with documented aliases).
//!
//! Does not require Chrome. Fails closed if a tool is omitted from the matrix.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn official_tools() -> Vec<String> {
    let path = workspace_root().join("tests/fixtures/tool-reference.md");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read tool-reference at {}: {e}", path.display());
    });
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("### `") {
            if let Some(name) = rest.strip_suffix('`') {
                out.push(name.to_string());
            }
        }
    }
    out
}

fn matrix_tool_refs() -> BTreeSet<String> {
    let path = workspace_root().join("docs_prd/parity_devtools_matrix.md");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read matrix at {}: {e}", path.display());
    });
    let mut set = BTreeSet::new();
    for line in text.lines() {
        if !line.starts_with('|') {
            continue;
        }
        let cols: Vec<&str> = line.split('|').map(str::trim).collect();
        // | tool | cli | status | notes |  => cols[1]=tool, cols[2]=cli, cols[3]=status
        if cols.len() < 4 {
            continue;
        }
        let tool = cols[1];
        let status = cols[3];
        if tool.is_empty()
            || tool == "Tool"
            || tool.starts_with("---")
            || tool == "Tool-ref"
            || tool == "Capability"
        {
            continue;
        }
        if matches!(status, "Closed" | "Partial" | "Open") {
            set.insert(tool.to_string());
        }
    }
    set
}

fn covered_in_matrix(official: &str, matrix: &BTreeSet<String>) -> bool {
    if matrix.contains(official) {
        return true;
    }
    // navigate_page expands to four CLI navigation cmds in the matrix.
    if official == "navigate_page" {
        return [
            "navigate url",
            "navigate back",
            "navigate forward",
            "navigate reload",
        ]
        .iter()
        .any(|a| matrix.contains(*a));
    }
    false
}

#[test]
fn official_tools_count_is_52() {
    let tools = official_tools();
    assert_eq!(
        tools.len(),
        52,
        "expected 52 official tools in tool-reference, got {}",
        tools.len()
    );
}

#[test]
fn every_official_tool_has_matrix_row() {
    let official = official_tools();
    let matrix = matrix_tool_refs();
    let mut missing = Vec::new();
    for t in &official {
        if !covered_in_matrix(t, &matrix) {
            missing.push(t.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "official tools missing from parity matrix: {missing:?}"
    );
}

#[test]
fn object_details_is_closed_in_matrix() {
    let path = workspace_root().join("docs_prd/parity_devtools_matrix.md");
    let text = fs::read_to_string(path).unwrap();
    assert!(
        text.contains("get_heapsnapshot_object_details"),
        "matrix must list get_heapsnapshot_object_details"
    );
    assert!(
        text.contains("heap object-details"),
        "matrix must map to heap object-details CLI"
    );
    // Status Closed on that row
    let mut found_closed = false;
    for line in text.lines() {
        if line.contains("get_heapsnapshot_object_details") && line.contains("Closed") {
            found_closed = true;
            break;
        }
    }
    assert!(found_closed, "object_details row must be Closed");
}

#[test]
fn heap_object_details_registered_in_cli_help() {
    use assert_cmd::cargo::cargo_bin_cmd;
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(["heap", "--help"])
        .assert()
        .success();
    let help = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        help.contains("object-details"),
        "heap --help must list object-details; got:\n{help}"
    );
}
