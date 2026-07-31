// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenshot and screencast encoding quality defaults.

/// Default JPEG quality when `grab`/`screenshot` omits `--quality` (1..=100).
///
/// Operator override: XDG `config set default_jpeg_quality <n>`.
pub const DEFAULT_JPEG_QUALITY: u8 = 80;

/// Default screencast CDP JPEG quality (1..=100).
///
/// Operator override: XDG `config set screencast_jpeg_quality <n>`.
pub const DEFAULT_SCREENCAST_JPEG_QUALITY: u8 = 60;

/// Screencast start: immediate pump iterations after Page.startScreencast.
pub const DEFAULT_SCREENCAST_START_PUMP_ITERS: u32 = 15;

/// Screencast stop: drain pump iterations before stopScreencast.
pub const DEFAULT_SCREENCAST_STOP_PUMP_ITERS: u32 = 40;
