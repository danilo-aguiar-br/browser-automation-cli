// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::cookies;

use super::super::OneShotSession;

impl OneShotSession {
    /// Cookies for `url`, or for the current page when `url` is `None`.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`]
    /// when no page is active, and with `"cookie list failed: …"` when
    /// `Network.getCookies` / `Network.getAllCookies` is refused — usually a
    /// session where the `Network` domain was never enabled.
    ///
    /// No cookies is not an error: the envelope reports `empty: true`, so an
    /// agent never has to infer absence from the shape of the payload.
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`]
    /// when no page is active, and with
    /// [`ErrorKind::Usage`] when
    /// `cookies_json` is not valid JSON or is not an ARRAY of cookie objects —
    /// a single object is refused rather than wrapped, because guessing here
    /// would hide a payload the caller built wrong.
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"cookie set failed: …"` — when `Network.setCookies` rejects the batch,
    /// which is what happens when an entry carries neither `url` nor `domain`
    /// and the current page URL could not be read to fill it in.
    pub async fn cookie_set(&mut self, cookies_json: &str) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let parsed: Value = crate::json_util::parse_cli_json_value(cookies_json, "cookie set")
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("cookie set JSON invalid: {}", e.message()),
                    crate::i18n::suggestion_key("cookie_json_example", None),
                )
            })?;
        let arr = parsed.as_array().ok_or_else(|| {
            // The two arguments used to be swapped here: the catalog string sat
            // in the MESSAGE slot, which agents match on and which stays
            // English by contract, while the suggestion carried a literal that
            // named `--json`. That flag exists, so a flag-existence check passes
            // on it, but it is the global envelope switch and takes no payload —
            // the cookie array belongs to `--cookies-json`, exactly as the
            // sibling error above already said.
            CliError::with_suggestion(
                ErrorKind::Usage,
                "cookie set expects a JSON array of cookie objects",
                crate::i18n::suggestion_key("cookie_json_example", None),
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
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`]
    /// when no page is active, and with `"cookie clear failed: …"` when
    /// `Network.clearBrowserCookies` is refused.
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
