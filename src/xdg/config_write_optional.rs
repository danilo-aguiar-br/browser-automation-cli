// SPDX-License-Identifier: MIT OR Apache-2.0
//! Optional-key emission for the XDG `config.toml` writer (SRP split from
//! `config_write`).
//!
//! # Why this is its own module
//!
//! `write_config` builds the file from one large `format!` naming every key with
//! a constant default. The keys here are the other shape: `Option` fields that
//! must be emitted ONLY when set, so an unset key stays on its default instead
//! of being frozen into the file at whatever value it happened to hold.
//!
//! The two shapes were interleaved in one function until 2026-08-10, when adding
//! six missing keys pushed `config_write.rs` past the 300-line gate. Splitting on
//! that boundary is not a line-count dodge: "render the template" and "append the
//! keys the template cannot express" are different jobs, and only the second one
//! grows every time a key is added.
//!
//! `scripts/config-roundtrip-check.sh` scans BOTH files, so a key emitted here
//! satisfies the writer half of the round-trip invariant.

use super::config_model::ProductConfig;

/// Append every `Option` key that is set, one `key = value` line each.
///
/// Strings are quoted; booleans and numbers are bare, which is what the reader
/// in `config_io::apply_toml_kv` parses back.
pub(super) fn append_optional_keys(body: &mut String, cfg: &ProductConfig) {
    // Keys the template above never listed. Measured 2026-08-09.
    //
    // `write_config` rebuilds the whole file from a template that names every
    // key one by one, so a key absent from that template is DROPPED on every
    // write. Seventeen were absent, `browser_mode`, `stealth` and
    // `stealth_profile` among them: `config set` answered ok, the value never
    // reached the disk, and `config get` read back null. Worse, setting ANY
    // key rewrote the file and wiped all seventeen at once.
    //
    // Emitted only when set, so an unset key stays on its constant default
    // rather than being frozen into the file at the value it happens to have.
    if let Some(v) = cfg.browser_mode.as_deref() {
        body.push_str(&format!("browser_mode = \"{v}\"\n"));
    }
    if let Some(v) = cfg.stealth_profile.as_deref() {
        body.push_str(&format!("stealth_profile = \"{v}\"\n"));
    }
    if let Some(v) = cfg.stealth {
        body.push_str(&format!("stealth = {v}\n"));
    }
    if let Some(v) = cfg.cdp_proxy_bypass_loopback {
        body.push_str(&format!("cdp_proxy_bypass_loopback = {v}\n"));
    }
    if let Some(v) = cfg.http2_enabled {
        body.push_str(&format!("http2_enabled = {v}\n"));
    }
    if let Some(v) = cfg.http2_adaptive_window {
        body.push_str(&format!("http2_adaptive_window = {v}\n"));
    }
    if let Some(v) = cfg.http2_initial_stream_window_size {
        body.push_str(&format!("http2_initial_stream_window_size = {v}\n"));
    }
    if let Some(v) = cfg.http2_initial_connection_window_size {
        body.push_str(&format!("http2_initial_connection_window_size = {v}\n"));
    }
    if let Some(v) = cfg.http2_max_header_list_size {
        body.push_str(&format!("http2_max_header_list_size = {v}\n"));
    }
    if let Some(v) = cfg.http2_max_frame_size {
        body.push_str(&format!("http2_max_frame_size = {v}\n"));
    }
    if let Some(v) = cfg.image_avif_speed {
        body.push_str(&format!("image_avif_speed = {v}\n"));
    }
    if let Some(v) = cfg.svg_max_bytes {
        body.push_str(&format!("svg_max_bytes = {v}\n"));
    }
    if let Some(v) = cfg.svg_max_depth {
        body.push_str(&format!("svg_max_depth = {v}\n"));
    }
    if let Some(v) = cfg.svg_max_entities {
        body.push_str(&format!("svg_max_entities = {v}\n"));
    }
    if let Some(v) = cfg.gif_max_frames {
        body.push_str(&format!("gif_max_frames = {v}\n"));
    }
    if let Some(v) = cfg.manifest_max_bytes {
        body.push_str(&format!("manifest_max_bytes = {v}\n"));
    }
    if let Some(v) = cfg.manifest_max_variants {
        body.push_str(&format!("manifest_max_variants = {v}\n"));
    }
    // Six more the 2026-08-09 sweep missed, measured 2026-08-10.
    //
    // Fixing the seventeen above closed the instances, not the class: the
    // template is still hand-written, so `CONFIG_KEYS` and this function drift
    // apart silently. These six are the drift that survived.
    //
    // `proxy_username` and `proxy_password` are the costly pair. Both this file
    // and `config_ops::key_entries` tell the operator to keep proxy credentials
    // here BECAUSE argv is visible in the process table — and the channel named
    // as the safe one discarded them, leaving `--proxy` as the only one that
    // worked. `scripts/config-roundtrip-check.sh` now fails the build when a
    // declared key is missing from either side, so the class cannot come back.
    if let Some(v) = cfg.proxy_url.as_deref() {
        body.push_str(&format!("proxy_url = \"{v}\"\n"));
    }
    if let Some(v) = cfg.proxy_bypass.as_deref() {
        body.push_str(&format!("proxy_bypass = \"{v}\"\n"));
    }
    if let Some(v) = cfg.proxy_username.as_deref() {
        body.push_str(&format!("proxy_username = \"{v}\"\n"));
    }
    if let Some(v) = cfg.proxy_password.as_deref() {
        body.push_str(&format!("proxy_password = \"{v}\"\n"));
    }
    if let Some(v) = cfg.stealth_seed.as_deref() {
        body.push_str(&format!("stealth_seed = \"{v}\"\n"));
    }
    if let Some(v) = cfg.robots_user_agent.as_deref() {
        body.push_str(&format!("robots_user_agent = \"{v}\"\n"));
    }
}
