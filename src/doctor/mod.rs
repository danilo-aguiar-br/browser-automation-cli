//! Local diagnostics for one-shot installs (no multi-process daemon).

use serde_json::json;

use crate::envelope::print_success_json;
use crate::install;
use crate::native::cdp::chrome;

/// Options for local install diagnostics.
#[derive(Default, Clone, Copy)]
pub struct DoctorOptions {
    /// Skip network checks.
    pub offline: bool,
    /// Run a reduced check set.
    pub quick: bool,
    /// Attempt automatic remediations when supported.
    pub fix: bool,
    /// Emit JSON envelope on stdout.
    pub json: bool,
}

/// Run doctor checks and return a process exit code (`0` = all pass).
pub fn run_doctor(opts: DoctorOptions) -> i32 {
    let mut checks = Vec::new();
    let mut failed = false;

    let chrome = chrome::find_chrome();
    match &chrome {
        Some(p) => checks.push(
            json!({"id":"chrome","status":"pass","message": format!("found {}", p.display())}),
        ),
        None => {
            failed = true;
            checks.push(json!({
                "id":"chrome",
                "status":"fail",
                "message":"Chrome/Chromium not found on PATH or cache",
                "fix":"install Chromium or set executable path"
            }));
        }
    }

    let cache = install::get_browsers_dir();
    checks.push(json!({
        "id":"browsers_dir",
        "status":"info",
        "message": format!("cache dir {}", cache.display())
    }));

    if !opts.quick && chrome.is_some() {
        match crate::browser::block_on_browser(async {
            let mut s = crate::browser::OneShotSession::launch_headless().await?;
            let _ = s
                .goto("about:blank", crate::robots::RobotsPolicy::Ignore)
                .await?;
            let _ = s.shutdown().await;
            Ok::<_, crate::error::CliError>(())
        }) {
            Ok(()) => checks
                .push(json!({"id":"launch","status":"pass","message":"headless about:blank ok"})),
            Err(e) => {
                failed = true;
                checks.push(json!({"id":"launch","status":"fail","message": e.message()}));
            }
        }
    }

    // Lighthouse: external binary only (PATH or XDG); never npm (GAP-A010 / LH-2).
    let lh_resolved = resolve_lighthouse_for_doctor();
    let lighthouse_present = lh_resolved.is_some();
    match &lh_resolved {
        Some((path, source)) => checks.push(json!({
            "id": "lighthouse",
            "status": "pass",
            "message": format!("found {path} (source={source})"),
            "lighthouse_present": true,
            "lighthouse_resolved": path,
            "lighthouse_source": source,
        })),
        None => {
            let mut fix_msg = None;
            if opts.fix {
                fix_msg = Some(
                    "browser-automation-cli config set lighthouse_path /absolute/path/to/lighthouse",
                );
            }
            let xdg_path = crate::xdg::lighthouse_path_from_config().filter(|s| !s.is_empty());
            let mut entry = json!({
                "id": "lighthouse",
                "status": "info",
                "message": "lighthouse not on PATH or XDG (optional external binary; e2e may use mock-lighthouse.sh)",
                "lighthouse_present": false,
                "lighthouse_resolved": null,
                "lighthouse_source": "missing",
                "lighthouse_path_xdg": xdg_path,
                "suggestion": "browser-automation-cli config set lighthouse_path /absolute/path/to/lighthouse",
            });
            if let Some(fix) = fix_msg {
                entry
                    .as_object_mut()
                    .unwrap()
                    .insert("fix".into(), json!(fix));
            }
            checks.push(entry);
        }
    }

    // Cache backend health (R-LIVE-3): PING only when XDG cache_backend=redis.
    checks.push(cache_redis_check());

    let ffmpeg_present = which_bin("ffmpeg");
    checks.push(json!({
        "id": "ffmpeg",
        "status": if ffmpeg_present.is_some() { "pass" } else { "info" },
        "message": ffmpeg_present
            .as_deref()
            .map(|p| format!("found {p}"))
            .unwrap_or_else(|| "ffmpeg not on PATH (optional for media pipelines)".into()),
        "ffmpeg_present": ffmpeg_present.is_some(),
    }));

    // GAP-009: report Job Object capability (real on Windows, stub elsewhere).
    checks.push(json!({
        "id": "windows_job_object",
        "status": "info",
        "message": crate::win_job::capability_summary(),
        "supported": crate::win_job::platform_supports_job_objects(),
    }));

    // Wire offline: skip any future network probes; record mode for agents.
    if opts.offline {
        checks.push(json!({
            "id": "offline",
            "status": "pass",
            "message": "offline mode: network probes skipped",
            "offline": true,
        }));
    }

    // Wire fix: when lighthouse missing and --fix, config path is the remediation
    // (already attached on the lighthouse check). When chrome missing, restate install.
    if opts.fix && chrome.is_none() {
        if let Some(c) = checks
            .iter_mut()
            .find(|c| c.get("id").and_then(|v| v.as_str()) == Some("chrome"))
        {
            if let Some(obj) = c.as_object_mut() {
                obj.insert(
                    "fix".into(),
                    json!("install system Chrome/Chromium or: browser-automation-cli config set chrome_path /path/to/chrome"),
                );
            }
        }
    }

    let data = json!({
        "schema_version": 1,
        "checks": checks,
        "lighthouse_present": lighthouse_present,
        "ffmpeg_present": ffmpeg_present.is_some(),
        "offline": opts.offline,
        "fix_requested": opts.fix,
        "ok": !failed,
    });

    if opts.json {
        let _ = print_success_json(data);
    } else {
        for c in checks {
            println!(
                "[{}] {} — {}",
                c.get("status").and_then(|s| s.as_str()).unwrap_or("?"),
                c.get("id").and_then(|s| s.as_str()).unwrap_or("?"),
                c.get("message").and_then(|s| s.as_str()).unwrap_or("")
            );
        }
    }

    if failed {
        1
    } else {
        0
    }
}

/// Resolve lighthouse for doctor: XDG path if executable, else PATH.
fn resolve_lighthouse_for_doctor() -> Option<(String, &'static str)> {
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
fn cache_redis_check() -> serde_json::Value {
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

fn which_bin(name: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate.display().to_string());
            }
            #[cfg(windows)]
            {
                let with_exe = dir.join(format!("{name}.exe"));
                if with_exe.is_file() {
                    return Some(with_exe.display().to_string());
                }
            }
        }
        None
    })
}
