//! Color output utilities.
//!
//! Colors are off by default (agent-friendly). Enable with
//! `browser-automation-cli config set color true` (XDG config). Setting `NO_COLOR`
//! to any value disables colors per <https://no-color.org/> (OS convention).

use std::env;
use std::sync::OnceLock;

/// Returns true if color output is enabled.
///
/// Priority: `NO_COLOR` (presence disables, per OS convention) >
/// XDG config `color = true` > default (off).
pub fn is_enabled() -> bool {
    static COLORS_ENABLED: OnceLock<bool> = OnceLock::new();
    *COLORS_ENABLED.get_or_init(|| {
        if env::var_os("NO_COLOR").is_some() {
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
