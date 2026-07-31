// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Buffered console entries, filtered by level.
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
        let mut messages: Vec<Value> = Vec::new();
        let mut include_mode = "current_navigation";
        if include_preserved {
            for ring in &self.console_preserved {
                messages.extend(ring.iter().cloned());
            }
            messages.extend(self.console_log.iter().cloned());
            include_mode = if self.console_preserved.is_empty() {
                "process_local_only"
            } else {
                "preserved_ring"
            };
        } else {
            messages.extend(self.console_log.iter().cloned());
        }
        if let Some(types_csv) = types {
            let wanted: Vec<String> = types_csv
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !wanted.is_empty() {
                // PAR-74/84: filter_cpu when buffer large (threshold in concurrency).
                messages = crate::concurrency::filter_cpu(messages, |m| {
                    let level = m
                        .get("level")
                        .or_else(|| m.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    wanted.iter().any(|w| level.contains(w))
                });
            }
        }
        if let Some(sw) = service_worker_id {
            let sw = sw.to_string();
            messages = crate::concurrency::filter_cpu(messages, |m| {
                m.get("service_worker_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s == sw)
                    .unwrap_or(false)
            });
        }
        let total = messages.len();
        let page = page_idx.unwrap_or(0);
        let size = page_size.unwrap_or(total.max(1));
        let start = page.saturating_mul(size).min(total);
        let end = (start + size).min(total);
        let page_msgs = messages[start..end].to_vec();
        Ok(json!({
            "messages": page_msgs,
            "count": page_msgs.len(),
            "total": total,
            "page_idx": page,
            "page_size": size,
            "include_preserved": include_preserved,
            "include_preserved_mode": include_mode,
        }))
    }

    /// One buffered console entry by index.
    pub fn console_get(&mut self, id: usize) -> Result<Value, CliError> {
        // Full unpaginated list for get-by-id
        let list = self.console_list(None, None, None, true, None)?;
        let total = list.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        // Prefer original buffer for stable ids
        self.drain_events();
        self.console_log
            .get(id)
            .cloned()
            .map(|m| json!({ "id": id, "message": m }))
            .ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Data,
                    format!("console message id {id} not found (count={total})"),
                    "Use console list to inspect ids (0-based index)",
                )
            })
    }

    /// Empty the console buffer without touching errors.
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
    pub async fn console_dump(&mut self, path: &Path) -> Result<Value, CliError> {
        // Ensure capture is armed (same contract as list/clear).
        let _ = self.console_list(None, None, None, true, None)?;
        // GAP-021: always write a valid JSON array (empty buffer → `[]`, never 0-byte file).
        let messages = self.console_log.clone();
        let body = serde_json::to_vec_pretty(&messages)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("console dump serialize: {e}")))?;
        let count = messages.len();
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
