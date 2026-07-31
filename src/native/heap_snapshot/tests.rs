// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::Write;
use std::path::Path;

use super::*;

/// Tiny graph:
/// root(0) -prop-> A(1) -prop-> B(2)
/// root also retains C(3)
/// B retained only via A.
fn write_fixture(path: &Path) {
    // node_fields: type, name, id, self_size, edge_count
    // nodes: root, A, B, C
    // edges for root: 2 (to A, to C); A: 1 (to B); B: 0; C: 0
    // to_node is flat index = node_index * 5
    let body = r#"{
        "snapshot": {
            "meta": {
                "node_fields": ["type","name","id","self_size","edge_count"],
                "node_types": [["hidden","object","string","synthetic"]],
                "edge_fields": ["type","name_or_index","to_node"],
                "edge_types": [["context","element","property","internal","hidden","shortcut","weak"]]
            },
            "node_count": 4,
            "edge_count": 3
        },
        "nodes": [
            3, 0, 10, 0, 2,
            1, 1, 11, 100, 1,
            1, 2, 12, 50, 0,
            1, 3, 13, 25, 0
        ],
        "edges": [
            2, 4, 5,
            2, 5, 15,
            2, 6, 10
        ],
        "strings": ["(GC roots)", "A", "B", "C", "toA", "toC", "toB"]
    }"#;
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

#[test]
fn summarize_minimal_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.heapsnapshot");
    write_fixture(&path);
    let s = summarize(&path).unwrap();
    assert_eq!(s["node_count"], 4);
    assert_eq!(s["offline"], true);
}

#[test]
fn edges_and_retainers_real_graph() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("g.heapsnapshot");
    write_fixture(&path);

    // node id 12 = B
    let edges_b = node_op(&path, 12, "edges").unwrap();
    assert_eq!(edges_b["edge_count"], 0);

    let retainers_b = node_op(&path, 12, "retainers").unwrap();
    assert_eq!(retainers_b["retainer_count"], 1);
    let r0 = &retainers_b["retainers"][0];
    assert_eq!(r0["from_id"], 11); // A

    let edges_a = node_op(&path, 11, "edges").unwrap();
    assert_eq!(edges_a["edge_count"], 1);
    assert_eq!(edges_a["edges"][0]["to_id"], 12);
}

#[test]
fn dominators_chain_includes_root_and_node() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.heapsnapshot");
    write_fixture(&path);
    let d = node_op(&path, 12, "dominators").unwrap();
    let chain = d["dominator_chain"].as_array().unwrap();
    assert!(chain.len() >= 2);
    let last = chain.last().unwrap();
    assert_eq!(last["id"], 12);
    let first = &chain[0];
    assert_eq!(first["id"], 10);
}

#[test]
fn retaining_paths_finds_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.heapsnapshot");
    write_fixture(&path);
    let p = node_op(&path, 12, "paths").unwrap();
    let paths = p["paths"].as_array().unwrap();
    assert!(!paths.is_empty());
    assert!(paths[0]["nodes"].as_array().unwrap().len() >= 2);
}

#[test]
fn class_nodes_lists_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.heapsnapshot");
    write_fixture(&path);
    // rank classes; A/B/C each count 1 — any rank 1+ works if class exists
    let cn = class_nodes(&path, 1).unwrap();
    assert!(!cn["nodes"].as_array().unwrap().is_empty());
    assert_eq!(cn["offline"], true);
}

#[test]
fn close_snapshot_flags_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.heapsnapshot");
    write_fixture(&path);
    let c = close_snapshot(&path).unwrap();
    assert_eq!(c["closed"], true);
}

#[test]
fn dup_strings_counts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dups.heapsnapshot");
    let body = r#"{
        "snapshot": { "meta": {
            "node_fields": ["type","name","id","self_size","edge_count"],
            "node_types": [["object"]],
            "edge_fields": ["type","name_or_index","to_node"],
            "edge_types": [["property"]]
        }, "node_count": 0, "edge_count": 0 },
        "nodes": [],
        "edges": [],
        "strings": ["a", "b", "a", "a", "c", "b"]
    }"#;
    std::fs::write(&path, body).unwrap();
    let d = duplicate_strings(&path).unwrap();
    assert_eq!(d["duplicate_groups"], 2);
}

#[test]
fn object_details_includes_distance_and_retained() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("obj.heapsnapshot");
    write_fixture(&path);
    // B id=12: self 50, retained should include only self if nothing dominated
    let o = object_details(&path, 12).unwrap();
    assert_eq!(o["op"], "object-details");
    assert_eq!(o["offline"], true);
    let obj = &o["object"];
    assert_eq!(obj["id"], 12);
    assert_eq!(obj["name"], "B");
    assert_eq!(obj["self_size"], 50);
    assert!(obj["retained_size"].as_u64().unwrap() >= 50);
    assert_eq!(obj["distance"], 2); // root -> A -> B
    assert_eq!(obj["retainer_count"], 1);
    assert_eq!(obj["detachedness"], "unknown");

    // A id=11 retains B (50) + self 100
    let a = object_details(&path, 11).unwrap();
    let ao = &a["object"];
    assert_eq!(ao["distance"], 1);
    assert!(ao["retained_size"].as_u64().unwrap() >= 150);
}
