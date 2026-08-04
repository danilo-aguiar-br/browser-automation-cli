// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wave-6 pure-Rust codec gates: AVIF encode, HEIC decode, SVG raster, GIF
//! multi-frame, and the SIMD resize backend.
//!
//! Every test builds its own fixture in-process. Nothing is downloaded, and no
//! binary blob is committed, so the suite runs identically offline.

use browser_automation_cli::image_local;
#[allow(unused_imports)]
use image::GenericImageView;

/// Build a small RGB gradient as raw pixels.
fn gradient(w: u32, h: u32) -> image::DynamicImage {
    let mut img = image::RgbImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = image::Rgb([(x * 7 % 256) as u8, (y * 11 % 256) as u8, 0x40]);
    }
    image::DynamicImage::ImageRgb8(img)
}

/// Encode a two-frame animated GIF: red frame then blue frame.
fn animated_gif(frames: u32) -> Vec<u8> {
    use image::codecs::gif::GifEncoder;
    let mut buf = Vec::new();
    {
        let mut enc = GifEncoder::new(std::io::Cursor::new(&mut buf));
        for i in 0..frames {
            let mut f = image::RgbaImage::new(8, 8);
            let shade = (i * 40 % 256) as u8;
            for px in f.pixels_mut() {
                *px = image::Rgba([shade, 0x20, 0xFF - shade, 0xFF]);
            }
            enc.encode_frame(image::Frame::from_parts(
                f,
                0,
                0,
                image::Delay::from_saturating_duration(std::time::Duration::from_millis(60)),
            ))
            .expect("encode gif frame");
        }
    }
    buf
}

// ── GAP-IMG-093: GIF multi-frame ───────────────────────────────────────────

#[test]
fn gif_frame_count_reports_real_animation_length() {
    let bytes = animated_gif(5);
    let n = image_local::frame_count(&bytes).expect("frame count");
    assert_eq!(n, 5, "envelope must not hard-code frame_count: 1");
}

#[test]
fn gif_single_frame_still_counts_one() {
    let bytes = animated_gif(1);
    assert_eq!(image_local::frame_count(&bytes).unwrap(), 1);
}

#[test]
fn gif_extract_frames_preserves_order_and_delay() {
    let bytes = animated_gif(3);
    let frames = image_local::extract_frames(&bytes).expect("extract");
    assert_eq!(frames.len(), 3);
    for (i, f) in frames.iter().enumerate() {
        assert_eq!(f.index as usize, i);
        assert_eq!(f.image.width(), 8);
        // 60 ms was requested; GIF stores delay in centiseconds, so the
        // round-trip is exact here.
        assert_eq!(f.delay_ms, 60, "frame {i} delay");
    }
}

#[test]
fn gif_reassemble_round_trips_frame_count() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("rebuilt.gif");
    let frames = image_local::extract_frames(&animated_gif(4)).unwrap();
    let written = image_local::reassemble(frames, Some(80), &out).expect("reassemble");
    assert!(written > 0);

    let rebuilt = std::fs::read(&out).unwrap();
    assert_eq!(image_local::frame_count(&rebuilt).unwrap(), 4);
    let back = image_local::extract_frames(&rebuilt).unwrap();
    assert_eq!(back[0].delay_ms, 80, "explicit delay override must survive");
}

#[test]
fn gif_write_frame_png_emits_a_real_png() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("frame2.png");
    image_local::write_frame_png(&animated_gif(4), 2, &out).expect("write frame");
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(
        image_local::detect_format(&bytes).unwrap(),
        image_local::DetectedFormat::Png
    );
}

// ── GAP-IMG-090: AVIF ──────────────────────────────────────────────────────

#[test]
fn avif_decode_is_never_available() {
    // Not a scope decision: rav1d and rav1d-safe both declare cc + nasm-rs as
    // unconditional build-dependencies, so no C-free decoder exists.
    assert!(!image_local::avif_decode_available());
}

#[test]
#[cfg_attr(not(feature = "image-avif"), ignore = "requires --features image-avif")]
fn avif_encode_produces_a_detectable_avif_container() {
    let img = gradient(32, 24);
    let bytes = image_local::encode_avif_to_vec(&img, 60).expect("avif encode");
    assert!(!bytes.is_empty());
    assert_eq!(
        image_local::detect_format(&bytes).unwrap(),
        image_local::DetectedFormat::Avif,
        "encoder must emit a real ftyp avif brand"
    );
}

#[test]
#[cfg_attr(feature = "image-avif", ignore = "feature is on in this build")]
fn avif_encode_fails_closed_without_the_feature() {
    let err = image_local::parse_output_format("avif").unwrap_err();
    assert!(
        err.message().contains("image-avif"),
        "error must name the missing feature, got: {}",
        err.message()
    );
}

// ── GAP-IMG-091: HEIC ──────────────────────────────────────────────────────

#[test]
fn heic_encode_is_never_available_and_says_why() {
    assert!(!image_local::heic_encode_available());
    let err = image_local::parse_output_format("heic").unwrap_err();
    assert!(
        err.message().contains("HEVC"),
        "message must name the real blocker, got: {}",
        err.message()
    );
}

#[test]
fn heic_magic_is_detected_regardless_of_decode_support() {
    // Minimal ISOBMFF header with the `heic` brand.
    let mut bytes = vec![0u8, 0, 0, 0x18];
    bytes.extend_from_slice(b"ftypheic");
    bytes.extend_from_slice(&[0u8; 16]);
    assert_eq!(
        image_local::detect_format(&bytes).unwrap(),
        image_local::DetectedFormat::Heic
    );
}

// ── GAP-IMG-092: SVG ───────────────────────────────────────────────────────

const SAFE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="30"><rect width="40" height="30" fill="#0a7"/></svg>"##;

#[test]
fn svg_is_detected_by_token_scan() {
    assert!(image_local::looks_like_svg(SAFE_SVG.as_bytes()));
    assert_eq!(
        image_local::detect_format(SAFE_SVG.as_bytes()).unwrap(),
        image_local::DetectedFormat::Svg
    );
}

#[test]
fn svg_sanitizer_accepts_a_plain_document() {
    let report = image_local::sanitize_svg(SAFE_SVG.as_bytes()).expect("safe svg");
    assert_eq!(report.entities, 0);
    assert!(report.max_depth >= 1);
}

#[test]
#[cfg_attr(not(feature = "image-svg"), ignore = "requires --features image-svg")]
fn svg_rasterizes_at_its_intrinsic_size() {
    let raster = image_local::rasterize(SAFE_SVG.as_bytes(), 1.0).expect("raster");
    assert_eq!((raster.width, raster.height), (40, 30));
}

#[test]
#[cfg_attr(not(feature = "image-svg"), ignore = "requires --features image-svg")]
fn svg_scale_multiplies_output_dimensions() {
    let raster = image_local::rasterize(SAFE_SVG.as_bytes(), 2.0).expect("raster");
    assert_eq!((raster.width, raster.height), (80, 60));
}

// ── GAP-IMG-095: SIMD resize ───────────────────────────────────────────────

#[test]
fn resize_backend_matches_the_compiled_feature() {
    let expected = if cfg!(feature = "image-simd-resize") {
        "fast_image_resize"
    } else {
        "image::imageops"
    };
    assert_eq!(image_local::resize_backend(), expected);
}

#[test]
fn resize_hits_the_requested_dimensions_exactly() {
    let src = gradient(64, 48);
    let out = image_local::resize_exact(&src, 32, 24).expect("resize");
    assert_eq!((out.width(), out.height()), (32, 24));
}

#[test]
fn resize_upscale_is_also_exact() {
    let src = gradient(16, 16);
    let out = image_local::resize_exact(&src, 40, 25).expect("upscale");
    assert_eq!((out.width(), out.height()), (40, 25));
}

#[test]
fn resize_preserves_visual_content_not_just_geometry() {
    // A solid-colour source must stay that colour after resampling; a backend
    // wired to the wrong pixel layout would show up here as channel swap.
    let mut src = image::RgbaImage::new(32, 32);
    for px in src.pixels_mut() {
        *px = image::Rgba([0x10, 0x90, 0xE0, 0xFF]);
    }
    let out = image_local::resize_exact(&image::DynamicImage::ImageRgba8(src), 16, 16).unwrap();
    let px = out.to_rgba8();
    let mid = px.get_pixel(8, 8);
    assert_eq!(mid.0[0], 0x10, "red channel");
    assert_eq!(mid.0[1], 0x90, "green channel");
    assert_eq!(mid.0[2], 0xE0, "blue channel");
}

// ── GAP-IMG-093: honesty of the degraded path ──────────────────────────────

#[test]
fn gif_info_marks_the_frame_count_as_exact_when_the_walk_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ok.gif");
    std::fs::write(&path, animated_gif(3)).unwrap();
    let v = image_local::info(&image_local::ImageSource::Path(path), false, None).unwrap();
    assert_eq!(v["frame_count"], 3);
    assert_eq!(v["frame_count_exact"], true);
    assert_eq!(v["animated"], true);
}

#[test]
fn gif_info_flags_a_degraded_frame_count_instead_of_claiming_one() {
    // Truncate after the header so the image decodes nothing extra but the
    // frame walk fails. The envelope must not silently report a confident 1.
    let full = animated_gif(4);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("truncated.gif");
    std::fs::write(&path, &full[..full.len() / 2]).unwrap();

    match image_local::info(&image_local::ImageSource::Path(path), false, None) {
        Ok(v) => assert_eq!(
            v["frame_count_exact"], false,
            "a failed frame walk must be visible in the envelope, got {v}"
        ),
        // A truncation severe enough to kill the base decode is also acceptable:
        // the honesty requirement is that `info` never lies, not that it succeeds.
        Err(e) => assert_eq!(e.kind(), browser_automation_cli::error::ErrorKind::Data),
    }
}

#[test]
fn non_gif_formats_report_an_exact_single_frame() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.png");
    let img = gradient(6, 6);
    img.save(&path).unwrap();
    let v = image_local::info(&image_local::ImageSource::Path(path), false, None).unwrap();
    assert_eq!(v["frame_count"], 1);
    assert_eq!(v["frame_count_exact"], true);
    assert_eq!(v["animated"], false);
}
