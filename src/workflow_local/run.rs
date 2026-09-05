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
    // The failing step's id, kind and message, kept together because all three
    // reach the envelope: the id names WHERE, the kind decides the process exit
    // code, and the message says what to change.
    let mut failed: Option<(String, ErrorKind, String)> = None;
    // ONE runtime for the whole manifest, born OUTSIDE the loop.
    //
    // Measured 2026-09-04: every step reached `execute_offline_step`, whose
    // `scrape` / `batch-scrape` arms called `block_on_io` — a helper that builds
    // AND tears down a Tokio runtime on each call — so an N-step manifest paid
    // for N runtimes. Sharing one is safe only because the signal task inside
    // `block_on_with_shutdown` is bound to an `AbortOnDrop` guard and dies with
    // the call; without that guard, N calls would leave N tasks parked in
    // `shutdown_signal()`, which is what blocked this hoist until now.
    let rt = crate::runtime_util::build_io_runtime()?;
    // The loop is wrapped so no `?` inside it can jump over `shutdown_runtime`:
    // a journal write that fails mid-run must still tear the runtime down under
    // its deadline instead of leaking it into an unbounded `Drop`.
    let stepped = (|| -> Result<(), CliError> {
        for sid in &order {
            let step = &by_id[sid];
            // Fail-fast if dependency failed (tracked only in this run).
            if let Some((ref f, _, _)) = failed {
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

            match execute_offline_step(&rt, step) {
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
                    failed = Some((sid.clone(), e.kind(), msg));
                }
            }
        }
        Ok(())
    })();
    crate::runtime_util::shutdown_runtime(rt);
    stepped?;

    let status = if failed.is_some() { "failed" } else { "ok" };
    conn.execute(
        "UPDATE runs SET status=?2, finished_at=?3 WHERE run_id=?1",
        params![run_id, status, now_rfc3339()],
    )
    .ok();

    let payload = json!({
        "run_id": run_id,
        "correlation_id": correlation,
        "status": status,
        "journal": jpath.display().to_string(),
        "order": order,
        "steps": results,
        "note": "offline/data steps executed in-process; browser @eN multi-step remains in `run --script`",
    });

    // A failed run is reported as a FAILURE, not as a successful report about a
    // failure.
    //
    // Measured 2026-08-31: a manifest whose only step was refused answered
    // `ok: true` at the top of the envelope with `"status": "failed"` nested
    // inside it, and exit 0. An agent branches on the exit code before it
    // parses anything, so the one field it reads said the workflow worked. The
    // truth was present and unreachable, which is worse than absent: absent
    // prompts a second look.
    //
    // The payload rides along on the error so the journal path, the step order
    // and every step result stay available — a caller debugging a failure needs
    // MORE of that, not less. The kind is the failing step's own, so a manifest
    // rejected for a bad key exits 2 like any other usage error rather than
    // being flattened into one generic workflow code.
    match failed {
        Some((sid, kind, msg)) => Err(CliError::new(
            kind,
            format!("workflow step `{sid}` failed: {msg}"),
        )
        .with_data(payload)),
        None => Ok(payload),
    }
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
    // Same triple as `workflow_run`, for the same reason: a resumed run that
    // fails must fail, not report success about a failure.
    let mut failed: Option<(String, ErrorKind, String)> = None;
    // Same single-runtime hoist as `workflow_run`, for the same reason; no `?`
    // runs inside this loop, so the teardown below is reached on every path.
    let rt = crate::runtime_util::build_io_runtime()?;
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
        if let Some((ref f, _, _)) = failed {
            results.push(json!({
                "id": sid,
                "cmd": step.cmd,
                "ok": false,
                "skipped": true,
                "reason": format!("after_failure:{f}"),
            }));
            continue;
        }
        match execute_offline_step(&rt, step) {
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
                failed = Some((sid.clone(), e.kind(), msg));
            }
        }
    }
    crate::runtime_util::shutdown_runtime(rt);
    let status = if failed.is_some() { "failed" } else { "ok" };
    conn.execute(
        "UPDATE runs SET status=?2, finished_at=?3 WHERE run_id=?1",
        params![run_id, status, now_rfc3339()],
    )
    .ok();
    let payload = json!({
        "run_id": run_id,
        "correlation_id": correlation,
        "status": status,
        "journal": jpath.display().to_string(),
        "order": order,
        "steps": results,
        "resume": true,
    });
    match failed {
        Some((sid, kind, msg)) => Err(CliError::new(
            kind,
            format!("workflow step `{sid}` failed on resume: {msg}"),
        )
        .with_data(payload)),
        None => Ok(payload),
    }
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
