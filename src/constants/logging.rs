// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local tracing level and rotated log-file retention defaults.

/// Default local tracing filter when argv flags are quiet and XDG `log_level` is unset.
///
/// Operator override: argv (`-q`/`-v`/`--debug`) or XDG `config set log_level <EnvFilter>`.
pub const DEFAULT_LOG_LEVEL: &str = "error";

/// Default retained rotated log files under XDG state (`log_to_file`).
///
/// Operator override: XDG `config set max_log_files <n>` (`MAX_LOG_FILES_MIN`..=`MAX_LOG_FILES_CAP`).
pub const DEFAULT_MAX_LOG_FILES: u32 = 14;

/// Minimum allowed `max_log_files` (config set clamp).
pub const MAX_LOG_FILES_MIN: u32 = 1;

/// Maximum allowed `max_log_files` (config set clamp; ~3 months daily).
pub const MAX_LOG_FILES_CAP: u32 = 90;

/// Default rolling policy when XDG `log_rotation` is unset (`daily` \| `hourly` \| `never`).
pub const DEFAULT_LOG_ROTATION: &str = "daily";
