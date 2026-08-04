// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for audio_local (magic, plan, select, io_path).

use super::magic::{detect_container, DetectedAudio};
use super::validate::{is_lossy_codec, parse_output_format, resolve_audio_plan, OutputFormat};
use serde_json::json;

#[test]
fn magic_wav_ogg_flac_mp3_id3() {
    assert_eq!(
        detect_container(b"RIFF\0\0\0\0WAVEfmt ").unwrap(),
        DetectedAudio::Wav
    );
    assert_eq!(
        detect_container(b"OggS\0\0\0\0").unwrap(),
        DetectedAudio::Ogg
    );
    assert_eq!(
        detect_container(b"fLaC\0\0\0\0").unwrap(),
        DetectedAudio::Flac
    );
    assert_eq!(
        detect_container(b"ID3\x03\0\0\0\0").unwrap(),
        DetectedAudio::Mp3
    );
}

#[test]
fn magic_rejects_short() {
    assert!(detect_container(b"ID").is_err());
}

#[test]
fn parse_formats() {
    assert_eq!(parse_output_format("mp3").unwrap(), OutputFormat::Mp3);
    assert_eq!(parse_output_format("m4a").unwrap(), OutputFormat::M4a);
    assert_eq!(parse_output_format("opus").unwrap(), OutputFormat::Opus);
    assert!(parse_output_format("xyz").is_err());
}

#[test]
fn lossy_suggestion_key_resolves() {
    let s = crate::i18n::suggestion_key("audio_lossy_transcode", Some("en"));
    assert!(!s.is_empty());
    assert!(
        s.to_ascii_lowercase().contains("lossy") || s.contains("recompress"),
        "unexpected suggestion: {s}"
    );
}

#[test]
fn lossy_detect() {
    assert!(is_lossy_codec("mp3"));
    assert!(is_lossy_codec("aac"));
    assert!(!is_lossy_codec("flac"));
    assert!(!is_lossy_codec("pcm_s16le"));
}

#[test]
fn plan_copy_when_muxable() {
    let p = resolve_audio_plan(OutputFormat::Mp3, Some("copy"), Some("mp3"), true);
    assert!(p.stream_copy);
    assert!(!p.lossy_transcode);
}

#[test]
fn plan_auto_reencode_lossy() {
    let p = resolve_audio_plan(OutputFormat::Mp3, Some("copy"), Some("aac"), true);
    assert!(p.auto_reencoded);
    assert!(p.lossy_transcode);
    assert!(!p.stream_copy);
}

#[test]
fn project_fields_aliases() {
    let v = json!({
        "action": "info",
        "container": "wav",
        "size_bytes": 12,
        "path": "/tmp/a.wav",
        "duration_secs": 1.5,
        "audio_codec": "pcm_s16le",
    });
    let p = super::project_fields(v, Some("format,bytes,path,duration,codec"));
    let o = p.as_object().unwrap();
    assert!(o.contains_key("container"));
    assert!(o.contains_key("size_bytes"));
    assert!(o.contains_key("path"));
    assert!(o.contains_key("duration_secs"));
    assert!(o.contains_key("audio_codec"));
    assert!(o.contains_key("action"));
}

#[test]
fn io_open_err_permission_suggestion() {
    let p = std::path::Path::new("/no/such/audio.bin");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let err = super::magic::io_open_err(p, &e);
    assert!(err.suggestion().is_some());
}

#[test]
fn io_path_err_rename_permission() {
    let p = std::path::Path::new("/no/such/out.mp3");
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let err = super::magic::io_path_err(p, "rename", &e);
    assert!(err.suggestion().is_some());
}
