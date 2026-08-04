// SPDX-License-Identifier: MIT OR Apache-2.0

use super::magic::{detect_container, DetectedContainer};
use super::validate::{
    parse_output_container, resolve_effective_codecs, validate_codec_for_container, OutputContainer,
};

#[test]
fn magic_ftyp_mp4() {
    let mut b = vec![0u8; 12];
    b[4..8].copy_from_slice(b"ftyp");
    b[8..12].copy_from_slice(b"isom");
    assert_eq!(detect_container(&b).unwrap(), DetectedContainer::Mp4);
}

#[test]
fn magic_ftyp_qt() {
    let mut b = vec![0u8; 12];
    b[4..8].copy_from_slice(b"ftyp");
    b[8..12].copy_from_slice(b"qt  ");
    assert_eq!(detect_container(&b).unwrap(), DetectedContainer::Mov);
}

#[test]
fn magic_avi() {
    let mut b = vec![0u8; 12];
    b[0..4].copy_from_slice(b"RIFF");
    b[8..12].copy_from_slice(b"AVI ");
    assert_eq!(detect_container(&b).unwrap(), DetectedContainer::Avi);
}

#[test]
fn magic_ebml() {
    let b = [0x1A, 0x45, 0xDF, 0xA3, 0, 0, 0, 0];
    assert_eq!(
        detect_container(&b).unwrap(),
        DetectedContainer::MatroskaOrWebm
    );
}

#[test]
fn magic_mpeg_ps() {
    let b = [0x00, 0x00, 0x01, 0xBA, 0, 0, 0, 0];
    assert_eq!(detect_container(&b).unwrap(), DetectedContainer::MpegPs);
}

#[test]
fn magic_asf() {
    let b = [0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0, 0, 0, 0];
    assert_eq!(detect_container(&b).unwrap(), DetectedContainer::Asf);
}

#[test]
fn magic_rejects_short() {
    assert!(detect_container(&[0, 1, 2]).is_err());
}

#[test]
fn webm_rejects_h264() {
    let err =
        validate_codec_for_container(OutputContainer::Webm, "h264", Some("opus")).unwrap_err();
    assert!(err.message().contains("incompatible") || err.message().contains("WebM"));
}

#[test]
fn webm_accepts_vp9_opus() {
    validate_codec_for_container(OutputContainer::Webm, "vp9", Some("opus")).unwrap();
}

#[test]
fn parse_format() {
    assert_eq!(parse_output_container("MP4").unwrap(), OutputContainer::Mp4);
    assert!(parse_output_container("flv").is_err());
}

#[test]
fn project_fields_keeps_action() {
    let v = serde_json::json!({"action":"info","a":1,"b":2});
    let p = crate::json_util::project_fields_plain(v, Some("a"));
    assert_eq!(p["action"], "info");
    assert_eq!(p["a"], 1);
    assert!(p.get("b").is_none());
}

#[test]
fn project_fields_aliases_info_format_bytes() {
    let v = serde_json::json!({
        "action": "info",
        "container": "mp4",
        "size_bytes": 42,
        "duration_secs": 1.5,
        "path": "/tmp/x.mp4",
        "extra": true
    });
    let p = crate::video_local::project_fields(v, Some("format,bytes,path"));
    assert_eq!(p["action"], "info");
    assert_eq!(p["container"], "mp4");
    assert_eq!(p["size_bytes"], 42);
    assert_eq!(p["path"], "/tmp/x.mp4");
    assert!(p.get("extra").is_none());
    assert!(p.get("format").is_none());
    assert!(p.get("bytes").is_none());
}

#[test]
fn project_fields_aliases_convert_bytes_format() {
    let v = serde_json::json!({
        "action": "convert",
        "container_out": "webm",
        "bytes_out": 99,
        "path_out": "/tmp/o.webm",
        "auto_reencoded": true
    });
    let p = crate::video_local::project_fields(v, Some("format,bytes,path"));
    assert_eq!(p["container_out"], "webm");
    assert_eq!(p["bytes_out"], 99);
    assert_eq!(p["path_out"], "/tmp/o.webm");
}

#[test]
fn run_unknown_cmd_suggestion_excludes_video() {
    let s = crate::commands::run::run_unknown_cmd_suggestion("video");
    assert!(s.contains("intentionally excluded"), "{s}");
    assert!(s.contains("path-light video"), "{s}");
    assert!(s.contains("Supported:"), "{s}");
}

#[test]
fn smart_copy_webm_from_h264_auto_reencodes() {
    let plan = resolve_effective_codecs(
        OutputContainer::Webm,
        "copy",
        "copy",
        false,
        Some("h264"),
        Some("aac"),
        true,
    )
    .unwrap();
    assert!(!plan.stream_copy);
    assert!(plan.auto_reencoded);
    assert_eq!(
        plan.reencode_reason,
        Some("copy_incompatible_with_container")
    );
    assert!(plan.video_ffmpeg.contains("vpx") || plan.video_ffmpeg == "libvpx-vp9");
    assert!(plan.audio_ffmpeg.contains("opus"));
}

#[test]
fn smart_copy_mkv_from_h264_stays_copy() {
    let plan = resolve_effective_codecs(
        OutputContainer::Mkv,
        "copy",
        "copy",
        false,
        Some("h264"),
        Some("aac"),
        true,
    )
    .unwrap();
    assert!(plan.stream_copy);
    assert!(!plan.auto_reencoded);
    assert_eq!(plan.video_ffmpeg, "copy");
}

#[test]
fn smart_copy_webm_without_probe_reencodes() {
    let plan = resolve_effective_codecs(
        OutputContainer::Webm,
        "copy",
        "copy",
        false,
        None,
        None,
        false,
    )
    .unwrap();
    assert!(plan.auto_reencoded);
    assert_eq!(plan.reencode_reason, Some("ffprobe_unavailable"));
}

#[test]
fn ssrf_loopback_rejected_on_download() {
    // Policy is enforced before network I/O; block_on not required for URL check path.
    let err = crate::net::assert_safe_http_url("http://127.0.0.1/x.mp4").unwrap_err();
    let msg = err.message().to_ascii_lowercase();
    assert!(msg.contains("ssrf") || msg.contains("blocked") || msg.contains("127.0.0.1"));
}

#[test]
fn io_open_err_permission_has_suggestion() {
    let p = std::path::Path::new("/tmp/no-such-video-ro.mp4");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = crate::video_local::magic::io_open_err(p, &e);
    assert!(err.suggestion().is_some(), "permission open must suggest");
    assert!(err.message().contains("video open"));
}

#[test]
fn io_open_err_not_found_no_permission_suggestion_required() {
    let p = std::path::Path::new("/tmp/missing-video.mp4");
    let e = std::io::Error::new(std::io::ErrorKind::NotFound, "No such file");
    let err = crate::video_local::magic::io_open_err(p, &e);
    // not found: message only; suggestion optional
    assert!(err.message().contains("video open"));
}

#[test]
fn io_path_err_stat_permission_has_suggestion_and_op() {
    let p = std::path::Path::new("/tmp/no-stat-video.mp4");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = crate::video_local::magic::io_path_err(p, "stat", &e);
    assert!(err.suggestion().is_some(), "permission stat must suggest");
    assert!(err.message().contains("video stat"), "{}", err.message());
}

#[test]
fn io_path_err_mkdir_permission_has_suggestion_and_op() {
    let p = std::path::Path::new("/tmp/no-mkdir-video");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = crate::video_local::magic::io_path_err(p, "mkdir", &e);
    assert!(err.suggestion().is_some(), "permission mkdir must suggest");
    assert!(err.message().contains("video mkdir"), "{}", err.message());
}

#[test]
fn io_path_err_rename_permission_has_suggestion_and_op() {
    let p = std::path::Path::new("/tmp/no-rename-video.mp4");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = crate::video_local::magic::io_path_err(p, "rename", &e);
    assert!(err.suggestion().is_some(), "permission rename must suggest");
    assert!(err.message().contains("video rename"), "{}", err.message());
}
