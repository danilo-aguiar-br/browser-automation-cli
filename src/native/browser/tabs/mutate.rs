// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tab switching and closing, by index and by stable id.

use super::super::types::*;
use super::super::BrowserManager;
use crate::native::cdp::types::*;
use rustc_hash::FxHashSet;
use serde_json::{json, Value};

impl BrowserManager {
    /// Focus a tab by its POSITION in the list.
    ///
    /// Positional and therefore fragile across closes; prefer
    /// [`tab_switch_by_id`](Self::tab_switch_by_id), which survives them.
    ///
    /// Domain enable (`Page.enable` / `Runtime.enable`) is **best-effort**: a
    /// JavaScript dialog is page-modal and can make those CDP calls time out on
    /// the owner target. We still update `active_page_index` so
    /// `Page.handleJavaScriptDialog` can target the correct session (GAP-041
    /// multi-tab). Cached url/title are used when live probes hang.
    ///
    /// # Errors
    ///
    /// Fails only with `"Tab index N out of range (0-M)"` when `index`
    /// addresses no tracked tab. Nothing after the bounds check can fail: the
    /// domain enable is bounded by
    /// [`TAB_SWITCH_DOMAIN_ENABLE_BUDGET_MS`](crate::constants::TAB_SWITCH_DOMAIN_ENABLE_BUDGET_MS)
    /// and reported as `domains_enabled: false`, `Page.bringToFront` is
    /// best-effort, and the url/title probes fall back to the cached values.
    pub async fn tab_switch(&mut self, index: usize) -> Result<Value, String> {
        if index >= self.pages.len() {
            return Err(format!(
                "Tab index {} out of range (0-{})",
                index,
                self.pages.len().saturating_sub(1)
            ));
        }

        self.active_page_index = index;
        let session_id = self.pages[index].session_id.clone();
        // Bound wait: open JS dialogs stall Page.enable on that session.
        let domains_enabled = match tokio::time::timeout(
            std::time::Duration::from_millis(crate::constants::TAB_SWITCH_DOMAIN_ENABLE_BUDGET_MS),
            self.enable_domains(&session_id),
        )
        .await
        {
            Ok(Ok(())) => true,
            Ok(Err(_)) | Err(_) => false,
        };

        // Bring tab to front (best-effort; may also stall under an open dialog).
        let _ = self
            .client
            .send_command("Page.bringToFront", None, Some(&session_id))
            .await;

        let (url, title) = if domains_enabled {
            let url = self.get_url().await.unwrap_or_default();
            let title = self.get_title().await.unwrap_or_default();
            if let Some(page) = self.pages.get_mut(index) {
                page.url = url.clone();
                page.title = title.clone();
            }
            (url, title)
        } else {
            let page = &self.pages[index];
            (page.url.clone(), page.title.clone())
        };

        let page = &self.pages[index];
        Ok(json!({
            "tabId": format_tab_id(page.tab_id),
            "label": page.label,
            "url": url,
            "title": title,
            "domains_enabled": domains_enabled,
        }))
    }

    /// Close a tab by position, or the active one when `index` is `None`.
    ///
    /// # Errors
    ///
    /// Fails with `"Tab index N out of range"` when `index` addresses no
    /// tracked tab, with `"Cannot close the last tab"` when only one remains —
    /// a session with no page cannot serve any later command — and with the
    /// domain-enable error for the tab that becomes active afterwards.
    /// `Target.closeTarget` itself is best-effort: the tab is already dropped
    /// from the registry, so a refusal must not fail the call.
    pub async fn tab_close(&mut self, index: Option<usize>) -> Result<Value, String> {
        let target_index = index.unwrap_or(self.active_page_index);

        if target_index >= self.pages.len() {
            return Err(format!("Tab index {target_index} out of range"));
        }

        if self.pages.len() <= 1 {
            return Err("Cannot close the last tab".to_string());
        }

        let page = self.pages.remove(target_index);
        self.update_active_page_after_removal(target_index);
        let closed_tab_id = page.tab_id;
        let closed_label = page.label.clone();
        let _ = self
            .client
            .send_command_typed::<_, Value>(
                "Target.closeTarget",
                &CloseTargetParams {
                    target_id: page.target_id,
                },
                None,
            )
            .await;

        let session_id = self.pages[self.active_page_index].session_id.clone();
        self.enable_domains(&session_id).await?;

        Ok(json!({
            "tabId": format_tab_id(closed_tab_id),
            "label": closed_label,
            "closed": true,
        }))
    }

    // -----------------------------------------------------------------------
    // Emulation
    // -----------------------------------------------------------------------

    /// Focus a tab by its stable id (`t2`). Unaffected by closes elsewhere.
    ///
    /// # Errors
    ///
    /// Fails with `"Tab ID N not found"` when no tracked tab carries that
    /// stable id — it was closed, or it belongs to another session. Otherwise
    /// propagates [`tab_switch`](Self::tab_switch), whose bounds check cannot
    /// fail on an index just resolved from the registry.
    pub async fn tab_switch_by_id(&mut self, tab_id: u32) -> Result<Value, String> {
        let index = self
            .pages
            .iter()
            .position(|p| p.tab_id == tab_id)
            .ok_or_else(|| format!("Tab ID {tab_id} not found"))?;
        self.tab_switch(index).await
    }

    /// Close a tab by stable id, or the active one when `tab_id` is `None`.
    ///
    /// # Errors
    ///
    /// Fails with `"Tab ID N not found"` when `tab_id` is `Some` and no
    /// tracked tab carries it. Otherwise propagates
    /// [`tab_close`](Self::tab_close): `"Cannot close the last tab"`, or a
    /// failed domain enable on the tab that becomes active.
    pub async fn tab_close_by_id(&mut self, tab_id: Option<u32>) -> Result<Value, String> {
        let index = match tab_id {
            Some(id) => Some(
                self.pages
                    .iter()
                    .position(|p| p.tab_id == id)
                    .ok_or_else(|| format!("Tab ID {id} not found"))?,
            ),
            None => None,
        };
        self.tab_close(index).await
    }

    /// Mint the next stable tab id.
    ///
    /// Monotonic within the session and never reused, so a closed `t2` does
    /// not come back as a different tab.
    pub fn assign_tab_id(&mut self) -> u32 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
    }

    /// Start tracking a tab.
    pub fn add_page(&mut self, page: PageInfo) {
        let index = self.pages.len();
        self.pages.push(page);
        self.active_page_index = index;
    }

    /// Refresh a tracked tab from a `Target.targetInfoChanged` payload.
    ///
    /// Returns whether a tracked tab matched, which is how the caller tells an
    /// update apart from an event about a target it does not follow.
    pub fn update_page_target_info(&mut self, target: &TargetInfo) -> bool {
        update_page_target_info_in_pages(&mut self.pages, target)
    }

    /// Stop tracking a tab, fixing up the active index if it shifted.
    pub fn remove_page_by_target_id(&mut self, target_id: &str) {
        if let Some(pos) = self.pages.iter().position(|p| p.target_id == target_id) {
            self.pages.remove(pos);
            self.update_active_page_after_removal(pos);
        }
    }

    /// Whether a Chrome target id is currently tracked.
    pub fn has_target(&self, target_id: &str) -> bool {
        self.pages.iter().any(|p| p.target_id == target_id)
    }

    /// Number of tracked tabs.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns the stable `tab_id` of the currently active page, if any.
    pub fn active_tab_id(&self) -> Option<u32> {
        self.pages.get(self.active_page_index).map(|p| p.tab_id)
    }

    /// Returns true if a tab with the given stable `tab_id` is still open.
    pub fn has_tab_id(&self, tab_id: u32) -> bool {
        self.pages.iter().any(|p| p.tab_id == tab_id)
    }

    /// Borrow the live page registry (no clone of tab strings).
    pub fn pages(&self) -> &[PageInfo] {
        &self.pages
    }

    /// Owned snapshot of the page registry (clone-all; prefer [`Self::pages`] when a borrow suffices).
    pub fn pages_list(&self) -> Vec<PageInfo> {
        self.pages.clone()
    }

    /// Origins visited in this session, used to scope storage capture.
    pub fn visited_origins(&self) -> &FxHashSet<String> {
        &self.visited_origins
    }
}
