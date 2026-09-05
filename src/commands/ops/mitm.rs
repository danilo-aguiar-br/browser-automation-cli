// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::cli::MitmAction;
use crate::commands::common::emit_ok_summary;
use crate::error::{CliError, ErrorKind};
use crate::etd::{with_target, TargetSource};

pub(crate) fn handle_mitm(action: MitmAction, json: bool) -> Result<(), CliError> {
    let etd = mitm_target(&action);
    let data = match action {
        MitmAction::Status { capture_path } => crate::mitm_local::status(capture_path.as_deref())?,
        MitmAction::List {
            host,
            limit,
            capture_path,
        } => crate::mitm_local::list(host.as_deref(), limit, capture_path.as_deref())?,
        MitmAction::Get { id, capture_path } => {
            crate::mitm_local::get(id, capture_path.as_deref())?
        }
        MitmAction::Har { out, capture_path } => {
            crate::mitm_local::export_har(&out, capture_path.as_deref())?
        }
        MitmAction::Export {
            format,
            out,
            capture_path,
        } => {
            if format.eq_ignore_ascii_case("har") {
                crate::mitm_local::export_har(&out, capture_path.as_deref())?
            } else {
                let (path, explicit) =
                    crate::mitm_local::resolve_capture_path(capture_path.as_deref())?;
                let cap = crate::mitm_local::MitmCapture::load_scoped(
                    &path,
                    crate::mitm_local::policy::redact_secrets(),
                    explicit,
                )?;
                let body = serde_json::to_vec_pretty(&serde_json::json!({
                    "count": cap.items.len(),
                    "items": cap.items,
                }))
                .map_err(|e| CliError::new(ErrorKind::Data, format!("export: {e}")))?;
                // Outer CLI sync path (PAR-83): write_bytes_sync for large capture dumps.
                crate::concurrency::write_bytes_sync(&out, &body)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("export write: {e}")))?;
                serde_json::json!({
                    "path": out.display().to_string(),
                    "format": format,
                    "count": cap.items.len(),
                })
            }
        }
        MitmAction::Domains { capture_path } => {
            crate::mitm_local::domains(capture_path.as_deref())?
        }
        MitmAction::Apis { kind, capture_path } => {
            crate::mitm_local::apis(kind.as_deref(), capture_path.as_deref())?
        }
        MitmAction::InitCa => crate::mitm_local::ensure_ca()?,
        MitmAction::Start { seconds } => {
            crate::browser::block_on_browser(crate::mitm_local::start_proxy_oneshot(seconds))?
        }
        MitmAction::CaptureUrl {
            url,
            seconds,
            har,
            hosts,
            capture_hosts,
        } => crate::browser::block_on_browser(crate::mitm_local::capture_url_oneshot(
            &url,
            seconds,
            har.as_deref(),
            hosts.as_deref(),
            capture_hosts.as_deref(),
        ))?,
        MitmAction::Graphql {
            limit,
            capture_path,
        } => crate::mitm_local::graphql(limit, capture_path.as_deref())?,
        MitmAction::Ws { action } => match action {
            crate::cli::MitmWsAction::List {
                limit,
                capture_path,
            } => crate::mitm_local::ws_list(limit, capture_path.as_deref())?,
            crate::cli::MitmWsAction::Get { id, capture_path } => {
                crate::mitm_local::ws_get(id, capture_path.as_deref())?
            }
        },
        MitmAction::Block { host, path } => {
            crate::mitm_local::block_rule(host.as_deref(), path.as_deref())?
        }
        MitmAction::Allow { host } => crate::mitm_local::allow_host(&host)?,
        MitmAction::Redact { secrets } => crate::mitm_local::redact_policy(secrets)?,
    };
    let data = match etd {
        Some(resolved) => with_target(data, &resolved, TargetSource::Argv),
        None => data,
    };
    emit_ok_summary(data, json, "mitm")
}

/// The rule a `mitm` action writes, or `None` for the read-only actions.
///
/// Both writers already refuse without a target — `block` needs `--host` and/or
/// `--path`, `allow` takes `--host` as a required value — so the source is
/// always argv. Publishing it is what lets a gate prove that, instead of the
/// gate having to trust the help text.
fn mitm_target(action: &MitmAction) -> Option<String> {
    match action {
        MitmAction::Block { host, path } => Some(format!(
            "{}{}",
            host.as_deref().unwrap_or("*"),
            path.as_deref().unwrap_or("")
        )),
        MitmAction::Allow { host } => Some(host.clone()),
        _ => None,
    }
}
