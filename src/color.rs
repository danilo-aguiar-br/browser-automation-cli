// SPDX-License-Identifier: MIT OR Apache-2.0
//! Color output utilities.
//!
//! Colors are off by default (agent-friendly). Enable with
//! `browser-automation-cli config set color true` (XDG config).
//!
//! Priority (first match wins):
//! 1. process override via [`set_plain`](crate::color::set_plain) / `--plain`
//! 2. `NO_COLOR` (any value) — <https://no-color.org/>
//! 3. `CLICOLOR=0` or `CLICOLOR_FORCE=0`
//! 4. `TERM=dumb`
//! 5. XDG config `color = true`
//! 6. default off

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Process-level plain override (`--plain`).
///
/// # Concurrency
///
/// Single stable address shared by all threads. Uses `Ordering::Relaxed`
/// because the flag is a pure boolean preference with no dependent data
/// publication (no other memory is synchronized through this atomic).
static PLAIN_OVERRIDE: AtomicBool = AtomicBool::new(false);

/// Cached result of env/XDG color policy (computed once per process).
///
/// # Concurrency
///
/// `OnceLock` ensures a single init even under concurrent first calls.
/// After init the value is immutable. `--plain` is checked *before* this
/// cache so the process override always wins without needing a reset.
static COLORS_ENABLED: OnceLock<bool> = OnceLock::new();

/// Force plain (no ANSI) for this process. Called from CLI dispatch for `--plain`.
pub fn set_plain(plain: bool) {
    // Relaxed: no other memory depends on this store for correctness.
    PLAIN_OVERRIDE.store(plain, Ordering::Relaxed);
}

/// Returns true if color output is enabled.
pub fn is_enabled() -> bool {
    // Relaxed: pure flag read; `--plain` must short-circuit before the cache.
    if PLAIN_OVERRIDE.load(Ordering::Relaxed) {
        return false;
    }
    *COLORS_ENABLED.get_or_init(|| {
        if env::var_os("NO_COLOR").is_some() {
            return false;
        }
        if env::var_os("CLICOLOR")
            .map(|v| v == "0")
            .unwrap_or(false)
        {
            return false;
        }
        if env::var_os("CLICOLOR_FORCE")
            .map(|v| v == "0")
            .unwrap_or(false)
        {
            return false;
        }
        if env::var_os("TERM").is_some_and(|t| t == "dumb") {
            return false;
        }
        crate::xdg::load_config()
            .ok()
            .and_then(|c| c.color)
            .unwrap_or(false)
    })
}

/// Format text in red (errors)
pub fn red(text: &str) -> String {
    if is_enabled() {
        format!("\x1b[31m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

/// Format text in green (success)
pub fn green(text: &str) -> String {
    if is_enabled() {
        format!("\x1b[32m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

/// Format text in yellow (warnings)
pub fn yellow(text: &str) -> String {
    if is_enabled() {
        format!("\x1b[33m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

/// Format text in cyan (info)
pub fn cyan(text: &str) -> String {
    if is_enabled() {
        format!("\x1b[36m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

/// Format text in bold
pub fn bold(text: &str) -> String {
    if is_enabled() {
        format!("\x1b[1m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

/// Format text dimmed
pub fn dim(text: &str) -> String {
    if is_enabled() {
        format!("\x1b[2m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_override_disables_color() {
        set_plain(true);
        assert!(!is_enabled());
        set_plain(false);
    }
}
