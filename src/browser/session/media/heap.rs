// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession media methods (componentized).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Capture a heap snapshot to `path` via HeapProfiler.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`]
    /// when no page is active, when `HeapProfiler.takeHeapSnapshot` is refused
    /// (`"heap take: …"`), and when the polling budget elapses with no chunk
    /// received — `"heap take produced empty snapshot"`, which is what an
    /// engine with no `HeapProfiler` domain produces. `HeapProfiler.enable` is
    /// best-effort and its refusal alone is not reported.
    ///
    /// Fails with [`ErrorKind::Io`] —
    /// `"heap take write: …"` — when the assembled snapshot cannot be written.
    /// Nothing is written when no chunk arrived, so a failed capture never
    /// leaves a zero-byte snapshot behind.
    pub async fn heap_take(&mut self, path: &Path) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.heap_chunks.clear();
        self.heap_snapshot_finished = false;
        self.heap_bytes = 0;
        self.heap_overflow = false;
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
        // Refuse a snapshot that outgrew its budget rather than write the
        // slices that did arrive. Concatenating a prefix of a JSON document
        // yields invalid JSON, and it would land on disk with `ok: true` and a
        // byte count that looks entirely healthy — the caller would only find
        // out when something else failed to parse it, far from here.
        if self.heap_overflow {
            self.heap_bytes = 0;
            self.heap_overflow = false;
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!(
                    "heap snapshot exceeded the heap_snapshot_max_bytes budget ({} bytes) while \
                     streaming; a partial snapshot is not valid JSON, so nothing was written",
                    crate::xdg::policy::policy_u64(
                        crate::xdg::policy::key::HEAP_SNAPSHOT_MAX_BYTES
                    )
                ),
                crate::i18n::suggestion_key("heap_capture_failed", None),
            ));
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
        self.heap_bytes = 0;
        Ok(json!({
            "heap": "take",
            "path": path.to_string_lossy(),
            "bytes": bytes,
        }))
    }

    /// Summarise a heap snapshot file without a live session.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`], carrying the
    /// `heap_snapshot_input` suggestion, for every offline failure: `path`
    /// unreadable, the file above the `heap_snapshot_max_bytes` budget, an
    /// allocation that does not fit in host RAM, or JSON that is not a V8
    /// `.heapsnapshot`. The last case is reported as `Io` rather than
    /// [`ErrorKind::Data`] because every
    /// offline heap error takes the same mapping.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when the
    /// closing summary cannot be built: `path` unreadable, over budget, or not
    /// a V8 `.heapsnapshot`. There is no handle to release in a one-shot
    /// process — the file is re-read to produce the summary — so "close" here
    /// can fail for exactly the reasons a read can.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when either
    /// snapshot cannot be loaded — `base` is tried first — for an unreadable
    /// path, a file over the `heap_snapshot_max_bytes` budget, or JSON that is
    /// not a V8 `.heapsnapshot`. Both files are held in memory at once, so a
    /// pair that each pass the budget alone can still exhaust RAM.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when `path`
    /// cannot be loaded: unreadable, over the `heap_snapshot_max_bytes`
    /// budget, or not a V8 `.heapsnapshot`.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when `path`
    /// cannot be loaded. The scan itself cannot fail: previews are cut by
    /// CHARACTER, so a multi-byte string truncates instead of panicking, and
    /// the list is capped rather than refused.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when `path`
    /// cannot be loaded, and with the same kind when `id` is out of range —
    /// it is a 1-based RANK into the classes sorted by instance count, so `0`
    /// and any value above the class count are refused. The node list is
    /// capped by `heap_max_class_nodes` and reports `truncated: true` rather
    /// than failing.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when `path`
    /// cannot be loaded, and when `node` matches neither a V8 node id nor a
    /// valid node index.
    ///
    /// An `op` outside `edges`, `retainers`, `dominators`, `paths` and
    /// `object-details` is **not** an error: it echoes the op name with the
    /// node info and no payload, so a typo reads as an empty answer rather
    /// than as a rejection.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Io`] when `path`
    /// cannot be loaded, and when `node` matches neither a V8 node id nor a
    /// valid node index.
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
