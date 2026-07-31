// SPDX-License-Identifier: MIT OR Apache-2.0
//! Feature flags for multi-step `run` (mirrors global CLI gates).

/// Feature flags for multi-step `run` (mirrors global CLI gates).
#[derive(Debug, Clone, Copy, Default)]
pub struct RunFlags {
    pub experimental_vision: bool,
    pub experimental_screencast: bool,
    pub category_memory: bool,
    pub category_extensions: bool,
    pub category_third_party: bool,
    pub category_webmcp: bool,
    /// GAP-020: emit one NDJSON line per step on stdout during run.
    pub json_steps: bool,
    /// Per-step wall-clock timeout seconds (0 = no per-step override).
    pub step_timeout_secs: u64,
}

impl RunFlags {
    /// Project CLI global gates into the multi-step dispatcher.
    // One parameter per global gate flag; a struct here would just duplicate
    // `GlobalOpts` fields.
    #[allow(clippy::too_many_arguments)]
    pub fn from_globals(
        experimental_vision: bool,
        experimental_screencast: bool,
        category_memory: bool,
        category_extensions: bool,
        category_third_party: bool,
        category_webmcp: bool,
        json_steps: bool,
        step_timeout_secs: u64,
    ) -> Self {
        Self {
            experimental_vision,
            experimental_screencast,
            category_memory,
            category_extensions,
            category_third_party,
            category_webmcp,
            json_steps,
            step_timeout_secs,
        }
    }
}
