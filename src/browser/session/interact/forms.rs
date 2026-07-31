// SPDX-License-Identifier: MIT OR Apache-2.0
//! Form interaction: multi-field fill and file upload.

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Fill multiple form fields on the **active** page.
    ///
    /// # Parallelism (rules_rust_paralelismo)
    ///
    /// **Sequential by design** on a single CDP `Page`: each `write` may change
    /// focus, validation, or DOM. Concurrent fills on the same session would race
    /// focus and produce non-deterministic agent results. Multi-field forms do not
    /// justify multi-process Chrome (product law: one residual profile).
    /// Fill multiple form fields from a map of target → value.
    pub async fn fill_form(
        &mut self,
        fields: &[(String, String)],
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        let mut filled = Vec::new();
        for (target, value) in fields {
            self.write(target, value, false).await?;
            filled.push(json!({ "target": target, "value_len": value.len() }));
        }
        let data = json!({ "filled": filled, "count": filled.len() });
        self.attach_snapshot_if(include_snapshot, data).await
    }

    /// Set files on a file input without a native picker.
    pub async fn upload(
        &mut self,
        target: &str,
        path: &Path,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        if !path.is_file() {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("upload path is not a regular file: {}", path.display()),
                crate::i18n::suggestion_key("file_path_invalid", None),
            ));
        }
        let abs = path
            .canonicalize()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("upload canonicalize: {e}")))?;
        self.manager
            .upload_files(
                target,
                &[abs.to_string_lossy().to_string()],
                &self.ref_map,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("upload failed: {e}"),
                    crate::i18n::suggestion_key("target_ref_from_view", None),
                )
            })?;
        self.drain_events();
        let data = json!({
            "uploaded": target,
            "path": abs.to_string_lossy(),
        });
        self.attach_snapshot_if(include_snapshot, data).await
    }
}
