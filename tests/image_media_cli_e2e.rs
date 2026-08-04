// SPDX-License-Identifier: MIT OR Apache-2.0
//! End-to-end gates that drive the **real binary** for the Wave-6 media
//! surfaces: AVIF encode, SVG raster + sanitiser, GIF multi-frame, the SIMD
//! resize backend, and IPTC/XMP metadata.
//!
//! `tests/image_wave6_codecs.rs` and `tests/image_metadata_iptc_xmp.rs` already
//! cover the same features at the library level. They cannot catch a surface
//! that is implemented but never reachable through `main`, which is the failure
//! mode this file exists to close: every assertion here spawns
//! `browser-automation-cli` and parses the JSON envelope it printed on stdout.
//!
//! Fixtures are built in-process. Nothing is downloaded and no binary blob is
//! committed, so the suite runs identically offline.

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;

/// Run the built binary and return its parsed stdout envelope plus exit code.
///
/// The envelope is the contract, so a non-zero exit is returned rather than
/// asserted away: several gates below exist precisely to pin a *failure*
/// envelope.
/// # Why `CARGO_BIN_EXE_` and not `Command::cargo_bin`
///
/// `cargo_bin` resolves to `target/debug/browser-automation-cli`, a path that
/// ANY build overwrites regardless of its feature set. A `cargo build` with
/// default features silently replaces the binary a later
/// `cargo test --all-features` is about to drive, and the feature-gated gates
/// below then fail with "requires the `image-avif` Cargo feature, which is off
/// in this build" — a true statement about the wrong binary.
///
/// `CARGO_BIN_EXE_<name>` is set by cargo at test-COMPILE time and points at the
/// binary built with the SAME feature set as this test, so the two can never
/// disagree. Same defect class as a schema gate comparing against a stale
/// artifact: the test was right, the thing it measured was not the thing it
/// meant to measure.
const CLI_BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn run(args: &[&str]) -> (Value, i32) {
    let out = Command::new(CLI_BIN)
        .args(args)
        .output()
        .expect("spawn cli");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not one JSON envelope ({e}): {stdout}"));
    (value, out.status.code().unwrap_or(-1))
}

/// Assert the envelope reports success and hand back its `data` object.
fn data(env: &Value) -> &Value {
    assert_eq!(env["ok"], true, "expected ok envelope, got {env}");
    &env["data"]
}

/// Assert the envelope reports failure and hand back its error message.
fn error_message(env: &Value) -> &str {
    assert_eq!(env["ok"], false, "expected error envelope, got {env}");
    env["error"]["message"]
        .as_str()
        .expect("error.message is a string")
}

/// Write a small RGB gradient PNG and return its path.
fn write_gradient_png(dir: &Path, name: &str, w: u32, h: u32) -> std::path::PathBuf {
    let mut img = image::RgbImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = image::Rgb([(x * 7 % 256) as u8, (y * 11 % 256) as u8, 0x40]);
    }
    let path = dir.join(name);
    img.save(&path).expect("write png fixture");
    path
}

/// Write an animated GIF with `frames` distinct frames.
fn write_animated_gif(dir: &Path, name: &str, frames: u32) -> std::path::PathBuf {
    use image::codecs::gif::GifEncoder;
    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("create gif fixture");
    {
        let mut enc = GifEncoder::new(file);
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
    path
}

/// Splice `segment` in directly after the JPEG SOI marker of `jpeg`.
fn jpeg_with_segment(jpeg: &[u8], segment: &[u8]) -> Vec<u8> {
    assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "fixture must start with SOI");
    let mut out = Vec::with_capacity(jpeg.len() + segment.len());
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(segment);
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// Build a baseline JPEG with no metadata.
fn bare_jpeg() -> Vec<u8> {
    let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        16,
        16,
        image::Rgb([0xC0, 0x30, 0x30]),
    ));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Jpeg)
        .expect("encode jpeg fixture");
    buf.into_inner()
}

/// Build an `APP1` segment carrying an XMP packet.
fn app1_xmp_segment(packet: &str) -> Vec<u8> {
    const SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
    let payload_len = 2 + SIG.len() + packet.len();
    assert!(payload_len <= usize::from(u16::MAX), "segment too large");
    let mut seg = vec![0xFF, 0xE1];
    seg.extend_from_slice(&(payload_len as u16).to_be_bytes());
    seg.extend_from_slice(SIG);
    seg.extend_from_slice(packet.as_bytes());
    seg
}

/// Build an `APP13` segment carrying one Photoshop `8BIM` IPTC-NAA block.
///
/// `datasets` are `(IIM dataset id, value)` pairs in IIM record 2 (application).
fn app13_iptc_segment(datasets: &[(u8, &str)]) -> Vec<u8> {
    let mut iim = Vec::new();
    for (id, value) in datasets {
        assert!(value.len() <= usize::from(u16::MAX), "dataset too large");
        iim.push(0x1C);
        iim.push(0x02);
        iim.push(*id);
        iim.extend_from_slice(&(value.len() as u16).to_be_bytes());
        iim.extend_from_slice(value.as_bytes());
    }
    if iim.len() % 2 == 1 {
        iim.push(0x00);
    }

    let mut block = Vec::new();
    block.extend_from_slice(b"8BIM");
    block.extend_from_slice(&0x0404u16.to_be_bytes());
    // Empty Pascal resource name, padded to an even length.
    block.extend_from_slice(&[0x00, 0x00]);
    block.extend_from_slice(&(iim.len() as u32).to_be_bytes());
    block.extend_from_slice(&iim);

    const SIG: &[u8] = b"Photoshop 3.0\0";
    let payload_len = 2 + SIG.len() + block.len();
    assert!(payload_len <= usize::from(u16::MAX), "segment too large");
    let mut seg = vec![0xFF, 0xED];
    seg.extend_from_slice(&(payload_len as u16).to_be_bytes());
    seg.extend_from_slice(SIG);
    seg.extend_from_slice(&block);
    seg
}

// ── GAP-IMG-090: AVIF through the CLI ──────────────────────────────────────

#[test]
#[cfg_attr(not(feature = "image-avif"), ignore = "requires --features image-avif")]
fn cli_convert_emits_a_real_avif_container() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_gradient_png(dir.path(), "src.png", 64, 48);
    let out = dir.path().join("out.avif");

    let (env, code) = run(&[
        "--json",
        "image",
        "convert",
        "--path",
        src.to_str().unwrap(),
        "--format",
        "avif",
        "--quality",
        "60",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "convert should succeed: {env}");
    let d = data(&env);
    assert_eq!(d["format"], "avif");
    assert_eq!(d["engine"], "ravif", "envelope must name the real encoder");
    assert_eq!(
        (d["width"].as_u64(), d["height"].as_u64()),
        (Some(64), Some(48))
    );
    assert_eq!(d["quality_applied"], true, "avif is a lossy target");

    // `ftyp` brand rather than a file that merely has the right extension.
    let bytes = std::fs::read(&out).expect("avif written");
    assert_eq!(
        &bytes[4..12],
        b"ftypavif",
        "not an AVIF brand: {:?}",
        &bytes[..16]
    );
}

#[test]
#[cfg_attr(not(feature = "image-avif"), ignore = "requires --features image-avif")]
fn cli_refuses_to_decode_the_avif_it_just_wrote_and_names_the_blocker() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_gradient_png(dir.path(), "src.png", 32, 32);
    let out = dir.path().join("out.avif");
    let (_, code) = run(&[
        "--json",
        "image",
        "convert",
        "--path",
        src.to_str().unwrap(),
        "--format",
        "avif",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);

    let (env, code) = run(&["--json", "image", "info", "--path", out.to_str().unwrap()]);
    assert_eq!(code, 65, "decode must fail closed: {env}");
    let msg = error_message(&env);
    // Encode-only is a toolchain fact, so the message has to say so rather than
    // leaving an agent to infer that encode support implies decode support.
    assert!(msg.contains("avif"), "message must name the format: {msg}");
    assert!(
        msg.contains("decoder") || msg.contains("decode is not"),
        "message must name the missing decoder: {msg}"
    );
}

#[test]
fn cli_rejects_heic_encode_and_names_the_missing_hevc_encoder() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_gradient_png(dir.path(), "src.png", 16, 16);
    let (env, code) = run(&[
        "--json",
        "image",
        "convert",
        "--path",
        src.to_str().unwrap(),
        "--format",
        "heic",
        "-o",
        dir.path().join("x.heic").to_str().unwrap(),
    ]);
    assert_eq!(code, 2, "heic encode is a usage error: {env}");
    assert!(
        error_message(&env).contains("HEVC"),
        "message must name the real blocker: {env}"
    );
}

// ── GAP-IMG-092: SVG through the CLI ───────────────────────────────────────

const SAFE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="48"><rect width="64" height="48" fill="#0a7"/></svg>"##;

#[test]
#[cfg_attr(not(feature = "image-svg"), ignore = "requires --features image-svg")]
fn cli_rasterises_svg_at_its_intrinsic_size() {
    let dir = tempfile::tempdir().unwrap();
    let svg = dir.path().join("safe.svg");
    std::fs::write(&svg, SAFE_SVG).unwrap();

    let (env, code) = run(&["--json", "image", "info", "--path", svg.to_str().unwrap()]);
    assert_eq!(code, 0, "svg info should succeed: {env}");
    let d = data(&env);
    assert_eq!(d["format"], "svg");
    assert_eq!(
        (d["width"].as_u64(), d["height"].as_u64()),
        (Some(64), Some(48))
    );
}

/// Each hostile document must be refused, and the message must name the vector
/// that tripped rather than a generic parse failure.
#[test]
#[cfg_attr(not(feature = "image-svg"), ignore = "requires --features image-svg")]
fn cli_sanitiser_refuses_every_hostile_svg_vector() {
    // Five nested entities: expanding them would be 10^5 copies of the leaf.
    let bomb = concat!(
        "<?xml version=\"1.0\"?>\n<!DOCTYPE svg [\n",
        "<!ENTITY a \"aaaaaaaaaa\">\n",
        "<!ENTITY b \"&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;\">\n",
        "<!ENTITY c \"&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;\">\n",
        "<!ENTITY d \"&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;\">\n",
        "<!ENTITY e \"&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;\">\n",
        "]>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20\" height=\"20\"><text>&e;</text></svg>"
    );
    let script = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><script>fetch("http://evil.test")</script></svg>"##;
    let xlink = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="20" height="20"><image xlink:href="http://evil.test/x.png" width="20" height="20"/></svg>"##;

    let dir = tempfile::tempdir().unwrap();
    for (name, body, needle) in [
        ("bomb.svg", bomb, "entit"),
        ("script.svg", script, "script"),
        ("xlink.svg", xlink, "href"),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        let (env, code) = run(&["--json", "image", "info", "--path", path.to_str().unwrap()]);
        assert_eq!(code, 65, "{name} must fail closed: {env}");
        let msg = error_message(&env);
        assert!(
            msg.contains(needle),
            "{name} message must name the vector ({needle}): {msg}"
        );
    }
}

// ── GAP-IMG-093: GIF multi-frame through the CLI ───────────────────────────

#[test]
fn cli_info_reports_the_real_gif_frame_count() {
    let dir = tempfile::tempdir().unwrap();
    let gif = write_animated_gif(dir.path(), "anim.gif", 3);

    let (env, code) = run(&["--json", "image", "info", "--path", gif.to_str().unwrap()]);
    assert_eq!(code, 0, "gif info should succeed: {env}");
    let d = data(&env);
    assert_eq!(d["format"], "gif");
    assert_eq!(d["frame_count"], 3, "envelope must not hard-code 1: {env}");
    assert_eq!(d["frame_count_exact"], true);
    assert_eq!(d["animated"], true);
}

#[test]
fn cli_info_reports_one_exact_frame_for_a_single_frame_gif() {
    let dir = tempfile::tempdir().unwrap();
    let gif = write_animated_gif(dir.path(), "one.gif", 1);
    let (env, _) = run(&["--json", "image", "info", "--path", gif.to_str().unwrap()]);
    let d = data(&env);
    assert_eq!(d["frame_count"], 1);
    assert_eq!(d["animated"], false);
}

// ── GAP-IMG-095: SIMD resize through the CLI ───────────────────────────────

#[test]
fn cli_resize_hits_exact_dimensions_and_names_its_backend() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_gradient_png(dir.path(), "big.png", 128, 96);
    let out = dir.path().join("small.png");

    let (env, code) = run(&[
        "--json",
        "image",
        "resize",
        "--path",
        src.to_str().unwrap(),
        "--width",
        "48",
        "--height",
        "36",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "resize should succeed: {env}");
    let d = data(&env);
    assert_eq!(
        (d["width"].as_u64(), d["height"].as_u64()),
        (Some(48), Some(36))
    );

    // The backend is compile-time, so the envelope must agree with the build
    // rather than always claiming the SIMD path.
    let expected = if cfg!(feature = "image-simd-resize") {
        "fast_image_resize"
    } else {
        "image::imageops"
    };
    assert_eq!(d["resize_backend"], expected);

    // Whichever backend ran, the output has to be a decodable image of the
    // requested size — geometry in the envelope is not proof of valid pixels.
    let (env, code) = run(&["--json", "image", "info", "--path", out.to_str().unwrap()]);
    assert_eq!(code, 0, "resized output must decode: {env}");
    let d = data(&env);
    assert_eq!(
        (d["width"].as_u64(), d["height"].as_u64()),
        (Some(48), Some(36))
    );
}

// ── GAP-IMG-097: IPTC and XMP through the CLI ──────────────────────────────

#[test]
fn cli_info_reads_iptc_from_a_hand_built_app13_block() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("iptc.jpg");
    let segment = app13_iptc_segment(&[
        (105, "E2E Headline"),
        (80, "E2E Byline"),
        (120, "E2E Caption"),
    ]);
    std::fs::write(&path, jpeg_with_segment(&bare_jpeg(), &segment)).unwrap();

    let (env, code) = run(&["--json", "image", "info", "--path", path.to_str().unwrap()]);
    assert_eq!(code, 0, "info should succeed: {env}");
    let d = data(&env);
    assert_eq!(d["format"], "jpeg");
    assert_eq!(d["iptc"]["Headline"], "E2E Headline");
    assert_eq!(d["iptc"]["Byline"], "E2E Byline");
    assert_eq!(d["iptc"]["Caption"], "E2E Caption");
}

#[test]
fn cli_info_reads_xmp_from_a_hand_built_app1_segment() {
    let packet = concat!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>",
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">",
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">",
        "<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">",
        "<dc:title>E2E XMP Title</dc:title>",
        "<dc:creator>E2E Creator</dc:creator>",
        "</rdf:Description></rdf:RDF></x:xmpmeta>",
        "<?xpacket end=\"w\"?>"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("xmp.jpg");
    std::fs::write(
        &path,
        jpeg_with_segment(&bare_jpeg(), &app1_xmp_segment(packet)),
    )
    .unwrap();

    let (env, code) = run(&["--json", "image", "info", "--path", path.to_str().unwrap()]);
    assert_eq!(code, 0, "info should succeed: {env}");
    let d = data(&env);
    assert_eq!(d["xmp"]["dc:title"], "E2E XMP Title");
    assert_eq!(d["xmp"]["dc:creator"], "E2E Creator");
}

#[test]
fn cli_select_projects_the_metadata_fields_the_help_advertises() {
    // `--select` is the anti-token lever, so the metadata keys have to be
    // projectable — otherwise an agent must pull the whole envelope to see them.
    let dir = tempfile::tempdir().unwrap();
    let gif = write_animated_gif(dir.path(), "anim.gif", 3);
    let (env, code) = run(&[
        "--json",
        "image",
        "info",
        "--path",
        gif.to_str().unwrap(),
        "--select",
        "format,frame_count,frame_count_exact,animated,iptc,xmp",
    ]);
    assert_eq!(code, 0, "select should succeed: {env}");
    let d = data(&env);
    assert_eq!(d["frame_count"], 3);
    assert!(d.get("iptc").is_some(), "iptc must be projectable: {env}");
    assert!(d.get("xmp").is_some(), "xmp must be projectable: {env}");
    assert!(
        d.get("sha256").is_none(),
        "projection must drop unrequested fields: {env}"
    );
}
