// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::cli::{ConsoleAction, CookieAction, NetAction, PageAction};
use crate::commands::common::emit_ok;
use crate::commands::nav::session::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_console(
    life: &Lifecycle,
    action: ConsoleAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match action {
                ConsoleAction::List {
                    page_idx,
                    page_size,
                    types,
                    include_preserved,
                    service_worker_id,
                } => session.console_list(
                    page_idx,
                    page_size,
                    types.as_deref(),
                    include_preserved,
                    service_worker_id.as_deref(),
                ),
                ConsoleAction::Get { id } => session.console_get(id),
                ConsoleAction::Clear => session.console_clear(),
                ConsoleAction::Dump { path } => session.console_dump(&path).await,
            };
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok console {d}"))
    })
}

pub(crate) fn handle_net(
    life: &Lifecycle,
    action: NetAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match action {
                NetAction::List {
                    page_idx,
                    page_size,
                    resource_types,
                    include_preserved,
                } => session.net_list(
                    page_idx,
                    page_size,
                    resource_types.as_deref(),
                    include_preserved,
                ),
                NetAction::Get {
                    id,
                    request_path,
                    response_path,
                } => {
                    session
                        .net_get(&id, request_path.as_deref(), response_path.as_deref())
                        .await
                }
            };
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok net {d}"))
    })
}

pub(crate) fn handle_page(
    life: &Lifecycle,
    action: Option<PageAction>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let action = action.unwrap_or(PageAction::Info);
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            PageAction::Info => session.page_info().await?,
            PageAction::List => session.page_list().await?,
            PageAction::New {
                url,
                background,
                isolated_context,
            } => {
                session
                    .page_new(url.as_deref(), background, isolated_context.as_deref())
                    .await?
            }
            PageAction::Select {
                index,
                page_id,
                bring_to_front,
            } => {
                let idx = index.or(page_id).ok_or_else(|| {
                    CliError::with_suggestion(
                        ErrorKind::Usage,
                        "page select requires INDEX or --page-id",
                        crate::i18n::suggestion_key("page_select_target", None),
                    )
                })?;
                session.page_select(idx, bring_to_front).await?
            }
            PageAction::Close { index, page_id } => session.page_close(index.or(page_id)).await?,
            PageAction::TabId => {
                let tab = session.active_tab_id_string().ok_or_else(|| {
                    CliError::with_suggestion(
                        ErrorKind::Browser,
                        "no active tab id",
                        crate::i18n::suggestion_key("navigate_first", None),
                    )
                })?;
                serde_json::json!({
                    "tab_id": tab,
                    "tool": "get_tab_id",
                })
            }
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        if let Some(tab) = d.get("tab_id").and_then(|v| v.as_str()) {
            crate::output::writeln_stdout(format!("ok page tab-id={tab}"))?;
        } else if let (Some(u), Some(t)) = (
            d.get("url").and_then(|v| v.as_str()),
            d.get("title").and_then(|v| v.as_str()),
        ) {
            crate::output::writeln_stdout(format!("ok page url={u} title={t}"))?;
        } else {
            crate::output::writeln_stdout(format!("ok page {d}"))?;
        }
        Ok(())
    })
}

pub(crate) fn handle_cookie(
    life: &Lifecycle,
    action: CookieAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            CookieAction::List { url } => session.cookie_list(url.as_deref()).await?,
            CookieAction::Set { cookies_json: body } => session.cookie_set(&body).await?,
            CookieAction::Clear => session.cookie_clear().await?,
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok cookie {d}"))
    })
}
