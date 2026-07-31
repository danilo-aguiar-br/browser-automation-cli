// SPDX-License-Identifier: MIT OR Apache-2.0
//! Public `wait` entry points that normalise arguments into conditions.

use serde_json::Value;

use crate::error::CliError;

use super::super::super::OneShotSession;
use super::request::WaitRequest;

impl OneShotSession {
    /// Wait for a single text/selector condition with a millisecond budget.
    pub async fn wait_for(
        &mut self,
        ms: Option<u64>,
        text: Option<&str>,
        selector: Option<&str>,
        state: Option<&str>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        // Back-compat single text: treat as one-element OR set.
        let owned: Vec<String> = text.map(|t| vec![t.to_string()]).unwrap_or_default();
        self.wait_for_any(ms, &owned, selector, state, include_snapshot)
            .await
    }

    /// Wait until any of `texts` appears (OR), and/or selector/state/ms/url.
    ///
    /// GAP-019: CSS multi-selectors (`#a, #b`) and selector lists are OR-matched.
    /// GAP-024: optional `url` (exact) / `url_contains` / `navigation` (load lifecycle).
    #[allow(clippy::too_many_arguments)]
    /// Wait until any of the listed texts/selectors is satisfied.
    pub async fn wait_for_any(
        &mut self,
        ms: Option<u64>,
        texts: &[String],
        selector: Option<&str>,
        state: Option<&str>,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.wait_for_any_ex(
            ms,
            texts,
            selector,
            &[],
            state,
            None,
            None,
            false,
            include_snapshot,
        )
        .await
    }

    /// Full wait surface used by multi-step `run` (GAP-019/024).
    #[allow(clippy::too_many_arguments)]
    /// Full wait entry: text, selectors, URL, network-idle, DOM-stable (GAP-032).
    pub async fn wait_for_any_ex(
        &mut self,
        ms: Option<u64>,
        texts: &[String],
        selector: Option<&str>,
        selectors: &[String],
        state: Option<&str>,
        url_exact: Option<&str>,
        url_contains: Option<&str>,
        navigation: bool,
        include_snapshot: bool,
    ) -> Result<Value, CliError> {
        self.wait_for_conditions(
            WaitRequest {
                ms,
                texts,
                selector,
                selectors,
                state,
                url_exact,
                url_contains,
                navigation,
                ..WaitRequest::default()
            },
            include_snapshot,
        )
        .await
    }
}
