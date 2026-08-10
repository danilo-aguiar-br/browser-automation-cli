// SPDX-License-Identifier: MIT OR Apache-2.0
//! Workflow run / resume / status (sequential journal writes).

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::params;
use serde_json::{json, Value};
use uuid::Uuid;

use super::dag::{load_manifest, validate_dag};
use super::journal::{journal_path, now_rfc3339, open_db};
use super::offline::execute_offline_step;
use super::types::WorkflowStep;
use crate::error::{CliError, ErrorKind};

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
            crate::i18n::suggestion_key("workflow_run_first", None),
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
