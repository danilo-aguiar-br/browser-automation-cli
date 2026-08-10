// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession media methods (componentized).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Start Chrome Tracing / performance collection.
    pub async fn perf_start(
        &mut self,
        path: Option<&Path>,
        reload: bool,
        auto_stop: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.trace_chunks.clear();
        self.manager
            .client
            .send_command(
                "Tracing.start",
                Some(json!({
                    "categories": "devtools.timeline,v8.execute,blink.user_timing,disabled-by-default-devtools.timeline",
                    "transferMode": "ReportEvents",
                })),
                None,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("perf start: {e}")))?;
        let _ = self
            .manager
            .client
            .send_command_no_params("Performance.enable", Some(&session_id))
            .await;
        self.perf_active = true;
        if reload {
            let _ = self.reload(false).await?;
        }
        let mut out = json!({
            "perf": "start",
            "path": path.map(|p| p.to_string_lossy().to_string()),
            "reload": reload,
            "auto_stop": auto_stop,
        });
        if auto_stop {
            // tool-ref autoStop: stop after load/reload settles
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::DEFAULT_PERF_AUTOSTOP_SETTLE_MS,
                ),
            ))
            .await;
            let stop = self.perf_stop(path).await?;
            if let Some(obj) = out.as_object_mut() {
                obj.insert("auto_stopped".into(), json!(true));
                obj.insert("stop".into(), stop);
            }
        }
        Ok(out)
    }

    /// Stop performance collection and optionally write a trace file.
    pub async fn perf_stop(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        self.pump_events().await;
        self.tracing_complete = false;
        if self.perf_active {
            let _ = self
                .manager
                .client
                .send_command("Tracing.end", None, None)
                .await;
            self.perf_active = false;
        }
        // Wait for dataCollected + tracingComplete (budget via named constants).
        for _ in 0..crate::xdg::policy::policy_u32(
            crate::xdg::policy::key::DEFAULT_PERF_TRACE_OUTER_ITERS,
        ) {
            self.pump_events().await;
            if self.tracing_complete && !self.trace_chunks.is_empty() {
                for _ in 0..crate::xdg::policy::policy_u32(
                    crate::xdg::policy::key::DEFAULT_PERF_TRACE_INNER_ITERS,
                ) {
                    self.pump_events().await;
                    tokio::time::sleep(std::time::Duration::from_millis(
                        crate::xdg::policy::policy_u64(
                            crate::xdg::policy::key::DEFAULT_PERF_TRACE_INNER_SLICE_MS,
                        ),
                    ))
                    .await;
                }
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::DEFAULT_PERF_TRACE_OUTER_SLICE_MS,
                ),
            ))
            .await;
        }
        let body = self.trace_chunks.join("\n");
        let chunks = self.trace_chunks.len();
        self.last_trace_body = Some(body.clone());
        let mut out_path = path.map(|p| p.to_path_buf());
        if out_path.is_none() {
            // Default artifact so insight can always read a file after stop.
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            out_path = Some(PathBuf::from(format!("trace-{stamp}.ndjson")));
        }
        if let Some(ref p) = out_path {
            crate::concurrency::write_bytes_blocking(p.clone(), body.into_bytes())
                .await
                .map_err(|e| CliError::new(ErrorKind::Io, format!("perf stop write: {e}")))?;
            self.last_trace_path = Some(p.clone());
        }
        self.trace_chunks.clear();
        self.tracing_complete = false;
        // Synthetic insight sets for tool-ref performance_analyze_insight flow
        let set_id = format!(
            "set-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        Ok(json!({
            "perf": "stop",
            "path": out_path.map(|p| p.to_string_lossy().to_string()),
            "events": chunks,
            "available_insight_sets": [{
                "insight_set_id": set_id,
                "insights": [
                    "DocumentLatency",
                    "LCPBreakdown",
                    "CLSCulprits",
                    "INPBreakdown",
                    "RenderBlocking",
                    "ThirdParties"
                ]
            }],
        }))
    }

    /// Analyse the last in-session trace (or `path`) into agent-ready insights.
    pub async fn perf_insight(
        &mut self,
        name: Option<&str>,
        insight_set_id: Option<&str>,
    ) -> Result<Value, CliError> {
        self.pump_events().await;
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let live_metrics = self
            .manager
            .client
            .send_command("Performance.getMetrics", None, Some(&session_id))
            .await
            .ok();

        let offline = if let Some(ref p) = self.last_trace_path {
            crate::native::perf_insight::analyze_file(p, name).ok()
        } else if let Some(ref body) = self.last_trace_body {
            crate::native::perf_insight::analyze_text(body, name, None).ok()
        } else {
            None
        };

        Ok(json!({
            "perf": "insight",
            "name": name,
            "insight_name": name,
            "insight_set_id": insight_set_id,
            "live_metrics": live_metrics,
            "trace_insight": offline,
            "trace_path": self.last_trace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        }))
    }

    /// Offline insight from a previously written trace path (no browser required).
    pub fn perf_insight_file(path: &Path, name: Option<&str>) -> Result<Value, CliError> {
        crate::native::perf_insight::analyze_file(path, name).map_err(|e| {
            // Routed through the catalog rather than spelled here: an inline
            // literal stays English under `--lang pt-BR`, and it also escapes
            // the flag-existence check, which is how `--proxy` shipped as advice
            // for a flag that did not exist.
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("perf_trace_path", None),
            )
        })
    }
}
