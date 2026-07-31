// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tab and browser-context creation.

use super::super::types::*;
use super::super::BrowserManager;
use crate::native::cdp::types::*;
use serde_json::{json, Value};

impl BrowserManager {
    /// Open a tab in the default browser context.
    ///
    /// Shares cookies and storage with the existing tabs; use
    /// [`tab_new_in_context`](Self::tab_new_in_context) when isolation is wanted.
    pub async fn tab_new(
        &mut self,
        url: Option<&str>,
        label: Option<&str>,
    ) -> Result<Value, String> {
        self.tab_new_in_context(url, label, None).await
    }

    /// Create a tab, optionally inside a BrowserContext (cookie/storage isolation).
    pub async fn tab_new_in_context(
        &mut self,
        url: Option<&str>,
        label: Option<&str>,
        browser_context_id: Option<String>,
    ) -> Result<Value, String> {
        if let Some(label) = label {
            if !is_valid_label(label) {
                return Err(format!(
                    "Invalid tab label `{label}`; labels must start with a letter and contain only \
                     letters, digits, `-`, and `_`"
                ));
            }
            if self.has_label(label) {
                return Err(format!(
                    "Label `{label}` is already used by another tab; labels must be unique within a \
                     session"
                ));
            }
        }

        let target_url = url.unwrap_or("about:blank");

        let result: CreateTargetResult = self
            .client
            .send_command_typed(
                "Target.createTarget",
                &CreateTargetParams {
                    url: target_url.to_string(),
                    browser_context_id: browser_context_id.clone(),
                },
                None,
            )
            .await?;

        let attach: AttachToTargetResult = self
            .client
            .send_command_typed(
                "Target.attachToTarget",
                &AttachToTargetParams {
                    target_id: result.target_id.clone(),
                    flatten: true,
                },
                None,
            )
            .await?;

        self.enable_domains(&attach.session_id).await?;

        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        let index = self.pages.len();
        let label = label.map(|s| s.to_string());
        self.pages.push(PageInfo {
            tab_id,
            label: label.clone(),
            target_id: result.target_id,
            session_id: attach.session_id,
            url: target_url.to_string(),
            title: String::new(),
            target_type: "page".to_string(),
        });
        self.active_page_index = index;

        let mut out = json!({
            "tabId": format_tab_id(tab_id),
            "index": index,
            "label": label,
            "url": target_url,
            "total": self.pages.len(),
        });
        if let Some(ctx) = browser_context_id {
            if let Some(obj) = out.as_object_mut() {
                obj.insert("browser_context_id".into(), json!(ctx));
            }
        }
        Ok(out)
    }

    /// Create an isolated BrowserContext and return its id.
    pub async fn create_browser_context(&self) -> Result<String, String> {
        let result = self
            .client
            .send_command("Browser.createBrowserContext", None, None)
            .await?;
        result
            .get("browserContextId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Browser.createBrowserContext missing browserContextId".to_string())
    }
}
