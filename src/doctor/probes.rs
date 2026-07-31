// SPDX-License-Identifier: MIT OR Apache-2.0
//! Individual host probes used by `run_doctor`.
//!
//! Each probe answers one question about the host and holds no shared state,
//! so `run_doctor` reads as the checklist it is.

use super::*;

/// Resolve lighthouse for doctor: XDG path if executable, else PATH.
pub(super) fn resolve_lighthouse_for_doctor() -> Option<(String, &'static str)> {
    if let Some(xdg) = crate::xdg::lighthouse_path_from_config().filter(|s| !s.is_empty()) {
        let p = std::path::Path::new(&xdg);
        if p.is_file() {
            let source = if xdg.contains("mock-lighthouse") {
                "mock"
            } else {
                "xdg"
            };
            return Some((xdg, source));
        }
    }
    which_bin("lighthouse").map(|p| (p, "path"))
}

/// Report redis cache health from XDG only (no product env).
pub(super) fn cache_redis_check() -> serde_json::Value {
    let cfg = crate::xdg::load_config().unwrap_or_default();
    let backend = cfg
        .cache_backend
        .as_deref()
        .unwrap_or("sqlite")
        .to_ascii_lowercase();
    if backend != "redis" {
        return json!({
            "id": "cache_redis",
            "status": "info",
            "backend": backend,
            "message": format!("redis not active (cache_backend={backend})"),
        });
    }
    let url = cfg.cache_redis_url.as_deref().unwrap_or("");
    match crate::cache::RedisCache::connect(url) {
        Ok(_) => json!({
            "id": "cache_redis",
            "status": "pass",
            "backend": "redis",
            "message": "redis PING ok (XDG cache_redis_url)",
        }),
        Err(e) => json!({
            "id": "cache_redis",
            "status": "fail",
            "backend": "redis",
            "message": e.message(),
            "suggestion": "Start redis-server or: browser-automation-cli config set cache_backend sqlite",
        }),
    }
}

pub(super) fn which_bin(name: &str) -> Option<String> {
    crate::platform::which_bin(name).map(|p| p.display().to_string())
}
