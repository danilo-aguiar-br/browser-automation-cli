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

    // Lighthouse binary presence (external audit tool).
    let lighthouse_present = which_bin("lighthouse");
    match &lighthouse_present {
        Some(p) => checks.push(json!({
            "id": "lighthouse",
            "status": "pass",
            "message": format!("found {p}"),
            "lighthouse_present": true,
        })),
        None => checks.push(json!({
            "id": "lighthouse",
            "status": "info",
            "message": "lighthouse not on PATH (optional; use --lighthouse-path)",
            "lighthouse_present": false,
            "fix": "npm i -g lighthouse",
        })),
    }

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

    let _ = opts.offline;
    let _ = opts.fix;

    let data = json!({
        "schema_version": 1,
        "checks": checks,
        "lighthouse_present": lighthouse_present.is_some(),
        "ffmpeg_present": ffmpeg_present.is_some(),
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
