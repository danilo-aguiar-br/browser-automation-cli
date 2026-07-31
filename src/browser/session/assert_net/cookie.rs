// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::cookies;

use super::super::OneShotSession;

impl OneShotSession {
    /// Cookies for `url`, or for the current page when `url` is `None`.
    pub async fn cookie_list(&mut self, url: Option<&str>) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let cookies = if let Some(u) = url {
            cookies::get_cookies(&self.manager.client, &session_id, Some(vec![u.to_string()])).await
        } else {
            cookies::get_all_cookies(&self.manager.client, &session_id).await
        }
        .map_err(|e| CliError::new(ErrorKind::Browser, format!("cookie list failed: {e}")))?;
        // GAP-020: every reader reports `empty`, so an agent never has to infer
        // "nothing there" from the shape of the payload.
        Ok(json!({
            "cookies": cookies,
            "count": cookies.len(),
            "empty": cookies.is_empty(),
            "url_filter": url,
        }))
    }

    /// Set cookies from a JSON array, in one CDP call.
    pub async fn cookie_set(&mut self, cookies_json: &str) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let parsed: Value =
            crate::json_util::parse_cli_json_value(cookies_json, "cookie set").map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("cookie set JSON invalid: {}", e.message()),
                    r#"Use --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'"#,
                )
            })?;
        let arr = parsed.as_array().ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                crate::i18n::suggestion_key("json_array_objects", None),
                r#"Use --json '[{"name":"a","value":"b","url":"https://example.com"}]'"#,
            )
        })?;
        let current_url = self.manager.get_url().await.ok();
        cookies::set_cookies(
            &self.manager.client,
            &session_id,
            arr.clone(),
            current_url.as_deref(),
        )
        .await
        .map_err(|e| CliError::new(ErrorKind::Browser, format!("cookie set failed: {e}")))?;
        Ok(json!({ "ok": true, "set_count": arr.len() }))
    }

    /// Remove every cookie from the browser profile.
    pub async fn cookie_clear(&mut self) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        cookies::clear_cookies(&self.manager.client, &session_id)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("cookie clear failed: {e}")))?;
        Ok(json!({ "ok": true, "cleared": true }))
    }
}
