// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual GC policy constants (named — not product env knobs).

/// Prefix for CLI-owned temp Chrome user-data-dir profiles.
pub const CLI_CHROME_MARKER_PREFIX: &str = "browser-automation-cli-chrome-";

/// Chromium side-channel prefix under the process temp dir.
pub const CHROMIUM_TMP_PREFIX: &str = "org.chromium.Chromium.";

/// Hidden Chromium side-channel prefix under the process temp dir.
pub const CHROMIUM_TMP_DOT_PREFIX: &str = ".org.chromium.Chromium.";

/// Max file/dir payload size treated as Singleton-only residue (bytes).
pub const SINGLETON_MAX_BYTES: u64 = 4096;

/// Max directory entries allowed when classifying Singleton-only residue.
pub const SINGLETON_MAX_ENTRIES: usize = 8;

/// Tight window after launch for unattributed chromium tmp (seconds).
pub const INVOCATION_SIDE_CHANNEL_WINDOW_SECS: u64 = 5;

/// Skew allowance when comparing mtime/ctime to `not_before` (seconds).
pub const MTIME_SKEW_SECS: u64 = 2;

/// Minimum age for cross-run stale Singleton GC (seconds).
///
/// Avoids racing a Chromium process still creating files under the same uid.
pub const STALE_MIN_AGE_SECS: u64 = 60;
