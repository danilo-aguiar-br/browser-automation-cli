// SPDX-License-Identifier: MIT OR Apache-2.0
//! Workflow manifest types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One step in a one-shot offline workflow DAG (cmd + args + deps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Stable step id.
    pub id: String,
    /// CLI command name (e.g. goto, scrape, run).
    pub cmd: String,
    /// Optional argv/object for the step.
    #[serde(default)]
    pub args: Value,
    /// Dependencies (step ids that must complete first).
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// Workflow manifest file shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManifest {
    /// Optional name.
    #[serde(default)]
    pub name: Option<String>,
    /// Correlation id for envelopes.
    #[serde(default)]
    pub correlation_id: Option<String>,
    /// Steps forming a DAG.
    pub steps: Vec<WorkflowStep>,
}
