// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::element::{self};

use super::super::OneShotSession;

impl OneShotSession {
    /// Current URL and title of the active page.
    ///
    /// # Errors
    ///
    /// Never returns `Err`. Both reads are best-effort: a page that cannot be
    /// evaluated — no active tab, or a destroyed execution context — yields
    /// empty strings rather than a failure, so the envelope always describes
    /// what the session could see.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Data`] when the
    /// live URL does not match `value` — by substring under `contains`, byte
    /// for byte otherwise. The message carries both sides, and the suggestion
    /// names the usual cause: nothing navigated in this process, so the URL is
    /// still `about:blank`.
    ///
    /// A URL that cannot be read is not distinguished from a mismatch: it
    /// reads as the empty string and fails the same way.
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
                crate::i18n::suggestion_key("assert_url_navigate_first", None),
            ));
        }
        Ok(json!({ "assert": "url", "ok": true, "url": url, "value": value, "contains": contains }))
    }

    /// Assert that text is present on the page.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`]
    /// when no page is active, and with `"assert text: …"` when `target`
    /// resolves to no element or the `document.body.innerText` evaluation is
    /// refused. Unlike [`assert_url`](Self::assert_url), an unreadable page is
    /// reported as an error here rather than folded into a mismatch.
    ///
    /// Fails with [`ErrorKind::Data`] when the
    /// text is absent. The comparison is a case-sensitive substring, so a
    /// difference in capitalisation fails as "not found".
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
                crate::i18n::suggestion_key("assert_text_substring", None),
            ));
        }
        Ok(json!({ "assert": "text", "ok": true, "value": value, "target": target }))
    }

    /// Assert that console entries at `level` do not exceed `max`.
    ///
    /// Requires `--capture-console` on this same process: without it the buffer
    /// is empty and the assertion passes for the wrong reason.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] when
    /// `--capture-console` was not given on this invocation. That gate is the
    /// point of the method: without capture the buffer is empty and every
    /// threshold would pass, which is worse than refusing.
    ///
    /// Fails with [`ErrorKind::Data`] when the
    /// count of entries whose `type` equals `level` (case-insensitive) exceeds
    /// `max`. An unknown `level` matches nothing and therefore passes.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] without
    /// `--capture-console` on this invocation, and with
    /// [`ErrorKind::Data`] when the buffer
    /// holds any entry at all — including `log` and `info`, since this
    /// assertion is level-blind by design.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Usage`] without
    /// `--capture-console` on this invocation, and with
    /// [`ErrorKind::Data`] when any entry's
    /// `text`, `message` or `args` field contains `pattern`, compared
    /// case-insensitively. The match runs over the JSON rendering of those
    /// fields, so a pattern can also hit escaping and punctuation the console
    /// never printed.
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
            // `text` and `args` are two of the three keys the producer writes
            // in `ingest.rs`. `message` was a third spelling nothing has ever
            // written onto a console record, so this arm could not fire.
            //
            // It never changed an answer, because `text` is always present and
            // the chain short-circuits before reaching it. It is recorded here
            // anyway because of WHERE it survived: it is the same shape as the
            // `level` key removed from `console.rs` one wave earlier, and it
            // outlived that fix by sitting in the one file in this directory
            // the class gate did not inspect.
            let text = m
                .get("text")
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
