// SPDX-License-Identifier: MIT OR Apache-2.0
//! `WaitRequest`: the full set of conditions one `wait` can carry.

/// Every condition `wait` can be given (GAP-019/024/032).
///
/// All fields default to "not requested"; see
/// [`crate::browser::OneShotSession::wait_for_conditions`] for the OR algebra.
#[derive(Debug, Clone, Copy, Default)]
pub struct WaitRequest<'a> {
    /// Deadline in milliseconds; also the plain sleep when no condition is set.
    pub ms: Option<u64>,
    /// Substrings that must appear in page text (any match).
    pub texts: &'a [String],
    /// Single CSS selector to wait for.
    pub selector: Option<&'a str>,
    /// Multiple CSS selectors (all or any per state).
    pub selectors: &'a [String],
    /// Visibility/attachment state for selectors (`visible`, `hidden`, …).
    pub state: Option<&'a str>,
    /// Exact URL match condition.
    pub url_exact: Option<&'a str>,
    /// Substring URL match condition.
    pub url_contains: Option<&'a str>,
    /// Wait for a navigation to complete.
    pub navigation: bool,
    /// Quiet window in milliseconds with zero in-flight requests.
    pub network_idle_ms: Option<u64>,
    /// Minimum number of nodes the selector condition must match (default 1).
    pub min_count: Option<u64>,
    /// Window in milliseconds during which the serialized DOM must not change.
    pub dom_stable_ms: Option<u64>,
}
