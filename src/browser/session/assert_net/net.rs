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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] when
    /// `--capture-network` was not given on this invocation.
    ///
    /// Also fails with [`ErrorKind::Usage`] when `resource_types` names a token
    /// outside the CDP resource-type vocabulary. That refusal is what makes an
    /// empty answer readable: it means the page had no such resource, and no
    /// longer doubles as the reply to a typo.
    ///
    /// Nothing else fails. A valid filter selecting nothing, and a `page_idx`
    /// past the end, both answer with an empty `requests` array. `total` counts
    /// the FILTERED set rather than the buffer, because it is the denominator
    /// for paging within this answer: paging past the end leaves it untouched,
    /// while a narrowing filter lowers it.
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
        let (mut requests, include_mode) = super::buffers::compose_view(
            &self.network_preserved,
            &self.network_log,
            include_preserved,
        );
        if let Some(types_csv) = resource_types {
            // One reader for both surfaces: argv reaches it through clap's
            // value_parser, and `run --script` reaches it here, because a step
            // key never passes through clap. A second reader would drift and
            // start rejecting steps that argv still accepts.
            let wanted =
                crate::net::resource_type::parse_resource_types(types_csv).map_err(|message| {
                    CliError::with_suggestion(
                        ErrorKind::Usage,
                        message,
                        crate::i18n::suggestion_key("resource_type_vocabulary", None),
                    )
                })?;
            if !wanted.is_empty() {
                // Single key, not three. The old triple lookup read
                // `resource_type`, `type` and `resourceType`, and the capture
                // log wrote none of them, so the tolerance looked like
                // robustness while hiding that no producer existed at all.
                //
                // PAR-74/84: filter_cpu parallelises above CPU_MAP_THRESHOLD,
                // which is 32 in `src/concurrency/pool.rs`. No CLI invocation
                // reaches that branch: `net list` at the top level now refuses,
                // so the only caller is `run --script`, whose buffer is whatever
                // one scripted page produced — measured 25 on `rust-lang.org`.
                // The parallel path is kept because a long-running script can
                // exceed the threshold, not because the CLI ever does.
                requests = crate::concurrency::filter_cpu(requests, |r| {
                    let rt = r
                        .get("resourceType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    wanted.contains(&rt)
                });
            }
        }
        let total = requests.len();
        let pg = super::buffers::page_bounds(total, page_idx, page_size);
        let (page, size, start, end) = (pg.index, pg.size, pg.start, pg.end);
        let page_reqs = requests[start..end].to_vec();
        Ok(json!({
            "requests": page_reqs,
            "count": page_reqs.len(),
            "total": total,
            "page_idx": page,
            "page_size": size,
            "include_preserved": include_preserved,
            "include_preserved_mode": include_mode,
            // Truncation is DECLARED. A ring that silently forgets its oldest
            // rows answers with a subset and calls it the whole set.
            "dropped_oldest": self.network_dropped,
        }))
    }

    /// Resolve a network entry by 0-based index or CDP `requestId` string.
    ///
    /// # Errors
    ///
    /// Propagates [`net_list`](Self::net_list) for the capture gate —
    /// [`ErrorKind::Usage`] without
    /// `--capture-network` — and fails with
    /// [`ErrorKind::Data`] when a numeric `id`
    /// is out of range or a `requestId` string matches no entry.
    ///
    /// Fails with [`ErrorKind::Io`] when
    /// `request_path` or `response_path` cannot be serialized or written. Both
    /// dumps currently write the same request record; the response body is not
    /// fetched here.
    pub async fn net_get(
        &mut self,
        id: &str,
        request_path: Option<&Path>,
        response_path: Option<&Path>,
        include_preserved: bool,
    ) -> Result<Value, CliError> {
        // Index over the SAME composition `net_list` answers with, under the
        // same flag. Reading `self.network_log` directly made `net get 0`
        // resolve a different record than index 0 of
        // `net list --include-preserved`, and both replied ok:true.
        let listed = self.net_list(None, None, None, include_preserved)?;
        let requests: Vec<Value> = listed
            .get("requests")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let (index, req) = if let Ok(idx) = id.parse::<usize>() {
            let req = requests.get(idx).cloned().ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Data,
                    format!(
                        "network request index {idx} not found (count={})",
                        requests.len()
                    ),
                    crate::i18n::suggestion_key("net_get_index_or_request_id", None),
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
                        crate::i18n::suggestion_key("net_get_exact_request_id", None),
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
