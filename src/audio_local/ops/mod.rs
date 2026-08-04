// SPDX-License-Identifier: MIT OR Apache-2.0
//! High-level audio info / convert / trim / download (agent envelopes).

mod common;
mod info;
mod source;
mod transform;

pub(crate) use common::project_fields;
pub use info::info;
pub use source::AudioSource;
pub use transform::{convert, trim};

use super::download::download_audio;
use super::limits::AudioLimits;
use crate::error::CliError;
use serde_json::Value;
use std::path::Path;

/// Download URL and optionally project fields.
pub async fn download(
    url: &str,
    out: Option<&Path>,
    max_bytes: Option<usize>,
    require_audio: bool,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let _ = AudioLimits::from_xdg();
    let v = download_audio(url, out, max_bytes, require_audio).await?;
    Ok(project_fields(v, select))
}
