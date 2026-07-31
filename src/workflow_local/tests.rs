// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;
use serde_json::json;

#[test]
fn dag_topo() {
    let steps = vec![
        WorkflowStep {
            id: "a".into(),
            cmd: "noop".into(),
            args: json!({}),
            depends_on: vec![],
        },
        WorkflowStep {
            id: "b".into(),
            cmd: "noop".into(),
            args: json!({}),
            depends_on: vec!["a".into()],
        },
    ];
    let order = validate_dag(&steps).unwrap();
    assert_eq!(order, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn dag_cycle_detected() {
    let steps = vec![
        WorkflowStep {
            id: "a".into(),
            cmd: "noop".into(),
            args: json!({}),
            depends_on: vec!["b".into()],
        },
        WorkflowStep {
            id: "b".into(),
            cmd: "noop".into(),
            args: json!({}),
            depends_on: vec!["a".into()],
        },
    ];
    assert!(validate_dag(&steps).is_err());
}
