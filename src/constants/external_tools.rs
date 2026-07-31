// SPDX-License-Identifier: MIT OR Apache-2.0
//! External binary budgets and argv values (lighthouse, ffmpeg).

/// Default wall-clock timeout for external lighthouse CLI (seconds).
///
/// Operator override: XDG `config set lighthouse_timeout_secs` (1..=[`EXTERNAL_PROCESS_TIMEOUT_CAP_SECS`]).
pub const DEFAULT_LIGHTHOUSE_TIMEOUT_SECS: u64 = 300;

/// Default wall-clock timeout for optional ffmpeg screencast encode (seconds).
///
/// Operator override: XDG `config set ffmpeg_timeout_secs` (1..=[`EXTERNAL_PROCESS_TIMEOUT_CAP_SECS`]).
pub const DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS: u64 = 120;

/// Upper clamp for operator-configured external process timeouts (seconds).
pub const EXTERNAL_PROCESS_TIMEOUT_CAP_SECS: u64 = 3600;

/// Screencast ffmpeg input framerate (frames per second).
pub const SCREENCAST_FFMPEG_FRAMERATE: u32 = 10;

/// Screencast ffmpeg video codec for `.mp4` output.
pub const SCREENCAST_FFMPEG_VCODEC_MP4: &str = "libx264";

/// Screencast ffmpeg video codec for `.webm` output.
pub const SCREENCAST_FFMPEG_VCODEC_WEBM: &str = "libvpx-vp9";

/// Screencast ffmpeg pixel format.
pub const SCREENCAST_FFMPEG_PIX_FMT: &str = "yuv420p";

/// Lighthouse CLI `--chrome-flags` value (single argv after `=` form).
pub const LIGHTHOUSE_CHROME_FLAGS: &str = "--headless=new";

/// Lighthouse CLI `--only-categories` value.
pub const LIGHTHOUSE_ONLY_CATEGORIES: &str = "accessibility,seo,best-practices";
