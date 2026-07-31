// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenshot and page image capture helpers.
//!
//! # Workload
//!
//! **Mista:** multi-target rect resolve uses `join_bounded` (I/O CDP). Decode
//! and disk write of PNG/JPEG use `spawn_blocking` / `save_screenshot_async`
//! so Tokio workers are not pinned by `std::fs`.
//!
//! # Module map (Tier-3 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | Options / result / annotation types |
//! | `capture` | CDP `Page.captureScreenshot` orchestration |
//! | `annotate` | Overlay geometry + DOM inject |
//! | `save` | Base64 decode + XDG cache write |

mod annotate;
mod capture;
mod save;
mod types;

#[cfg(test)]
mod tests;

pub use capture::take_screenshot;
pub use save::save_screenshot_async;
pub use types::{AnnotationBox, ScreenshotAnnotation, ScreenshotOptions, ScreenshotResult};
