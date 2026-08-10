// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

#[test]
fn project_dirs_resolve() {
    assert!(project_dirs().is_ok());
    assert!(config_dir().unwrap().components().count() > 1);
}

#[test]
fn config_keys_catalog_non_empty() {
    assert!(config_ops::CONFIG_KEYS.len() >= 25);
    assert!(config_ops::CONFIG_KEYS.contains(&"screencast_jpeg_quality"));
    assert!(config_ops::CONFIG_KEYS.contains(&"interact_settle_ms"));
    assert!(config_ops::CONFIG_KEYS.contains(&"max_log_files"));
    assert!(config_ops::CONFIG_KEYS.contains(&"log_rotation"));
    assert!(config_ops::CONFIG_KEYS.contains(&"log_level"));
    assert!(config_ops::CONFIG_KEYS.contains(&"log_to_file"));
    assert!(config_ops::CONFIG_KEYS.contains(&"lighthouse_timeout_secs"));
    assert!(config_ops::CONFIG_KEYS.contains(&"ffmpeg_timeout_secs"));
}

#[test]
fn log_dir_under_state() {
    let log = log_dir().expect("log_dir");
    let state = state_dir().expect("state_dir");
    assert!(log.starts_with(&state));
    assert!(log.ends_with("log"));
}

#[test]
fn apply_toml_kv_parses_log_knobs() {
    let mut cfg = ProductConfig::default();
    config_io::apply_toml_kv(&mut cfg, "max_log_files", "21").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "log_rotation", "hourly").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "log_to_file", "true").expect("valid kv");
    assert_eq!(cfg.max_log_files, Some(21));
    assert_eq!(cfg.log_rotation.as_deref(), Some("hourly"));
    assert_eq!(cfg.log_to_file, Some(true));
}

#[test]
fn apply_toml_kv_parses_audio_keys() {
    let mut cfg = ProductConfig::default();
    config_io::apply_toml_kv(&mut cfg, "audio_max_input_bytes", "1000").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "audio_download_max_bytes", "2000").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "audio_default_format", "flac").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "audio_default_bitrate", "128k").expect("valid kv");
    assert_eq!(cfg.audio_max_input_bytes, Some(1000));
    assert_eq!(cfg.audio_download_max_bytes, Some(2000));
    assert_eq!(cfg.audio_default_format.as_deref(), Some("flac"));
    assert_eq!(cfg.audio_default_bitrate.as_deref(), Some("128k"));
}

#[test]
fn config_keys_include_audio_family() {
    for k in [
        "audio_max_input_bytes",
        "audio_download_max_bytes",
        "audio_default_format",
        "audio_default_bitrate",
    ] {
        assert!(
            config_ops::CONFIG_KEYS.contains(&k),
            "CONFIG_KEYS missing {k}"
        );
    }
}

#[test]
fn list_keys_catalog_includes_audio_family() {
    let v = config_ops::config_list_keys().expect("list-keys");
    let keys = v["keys"].as_array().expect("keys array");
    let names: Vec<&str> = keys
        .iter()
        .filter_map(|e| e.get("key").and_then(|k| k.as_str()))
        .collect();
    for k in [
        "audio_max_input_bytes",
        "audio_download_max_bytes",
        "audio_default_format",
        "audio_default_bitrate",
    ] {
        assert!(names.contains(&k), "list-keys catalog missing {k}");
    }
}

#[test]
fn apply_toml_kv_media_max_zero_is_none() {
    let mut cfg = ProductConfig::default();
    config_io::apply_toml_kv(&mut cfg, "audio_max_input_bytes", "0").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "audio_download_max_bytes", "0").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "image_max_input_bytes", "0").expect("valid kv");
    config_io::apply_toml_kv(&mut cfg, "video_max_input_bytes", "0").expect("valid kv");
    assert_eq!(cfg.audio_max_input_bytes, None);
    assert_eq!(cfg.audio_download_max_bytes, None);
    assert_eq!(cfg.image_max_input_bytes, None);
    assert_eq!(cfg.video_max_input_bytes, None);
}

#[test]
fn full_dump_omits_json_null_values() {
    let v = config_ops::config_get(None).expect("full get");
    // config_get(None) returns full dump object (may be wrapped by caller;
    // unit path uses same full_dump via empty key).
    let data = if v.get("lang").is_some() || v.get("timeout").is_some() {
        &v
    } else {
        v.get("data").unwrap_or(&v)
    };
    let obj = data.as_object().expect("object dump");
    for (k, val) in obj {
        assert!(
            !val.is_null(),
            "full_dump must omit null keys (found null at {k})"
        );
    }
}
