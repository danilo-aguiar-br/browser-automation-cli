// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::CaptureOpts;
use crate::cli::{ConsoleAction, CookieAction, NetAction, PageAction};
use crate::commands::common::emit_ok_summary;
use crate::commands::nav::session::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::etd::{with_target, TargetSource};
use crate::lifecycle::Lifecycle;

/// Refuse a top-level capture read, which can only ever answer empty.
///
/// `console` and `net` read a buffer filled by CDP events during THIS process.
/// Reached as a top-level subcommand the process launches Chrome, navigates to
/// `about:blank` and reads a buffer born empty one statement earlier, so the
/// answer was always `ok: true` with `count: 0` — measured 2026-08-28, on both
/// verbs, and published as a `README` example that could never work.
///
/// An empty answer is the worst possible reply here, because it is exactly what
/// a page with no matching traffic returns, so the caller cannot tell the two
/// apart. The refusal costs no browser launch, which is also what makes it
/// cheap: the old reply spent 1385 ms starting Chrome to say nothing.
///
/// # Errors
///
/// Always [`ErrorKind::Usage`], naming the surface that does work.
///
/// # Why this returns the error and not `Result<(), CliError>`
///
/// The first shape of this helper returned `Result` and was reached with `?`.
/// At a site that called it unconditionally, that made the rest of the function
/// unreachable — and invisibly so, because `unreachable_code` fires only on
/// divergence expressed in the TYPE, never on an `Err` that is a runtime fact.
/// `-D warnings` stayed green over forty-five dead lines. Returning the error
/// forces a `return Err(..)` the compiler can reason about, so the same mistake
/// cannot repeat in silence.
fn refuse_capture_outside_run(verb: &str) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!(
            "{verb} reads a capture buffer this invocation just created, so it can only \
             answer empty; capture does not survive a process boundary. Use \
             `run --script` with the capture flag, where the same verb is a step."
        ),
        crate::i18n::suggestion_key("capture_needs_run", None),
    )
}

/// Dispatch `console` over the buffer captured with `--capture-console`.
///
/// # Errors
///
/// Fails with [`ErrorKind::Usage`] for `list` and `get`: see
/// [`refuse_capture_outside_run`]. `clear` and `dump` still run.
pub(crate) fn handle_console(
    life: &Lifecycle,
    action: ConsoleAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    // One match decides the whole dispatch, so no arm of it can be dead. The
    // earlier shape refused with a `matches!` guard and then matched AGAIN over
    // all four actions, leaving `List` and `Get` as arms the guard had already
    // made unreachable — the same corpse this file was fixing, one scope down.
    //
    // The refusal covers the two actions whose answer is a CLAIM about the
    // page. `dump` writes `[]` to a file and `clear` reports what it cleared:
    // an empty artifact is visible to whoever opens it, while an empty `list`
    // is indistinguishable from a page that genuinely had no messages. The
    // COOKBOOK documents the empty-array guarantee of `dump`, and refusing it
    // would break a contract this product published on purpose.
    let data = match action {
        ConsoleAction::List { .. } | ConsoleAction::Get { .. } => {
            return Err(refuse_capture_outside_run("console list/get"))
        }
        ConsoleAction::Clear => {
            with_session_blank(life, capture, timeout_secs, |mut session| async move {
                let v = session.console_clear()?;
                Ok((session, v))
            })?
        }
        ConsoleAction::Dump { path } => {
            with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                let v = session.console_dump(&path).await?;
                Ok((session, v))
            })?
        }
    };
    emit_ok_summary(data, json, "console")
}

/// Dispatch `net` over the buffer captured with `--capture-network`.
///
/// # Errors
///
/// Always fails with [`ErrorKind::Usage`]: see [`refuse_capture_outside_run`].
/// Unlike `console`, `net` has no action that survives the refusal: `NetAction`
/// is `List` and `Get`, and both are reads of the buffer born empty. Every
/// parameter is therefore unused on purpose, and the leading underscores say
/// so; the signature stays shaped like its siblings because the dispatcher
/// calls all four the same way.
pub(crate) fn handle_net(
    _life: &Lifecycle,
    _action: NetAction,
    _capture: CaptureOpts,
    _timeout_secs: u64,
    _json: bool,
) -> Result<(), CliError> {
    Err(refuse_capture_outside_run("net"))
}

pub(crate) fn handle_page(
    life: &Lifecycle,
    action: Option<PageAction>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let action = action.unwrap_or(PageAction::Info);
    let etd = page_target(&action);
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
                no_bring_to_front,
            } => {
                // The negation wins; see the flag's own note for why the
                // positive form could not express the OFF value.
                let bring_to_front = bring_to_front && !no_bring_to_front;
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
    let data = match etd {
        Some((resolved, source)) => with_target(data, &resolved, source),
        None => data,
    };
    // The hand-written branches this replaced picked two fields and renamed one
    // of them: the envelope key is `tab_id` and the text line printed `tab-id`,
    // so a reader who saw the line and went looking for that key in `--json`
    // found nothing. The shared summary names every field exactly as the
    // envelope spells it, which is the property that makes the two modes
    // describe the same answer.
    emit_ok_summary(data, json, "page")
}

/// The tab a `page` action mutates, or `None` for the read-only actions.
///
/// `Close` with no `--index` is the case this exists for: it falls back to the
/// active tab, which is process state and not something the caller named.
/// `Info`, `List` and `TabId` return `None` because they mutate nothing, and
/// annotating a pure read would make the field meaningless where it matters.
fn page_target(action: &PageAction) -> Option<(String, TargetSource)> {
    match action {
        PageAction::Close { index, page_id } => Some(index.or(*page_id).map_or_else(
            || ("(active tab)".to_string(), TargetSource::Ambient),
            |i| (i.to_string(), TargetSource::Argv),
        )),
        PageAction::Select { index, page_id, .. } => Some(index.or(*page_id).map_or_else(
            || ("(active tab)".to_string(), TargetSource::Ambient),
            |i| (i.to_string(), TargetSource::Argv),
        )),
        PageAction::New { url, .. } => Some((
            url.clone()
                .unwrap_or_else(|| crate::constants::ABOUT_BLANK.to_string()),
            url.as_ref()
                .map_or(TargetSource::Ambient, |_| TargetSource::Argv),
        )),
        PageAction::Info | PageAction::List | PageAction::TabId => None,
    }
}

pub(crate) fn handle_cookie(
    life: &Lifecycle,
    action: CookieAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    // `cookie clear` wipes the whole jar, and `--all` is now required to say so.
    // The scope is still "all" — CDP has no partial clear — but it is now CHOSEN
    // in argv instead of inferred from an absent argument, so the source is
    // `Argv`. Before, a bare `cookie clear` swept everything and reported the
    // scope it had picked for itself as `Ambient`: honest about the ambiguity,
    // and still destroying data nobody had named.
    let etd = match &action {
        CookieAction::Clear { .. } => Some(("all".to_string(), TargetSource::Argv)),
        CookieAction::Set { .. } => Some(("(cookies-json)".to_string(), TargetSource::Argv)),
        CookieAction::List { .. } => None,
    };
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            CookieAction::List { url } => session.cookie_list(url.as_deref()).await?,
            CookieAction::Set { cookies_json: body } => session.cookie_set(&body).await?,
            CookieAction::Clear { .. } => session.cookie_clear().await?,
        };
        Ok((session, v))
    })?;
    let data = match etd {
        Some((resolved, source)) => with_target(data, &resolved, source),
        None => data,
    };
    emit_ok_summary(data, json, "cookie")
}
