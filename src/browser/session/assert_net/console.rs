// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Buffered console entries, filtered by level.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] when
    /// `--capture-console` was not given on this invocation — the buffer would
    /// be empty for a reason that has nothing to do with the page.
    ///
    /// Also fails with [`ErrorKind::Usage`] when `service_worker_id` is given.
    /// No producer writes that key: the console ingest records `type`, `text`
    /// and `args`, so the filter could only ever select zero. It answered with
    /// an empty array and `ok: true` — indistinguishable from a worker that
    /// genuinely logged nothing, which is the exact defect this module was
    /// audited for. Refusing costs the caller a clear message instead of a
    /// silent lie, and the flag stays so the refusal can name itself rather
    /// than becoming `unexpected argument`.
    ///
    /// Nothing else fails. A `types` filter that matches nothing, and a
    /// `page_idx` past the end, both answer with an empty `messages` array and
    /// the real `total`.
    pub fn console_list(
        &mut self,
        page_idx: Option<usize>,
        page_size: Option<usize>,
        types: Option<&str>,
        include_preserved: bool,
        service_worker_id: Option<&str>,
    ) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "console list requires --capture-console",
                crate::i18n::suggestion_key("capture_console", None),
            ));
        }
        self.drain_events();
        let (mut messages, include_mode) = super::buffers::compose_view(
            &self.console_preserved,
            &self.console_log,
            include_preserved,
        );
        if let Some(types_csv) = types {
            let wanted: Vec<String> = crate::agent_ops::path::split_csv_lower(types_csv);
            if !wanted.is_empty() {
                // PAR-74/84: filter_cpu when buffer large (threshold in concurrency).
                messages = crate::concurrency::filter_cpu(messages, |m| {
                    // `type` is the ONLY key the producer writes for a level.
                    // This read used to try `level` first and fall back to
                    // `type`. Nothing has ever written `level`, so the first
                    // arm was dead — and it is the same shape that hid the
                    // defect this release exists to close: an alternative
                    // spelling reads as tolerance while it is really evidence
                    // that nobody checked which name the producer uses.
                    //
                    // It survived the fix for its own sibling. The refusal
                    // eight lines below rejects `service_worker_id` and states
                    // that ingest writes "`type`, `text` and `args` and nothing
                    // else" — a sentence that already condemned `level`, four
                    // lines above it, in the same function.
                    let level = m
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    wanted.iter().any(|w| level.contains(w))
                });
            }
        }
        if service_worker_id.is_some() {
            // Consumer without a producer, refused instead of answered. The
            // ingest path writes `type`, `text` and `args` and nothing else, so
            // reading `service_worker_id` here selected zero records every time
            // while reporting success. The sibling defect in `net list` was the
            // same shape and was fixed by giving the field a producer; here
            // there is none to give without mapping execution contexts to
            // targets, which is a feature and not a repair.
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "console list cannot filter by service worker: the console buffer \
                 records type, text and args, and carries no worker attribution, \
                 so this filter can only answer empty. Use `--types` to narrow, or \
                 `eval --service-worker-id` to address a worker directly.",
                crate::i18n::suggestion_key("console_no_worker_attribution", None),
            ));
        }
        let total = messages.len();
        let pg = super::buffers::page_bounds(total, page_idx, page_size);
        let (page, size, start, end) = (pg.index, pg.size, pg.start, pg.end);
        let page_msgs = messages[start..end].to_vec();
        Ok(json!({
            "messages": page_msgs,
            "count": page_msgs.len(),
            "total": total,
            "page_idx": page,
            "page_size": size,
            "include_preserved": include_preserved,
            "include_preserved_mode": include_mode,
            // Truncation is DECLARED, for the same reason as `net list`.
            "dropped_oldest": self.console_dropped,
        }))
    }

    /// One buffered console entry by index.
    ///
    /// # Errors
    ///
    /// Propagates [`console_list`](Self::console_list) for the capture gate —
    /// [`ErrorKind::Usage`] without
    /// `--capture-console` — and fails with
    /// [`ErrorKind::Data`] when `id` addresses
    /// no entry.
    ///
    /// Ids index whatever `include_preserved` selects, so they address exactly
    /// the records `console_list` answered with under the same flag. Until
    /// 0.1.9 the id always indexed the current-navigation buffer while the
    /// count in the error came from the full list, so `console get 0` could
    /// return a different message than index 0 of
    /// `console list --include-preserved` — with `ok: true` on both, which left
    /// the caller no way to notice.
    pub fn console_get(&mut self, id: usize, include_preserved: bool) -> Result<Value, CliError> {
        // Full unpaginated list for get-by-id: one composition, one id space.
        let list = self.console_list(None, None, None, include_preserved, None)?;
        let messages: Vec<Value> = list
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let total = messages.len();
        messages
            .get(id)
            .cloned()
            .map(|m| json!({ "id": id, "message": m }))
            .ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Data,
                    format!("console message id {id} not found (count={total})"),
                    crate::i18n::suggestion_key("console_list_ids", None),
                )
            })
    }

    /// Empty the console buffer without touching errors.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] when
    /// `--capture-console` was not given on this invocation. An already-empty
    /// buffer is not an error; it reports `cleared: 0`.
    pub fn console_clear(&mut self) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "console clear requires --capture-console",
                crate::i18n::suggestion_key("capture_console", None),
            ));
        }
        self.drain_events();
        let n = self.console_log.len();
        self.console_log.clear();
        Ok(json!({ "cleared": n }))
    }

    /// Write the console buffer to a file as JSON.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] when
    /// `--capture-console` was not given, with
    /// [`ErrorKind::Data`] on
    /// `"console dump serialize: …"`, and with
    /// [`ErrorKind::Io`] on
    /// `"console dump write: …"` — an unwritable `path`, a missing parent
    /// directory, or a full disk.
    ///
    /// An empty buffer is not an error: `[]` is written, so the artifact is
    /// always valid JSON rather than a zero-byte file.
    pub async fn console_dump(&mut self, path: &Path) -> Result<Value, CliError> {
        // Ensure capture is armed (same contract as list/clear).
        let _ = self.console_list(None, None, None, true, None)?;
        // GAP-021: always write a valid JSON array (empty buffer → `[]`, never 0-byte file).
        //
        // Serialised straight from the buffer. The clone this replaced copied
        // every captured message only to hand the copy to a serializer that
        // takes a reference, and the buffer is unbounded up to the ring cap.
        let body = serde_json::to_vec_pretty(&self.console_log)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("console dump serialize: {e}")))?;
        let count = self.console_log.len();
        // PAR-78: disk off async / block_on worker (docsrs spawn_blocking).
        crate::concurrency::write_bytes_blocking(path.to_path_buf(), body)
            .await
            .map_err(|e| CliError::new(ErrorKind::Io, format!("console dump write: {e}")))?;
        Ok(json!({
            "path": path.to_string_lossy(),
            "count": count,
            "format": "json_array",
        }))
    }
}
