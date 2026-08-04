// SPDX-License-Identifier: MIT OR Apache-2.0
//! `video download` pointed at something that is not a media file.
//!
//! Two mistakes dominate in practice and need different answers: a site player
//! page (rejected by rule, permanently) and an HLS/DASH manifest (supported,
//! but by the manifest parser). A single "bad magic bytes" message conflates
//! them and leaves an agent retrying the same call.

use browser_automation_cli::video_local::{self, NonMediaBody};

const PLAYER_PAGE: &str = r#"<!DOCTYPE html>
<html><head><title>Watch — Example</title></head>
<body><div id="player" data-video-id="abc123"></div></body></html>"#;

const HLS_MASTER: &str = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nlow/index.m3u8\n";

const DASH_MPD: &str = r#"<?xml version="1.0"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static"><Period/></MPD>"#;

#[test]
fn html_player_page_is_classified_as_such() {
    assert_eq!(
        video_local::classify_non_media(PLAYER_PAGE.as_bytes()),
        Some(NonMediaBody::HtmlPage)
    );
}

#[test]
fn hls_manifest_is_not_mistaken_for_a_player_page() {
    assert_eq!(
        video_local::classify_non_media(HLS_MASTER.as_bytes()),
        Some(NonMediaBody::Manifest)
    );
}

#[test]
fn dash_mpd_is_classified_as_a_manifest_despite_being_xml() {
    // An MPD opens with `<?xml`, the same as an XHTML player page would.
    assert_eq!(
        video_local::classify_non_media(DASH_MPD.as_bytes()),
        Some(NonMediaBody::Manifest)
    );
}

#[test]
fn real_media_bytes_are_not_classified_as_non_media() {
    // Minimal ISO BMFF header.
    let mut mp4 = vec![0u8, 0, 0, 0x18];
    mp4.extend_from_slice(b"ftypisom");
    mp4.extend_from_slice(&[0u8; 16]);
    assert_eq!(video_local::classify_non_media(&mp4), None);
}

#[test]
fn arbitrary_binary_falls_through_to_the_generic_magic_error() {
    assert_eq!(
        video_local::classify_non_media(&[0xDE, 0xAD, 0xBE, 0xEF]),
        None
    );
}

#[test]
fn player_page_error_states_the_rule_and_does_not_promise_a_future_release() {
    let err =
        video_local::non_media_error(NonMediaBody::HtmlPage, "https://example.test/watch?v=1");
    let msg = err.message();
    assert!(
        msg.contains("rejected by rule"),
        "must cite the rule: {msg}"
    );
    assert!(
        msg.contains("access control"),
        "must state why the rule exists: {msg}"
    );
    // "not yet", "future", "planned" would tell an agent to retry later.
    let lower = msg.to_ascii_lowercase();
    for weasel in ["not yet", "future", "planned", "coming soon", "roadmap"] {
        assert!(
            !lower.contains(weasel),
            "a permanent boundary must not read as deferred ({weasel:?}): {msg}"
        );
    }
}

#[test]
fn player_page_error_echoes_the_offending_url() {
    let err =
        video_local::non_media_error(NonMediaBody::HtmlPage, "https://example.test/watch?v=1");
    assert!(err.message().contains("https://example.test/watch?v=1"));
}

#[test]
fn manifest_error_points_at_the_parser_rather_than_rejecting() {
    let err = video_local::non_media_error(NonMediaBody::Manifest, "https://cdn.test/master.m3u8");
    let msg = err.message();
    assert!(msg.contains("manifest"), "{msg}");
    assert!(
        !msg.contains("rejected by rule"),
        "manifests are supported; only site extraction is refused: {msg}"
    );
}
