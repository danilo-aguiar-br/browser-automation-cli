// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tab lifecycle: list, open, select, close.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// List open pages/tabs with index and URL.
    pub async fn page_list(&mut self) -> Result<Value, CliError> {
        self.drain_events();
        // Borrow registry: avoid cloning every PageInfo string field for JSON projection.
        let pages: Vec<Value> = self
            .manager
            .pages()
            .iter()
            .map(|p| {
                json!({
                    "tab_id": p.tab_id,
                    "label": p.label,
                    "url": p.url,
                    "title": p.title,
                    "target_type": p.target_type,
                })
            })
            .collect();
        let active = self.manager.active_tab_id();
        Ok(json!({
            "pages": pages,
            "count": pages.len(),
            "active_tab_id": active,
        }))
    }

    /// Create a page. `isolated_context`: `None` = shared; `Some(name)` = named isolated
    /// BrowserContext (tool-ref isolatedContext string; GAP-004). Same name reuses context.
    /// Open a new page/tab, optionally navigating to `url`.
    pub async fn page_new(
        &mut self,
        url: Option<&str>,
        background: bool,
        isolated_context: Option<&str>,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let mut isolation_note = None;
        let mut isolation_limitation = None;
        let isolated_name = isolated_context
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let ctx_id = if let Some(ref name) = isolated_name {
            if let Some(existing) = self.named_contexts.get(name).cloned() {
                isolation_note = Some(format!(
                    "reused named BrowserContext `{name}` for cookie/storage sharing"
                ));
                Some(existing)
            } else {
                match self.manager.create_browser_context().await {
                    Ok(id) => {
                        self.named_contexts.insert(name.clone(), id.clone());
                        isolation_note = Some(format!(
                            "created named BrowserContext `{name}` for cookie/storage isolation within this one-shot process"
                        ));
                        Some(id)
                    }
                    Err(e) => {
                        isolation_limitation =
                            Some("isolated_context_unsupported_on_this_browser".to_string());
                        isolation_note = Some(format!(
                            "isolatedContext `{name}` requested but Browser.createBrowserContext unavailable ({e}); tab uses shared browser context"
                        ));
                        None
                    }
                }
            }
        } else {
            None
        };
        let mut data = self
            .manager
            .tab_new_in_context(url, None, ctx_id)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("page new failed: {e}")))?;
        // tool-ref background: do not switch focus when true (best-effort)
        if !background {
            if let Some(idx) = data.get("index").and_then(|v| v.as_u64()) {
                let _ = self.manager.tab_switch(idx as usize).await;
            }
        }
        if let Some(obj) = data.as_object_mut() {
            obj.insert("background".into(), json!(background));
            obj.insert("isolated_context".into(), json!(isolated_name.as_deref()));
            obj.insert("isolated".into(), json!(isolated_name.is_some()));
            if let Some(n) = isolation_note {
                obj.insert("note".into(), json!(n));
            }
            if let Some(lim) = isolation_limitation {
                obj.insert("limitation".into(), json!(lim));
            }
        }
        self.drain_events();
        Ok(data)
    }

    /// Activate a page by index or target id.
    pub async fn page_select(
        &mut self,
        index: usize,
        bring_to_front: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let mut data =
            self.manager.tab_switch(index).await.map_err(|e| {
                CliError::new(ErrorKind::Browser, format!("page select failed: {e}"))
            })?;
        if bring_to_front {
            if let Ok(session_id) = self.manager.active_session_id() {
                let _ = self
                    .manager
                    .client
                    .send_command("Page.bringToFront", None, Some(session_id))
                    .await;
            }
        }
        if let Some(obj) = data.as_object_mut() {
            obj.insert("bring_to_front".into(), json!(bring_to_front));
        }
        self.ref_map.clear();
        self.drain_events();
        Ok(data)
    }

    /// Close a page by index (or the active page when `None`).
    pub async fn page_close(&mut self, index: Option<usize>) -> Result<Value, CliError> {
        self.drain_events();
        let data =
            self.manager.tab_close(index).await.map_err(|e| {
                CliError::new(ErrorKind::Browser, format!("page close failed: {e}"))
            })?;
        self.ref_map.clear();
        self.drain_events();
        Ok(data)
    }
}
