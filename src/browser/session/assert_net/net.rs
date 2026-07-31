// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Captured network requests.
    ///
    /// Requires `--capture-network` on this same process; capture does not
    /// survive the end of the invocation that enabled it.
    pub fn net_list(
        &mut self,
        page_idx: Option<usize>,
        page_size: Option<usize>,
        resource_types: Option<&str>,
        include_preserved: bool,
    ) -> Result<Value, CliError> {
        if !self.capture.network {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "net list requires --capture-network",
                crate::i18n::suggestion_key("capture_network", None),
            ));
        }
        self.drain_events();
        let mut requests: Vec<Value> = Vec::new();
        let mut include_mode = "current_navigation";
        if include_preserved {
            for ring in &self.network_preserved {
                requests.extend(ring.iter().cloned());
            }
            requests.extend(self.network_log.iter().cloned());
            include_mode = if self.network_preserved.is_empty() {
                "process_local_only"
            } else {
                "preserved_ring"
            };
        } else {
            requests.extend(self.network_log.iter().cloned());
        }
        if let Some(types_csv) = resource_types {
            let wanted: Vec<String> = types_csv
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !wanted.is_empty() {
                // PAR-74/84: filter_cpu when buffer large.
                requests = crate::concurrency::filter_cpu(requests, |r| {
                    let rt = r
                        .get("resource_type")
                        .or_else(|| r.get("type"))
                        .or_else(|| r.get("resourceType"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    wanted.iter().any(|w| rt.contains(w))
                });
            }
        }
        let total = requests.len();
        let page = page_idx.unwrap_or(0);
        let size = page_size.unwrap_or(total.max(1));
        let start = page.saturating_mul(size).min(total);
        let end = (start + size).min(total);
        let page_reqs = requests[start..end].to_vec();
        Ok(json!({
            "requests": page_reqs,
            "count": page_reqs.len(),
            "total": total,
            "page_idx": page,
            "page_size": size,
            "include_preserved": include_preserved,
            "include_preserved_mode": include_mode,
        }))
    }

    /// Resolve a network entry by 0-based index or CDP `requestId` string.
    pub async fn net_get(
        &mut self,
        id: &str,
        request_path: Option<&Path>,
        response_path: Option<&Path>,
    ) -> Result<Value, CliError> {
        let _ = self.net_list(None, None, None, true)?;
        let requests = self.network_log.clone();
        let (index, req) = if let Ok(idx) = id.parse::<usize>() {
            let req = requests.get(idx).cloned().ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Data,
                    format!(
                        "network request index {idx} not found (count={})",
                        requests.len()
                    ),
                    "Use net list; pass 0-based index or requestId string",
                )
            })?;
            (idx, req)
        } else {
            let (idx, req) = requests
                .iter()
                .enumerate()
                .find(|(_, r)| {
                    r.get("requestId")
                        .and_then(|v| v.as_str())
                        .map(|rid| rid == id)
                        .unwrap_or(false)
                })
                .map(|(i, r)| (i, r.clone()))
                .ok_or_else(|| {
                    CliError::with_suggestion(
                        ErrorKind::Data,
                        format!(
                            "network requestId {id} not found (count={})",
                            requests.len()
                        ),
                        "Use net list; pass 0-based index or exact requestId",
                    )
                })?;
            (idx, req)
        };
        let request_id = req
            .get("requestId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // PAR-79: optional path dumps off async / block_on worker.
        if let Some(p) = request_path {
            let body = serde_json::to_vec_pretty(&req)
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get serialize: {e}")))?;
            crate::concurrency::write_bytes_blocking(p.to_path_buf(), body)
                .await
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get request-path: {e}")))?;
        }
        if let Some(p) = response_path {
            let body = serde_json::to_vec_pretty(&req)
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get serialize: {e}")))?;
            crate::concurrency::write_bytes_blocking(p.to_path_buf(), body)
                .await
                .map_err(|e| CliError::new(ErrorKind::Io, format!("net get response-path: {e}")))?;
        }
        Ok(json!({
            "id": index,
            "requestId": request_id,
            "request": req,
            "request_path": request_path.map(|p| p.to_string_lossy().to_string()),
            "response_path": response_path.map(|p| p.to_string_lossy().to_string()),
        }))
    }
}
