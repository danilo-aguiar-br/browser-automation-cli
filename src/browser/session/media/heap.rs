// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession media methods (componentized).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Capture a heap snapshot to `path` via HeapProfiler.
    pub async fn heap_take(&mut self, path: &Path) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.heap_chunks.clear();
        self.heap_snapshot_finished = false;
        let _ = self
            .manager
            .client
            .send_command_no_params("HeapProfiler.enable", Some(&session_id))
            .await;
        self.manager
            .client
            .send_command(
                "HeapProfiler.takeHeapSnapshot",
                Some(json!({ "reportProgress": true })),
                Some(&session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("heap take: {e}")))?;
        // Wait for chunks + progress finished (named constants; no product env).
        for _ in
            0..crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_HEAP_OUTER_ITERS)
        {
            self.drain_events();
            if self.heap_snapshot_finished && !self.heap_chunks.is_empty() {
                for _ in 0..crate::xdg::policy::policy_u32(
                    crate::xdg::policy::key::DEFAULT_HEAP_INNER_ITERS,
                ) {
                    self.drain_events();
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
        for _ in
            0..crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_HEAP_FINAL_ITERS)
        {
            self.drain_events();
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::DEFAULT_PERF_TRACE_INNER_SLICE_MS,
                ),
            ))
            .await;
        }
        let body = self.heap_chunks.join("");
        let bytes = body.len();
        if bytes == 0 {
            return Err(CliError::with_suggestion(
                ErrorKind::Browser,
                "heap take produced empty snapshot (no HeapProfiler chunks received)",
                crate::i18n::suggestion_key("heap_capture_failed", None),
            ));
        }
        crate::concurrency::write_bytes_blocking(path.to_path_buf(), body.into_bytes())
            .await
            .map_err(|e| CliError::new(ErrorKind::Io, format!("heap take write: {e}")))?;
        self.heap_chunks.clear();
        self.heap_snapshot_finished = false;
        Ok(json!({
            "heap": "take",
            "path": path.to_string_lossy(),
            "bytes": bytes,
        }))
    }

    /// Summarise a heap snapshot file without a live session.
    pub fn heap_file_summary(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::summarize(path).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// Close/release a heap snapshot file handle.
    pub fn heap_close(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::close_snapshot(path).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// Diff two heap snapshot files (base vs current).
    pub fn heap_compare(base: &Path, current: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::compare(base, current).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// Return detailed heap statistics for a snapshot file.
    pub fn heap_details(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::details(path).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// List duplicated strings from a heap snapshot file.
    pub fn heap_dup_strings(path: &Path) -> Result<Value, CliError> {
        crate::native::heap_snapshot::duplicate_strings(path).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// List nodes for a class id in a heap snapshot file.
    pub fn heap_class_nodes(path: &Path, id: u64) -> Result<Value, CliError> {
        crate::native::heap_snapshot::class_nodes(path, id).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// Run a node-level heap op (`retainers`, `dominators`, …).
    pub fn heap_node_op(path: &Path, node: u64, op: &str) -> Result<Value, CliError> {
        crate::native::heap_snapshot::node_op(path, node, op).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }

    /// Offline object details for one node id (distance, retained size, detachedness).
    /// Return object details for a heap node id.
    pub fn heap_object_details(path: &Path, node: u64) -> Result<Value, CliError> {
        crate::native::heap_snapshot::object_details(path, node).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Io,
                e,
                crate::i18n::suggestion_key("heap_snapshot_input", None),
            )
        })
    }
}
