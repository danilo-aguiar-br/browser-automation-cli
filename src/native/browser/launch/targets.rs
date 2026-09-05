// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP target discovery and attach.

use serde_json::{json, Value};

use crate::native::browser::types::{should_track_target, PageInfo};
use crate::native::browser::BrowserManager;
use crate::native::cdp::types::{
    AttachToTargetParams, AttachToTargetResult, CreateTargetParams, CreateTargetResult,
    GetTargetsResult, SetDiscoverTargetsParams, TargetInfo,
};

impl BrowserManager {
    /// Discover page targets, attach to each, and enable domains on the active one.
    ///
    /// `pub(super)` so the Lightpanda startup loop in the sibling module can drive
    /// the same discovery under a deadline.
    pub(super) async fn discover_and_attach_targets(&mut self) -> Result<(), String> {
        self.client
            .send_command_typed::<_, Value>(
                "Target.setDiscoverTargets",
                &SetDiscoverTargetsParams { discover: true },
                None,
            )
            .await?;

        let result: GetTargetsResult = self
            .client
            .send_command_typed("Target.getTargets", &json!({}), None)
            .await?;

        let page_targets: Vec<TargetInfo> = result
            .target_infos
            .into_iter()
            .filter(should_track_target)
            .collect();

        if page_targets.is_empty() {
            // Create a new tab
            let result: CreateTargetResult = self
                .client
                .send_command_typed(
                    "Target.createTarget",
                    &CreateTargetParams {
                        url: crate::constants::ABOUT_BLANK.to_string(),
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
                url: crate::constants::ABOUT_BLANK.to_string(),
                title: String::new(),
                target_type: "page".to_string(),
            });
            self.active_page_index = 0;
            self.enable_domains(&attach_result.session_id).await?;
        } else {
            // Parallel attach (I/O CDP) then assign tab ids in stable target order.
            let cdp_limit =
                crate::concurrency::effective_limit_capped(crate::concurrency::CDP_FANOUT_CAP);
            let client = std::sync::Arc::clone(&self.client);
            let attach_futs: Vec<_> = page_targets
                .iter()
                .map(|target| {
                    let client = std::sync::Arc::clone(&client);
                    let tid = target.target_id.clone();
                    async move {
                        client
                            .send_command_typed::<_, AttachToTargetResult>(
                                "Target.attachToTarget",
                                &AttachToTargetParams {
                                    target_id: tid,
                                    flatten: true,
                                },
                                None,
                            )
                            .await
                    }
                })
                .collect();
            let attach_results =
                crate::concurrency::join_bounded_ordered(attach_futs, cdp_limit).await;
            for (target, attach_result) in page_targets.iter().zip(attach_results) {
                let attach_result = attach_result?;
                let tab_id = self.next_tab_id;
                self.next_tab_id += 1;
                self.pages.push(PageInfo {
                    tab_id,
                    label: None,
                    target_id: target.target_id.clone(),
                    session_id: attach_result.session_id.clone(),
                    url: target.url.clone(),
                    title: target.title.clone(),
                    target_type: target.target_type.clone(),
                });
            }

            self.active_page_index = 0;
            let session_id = self.pages[0].session_id.clone();
            self.enable_domains(&session_id).await?;
        }

        Ok(())
    }
}
