// SPDX-License-Identifier: MIT OR Apache-2.0
//! Configuration loading surface (rules layout: `src/config.rs`).
//!
//! Product law: **XDG only** — no product environment variables at runtime.
//! This module re-exports [`crate::xdg`] so the clap rules layout name exists
//! while a single implementation remains the source of truth.

pub use crate::xdg::*;

/// Resolve effective global wall-clock timeout in seconds.
///
/// Priority: explicit CLI `--timeout` when `> 0`, else XDG config `timeout`, else `0`
/// (no override; per-operation defaults apply).
pub fn resolve_global_timeout(cli_timeout_secs: u64) -> u64 {
    if cli_timeout_secs > 0 {
        return cli_timeout_secs;
    }
    load_config()
        .ok()
        .and_then(|c| c.timeout)
        .filter(|&t| t > 0)
        .unwrap_or(0)
}

/// Resolve per-step timeout for `run` scripts.
///
/// Priority: CLI `--step-timeout` when `> 0`, else inherit `global_timeout_secs`.
pub fn resolve_step_timeout(cli_step_timeout_secs: u64, global_timeout_secs: u64) -> u64 {
    if cli_step_timeout_secs > 0 {
        cli_step_timeout_secs
    } else {
        global_timeout_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_global_timeout_prefers_cli() {
        // CLI override must win regardless of XDG (we cannot force XDG here).
        assert_eq!(resolve_global_timeout(42), 42);
    }

    #[test]
    fn resolve_step_timeout_inherits_global() {
        assert_eq!(resolve_step_timeout(0, 30), 30);
        assert_eq!(resolve_step_timeout(5, 30), 5);
        assert_eq!(resolve_step_timeout(0, 0), 0);
    }
}