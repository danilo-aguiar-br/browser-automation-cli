// SPDX-License-Identifier: MIT OR Apache-2.0
//! HLS and DASH manifest parsing gates (GAP-VID-WAVE-C).
//!
//! # Why this file is library-level
//!
//! These tests pin `video_local::parse_manifest` directly, without spawning a
//! process, because the parser's contract is worth stating on its own terms:
//! ladders are enumerated, media playlists are summarised rather than dumped,
//! relative URIs absolutise against the manifest URL, and nothing is fetched.
//!
//! For a long stretch the parser had no caller — the doc here used to say so,
//! and named `src/commands/local_video.rs` as the file that would change. It
//! did: `video manifest` now routes a body into this parser. The end-to-end
//! surface is covered by `tests/video_manifest_cli_gate.rs`; this file stays
//! library-level so a parser regression is reported as a parser regression.
//!
//! `video info` still rejects an `.m3u8` or `.mpd` on magic bytes. That is
//! correct for a container probe and useless for a playlist, which is exactly
//! why `manifest` is a separate action rather than a branch inside `info`.
//!
//! Every fixture is inline text. Nothing is downloaded — which is also the
//! property [`parse_no_download_is_structural`] exists to state.

#![cfg(feature = "media-manifest")]

use browser_automation_cli::video_local::{
    detect_manifest_kind, manifest_available, parse_manifest, ManifestKind,
};

const MASTER_M3U8: &str = r#"#EXTM3U
#EXT-X-VERSION:4
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aud",NAME="English",LANGUAGE="en",DEFAULT=YES,URI="audio/en.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,CODECS="avc1.4d401e,mp4a.40.2",FRAME-RATE=25.000
v360/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2400000,RESOLUTION=1280x720,CODECS="avc1.4d401f,mp4a.40.2",FRAME-RATE=25.000
v720/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1920x1080,CODECS="avc1.640028,mp4a.40.2",FRAME-RATE=25.000
v1080/index.m3u8
"#;

const MEDIA_M3U8: &str = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:0
#EXTINF:6.000,
seg0.ts
#EXTINF:6.000,
seg1.ts
#EXTINF:4.500,
seg2.ts
#EXT-X-ENDLIST
"#;

const MPD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" minBufferTime="PT2S" mediaPresentationDuration="PT30S">
  <Period id="p0">
    <AdaptationSet mimeType="video/mp4" segmentAlignment="true">
      <Representation id="v360" bandwidth="800000" width="640" height="360" codecs="avc1.4d401e" frameRate="25"><BaseURL>v360/</BaseURL></Representation>
      <Representation id="v720" bandwidth="2400000" width="1280" height="720" codecs="avc1.4d401f" frameRate="25"><BaseURL>v720/</BaseURL></Representation>
    </AdaptationSet>
    <AdaptationSet mimeType="audio/mp4">
      <Representation id="a1" bandwidth="128000" codecs="mp4a.40.2"><BaseURL>aud/</BaseURL></Representation>
    </AdaptationSet>
  </Period>
</MPD>
"#;

#[test]
fn manifest_parsing_is_available_under_its_feature() {
    assert!(manifest_available());
}

#[test]
fn detect_kind_separates_master_media_and_dash() {
    assert_eq!(
        detect_manifest_kind(MASTER_M3U8.as_bytes()),
        Some(ManifestKind::HlsMaster)
    );
    assert_eq!(
        detect_manifest_kind(MEDIA_M3U8.as_bytes()),
        Some(ManifestKind::HlsMedia)
    );
    assert_eq!(
        detect_manifest_kind(MPD.as_bytes()),
        Some(ManifestKind::Dash)
    );
    assert_eq!(detect_manifest_kind(b"not a manifest at all"), None);
}

#[test]
fn hls_master_lists_every_variant_with_its_ladder_metadata() {
    let v = parse_manifest(MASTER_M3U8.as_bytes(), None).expect("parse master");
    assert_eq!(v["kind"], "hls_master");
    assert_eq!(v["engine"], "m3u8-rs");
    assert_eq!(v["variant_count"], 3);
    assert_eq!(v["variants_truncated"], false);

    let variants = v["variants"].as_array().expect("variants array");
    assert_eq!(variants.len(), 3);
    assert_eq!(variants[0]["bandwidth"], 800_000);
    assert_eq!(variants[0]["resolution"], "640x360");
    assert_eq!(variants[2]["resolution"], "1920x1080");
    // Selection is the whole point of a master playlist: the ladder must be
    // ordered as declared so "pick the highest bitrate" is a stable operation.
    let ladder: Vec<u64> = variants
        .iter()
        .map(|x| x["bandwidth"].as_u64().unwrap())
        .collect();
    assert_eq!(ladder, vec![800_000, 2_400_000, 6_000_000]);

    let alts = v["alternatives"].as_array().expect("alternatives array");
    assert_eq!(alts.len(), 1);
    assert_eq!(alts[0]["language"], "en");
    assert_eq!(alts[0]["default"], true);
}

#[test]
fn hls_variant_uris_are_absolutised_against_the_manifest_url() {
    let base = "https://cdn.example.test/live/master.m3u8";
    let v = parse_manifest(MASTER_M3U8.as_bytes(), Some(base)).expect("parse master");
    assert_eq!(
        v["variants"][0]["uri"], "https://cdn.example.test/live/v360/index.m3u8",
        "a relative variant URI is not actionable without the base"
    );
    assert_eq!(
        v["alternatives"][0]["uri"],
        "https://cdn.example.test/live/audio/en.m3u8"
    );
}

#[test]
fn hls_media_playlist_summarises_instead_of_dumping_segment_uris() {
    let v = parse_manifest(MEDIA_M3U8.as_bytes(), None).expect("parse media");
    assert_eq!(v["kind"], "hls_media");
    assert_eq!(v["segment_count"], 3);
    assert_eq!(v["end_list"], true);
    assert_eq!(v["target_duration"], 6);
    let total = v["total_duration_secs"].as_f64().expect("duration");
    assert!((total - 16.5).abs() < 0.01, "summed duration was {total}");
    // Clean stdout: a long VOD playlist must not flood the envelope.
    assert!(
        v.get("segments").is_none(),
        "media playlists must not emit a full segment list: {v}"
    );
    assert_eq!(v["first_segment_uri"], "seg0.ts");
}

#[test]
fn dash_lists_representations_across_adaptation_sets() {
    let v = parse_manifest(MPD.as_bytes(), None).expect("parse mpd");
    assert_eq!(v["kind"], "dash");
    assert_eq!(v["engine"], "dash-mpd");
    assert_eq!(v["period_count"], 1);
    assert_eq!(v["representation_count"], 3, "2 video + 1 audio");

    let reps = v["representations"].as_array().expect("representations");
    assert_eq!(reps.len(), 3);
    assert_eq!(reps[0]["id"], "v360");
    assert_eq!(reps[0]["width"], 640);
    assert_eq!(reps[0]["adaptation_set"], 0);
    assert_eq!(reps[1]["height"], 720);
    // The audio set is a different adaptation set, and must be reported as such
    // so a caller can tell renditions apart from alternate tracks.
    assert_eq!(reps[2]["adaptation_set"], 1);
    assert_eq!(reps[2]["mime_type"], "audio/mp4");
}

#[test]
fn dash_base_urls_are_absolutised_against_the_manifest_url() {
    let v = parse_manifest(
        MPD.as_bytes(),
        Some("https://cdn.example.test/v/stream.mpd"),
    )
    .expect("parse mpd");
    assert_eq!(
        v["representations"][0]["base_url"],
        "https://cdn.example.test/v/v360/"
    );
}

#[test]
fn a_body_that_is_neither_dialect_fails_closed() {
    let err =
        parse_manifest(b"<html><body>not a playlist</body></html>", None).expect_err("must reject");
    assert!(
        err.message().contains("HLS") || err.message().contains("DASH"),
        "message must name what was expected: {}",
        err.message()
    );
}

#[test]
fn parse_no_download_is_structural() {
    // `parse` takes bytes and an optional base URL — it has no I/O handle, so
    // "does not fetch the stream by default" is enforced by the signature
    // rather than by a flag a caller could flip. Parsing a manifest whose every
    // URI points at an unroutable host must therefore still succeed offline.
    let body = MASTER_M3U8.replace("v360/index.m3u8", "https://192.0.2.1/never-fetched.m3u8");
    let v = parse_manifest(body.as_bytes(), None).expect("parse must not touch the network");
    assert_eq!(
        v["variants"][0]["uri"],
        "https://192.0.2.1/never-fetched.m3u8"
    );
}

#[test]
fn an_oversized_manifest_is_refused_before_parsing() {
    // `manifest_max_bytes` is the ceiling; a body past it must not reach the
    // parser at all.
    let max = browser_automation_cli::xdg::resolve_manifest_max_bytes();
    let mut body = String::from("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\na.m3u8\n");
    body.push_str(&"#\n".repeat(max));
    let err = parse_manifest(body.as_bytes(), None).expect_err("must refuse oversized body");
    assert!(
        err.message().contains("manifest_max_bytes"),
        "message must name the ceiling knob: {}",
        err.message()
    );
}
