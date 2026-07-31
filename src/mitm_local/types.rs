// SPDX-License-Identifier: MIT OR Apache-2.0
//! Capture store types and persistence.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

use super::util::{atomic_write, redact_headers};

/// Stable map type alias for headers.
pub type BTreeMapString = std::collections::BTreeMap<String, String>;

/// One captured HTTP(S) exchange (agent-facing, secrets redacted by default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedExchange {
    /// Monotonic id within the capture.
    pub id: u64,
    /// Request method.
    pub method: String,
    /// Absolute URL.
    pub url: String,
    /// HTTP status if known.
    pub status: Option<u16>,
    /// Resource / content type hint.
    pub content_type: Option<String>,
    /// Request headers (redacted).
    pub request_headers: BTreeMapString,
    /// Response headers (redacted).
    pub response_headers: BTreeMapString,
    /// Truncated request body.
    pub request_body: Option<String>,
    /// Truncated response body.
    pub response_body: Option<String>,
    /// Host extracted from URL.
    pub host: Option<String>,
    /// Wall-clock unix millis.
    pub started_ms: u64,
}

/// One captured WebSocket frame (agent-facing, truncated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedWsFrame {
    /// Direction: client|server|unknown.
    pub direction: String,
    /// Frame kind hint.
    pub kind: String,
    /// Truncated payload preview.
    pub preview: String,
    /// Wall-clock unix millis.
    pub ts_ms: u64,
}

/// In-memory + disk-backed capture for one process.
#[derive(Debug, Default)]
pub struct MitmCapture {
    /// Captured exchanges.
    pub items: Vec<CapturedExchange>,
    /// Captured WebSocket frames in this process.
    pub ws_frames: Vec<CapturedWsFrame>,
    /// Next id.
    next_id: u64,
    /// Optional path for persistence.
    path: Option<PathBuf>,
    /// Redact Authorization/Cookie by default.
    redact: bool,
    /// Pid that wrote the loaded file (`None` for a fresh in-memory capture).
    owner_pid: Option<u32>,
}

impl MitmCapture {
    /// Create a new capture optionally bound to a path.
    pub fn new(path: Option<PathBuf>, redact: bool) -> Self {
        Self {
            items: Vec::new(),
            ws_frames: Vec::new(),
            next_id: 0,
            path,
            redact,
            owner_pid: None,
        }
    }

    /// Record a WebSocket frame (capped).
    pub fn push_ws(&mut self, frame: CapturedWsFrame) {
        if self.ws_frames.len()
            < crate::xdg::policy::policy_usize(crate::xdg::policy::key::MITM_WS_FRAMES_CAP)
        {
            self.ws_frames.push(frame);
        }
    }

    /// Append an exchange.
    pub fn push(&mut self, mut ex: CapturedExchange) {
        if self.redact {
            redact_headers(&mut ex.request_headers);
            redact_headers(&mut ex.response_headers);
        }
        ex.id = self.next_id;
        self.next_id += 1;
        self.items.push(ex);
    }

    /// Persist JSON snapshot.
    pub fn save(&self) -> Result<PathBuf, CliError> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| CliError::new(ErrorKind::Config, "mitm capture path not set"))?;
        if let Some(parent) = path.parent() {
            xdg::ensure_dir(parent)?;
        }
        let body = serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            // GAP-009: stamp the writer so a later process cannot silently read
            // another invocation's capture.
            "owner_pid": std::process::id(),
            "count": self.items.len(),
            "ws_count": self.ws_frames.len(),
            "items": self.items,
            "ws_frames": self.ws_frames,
        }))
        .map_err(|e| CliError::new(ErrorKind::Data, format!("serialize mitm capture: {e}")))?;
        atomic_write(&path, &body)?;
        Ok(path)
    }

    /// Load from disk, enforcing the one-shot state boundary (GAP-009).
    ///
    /// `explicit_path` is `true` when the operator named the file with
    /// `--capture-path`. Without it, a capture written by a *different* process
    /// is refused rather than served: cross-invocation reads violate the state
    /// boundary and can leak another session's secrets.
    pub fn load_scoped(path: &Path, redact: bool, explicit_path: bool) -> Result<Self, CliError> {
        let cap = Self::load(path, redact)?;
        if explicit_path || cap.owner_pid.is_none() {
            return Ok(cap);
        }
        if cap.owner_pid == Some(std::process::id()) {
            return Ok(cap);
        }
        // State-boundary policy, not a malformed argv: the caller asked a valid
        // question and the answer is gated behind --capture-path (exit 64).
        Err(CliError::with_suggestion(
            ErrorKind::CapabilityDisabled,
            format!(
                "mitm capture at {} belongs to another invocation (pid {})",
                path.display(),
                cap.owner_pid.unwrap_or(0)
            ),
            crate::i18n::suggestion_key("mitm_capture_path", None),
        ))
    }

    /// Load from disk.
    pub fn load(path: &Path, redact: bool) -> Result<Self, CliError> {
        if !path.exists() {
            return Ok(Self::new(Some(path.to_path_buf()), redact));
        }
        let v: Value =
            crate::json_util::read_json_value_file(path, crate::xdg::resolve_max_json_file_bytes())
                .map_err(|e| {
                    CliError::new(
                        e.kind(),
                        format!("mitm capture {}: {}", path.display(), e.message()),
                    )
                })?;
        let items: Vec<CapturedExchange> =
            serde_json::from_value(v.get("items").cloned().unwrap_or_else(|| json!([])))
                .map_err(|e| CliError::new(ErrorKind::Data, format!("mitm items: {e}")))?;
        let ws_frames: Vec<CapturedWsFrame> =
            serde_json::from_value(v.get("ws_frames").cloned().unwrap_or_else(|| json!([])))
                .unwrap_or_default();
        let next_id = items.iter().map(|i| i.id).max().map(|m| m + 1).unwrap_or(0);
        let owner_pid = v
            .get("owner_pid")
            .and_then(|p| p.as_u64())
            .map(|p| p as u32);
        Ok(Self {
            items,
            ws_frames,
            next_id,
            path: Some(path.to_path_buf()),
            redact,
            owner_pid,
        })
    }
}
