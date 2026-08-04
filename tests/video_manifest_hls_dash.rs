// SPDX-License-Identifier: MIT OR Apache-2.0
//! HLS and DASH manifest parsing (GAP-VID-WAVE-C).
//!
//! Parse-only: these tests assert that variants are described and that no
//! segment is ever fetched. Manifest bodies are inline fixtures, so the suite
//! runs offline.

use browser_automation_cli::video_local;

const HLS_MASTER: &str = r#"#EXTM3U
#EXT-X-VERSION:4
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aac",NAME="English",LANGUAGE="en",DEFAULT=YES,URI="audio/en.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,CODECS="avc1.4d401e,mp4a.40.2"
low/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2400000,RESOLUTION=1280x720,CODECS="avc1.4d401f,mp4a.40.2"
mid/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=5600000,RESOLUTION=1920x1080,CODECS="avc1.640028,mp4a.40.2"
high/index.m3u8
"#;

const HLS_MEDIA: &str = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-PLAYLIST-TYPE:VOD
#EXTINF:6.0,
seg0.ts
#EXTINF:6.0,
seg1.ts
#EXTINF:4.5,
seg2.ts
#EXT-X-ENDLIST
"#;

const DASH_MPD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" minBufferTime="PT2S" mediaPresentationDuration="PT30S">
  <Period id="p0">
    <AdaptationSet mimeType="video/mp4" segmentAlignment="true">
      <Representation id="v0" bandwidth="900000" width="640" height="360" codecs="avc1.4d401e">
        <BaseURL>video/360.mp4</BaseURL>
      </Representation>
      <Representation id="v1" bandwidth="2500000" width="1280" height="720" codecs="avc1.4d401f">
        <BaseURL>video/720.mp4</BaseURL>
      </Representation>
    </AdaptationSet>
    <AdaptationSet mimeType="audio/mp4">
      <Representation id="a0" bandwidth="128000" codecs="mp4a.40.2">
        <BaseURL>audio/128.mp4</BaseURL>
      </Representation>
    </AdaptationSet>
  </Period>
</MPD>
"#;

// ── Dialect detection (works without the feature) ──────────────────────────

#[test]
fn detects_hls_master_by_stream_inf() {
    assert_eq!(
        video_local::detect_manifest_kind(HLS_MASTER.as_bytes()),
        Some(video_local::ManifestKind::HlsMaster)
    );
}

#[test]
fn detects_hls_media_playlist() {
    assert_eq!(
        video_local::detect_manifest_kind(HLS_MEDIA.as_bytes()),
        Some(video_local::ManifestKind::HlsMedia)
    );
}

#[test]
fn detects_dash_mpd() {
    assert_eq!(
        video_local::detect_manifest_kind(DASH_MPD.as_bytes()),
        Some(video_local::ManifestKind::Dash)
    );
}

#[test]
fn rejects_a_body_that_is_neither_dialect() {
    assert_eq!(video_local::detect_manifest_kind(b"just some text\n"), None);
}

// ── HLS ────────────────────────────────────────────────────────────────────

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn hls_master_lists_every_variant_with_bandwidth_and_resolution() {
    let v = video_local::parse_manifest(HLS_MASTER.as_bytes(), None).expect("parse");
    assert_eq!(v["kind"], "hls_master");
    assert_eq!(v["variant_count"], 3);
    assert_eq!(v["variants_truncated"], false);

    let variants = v["variants"].as_array().unwrap();
    assert_eq!(variants[0]["bandwidth"], 800_000);
    assert_eq!(variants[2]["resolution"], "1920x1080");
    assert_eq!(variants[1]["uri"], "mid/index.m3u8");
}

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn hls_master_resolves_relative_uris_against_the_manifest_url() {
    let v = video_local::parse_manifest(
        HLS_MASTER.as_bytes(),
        Some("https://cdn.example.test/vod/master.m3u8"),
    )
    .expect("parse");
    assert_eq!(
        v["variants"][0]["uri"],
        "https://cdn.example.test/vod/low/index.m3u8"
    );
    assert_eq!(
        v["alternatives"][0]["uri"],
        "https://cdn.example.test/vod/audio/en.m3u8"
    );
}

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn hls_master_reports_alternative_audio_renditions() {
    let v = video_local::parse_manifest(HLS_MASTER.as_bytes(), None).unwrap();
    let alt = &v["alternatives"][0];
    assert_eq!(alt["group_id"], "aac");
    assert_eq!(alt["language"], "en");
    assert_eq!(alt["default"], true);
}

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn hls_media_summarises_segments_without_dumping_them() {
    let v = video_local::parse_manifest(HLS_MEDIA.as_bytes(), None).expect("parse");
    assert_eq!(v["kind"], "hls_media");
    assert_eq!(v["segment_count"], 3);
    assert_eq!(v["end_list"], true);
    assert_eq!(v["target_duration"], 6);
    let total = v["total_duration_secs"].as_f64().unwrap();
    assert!((total - 16.5).abs() < 0.01, "total duration {total}");
    // Clean stdout: the full segment list must never be inlined.
    assert!(
        v.get("segments").is_none(),
        "media playlist envelope must not carry every segment URI"
    );
    assert_eq!(v["first_segment_uri"], "seg0.ts");
}

// ── DASH ───────────────────────────────────────────────────────────────────

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn dash_lists_representations_across_adaptation_sets() {
    let v = video_local::parse_manifest(DASH_MPD.as_bytes(), None).expect("parse");
    assert_eq!(v["kind"], "dash");
    assert_eq!(v["period_count"], 1);
    assert_eq!(v["representation_count"], 3);

    let reps = v["representations"].as_array().unwrap();
    assert_eq!(reps[0]["id"], "v0");
    assert_eq!(reps[1]["height"], 720);
    // Audio-only representations inherit mimeType from their AdaptationSet.
    assert_eq!(reps[2]["mime_type"], "audio/mp4");
}

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn dash_resolves_relative_base_urls() {
    let v = video_local::parse_manifest(
        DASH_MPD.as_bytes(),
        Some("https://cdn.example.test/dash/manifest.mpd"),
    )
    .unwrap();
    assert_eq!(
        v["representations"][0]["base_url"],
        "https://cdn.example.test/dash/video/360.mp4"
    );
}

// ── Ceilings and fail-closed ───────────────────────────────────────────────

#[test]
#[cfg_attr(
    not(feature = "media-manifest"),
    ignore = "requires --features media-manifest"
)]
fn manifest_over_the_byte_ceiling_is_rejected() {
    // DEFAULT_MANIFEST_MAX_BYTES is 8 MB; build one just past it.
    let mut body = String::from("#EXTM3U\n#EXT-X-TARGETDURATION:6\n");
    while body.len() <= 8_000_000 {
        body.push_str("#EXTINF:6.0,\nseg.ts\n");
    }
    let err = video_local::parse_manifest(body.as_bytes(), None).unwrap_err();
    assert!(
        err.message().contains("manifest_max_bytes"),
        "error must name the knob, got: {}",
        err.message()
    );
}

#[test]
#[cfg_attr(feature = "media-manifest", ignore = "feature is on in this build")]
fn manifest_parsing_fails_closed_without_the_feature() {
    assert!(!video_local::manifest_available());
    let err = video_local::parse_manifest(HLS_MASTER.as_bytes(), None).unwrap_err();
    assert!(err.message().contains("media-manifest"));
}
