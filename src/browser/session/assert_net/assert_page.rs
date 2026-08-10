// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::element::{self};

use super::super::OneShotSession;

impl OneShotSession {
    /// Current URL and title of the active page.
    pub async fn page_info(&mut self) -> Result<Value, CliError> {
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();
        Ok(json!({ "url": url, "title": title }))
    }

    /// Assert the page URL, exactly or by substring.
    ///
    /// `contains` is what makes an assertion survive a query string the caller
    /// does not control.
    pub async fn assert_url(&mut self, value: &str, contains: bool) -> Result<Value, CliError> {
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let ok = if contains {
            url.contains(value)
        } else {
            url == value
        };
        if !ok {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!(
                    "assert url failed: got={url:?} expected contains={contains} value={value:?}"
                ),
                "Navigate first with goto in the same run",
            ));
        }
        Ok(json!({ "assert": "url", "ok": true, "url": url, "value": value, "contains": contains }))
    }

    /// Assert that text is present on the page.
    pub async fn assert_text(
        &mut self,
        value: &str,
        target: Option<&str>,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let haystack = if let Some(t) = target {
            let session_id = self
                .manager
                .active_session_id()
                .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                .to_string();
            element::get_element_text(
                &self.manager.client,
                &session_id,
                &self.ref_map,
                t,
                &self.iframe_sessions,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("assert text: {e}")))?
        } else {
            let v = self
                .manager
                .evaluate("document.body ? document.body.innerText : ''", None)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("assert text: {e}")))?;
            v.as_str().unwrap_or("").to_string()
        };

        if !haystack.contains(value) {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("assert text failed: value not found: {value:?}"),
                "Check view/extract in the same run; text match is substring",
            ));
        }
        Ok(json!({ "assert": "text", "ok": true, "value": value, "target": target }))
    }

    /// Assert that console entries at `level` do not exceed `max`.
    ///
    /// Requires `--capture-console` on this same process: without it the buffer
    /// is empty and the assertion passes for the wrong reason.
    pub async fn assert_console(&mut self, level: &str, max: u64) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "assert console requires --capture-console on the same invocation",
                crate::i18n::suggestion_key("console_capture_run", None),
            ));
        }
        self.drain_events();
        let level_l = level.to_ascii_lowercase();
        // ECO-11/PAR-85: count_cpu borrows the buffer (no full clone for cardinality).
        let count = crate::concurrency::count_cpu(&self.console_log, |m| {
            m.get("type")
                .and_then(|v| v.as_str())
                .map(|t| t.eq_ignore_ascii_case(&level_l))
                .unwrap_or(false)
        }) as u64;
        if count > max {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("assert console failed: level={level} count={count} max={max}"),
                crate::i18n::suggestion_key("console_assert_threshold", None),
            ));
        }
        Ok(json!({
            "assert": "console",
            "ok": true,
            "level": level,
            "count": count,
            "max": max,
        }))
    }

    /// GAP-025: assert the captured console buffer is empty (any level).
    pub async fn assert_console_empty(&mut self) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "assert console_empty requires --capture-console on the same invocation",
                crate::i18n::suggestion_key("console_capture_run", None),
            ));
        }
        self.drain_events();
        let count = self.console_log.len() as u64;
        if count > 0 {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("assert console_empty failed: count={count}"),
                crate::i18n::suggestion_key("console_assert_threshold", None),
            ));
        }
        Ok(json!({
            "assert": "console_empty",
            "ok": true,
            "count": 0,
        }))
    }

    /// GAP-025: assert no console message text matches `pattern` (substring, case-insensitive).
    pub async fn assert_console_no_match(&mut self, pattern: &str) -> Result<Value, CliError> {
        if !self.capture.console {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "assert console_no_match requires --capture-console on the same invocation",
                crate::i18n::suggestion_key("console_capture_run", None),
            ));
        }
        self.drain_events();
        let pat = pattern.to_ascii_lowercase();
        // ECO-11/PAR-85: count only — do not clone the full console buffer.
        let hits = crate::concurrency::count_cpu(&self.console_log, |m| {
            let text = m
                .get("text")
                .or_else(|| m.get("message"))
                .or_else(|| m.get("args"))
                .map(|v| v.to_string())
                .unwrap_or_default()
                .to_ascii_lowercase();
            text.contains(&pat)
        });
        if hits > 0 {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("assert console_no_match failed: pattern={pattern:?} hits={hits}"),
                crate::i18n::suggestion_key("console_assert_threshold", None),
            ));
        }
        Ok(json!({
            "assert": "console_no_match",
            "ok": true,
            "pattern": pattern,
            "hits": 0,
        }))
    }
}
