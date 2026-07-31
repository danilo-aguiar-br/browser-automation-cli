// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manifest load and DAG validation.

use std::collections::BTreeMap;
use std::path::Path;

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::DiGraph;

use super::types::{WorkflowManifest, WorkflowStep};
use crate::error::{CliError, ErrorKind};

/// Load manifest from JSON path (BOM-aware, size-limited, typed).
pub fn load_manifest(path: &Path) -> Result<WorkflowManifest, CliError> {
    crate::json_util::read_json_file(path, crate::xdg::resolve_max_json_file_bytes()).map_err(|e| {
        if e.kind() == ErrorKind::Data && !e.message().contains("invalid workflow") {
            CliError::new(
                ErrorKind::Data,
                format!("invalid workflow manifest: {}", e.message()),
            )
        } else {
            e
        }
    })
}

/// Validate DAG with petgraph; return topological order of step ids.
pub fn validate_dag(steps: &[WorkflowStep]) -> Result<Vec<String>, CliError> {
    let mut g: DiGraph<String, ()> = DiGraph::new();
    let mut idx: BTreeMap<String, petgraph::graph::NodeIndex> = BTreeMap::new();
    for s in steps {
        if idx.contains_key(&s.id) {
            return Err(CliError::new(
                ErrorKind::Data,
                format!("duplicate workflow step id: {}", s.id),
            ));
        }
        // One clone for the graph node; insert reuses the same owned key via entry.
        let id = s.id.clone();
        let n = g.add_node(id.clone());
        idx.insert(id, n);
    }
    for s in steps {
        let to = idx[&s.id];
        for dep in &s.depends_on {
            let from = idx.get(dep).ok_or_else(|| {
                CliError::new(
                    ErrorKind::Data,
                    format!("step {} depends on unknown id {dep}", s.id),
                )
            })?;
            g.add_edge(*from, to, ());
        }
    }
    if is_cyclic_directed(&g) {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "workflow DAG has a cycle",
            crate::i18n::suggestion_key("workflow_cycle", None),
        ));
    }
    let order = toposort(&g, None)
        .map_err(|_| CliError::new(ErrorKind::Data, "workflow toposort failed (cycle?)"))?;
    Ok(order.into_iter().map(|i| g[i].clone()).collect())
}
