// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `doctor` checklist: assemble every probe into one envelope.

use super::probes::{
    cache_redis_check, cookie_jar_scope_check, jpeg_decoder_toolchain_check,
    resolve_lighthouse_for_doctor, virtual_display_check, which_bin, xvfb_check,
};
use super::residual_policy;
use super::*;

/// Run every host probe and emit one envelope; returns the process exit code.
///
/// Probes are independent and none of them launches Chrome unless `--quick` is
/// absent, so the checklist stays cheap enough to run before any real work.
pub fn run_doctor(opts: DoctorOptions) -> i32 {
    let mut checks = Vec::new();
    let mut failed = false;

    let host = crate::platform::HostEnvironment::detect();
    checks.push(json!({
        "id": "host_environment",
        "status": "info",
        "message": host.summary(),
        "wsl": host.wsl,
        "container": host.container,
        "ci": host.ci,
        "termux": host.termux,
        "flatpak": host.flatpak,
        "snap": host.snap,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }));

    let chrome = chrome::find_chrome();
    match &chrome {
        Some(p) => {
            let sandbox = crate::platform::detect_browser_sandbox(p);
            let executable = crate::platform::is_executable_file(p);
            // Doctor-only smoke: `--version` (never on hot find_chrome path).
            let version = if executable {
                crate::platform::probe_binary_version(p)
            } else {
                None
            };
            let mut status = "pass";
            let mut message = format!("found {}", p.display());
            if sandbox.is_restricted() {
                status = "warn";
                message = format!(
                    "found {} (sandbox={}; CDP may fail — prefer system package or config set chrome_path)",
                    p.display(),
                    sandbox.as_str()
                );
            } else if !executable {
                status = "fail";
                failed = true;
                message = format!("found {} but not executable", p.display());
            } else if version.is_none() {
                status = "warn";
                message = format!(
                    "found {} but --version smoke failed (binary may be broken or sandboxed)",
                    p.display()
                );
            } else if let Some(ref v) = version {
                message = format!("found {} ({v})", p.display());
            }
            checks.push(json!({
                "id": "chrome",
                "status": status,
                "message": message,
                "path": p.display().to_string(),
                "sandbox": sandbox.as_str(),
                "executable": executable,
                "version": version,
            }));
        }
        None => {
            failed = true;
            checks.push(json!({
                "id":"chrome",
                "status":"fail",
                "message":"Chrome/Chromium not found on PATH, known install paths, registry App Paths, or product cache",
                "fix":"install Chromium/Chrome or: browser-automation-cli config set chrome_path /path/to/chrome",
                "sandbox": "none",
                "executable": false,
                "version": null,
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
                .goto(
                    crate::constants::ABOUT_BLANK,
                    crate::robots::RobotsPolicy::Ignore,
                )
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
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert("fix".into(), json!(fix));
                }
            }
            checks.push(entry);
        }
    }

    // Cache backend health (R-LIVE-3): PING only when XDG cache_backend=redis.
    checks.push(cache_redis_check());

    checks.push(xvfb_check());
    checks.push(virtual_display_check());
    checks.push(jpeg_decoder_toolchain_check());
    checks.push(cookie_jar_scope_check());

    // Optional OS binary (no pure-Rust H.264 drop-in). Prefer XDG ffmpeg_path.
    let ffmpeg_xdg = crate::xdg::ffmpeg_path_from_config()
        .filter(|p| crate::platform::is_executable_file(std::path::Path::new(p)));
    let ffmpeg_present = ffmpeg_xdg.clone().or_else(|| which_bin("ffmpeg"));
    let ffmpeg_source = if ffmpeg_xdg.is_some() {
        "xdg"
    } else if ffmpeg_present.is_some() {
        "path"
    } else {
        "missing"
    };
    checks.push(json!({
        "id": "ffmpeg",
        "status": if ffmpeg_present.is_some() { "pass" } else { "info" },
        "message": ffmpeg_present
            .as_deref()
            .map(|p| format!("found {p} ({ffmpeg_source})"))
            .unwrap_or_else(|| {
                "ffmpeg not found (optional: config set ffmpeg_path or install on PATH)".into()
            }),
        "ffmpeg_present": ffmpeg_present.is_some(),
        "ffmpeg_source": ffmpeg_source,
    }));

    // Optional ffprobe (video/audio info); sibling of ffmpeg via video_local resolve.
    let ffprobe_present = crate::video_local::resolve_ffprobe_bin();
    checks.push(json!({
        "id": "ffprobe",
        "status": if ffprobe_present.is_some() { "pass" } else { "info" },
        "message": ffprobe_present
            .as_ref()
            .map(|p| format!("found {}", p.display()))
            .unwrap_or_else(|| {
                "ffprobe not found (optional: install ffmpeg package or set ffmpeg_path next to ffprobe)".into()
            }),
        "ffprobe_present": ffprobe_present.is_some(),
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

    // Parallelism budget (rules_rust_paralelismo) — agent-visible, host-local only.
    checks.push(json!({
        "id": "concurrency_budget",
        "status": "info",
        "message": "bounded fan-out budget (CPU × free RAM × 50% / 64MiB, hard cap 64)",
        "budget": crate::concurrency::budget_report(),
    }));

    // RES-04: residual disk hygiene (path-light; no Chrome launch).
    // Lifecycle::new already ran BORN stale GC; report reflects post-BORN state
    // when doctor is invoked through the normal CLI entry (lib::run).
    let residual = crate::residual::residual_disk_report();
    // Interpretation lives in `residual_policy`: which counter is a defect and
    // which is healthy concurrency is a rule of its own, not part of assembling
    // the checklist (GAP-002 / GAP-006 / GAP-045).
    let verdict = residual_policy::residual_verdict(&residual);
    failed |= verdict.failed;
    checks.extend(verdict.checks);

    let data = json!({
        "schema_version": 1,
        "checks": checks,
        "lighthouse_present": lighthouse_present,
        "ffmpeg_present": ffmpeg_present.is_some(),
        "offline": opts.offline,
        "fix_requested": opts.fix,
        "ok": !failed,
        "concurrency": crate::concurrency::budget_report(),
        "residual": residual,
    });

    if opts.json {
        match print_success_json(data) {
            Ok(()) => {}
            Err(e) if e.kind() == crate::error::ErrorKind::BrokenPipe => return 141,
            // A discarded error here is the worst outcome the doctor can produce:
            // the agent asked for a payload the CLI could not deliver (an
            // impossible `--max-output-bytes`, say) and got exit 0 with an empty
            // stdout — a green light for a check that never ran. Every other
            // command already reports this through `emit_err`, which localizes
            // the suggestion and returns the real exit code.
            Err(e) => return crate::commands::common::emit_err(&e, true),
        }
    } else {
        for c in checks {
            let line = format!(
                "[{}] {} — {}",
                c.get("status").and_then(|s| s.as_str()).unwrap_or("?"),
                c.get("id").and_then(|s| s.as_str()).unwrap_or("?"),
                c.get("message").and_then(|s| s.as_str()).unwrap_or("")
            );
            match crate::output::writeln_stdout(line) {
                Ok(()) => {}
                Err(e) if e.kind() == crate::error::ErrorKind::BrokenPipe => return 141,
                // A failed write here (full disk, closed fd) must not be reported
                // as a clean bill of health. Human mode has no envelope, so the
                // error goes to stderr and the exit code carries the truth.
                Err(e) => return crate::commands::common::emit_err(&e, false),
            }
        }
        let _ = crate::output::flush_stdout();
    }

    if failed {
        1
    } else {
        0
    }
}
