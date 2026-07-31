// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tab bookkeeping: ensure a page, list, resolve refs, labels.

use super::super::types::*;
use super::super::BrowserManager;
use crate::native::cdp::types::*;
use serde_json::{json, Value};

impl BrowserManager {
    /// Guarantee at least one tab exists, creating one when there is none.
    ///
    /// A browser can be connected with zero pages, and every page-scoped
    /// command would then fail on a precondition the caller never set.
    pub async fn ensure_page(&mut self) -> Result<(), String> {
        if !self.pages.is_empty() {
            return Ok(());
        }

        let result: CreateTargetResult = self
            .client
            .send_command_typed(
                "Target.createTarget",
                &CreateTargetParams {
                    url: "about:blank".to_string(),
                    browser_context_id: None,
                },
                None,
            )
            .await?;

        let attach_result: AttachToTargetResult = self
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

        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        self.pages.push(PageInfo {
            tab_id,
            label: None,
            target_id: result.target_id,
            session_id: attach_result.session_id.clone(),
            url: "about:blank".to_string(),
            title: String::new(),
            target_type: "page".to_string(),
        });
        self.active_page_index = 0;
        self.enable_domains(&attach_result.session_id).await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Tab management
    // -----------------------------------------------------------------------

    /// Checks if `active_page_index` is still valid and adjusts it if not
    /// (e.g., after a tab was closed).
    pub fn update_active_page_if_needed(&mut self) {
        if self.pages.is_empty() {
            self.active_page_index = 0;
            return;
        }
        if self.active_page_index >= self.pages.len() {
            self.active_page_index = self.pages.len() - 1;
        }
    }

    pub(super) fn update_active_page_after_removal(&mut self, removed_index: usize) {
        self.active_page_index = active_page_index_after_removal(
            self.active_page_index,
            removed_index,
            self.pages.len(),
        );
    }

    /// Every tracked tab as envelope JSON, in tab order.
    pub fn tab_list(&self) -> Vec<Value> {
        self.pages
            .iter()
            .enumerate()
            .map(|(i, p)| {
                json!({
                    "tabId": format_tab_id(p.tab_id),
                    "label": p.label,
                    "title": p.title,
                    "url": p.url,
                    "type": p.target_type,
                    "active": i == self.active_page_index,
                })
            })
            .collect()
    }

    /// Resolve a user-supplied `TabRef` (either `t<N>` or a label) to the
    /// stable numeric `tab_id`. Returns a teaching error for unknown tabs.
    pub fn resolve_tab_ref(&self, tab_ref: &TabRef) -> Result<u32, String> {
        match tab_ref {
            TabRef::Id(id) => {
                if self.has_tab_id(*id) {
                    Ok(*id)
                } else {
                    Err(format!(
                        "Tab {} not found; run `browser-automation-cli tab` to list open tabs",
                        format_tab_id(*id)
                    ))
                }
            }
            TabRef::Label(name) => self
                .pages
                .iter()
                .find(|p| p.label.as_deref() == Some(name.as_str()))
                .map(|p| p.tab_id)
                .ok_or_else(|| {
                    format!(
                        "No tab with label `{name}`; run `browser-automation-cli tab` to list open tabs"
                    )
                }),
        }
    }

    /// Returns true iff a tab already carries the given label.
    pub fn has_label(&self, label: &str) -> bool {
        self.pages.iter().any(|p| p.label.as_deref() == Some(label))
    }
}
