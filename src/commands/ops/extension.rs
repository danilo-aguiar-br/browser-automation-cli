// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::cli::ExtensionAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_extension(
    life: &Lifecycle,
    action: ExtensionAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        ExtensionAction::List => {
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    let v = session.extension_list().await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok extension list {d}"))
            })
        }
        ExtensionAction::Install { path } => {
            let path_s = path.display().to_string();
            let data = block_on_browser_timeout(
                async move {
                    let mut session =
                        OneShotSession::launch_with_extensions(capture, vec![path_s.clone()])
                            .await?;
                    life.record_chrome(session.chrome_pid());
                    // Service workers may take a moment to register after --load-extension.
                    let mut listed = session.extension_list().await?;
                    for _ in 0..crate::xdg::policy::policy_u32(
                        crate::xdg::policy::key::EXTENSION_ATTACH_POLL_ITERS,
                    ) {
                        let count = listed.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                        if count > 0 {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(
                            crate::xdg::policy::policy_u64(
                                crate::xdg::policy::key::EXTENSION_ATTACH_POLL_MS,
                            ),
                        ))
                        .await;
                        listed = session.extension_list().await?;
                    }
                    let close = session.shutdown().await;
                    life.clear_chrome();
                    close?;
                    Ok(serde_json::json!({
                        "installed_path": path_s,
                        "load_extension": true,
                        "targets": listed,
                        "note": "one-shot: Chrome launched with --load-extension for this process only",
                    }))
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok extension install {d}"))
            })
        }
        ExtensionAction::Reload { id, path } => {
            // Match already owns `id`/`path` — move into the async block (no clone).
            let path_s = path.as_ref().map(|p| p.display().to_string());
            let data = block_on_browser_timeout(
                async move {
                    let mut session = if let Some(p) = path_s {
                        OneShotSession::launch_with_extensions(capture, vec![p]).await?
                    } else {
                        OneShotSession::launch_headless_with_capture(capture).await?
                    };
                    life.record_chrome(session.chrome_pid());
                    let v = session.extension_reload(&id).await;
                    let close = session.shutdown().await;
                    life.clear_chrome();
                    close?;
                    v
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok extension reload {d}"))
            })
        }
        ExtensionAction::Trigger { id, path } => {
            let path_s = path.as_ref().map(|p| p.display().to_string());
            let data = block_on_browser_timeout(
                async move {
                    let mut session = if let Some(p) = path_s {
                        OneShotSession::launch_with_extensions(capture, vec![p]).await?
                    } else {
                        OneShotSession::launch_headless_with_capture(capture).await?
                    };
                    life.record_chrome(session.chrome_pid());
                    let v = session.extension_trigger(&id).await;
                    let close = session.shutdown().await;
                    life.clear_chrome();
                    close?;
                    v
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok extension trigger {d}"))
            })
        }
        ExtensionAction::Uninstall { id } => {
            // One clone for human emit after the async move consumes `id`.
            let id_print = id.clone();
            // Prefer in-process unload when a session can be opened; otherwise honest metadata.
            let data = block_on_browser_timeout(
                async move {
                    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
                    life.with_ledger_mut(|ledger| {
                        ledger.chrome_launched = true;
                        ledger.chrome_pid = session.chrome_pid();
                        if let Some(dir) = session.temp_user_data_dir() {
                            ledger.profile_dir = Some(dir);
                        }
                    });
                    let v = session.extension_uninstall(&id).await;
                    let close = session.shutdown().await;
                    life.clear_chrome_and_profile();
                    close?;
                    v
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| {
                let effect = d.get("effect").and_then(|v| v.as_str()).unwrap_or("?");
                crate::output::writeln_stdout(format!(
                    "ok extension uninstall id={id_print} effect={effect}"
                ))
            })
        }
    }
}
