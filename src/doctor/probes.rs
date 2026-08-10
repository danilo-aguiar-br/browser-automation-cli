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

/// Whether a headed Chrome on this host can render into a private display.
///
/// # Why this reports instead of installing
///
/// The obvious fix for a missing `Xvfb` is to install it, and a CLI that runs
/// a package manager is a CLI that can modify the host it was asked to
/// observe — with `sudo`, silently, in the middle of a scrape. So this names
/// the package for the detected distribution and stops there.
///
/// `info` rather than `warn` on non-Linux and on headless: there is nothing
/// wrong with those hosts. A status that cries wolf on a healthy machine is a
/// status operators learn to skip.
pub(super) fn xvfb_check() -> serde_json::Value {
    if !cfg!(target_os = "linux") {
        return json!({
            "id": "xvfb",
            "status": "info",
            "message": "not applicable: this platform always has a compositor",
            "xvfb_present": false,
            "xvfb_required": false,
        });
    }
    let present = crate::native::cdp::xvfb::xvfb_available();
    if present {
        return json!({
            "id": "xvfb",
            "status": "pass",
            "message": "Xvfb found; a headed launch renders into a private display",
            "xvfb_present": true,
            "xvfb_required": true,
        });
    }
    json!({
        "id": "xvfb",
        "status": "info",
        "message": format!(
            "Xvfb not on PATH; a headed launch falls back to the current display. Install with: {}",
            xvfb_install_hint()
        ),
        "xvfb_present": false,
        "xvfb_required": true,
        "install_hint": xvfb_install_hint(),
    })
}

/// The install command for the detected distribution.
///
/// Read from `/etc/os-release`, which is a file, not an executed command:
/// probing a host must never run anything the operator did not ask for.
fn xvfb_install_hint() -> String {
    let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let field = |key: &str| -> String {
        release
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .unwrap_or("")
            .trim_matches(['"', '\''])
            .to_ascii_lowercase()
    };
    let family = format!("{} {}", field("ID="), field("ID_LIKE="));
    let has = |needle: &str| family.contains(needle);

    if has("fedora") || has("rhel") || has("centos") {
        "dnf install xorg-x11-server-Xvfb"
    } else if has("debian") || has("ubuntu") {
        "apt-get install xvfb"
    } else if has("arch") {
        "pacman -S xorg-server-xvfb"
    } else if has("suse") {
        "zypper install xorg-x11-server-Xvfb"
    } else if has("alpine") {
        "apk add xvfb"
    } else {
        "your distribution's Xvfb package"
    }
    .to_string()
}

/// Whether the JPEG decoder in this build compiles on current toolchains.
///
/// # What this is actually reporting
///
/// `image`'s `jpeg` feature pulls `zune-jpeg`. Stable 0.5.15 contains a
/// `warn!(...)` with no trailing semicolon while `zune-core`'s macro expands to
/// an attribute on a block — a statement, not an expression. The file compiles
/// only on toolchains that still tolerate it, and `rust-toolchain.toml` pins
/// one that does.
///
/// So this host builds and a macOS host on a newer toolchain does not. The
/// dependency note in `Cargo.toml` records why neither a version pin nor a
/// `[patch]` closes it. Reported rather than hidden, because a caller who hits
/// the build failure elsewhere deserves to find the explanation in the tool
/// rather than rediscover it.
pub(super) fn jpeg_decoder_toolchain_check() -> serde_json::Value {
    json!({
        "id": "jpeg_decoder_toolchain",
        "status": "info",
        "message": "zune-jpeg 0.5.15 (via image) needs a tolerant toolchain; \
                    rust-toolchain.toml pins one. Closes when 0.5.16 goes stable.",
        "pinned_toolchain": true,
        "upstream_fix": "zune-jpeg 0.5.16",
    })
}

/// State the lifetime of the HTTP cookie jar, and name the way to outlive it.
///
/// # Why a limit gets its own check
///
/// The jar is real and it is per-process: a warm-up on the origin root hands
/// its session to the deep request in the same invocation, then dies at DIE.
/// That is enough for the case it was built for and not enough for the case a
/// caller will assume, which is a session that survives to the next command.
///
/// Silence would read as a guarantee. A caller who assumes persistence sees a
/// fresh challenge on the second invocation, reads it as a flake, and retries
/// into a harder block — the exact escalation the disclosure exists to avoid.
/// So the scope is stated here, next to the checks an operator already reads,
/// and `storage export` is named as the supported way to carry a session.
///
/// Status is `info` rather than `warn`: nothing is broken, and a warning for a
/// working design trains the reader to skim past warnings that matter.
pub(super) fn cookie_jar_scope_check() -> serde_json::Value {
    json!({
        "id": "cookie_jar_scope",
        "status": "info",
        "message": "the HTTP cookie jar lives and dies with this process; \
                    use `storage export` / `storage import` to carry a session",
        "scope": "process",
        "persists_across_invocations": false,
        "carry_session_with": "storage export",
    })
}

/// Whether this host already has a display, and what `browser_mode = auto` does.
///
/// # Why these two facts belong in one check
///
/// Either one alone is harmless and together they decide where a window lands.
/// A host WITH a compositor and a mode that resolves headless is a host where
/// an explicit headed launch would draw on the operator's own screen unless a
/// private display is started first. Reporting the display without the resolved
/// mode leaves the reader to guess the half that matters.
///
/// # Why this exists at all
///
/// `auto` was documented as "headed inside a private virtual display on Linux"
/// and has always resolved to headless. The documentation was corrected, but a
/// corrected sentence is not a measurement: it goes stale the next time the
/// resolution changes. This check reads the live policy, so the answer cannot
/// drift away from the binary the operator is running.
///
/// `info` rather than `warn`: nothing is broken. The default is deliberate and
/// carries a latency bill, and a warning on a working design teaches operators
/// to skim past the warnings that matter.
pub(super) fn virtual_display_check() -> serde_json::Value {
    let auto_resolves = if crate::browser_policy::BrowserMode::Auto.launches_headless() {
        "headless"
    } else {
        "headed"
    };
    json!({
        "id": "virtual_display",
        "status": "info",
        "message": "`browser_mode = auto` resolves to headless; a private display \
                    is started only for an explicit headed launch on Linux",
        "host_has_display": crate::native::cdp::xvfb::host_has_display(),
        "browser_mode_auto_resolves": auto_resolves,
        "private_display_supported": cfg!(target_os = "linux"),
    })
}
