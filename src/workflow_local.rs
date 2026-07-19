// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot workflow journal (PRD §5H): DAG + SQLite, no live Page/@eN across processes.
//!
//! # Workload / parallelism
//!
//! - **DAG validate:** CPU-light (petgraph); sequential is fine.
//! - **Step execution:** sequential topo order with fail-fast. Independent ready
//!   sets are **not** fan-out parallel here because the SQLite journal is a
//!   single-writer resource and offline steps may share process-local caches.
//!   Parallelism lives inside each step (batch scrape, sg scan, find-paths).
//! - **Justification (rules_rust_paralelismo):** coordinating multi-step journal
//!   writes under a Mutex would add complexity without measured gain for typical
//!   agent workflows (few steps, I/O inside steps already bounded).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::DiGraph;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Workflow step in a manifest (no live browser handles).
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

/// Open or create journal DB under XDG state.
pub fn journal_path(name: Option<&str>) -> Result<PathBuf, CliError> {
    let dir = xdg::workflow_dir()?;
    xdg::ensure_dir(&dir)?;
    let file = name.unwrap_or("default");
    let safe: String = file
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(dir.join(format!("{safe}.sqlite")))
}

fn open_db(path: &Path) -> Result<Connection, CliError> {
    let conn = Connection::open(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("open workflow journal {}: {e}", path.display()),
        )
    })?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS steps (
            step_id TEXT PRIMARY KEY,
            cmd TEXT NOT NULL,
            status TEXT NOT NULL,
            depends_on TEXT NOT NULL DEFAULT '[]',
            result_json TEXT,
            error TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            correlation_id TEXT,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT
        );
        "#,
    )
    .map_err(|e| CliError::new(ErrorKind::Software, format!("workflow schema: {e}")))?;
    Ok(conn)
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

/// Load manifest from JSON path (BOM-aware, size-limited, typed).
pub fn load_manifest(path: &Path) -> Result<WorkflowManifest, CliError> {
    crate::json_util::read_json_file(path, crate::json_util::MAX_JSON_FILE_BYTES).map_err(|e| {
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
            "Remove circular depends_on edges",
        ));
    }
    let order = toposort(&g, None)
        .map_err(|_| CliError::new(ErrorKind::Data, "workflow toposort failed (cycle?)"))?;
    Ok(order.into_iter().map(|i| g[i].clone()).collect())
}

/// Run workflow one-shot: validate DAG, execute steps that are CLI-data commands,
/// journal state. Browser multi-step with @eN still requires nested `run` scripts.
pub fn workflow_run(manifest_path: &Path, journal: Option<&Path>) -> Result<Value, CliError> {
    let manifest = load_manifest(manifest_path)?;
    let order = validate_dag(&manifest.steps)?;
    let jpath = match journal {
        Some(p) => p.to_path_buf(),
        None => journal_path(manifest.name.as_deref())?,
    };
    let conn = open_db(&jpath)?;
    let run_id = Uuid::new_v4().to_string();
    let correlation = manifest
        .correlation_id
        .clone()
        .unwrap_or_else(|| run_id.clone());
    let started = now_rfc3339();
    conn.execute(
        "INSERT INTO runs (run_id, correlation_id, status, started_at) VALUES (?1, ?2, 'running', ?3)",
        params![run_id, correlation, started],
    )
    .map_err(|e| CliError::new(ErrorKind::Software, format!("insert run: {e}")))?;

    let mut by_id: BTreeMap<String, WorkflowStep> = BTreeMap::new();
    for s in &manifest.steps {
        by_id.insert(s.id.clone(), s.clone());
        let deps = serde_json::to_string(&s.depends_on).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT OR REPLACE INTO steps (step_id, cmd, status, depends_on, updated_at) VALUES (?1, ?2, 'pending', ?3, ?4)",
            params![s.id, s.cmd, deps, now_rfc3339()],
        )
        .map_err(|e| CliError::new(ErrorKind::Software, format!("insert step: {e}")))?;
    }

    let mut results = Vec::new();
    let mut failed: Option<String> = None;
    for sid in &order {
        let step = &by_id[sid];
        // Fail-fast if dependency failed (tracked only in this run).
        if let Some(ref f) = failed {
            conn.execute(
                "UPDATE steps SET status='skipped', error=?2, updated_at=?3 WHERE step_id=?1",
                params![sid, format!("skipped after failure of {f}"), now_rfc3339()],
            )
            .ok();
            results.push(json!({
                "id": sid,
                "cmd": step.cmd,
                "ok": false,
                "skipped": true,
            }));
            continue;
        }

        match execute_offline_step(step) {
            Ok(data) => {
                let body = serde_json::to_string(&data).unwrap_or_else(|_| "{}".into());
                conn.execute(
                    "UPDATE steps SET status='ok', result_json=?2, error=NULL, updated_at=?3 WHERE step_id=?1",
                    params![sid, body, now_rfc3339()],
                )
                .map_err(|e| CliError::new(ErrorKind::Software, format!("update step: {e}")))?;
                results.push(json!({
                    "id": sid,
                    "cmd": step.cmd,
                    "ok": true,
                    "data": data,
                }));
            }
            Err(e) => {
                let msg = e.to_string();
                conn.execute(
                    "UPDATE steps SET status='error', error=?2, updated_at=?3 WHERE step_id=?1",
                    params![sid, msg, now_rfc3339()],
                )
                .ok();
                results.push(json!({
                    "id": sid,
                    "cmd": step.cmd,
                    "ok": false,
                    "error": msg,
                }));
                failed = Some(sid.clone());
            }
        }
    }

    let status = if failed.is_some() { "failed" } else { "ok" };
    conn.execute(
        "UPDATE runs SET status=?2, finished_at=?3 WHERE run_id=?1",
        params![run_id, status, now_rfc3339()],
    )
    .ok();

    Ok(json!({
        "run_id": run_id,
        "correlation_id": correlation,
        "status": status,
        "journal": jpath.display().to_string(),
        "order": order,
        "steps": results,
        "note": "offline/data steps executed in-process; browser @eN multi-step remains in `run --script`",
    }))
}

/// Resume: skip steps already `ok` in journal; re-execute pending/error only.
pub fn workflow_resume(manifest_path: &Path, journal: Option<&Path>) -> Result<Value, CliError> {
    let manifest = load_manifest(manifest_path)?;
    let order = validate_dag(&manifest.steps)?;
    let jpath = match journal {
        Some(p) => p.to_path_buf(),
        None => journal_path(manifest.name.as_deref())?,
    };
    if !jpath.exists() {
        return Err(CliError::with_suggestion(
            ErrorKind::NoInput,
            format!("journal not found: {}", jpath.display()),
            "Run `workflow run` first or pass --journal",
        ));
    }
    let conn = open_db(&jpath)?;
    let mut done: BTreeMap<String, String> = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT step_id, status FROM steps")
            .map_err(|e| CliError::new(ErrorKind::Software, format!("resume prepare: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| CliError::new(ErrorKind::Software, format!("resume query: {e}")))?;
        for r in rows {
            let (id, st) =
                r.map_err(|e| CliError::new(ErrorKind::Software, format!("row: {e}")))?;
            done.insert(id, st);
        }
    }
    let run_id = Uuid::new_v4().to_string();
    let correlation = manifest
        .correlation_id
        .clone()
        .unwrap_or_else(|| run_id.clone());
    conn.execute(
        "INSERT INTO runs (run_id, correlation_id, status, started_at) VALUES (?1, ?2, 'running', ?3)",
        params![run_id, correlation, now_rfc3339()],
    )
    .map_err(|e| CliError::new(ErrorKind::Software, format!("insert resume run: {e}")))?;

    let mut by_id: BTreeMap<String, WorkflowStep> = BTreeMap::new();
    for s in &manifest.steps {
        by_id.insert(s.id.clone(), s.clone());
    }

    let mut results = Vec::new();
    let mut failed: Option<String> = None;
    for sid in &order {
        let step = &by_id[sid];
        if done.get(sid).map(|s| s.as_str()) == Some("ok") {
            results.push(json!({
                "id": sid,
                "cmd": step.cmd,
                "ok": true,
                "skipped": true,
                "reason": "already_ok",
            }));
            continue;
        }
        if let Some(ref f) = failed {
            results.push(json!({
                "id": sid,
                "cmd": step.cmd,
                "ok": false,
                "skipped": true,
                "reason": format!("after_failure:{f}"),
            }));
            continue;
        }
        match execute_offline_step(step) {
            Ok(data) => {
                let body = serde_json::to_string(&data).unwrap_or_else(|_| "{}".into());
                conn.execute(
                    "UPDATE steps SET status='ok', result_json=?2, error=NULL, updated_at=?3 WHERE step_id=?1",
                    params![sid, body, now_rfc3339()],
                )
                .ok();
                results.push(json!({
                    "id": sid,
                    "cmd": step.cmd,
                    "ok": true,
                    "data": data,
                    "resumed": true,
                }));
            }
            Err(e) => {
                let msg = e.to_string();
                conn.execute(
                    "UPDATE steps SET status='error', error=?2, updated_at=?3 WHERE step_id=?1",
                    params![sid, msg, now_rfc3339()],
                )
                .ok();
                results.push(json!({
                    "id": sid,
                    "cmd": step.cmd,
                    "ok": false,
                    "error": msg,
                    "resumed": true,
                }));
                failed = Some(sid.clone());
            }
        }
    }
    let status = if failed.is_some() { "failed" } else { "ok" };
    conn.execute(
        "UPDATE runs SET status=?2, finished_at=?3 WHERE run_id=?1",
        params![run_id, status, now_rfc3339()],
    )
    .ok();
    Ok(json!({
        "run_id": run_id,
        "correlation_id": correlation,
        "status": status,
        "journal": jpath.display().to_string(),
        "order": order,
        "steps": results,
        "resume": true,
    }))
}

/// Status of journal steps.
pub fn workflow_status(journal: Option<&Path>, name: Option<&str>) -> Result<Value, CliError> {
    let jpath = match journal {
        Some(p) => p.to_path_buf(),
        None => journal_path(name)?,
    };
    if !jpath.exists() {
        return Ok(json!({
            "journal": jpath.display().to_string(),
            "exists": false,
            "steps": [],
        }));
    }
    let conn = open_db(&jpath)?;
    let mut stmt = conn
        .prepare("SELECT step_id, cmd, status, error, updated_at FROM steps ORDER BY step_id")
        .map_err(|e| CliError::new(ErrorKind::Software, format!("status prepare: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "step_id": row.get::<_, String>(0)?,
                "cmd": row.get::<_, String>(1)?,
                "status": row.get::<_, String>(2)?,
                "error": row.get::<_, Option<String>>(3)?,
                "updated_at": row.get::<_, String>(4)?,
            }))
        })
        .map_err(|e| CliError::new(ErrorKind::Software, format!("status query: {e}")))?;
    let mut steps = Vec::new();
    for r in rows {
        steps.push(r.map_err(|e| CliError::new(ErrorKind::Software, format!("row: {e}")))?);
    }
    Ok(json!({
        "journal": jpath.display().to_string(),
        "exists": true,
        "count": steps.len(),
        "steps": steps,
    }))
}

fn execute_offline_step(step: &WorkflowStep) -> Result<Value, CliError> {
    match step.cmd.as_str() {
        "noop" | "echo" => Ok(json!({
            "cmd": step.cmd,
            "args": step.args,
            "ok": true,
        })),
        "parse" => {
            let path = step
                .args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "parse step needs args.path"))?;
            crate::scrape_local::parse_file(Path::new(path))
        }
        "scrape" => {
            // Offline workflow cannot launch browser without lifecycle; require engine=http.
            let url = step
                .args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "scrape step needs args.url"))?;
            let fmt = step
                .args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("text");
            let opts = crate::scrape_local::ScrapeOpts {
                format: crate::scrape_local::ScrapeFormat::parse(fmt)?,
                engine: "http".into(),
                only_main_content: step
                    .args
                    .get("only_main_content")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                ..Default::default()
            };
            // Block on async HTTP scrape (current_thread I/O runtime).
            let robots = crate::robots::RobotsPolicy::Honor;
            crate::runtime_util::block_on_io(crate::scrape_local::scrape_http(url, robots, &opts))
        }
        "batch-scrape" | "batch_scrape" => {
            let path = step
                .args
                .get("urls_file")
                .or_else(|| step.args.get("urls-file"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "batch-scrape needs args.urls_file")
                })?;
            let urls = crate::scrape_local::read_urls_file(Path::new(path))?;
            let opts = crate::scrape_local::ScrapeOpts {
                format: crate::scrape_local::ScrapeFormat::Text,
                engine: "http".into(),
                ..Default::default()
            };
            crate::runtime_util::block_on_io(crate::scrape_local::batch_scrape_http(
                &urls,
                crate::robots::RobotsPolicy::Honor,
                &opts,
                2,
            ))
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("workflow offline step unsupported cmd: {other}"),
            "Supported offline: noop, echo, parse, scrape (http), batch-scrape; use run --script for browser refs",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
