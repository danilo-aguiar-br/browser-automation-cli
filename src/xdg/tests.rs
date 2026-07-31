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
    config_io::apply_toml_kv(&mut cfg, "max_log_files", "21");
    config_io::apply_toml_kv(&mut cfg, "log_rotation", "hourly");
    config_io::apply_toml_kv(&mut cfg, "log_to_file", "true");
    assert_eq!(cfg.max_log_files, Some(21));
    assert_eq!(cfg.log_rotation.as_deref(), Some("hourly"));
    assert_eq!(cfg.log_to_file, Some(true));
}
