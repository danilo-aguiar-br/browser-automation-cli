// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for the local image pipeline.

use super::magic::{detect_format, verify_format_name, DetectedFormat};
use super::ops::{convert, info, resize, ImageSource};
use super::{encode_to_vec, parse_output_format, write_bytes_atomic, ImageLimits, OutputFormat};
use image::{DynamicImage, Rgba, RgbaImage};
use std::path::PathBuf;

fn tiny_png_bytes() -> Vec<u8> {
    let mut img = RgbaImage::new(4, 4);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgba([(x * 40) as u8, (y * 40) as u8, 200, 255]);
    }
    let dyn_img = DynamicImage::ImageRgba8(img);
    encode_to_vec(&dyn_img, OutputFormat::Image(image::ImageFormat::Png), 90).expect("png")
}

#[test]
fn magic_detects_png_jpeg_gif_webp_headers() {
    let png = tiny_png_bytes();
    assert_eq!(detect_format(&png).unwrap(), DetectedFormat::Png);
    assert!(verify_format_name(&png, "png"));

    assert_eq!(
        detect_format(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        DetectedFormat::Jpeg
    );
    assert_eq!(detect_format(b"GIF89a......").unwrap(), DetectedFormat::Gif);
    let mut webp = b"RIFF\x00\x00\x00\x00WEBP".to_vec();
    webp.extend_from_slice(&[0u8; 4]);
    assert_eq!(detect_format(&webp).unwrap(), DetectedFormat::Webp);
}

#[test]
fn magic_detects_avif_then_refuses_to_decode_it() {
    // size(4) + ftyp + avif
    let mut b = vec![0u8, 0, 0, 0x18];
    b.extend_from_slice(b"ftyp");
    b.extend_from_slice(b"avif");
    b.extend_from_slice(&[0u8; 8]);

    // Detection now succeeds: AVIF is a format this build can *encode*, so
    // conflating "unknown container" with "no decoder" would misinform an agent.
    let fmt = detect_format(&b).expect("avif brand is recognised");
    assert_eq!(fmt, super::DetectedFormat::Avif);
    assert_eq!(fmt.is_encodable(), super::avif_encode_available());

    // Decode stays refused in every configuration (no C-free AV1 decoder).
    assert!(!fmt.is_supported());
    let Err(err) = super::decode_bytes(&b, ImageLimits::from_xdg()) else {
        panic!("avif decode must be refused");
    };
    assert_eq!(err.kind(), crate::error::ErrorKind::Data);
    assert!(
        err.message().contains("decode"),
        "message must be about decode, got: {}",
        err.message()
    );
}

#[test]
fn info_reports_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.png");
    std::fs::write(&path, tiny_png_bytes()).unwrap();
    let v = info(&ImageSource::Path(path), false, None).unwrap();
    assert_eq!(v["format"], "png");
    assert_eq!(v["width"], 4);
    assert_eq!(v["height"], 4);
    assert_eq!(v["magic_ok"], true);
    assert!(v.get("base64").is_none());
}

#[test]
fn info_select_projects_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.png");
    std::fs::write(&path, tiny_png_bytes()).unwrap();
    let v = info(&ImageSource::Path(path), false, Some("format,width,height")).unwrap();
    assert_eq!(v["action"], "info");
    assert_eq!(v["format"], "png");
    assert_eq!(v["width"], 4);
    assert!(v.get("sha256").is_none());
    assert!(v.get("exif").is_none());
}

#[test]
fn convert_png_to_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    let dst = dir.path().join("b.jpg");
    std::fs::write(&src, tiny_png_bytes()).unwrap();
    let v = convert(
        &ImageSource::Path(src),
        "jpeg",
        Some(85),
        Some(&dst),
        true,
        false,
    )
    .unwrap();
    assert_eq!(v["format"], "jpeg");
    assert_eq!(v["magic_ok"], true);
    assert_eq!(v["exif_stripped"], true);
    assert_eq!(v["quality_applied"], true);
    assert_eq!(v["keep_exif_honored"], false);
    assert!(dst.is_file());
    let bytes = std::fs::read(&dst).unwrap();
    assert!(verify_format_name(&bytes, "jpeg"));
}

#[test]
fn resize_halves_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    let dst = dir.path().join("r.png");
    std::fs::write(&src, tiny_png_bytes()).unwrap();
    let v = resize(
        &ImageSource::Path(src),
        2,
        Some(2),
        false,
        Some(&dst),
        Some("png"),
        None,
    )
    .unwrap();
    assert_eq!(v["width"], 2);
    assert_eq!(v["height"], 2);
}

#[test]
fn limits_reject_oversized_buffer() {
    let lim = ImageLimits {
        max_input_bytes: 8,
        max_pixels: 1_000_000,
        default_quality: 80,
    };
    assert!(lim.check_input_len(9).is_err());
    assert!(lim.check_dimensions(100_000, 100_000).is_err());
}

#[test]
fn atomic_write_creates_file_without_tmp() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.bin");
    write_bytes_atomic(&path, b"hello").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    assert!(!tmp.exists());
}

#[test]
fn parse_format_accepts_aliases() {
    assert!(parse_output_format("jpg").is_ok());
    assert!(parse_output_format("PNG").is_ok());
    // AVIF encode is feature-gated, so the outcome must track the build rather
    // than be asserted as a constant (GAP-IMG-090).
    assert_eq!(
        parse_output_format("avif").is_ok(),
        super::avif_encode_available()
    );
    // HEIC encode is refused in every build: no pure-Rust HEVC encoder exists.
    assert!(parse_output_format("heic").is_err());
    assert!(!super::heic_encode_available());
    // AVIF decode likewise has no C-free path in any configuration.
    assert!(!super::avif_decode_available());
}

#[test]
fn screenshot_ext_webp_jpeg_png() {
    use crate::native::screenshot::screenshot_ext_for_format;
    assert_eq!(screenshot_ext_for_format("webp"), "webp");
    assert_eq!(screenshot_ext_for_format("jpeg"), "jpg");
    assert_eq!(screenshot_ext_for_format("jpg"), "jpg");
    assert_eq!(screenshot_ext_for_format("png"), "png");
}

#[test]
fn magic_first_ignores_lying_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("qr-like.bin");
    std::fs::write(&path, tiny_png_bytes()).unwrap();
    let img = super::decode_path_for_qr(&path).unwrap();
    assert_eq!(img.width(), 4);
    assert_eq!(img.height(), 4);
}

#[test]
fn ssrf_blocks_loopback_image_url() {
    let err = crate::net::assert_safe_http_url("http://127.0.0.1/img.png").unwrap_err();
    // Strict SSRF: usage or data — never success.
    assert!(matches!(
        err.kind(),
        crate::error::ErrorKind::Usage
            | crate::error::ErrorKind::Data
            | crate::error::ErrorKind::Unavailable
    ));
}

#[test]
fn exif_map_empty_on_png_without_exif() {
    let map = super::read_exif_map(&tiny_png_bytes(), false).unwrap();
    assert!(map.is_empty());
}

#[test]
fn exif_reads_jpeg_without_panic() {
    // Re-encoded JPEG has no EXIF APP1; empty map is correct and non-fatal.
    let dyn_img =
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3])));
    let bytes = encode_to_vec(&dyn_img, OutputFormat::Image(image::ImageFormat::Jpeg), 90).unwrap();
    let map = super::read_exif_map(&bytes, false).unwrap();
    assert!(map.is_empty());
    // GPS filter path must not panic on empty.
    let map_gps = super::read_exif_map(&bytes, true).unwrap();
    assert!(map_gps.is_empty());
}

/// Minimal JPEG with APP1 Exif + Orientation=1 (LE TIFF IFD).
fn jpeg_with_orientation_exif() -> Vec<u8> {
    // SOI + APP1(Exif) + tiny baseline body (1x1). Built for kamadak-exif Reader.
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"II"); // LE
    tiff.extend_from_slice(&42u16.to_le_bytes());
    tiff.extend_from_slice(&8u32.to_le_bytes()); // IFD offset
    tiff.extend_from_slice(&1u16.to_le_bytes()); // 1 entry
                                                 // tag 0x0112 Orientation, type SHORT(3), count 1, value 1
    tiff.extend_from_slice(&0x0112u16.to_le_bytes());
    tiff.extend_from_slice(&3u16.to_le_bytes());
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD
    let mut exif_payload = b"Exif\x00\x00".to_vec();
    exif_payload.extend_from_slice(&tiff);
    let app1_len = (exif_payload.len() + 2) as u16;
    let mut app1 = vec![0xFF, 0xE1];
    app1.extend_from_slice(&app1_len.to_be_bytes());
    app1.extend_from_slice(&exif_payload);

    // Minimal 1x1 grayscale JPEG body (DQT/SOF/DHT/SOS/EOI) without APP segments.
    let body: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07,
        0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
        0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27,
        0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34,
        0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B,
        0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x03, 0xFF, 0xC4, 0x00, 0x14, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00,
        0x00, 0x3F, 0x00, 0x7F, 0xFF, 0xD9,
    ];
    let mut out = Vec::with_capacity(body.len() + app1.len());
    out.extend_from_slice(&body[..2]); // SOI
    out.extend_from_slice(&app1);
    out.extend_from_slice(&body[2..]);
    out
}

#[test]
fn exif_reads_orientation_from_jpeg_app1() {
    // T7 strong: real APP1 EXIF Orientation tag (not empty re-encode).
    let bytes = jpeg_with_orientation_exif();
    let map = super::read_exif_map(&bytes, false).unwrap();
    assert!(
        map.keys().any(|k| k.contains("Orientation")),
        "expected Orientation in EXIF map, got {map:?}"
    );
    let map2 = super::read_exif_map(&bytes, true).unwrap();
    assert!(!map2.is_empty());
}

#[test]
fn convert_webp_reports_quality_not_applied() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.png");
    let dst = dir.path().join("b.webp");
    std::fs::write(&src, tiny_png_bytes()).unwrap();
    let v = convert(
        &ImageSource::Path(src),
        "webp",
        Some(50),
        Some(&dst),
        true,
        true, // keep_exif requested but not honored
    )
    .unwrap();
    assert_eq!(v["format"], "webp");
    assert_eq!(v["quality_applied"], false);
    assert_eq!(v["keep_exif_requested"], true);
    assert_eq!(v["keep_exif_honored"], false);
    assert_eq!(v["exif_stripped"], true);
}

#[test]
fn exif_select_tags_alias_projects_canonical_exif() {
    // Agent-friendly: select `tags` → canonical key `exif` (not empty envelope).
    let bytes = jpeg_with_orientation_exif();
    let map = super::read_exif_map(&bytes, false).unwrap();
    assert!(!map.is_empty());
    let full = serde_json::json!({
        "action": "exif",
        "path": "/tmp/o.jpg",
        "count": map.len(),
        "exif": map,
        "include_gps": false,
        "engine": "kamadak-exif",
    });
    let v = super::project_fields(full, Some("tags,path,tag_count"));
    assert!(
        v.get("exif").is_some(),
        "expected canonical exif key, got {v}"
    );
    assert_eq!(v["count"], map.len());
    assert!(v.get("tags").is_none());
    assert!(v.get("tag_count").is_none());
    assert_eq!(v["path"], "/tmp/o.jpg");
    assert_eq!(v["action"], "exif");
}

#[test]
fn io_open_err_permission_has_suggestion() {
    let p = std::path::Path::new("/tmp/no-such-image-ro.png");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = super::magic::io_open_err(p, &e);
    assert!(err.suggestion().is_some(), "permission open must suggest");
    assert!(err.message().contains("image open"), "{}", err.message());
}

#[test]
fn io_open_err_not_found_message_contains_open() {
    let p = std::path::Path::new("/tmp/missing-image.png");
    let e = std::io::Error::new(std::io::ErrorKind::NotFound, "No such file");
    let err = super::magic::io_open_err(p, &e);
    assert!(err.message().contains("image open"), "{}", err.message());
}

#[test]
fn io_path_err_rename_permission_has_suggestion_and_op() {
    let p = std::path::Path::new("/tmp/no-rename-image.png");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = super::magic::io_path_err(p, "rename", &e);
    assert!(err.suggestion().is_some(), "permission rename must suggest");
    assert!(err.message().contains("image rename"), "{}", err.message());
}
