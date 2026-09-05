// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser page/tab types and pure helpers (no CDP I/O).

use crate::native::cdp::types::*;

pub(crate) fn is_internal_chrome_target(url: &str) -> bool {
    url.starts_with("chrome://")
        || url.starts_with("chrome-extension://")
        || url.starts_with("devtools://")
}

pub(crate) fn should_track_target(target: &TargetInfo) -> bool {
    (target.target_type == "page" || target.target_type == "webview")
        && (target.url.is_empty() || !is_internal_chrome_target(&target.url))
}

pub(crate) fn update_page_target_info_in_pages(
    pages: &mut [PageInfo],
    target: &TargetInfo,
) -> bool {
    if let Some(page) = pages.iter_mut().find(|p| p.target_id == target.target_id) {
        page.url = target.url.clone();
        page.title = target.title.clone();
        page.target_type = target.target_type.clone();
        return true;
    }
    false
}

pub(crate) fn active_page_index_after_removal(
    active_page_index: usize,
    removed_index: usize,
    remaining_pages: usize,
) -> usize {
    if remaining_pages == 0 {
        return 0;
    }

    if removed_index < active_page_index {
        return active_page_index - 1;
    }

    if active_page_index >= remaining_pages {
        return remaining_pages - 1;
    }

    active_page_index
}

/// One tab this invocation is tracking.
///
/// Two identifiers coexist on purpose: `tab_id` is the STABLE id an agent
/// addresses (`t1`), while `target_id` and `session_id` are Chrome's own and
/// mean nothing outside this browser process.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Stable id minted by this session, rendered as `t1`, `t2`, …
    ///
    /// Never a positional index: closing a tab does not renumber the others,
    /// which is what keeps a reference valid across a multi-step script.
    pub tab_id: u32,
    /// Optional user-assigned label (e.g. "docs", "app"). Set via
    /// `tab new --label <name>`. Labels are agent-assigned and never
    /// auto-generated, never rewritten on navigation, and unique within a
    /// session. Agents use labels instead of `t<N>` for readable multi-tab
    /// workflows.
    pub label: Option<String>,
    /// Chrome's own target id for this tab.
    pub target_id: String,
    /// CDP session attached to the target; commands must carry it to land here.
    pub session_id: String,
    /// Current document URL, refreshed on navigation events.
    pub url: String,
    /// Current document title, refreshed on navigation events.
    pub title: String,
    /// Chrome target kind. Only `page` and `webview` are tracked.
    pub target_type: String, // "page" or "webview"
}

/// Canonical string form of a stable tab id: `t1`, `t2`, ... The `t` prefix
/// disambiguates stable ids from positional indices (which the CLI no longer
/// accepts) and matches the `@e<N>` convention used for element refs.
pub fn format_tab_id(tab_id: u32) -> String {
    format!("t{tab_id}")
}

/// A tab reference as parsed from CLI/JSON input. Either a stable id like
/// `t2` or a user-assigned label like `docs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabRef {
    /// A stable id, parsed from `t2`.
    Id(u32),
    /// A user-assigned label, parsed from anything that is not `t<N>`.
    Label(String),
}

impl TabRef {
    /// Parse a user-supplied string tab reference. Rejects bare integers
    /// with a teaching error so agents and scripts don't silently confuse
    /// stable ids with positional indices.
    ///
    /// # Errors
    ///
    /// Fails on an empty or whitespace-only `input`, on a `t<N>` whose digits
    /// overflow `u32`, on `t0` (ids start at `t1`), on a bare integer — which
    /// is refused rather than read as a position — and on a label that does
    /// not start with a letter or carries anything outside letters, digits,
    /// `-` and `_`. Every message names the accepted form.
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("Empty tab reference; expected `t<N>` (e.g. `t2`) or a label".to_string());
        }
        if let Some(digits) = input.strip_prefix('t').or_else(|| input.strip_prefix('T')) {
            if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                let id: u32 = digits.parse().map_err(|_| {
                    format!("Tab id `{input}` out of range; ids are incrementing positive integers")
                })?;
                if id == 0 {
                    return Err(format!("Tab id `{input}` is invalid; tab ids start at t1"));
                }
                return Ok(TabRef::Id(id));
            }
        }
        if input.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!(
                "Expected a tab id like `t{input}` or a label; positional integers are not accepted \
                 (run `browser-automation-cli tab` to list stable tab ids)"
            ));
        }
        if !is_valid_label(input) {
            return Err(format!(
                "Invalid tab label `{input}`; labels must start with a letter and contain only \
                 letters, digits, `-`, and `_`"
            ));
        }
        Ok(TabRef::Label(input.to_string()))
    }
}

/// Labels must look like identifiers: start with a letter, contain only
/// letters/digits/dashes/underscores. This keeps them distinguishable from
/// `t<N>` ids at a glance and safe to pass through shells without quoting.
pub fn is_valid_label(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// How far a navigation must get before the command returns.
///
/// Ordered loosest to strictest in practical terms, and the choice is a
/// correctness decision rather than a speed one: returning at `Load` on a
/// single-page app hands back a document whose content has not arrived yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitUntil {
    /// The `load` event fired: subresources are done.
    Load,
    /// The `DOMContentLoaded` event fired: markup is parsed, subresources may not be.
    DomContentLoaded,
    /// Network went quiet, which is the only one of these that catches
    /// content a single-page app fetches AFTER `load`.
    NetworkIdle,
    /// Return as soon as the navigation is issued, without waiting.
    None,
}

impl std::str::FromStr for WaitUntil {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "domcontentloaded" => Self::DomContentLoaded,
            "networkidle" => Self::NetworkIdle,
            "none" => Self::None,
            _ => Self::Load,
        })
    }
}

impl WaitUntil {
    /// Parse wait-until token; unknown values map to `Load`.
    pub fn parse_token(s: &str) -> Self {
        s.parse().unwrap_or(Self::Load)
    }
}
