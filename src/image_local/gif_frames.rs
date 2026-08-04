// SPDX-License-Identifier: MIT OR Apache-2.0
//! GIF animation frame probe, extraction, and reassembly (GAP-IMG-093).
//!
//! Before this module `image info` reported a hard-coded `frame_count: 1` for
//! every GIF, which is a lie an agent cannot detect. The decoder here walks the
//! real frame list under an XDG-backed ceiling and reports what is actually
//! there.

use std::io::Cursor;
use std::path::Path;

use image::codecs::gif::{GifDecoder, GifEncoder};
use image::{AnimationDecoder, Frame};

use super::atomic::write_bytes_atomic;
use crate::error::{CliError, ErrorKind};

/// One decoded animation frame plus its presentation delay.
pub struct GifFrame {
    /// Zero-based index in the animation.
    pub index: u32,
    /// Delay before the next frame, in milliseconds.
    pub delay_ms: u32,
    /// Fully composited RGBA frame (GIF disposal already applied).
    pub image: image::RgbaImage,
}

fn decode_err(e: &image::ImageError) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Data,
        format!("gif frame decode failed: {e}"),
        crate::i18n::suggestion_key("image_magic_invalid", None),
    )
}

fn too_many_frames(max: u32) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Data,
        format!("gif exceeds gif_max_frames {max}"),
        crate::i18n::suggestion_key("image_too_large", None),
    )
}

/// Count animation frames without retaining pixel buffers.
///
/// Frames are dropped as they are produced, so a 2 000-frame GIF costs one
/// frame of memory rather than 2 000. Returns `Err` when the animation exceeds
/// `gif_max_frames`.
pub fn frame_count(bytes: &[u8]) -> Result<u32, CliError> {
    let max = crate::xdg::resolve_gif_max_frames();
    let decoder = GifDecoder::new(Cursor::new(bytes)).map_err(|e| decode_err(&e))?;
    let mut n: u32 = 0;
    for frame in decoder.into_frames() {
        frame.map_err(|e| decode_err(&e))?;
        n = n.saturating_add(1);
        if n > max {
            return Err(too_many_frames(max));
        }
    }
    Ok(n)
}

/// Decode every frame, composited and RGBA, under the `gif_max_frames` ceiling.
pub fn extract_frames(bytes: &[u8]) -> Result<Vec<GifFrame>, CliError> {
    let max = crate::xdg::resolve_gif_max_frames();
    let decoder = GifDecoder::new(Cursor::new(bytes)).map_err(|e| decode_err(&e))?;
    let mut out = Vec::new();
    for (index, frame) in decoder.into_frames().enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        if index >= max {
            return Err(too_many_frames(max));
        }
        let frame = frame.map_err(|e| decode_err(&e))?;
        let (num, den) = frame.delay().numer_denom_ms();
        // `Delay` is an exact rational; collapse it to whole ms for the wire.
        let delay_ms = if den == 0 { 0 } else { num / den };
        out.push(GifFrame {
            index,
            delay_ms,
            image: frame.into_buffer(),
        });
    }
    Ok(out)
}

/// Write frame `index` to `out_path` as PNG (lossless, keeps alpha).
pub fn write_frame_png(bytes: &[u8], index: u32, out_path: &Path) -> Result<usize, CliError> {
    let frames = extract_frames(bytes)?;
    let frame = frames
        .into_iter()
        .find(|f| f.index == index)
        .ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Data,
                format!("gif has no frame at index {index}"),
                crate::i18n::suggestion_key("use_listed_value", None),
            )
        })?;
    let dynamic = image::DynamicImage::ImageRgba8(frame.image);
    super::encode::encode_to_path(
        &dynamic,
        out_path,
        super::encode::OutputFormat::Image(image::ImageFormat::Png),
        100,
    )
}

/// Reassemble RGBA frames into an animated GIF and write it atomically.
///
/// `delay_ms` overrides every frame delay when `Some`; otherwise each frame
/// keeps the delay it carried in.
pub fn reassemble(
    frames: Vec<GifFrame>,
    delay_ms: Option<u32>,
    out_path: &Path,
) -> Result<usize, CliError> {
    if frames.is_empty() {
        return Err(CliError::new(
            ErrorKind::NoInput,
            "gif reassemble needs at least one frame",
        ));
    }
    let max = crate::xdg::resolve_gif_max_frames();
    if u32::try_from(frames.len()).unwrap_or(u32::MAX) > max {
        return Err(too_many_frames(max));
    }
    let mut buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(Cursor::new(&mut buf));
        encoder
            .set_repeat(image::codecs::gif::Repeat::Infinite)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("gif repeat: {e}")))?;
        for f in frames {
            let ms = delay_ms.unwrap_or(f.delay_ms);
            let frame = Frame::from_parts(
                f.image,
                0,
                0,
                image::Delay::from_saturating_duration(std::time::Duration::from_millis(
                    u64::from(ms),
                )),
            );
            encoder
                .encode_frame(frame)
                .map_err(|e| CliError::new(ErrorKind::Data, format!("gif encode frame: {e}")))?;
        }
    }
    write_bytes_atomic(out_path, &buf)?;
    Ok(buf.len())
}
