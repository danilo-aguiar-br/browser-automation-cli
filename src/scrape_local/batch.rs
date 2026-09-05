// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded JoinSet batch scrape (I/O fan-out + cancel-first).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::error_page::{http_error_page, status_from_error_message};
use super::http::scrape_http;
use super::types::ScrapeOpts;

/// Batch scrape N URLs (HTTP engine, bounded JoinSet + Semaphore fan-out).
///
/// `concurrency == 0` uses the process budget ([`crate::concurrency::effective_limit`]).
///
/// # Bound (rules_rust_paralelismo)
///
/// Gate is `Arc<Semaphore>` with [`acquire_owned`](tokio::sync::Semaphore::acquire_owned)
/// moved into each spawned task (RAII permit). Never unbounded `spawn` loops.
#[tracing::instrument(level = "debug", skip(urls, opts), fields(n = urls.len(), concurrency))]
pub async fn batch_scrape_http(
    urls: &[String],
    robots: RobotsPolicy,
    opts: &ScrapeOpts,
    concurrency: usize,
) -> Result<Value, CliError> {
    let concurrency = crate::concurrency::resolve_permits(concurrency);
    // Pre-size for known fan-out (rules: with_capacity when length is known).
    let mut results = Vec::with_capacity(urls.len());
    let mut errors = Vec::new();
    use std::sync::Arc;

    use tokio::task::JoinSet;
    // `semaphore_with` clamps to the process bounds; constructing the Semaphore
    // by hand here skipped that, so a caller-supplied permit count reached the
    // pool unchecked.
    let sem = crate::concurrency::semaphore_with(concurrency);
    tracing::debug!(
        available_permits = sem.available_permits(),
        concurrency,
        n = urls.len(),
        "batch_scrape_http fan-out"
    );
    let mut set: JoinSet<Result<Value, CliError>> = JoinSet::new();
    let cancel = crate::lifecycle::current_cancel();
    for u in urls {
        if cancel.is_cancelled() {
            errors.push(json!({ "error": "cancelled", "url": u }));
            break;
        }
        // Acquire before spawn so peak in-flight never exceeds permits
        // (even if JoinSet buffers handles). Race cancel so SIGINT during
        // admission does not wait for a free permit forever.
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                errors.push(json!({ "error": "cancelled", "url": u }));
                break;
            }
            p = Arc::clone(&sem).acquire_owned() => p.map_err(|_| {
                CliError::new(ErrorKind::Software, "concurrency semaphore closed")
            })?,
        };
        let u = u.clone();
        let opts = opts.clone();
        set.spawn(async move {
            let _permit = permit; // drop at end of task → permit returns (incl. panic)
            scrape_http(&u, robots, &opts).await
        });
    }
    if cancel.is_cancelled() {
        set.abort_all();
    }
    loop {
        let joined = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                set.abort_all();
                None
            }
            j = set.join_next() => j,
        };
        let Some(joined) = joined else {
            break;
        };
        match joined {
            Ok(Ok(v)) => results.push(v),
            Ok(Err(e)) => {
                let msg = e.to_string();
                let mut row = http_error_page("", &msg, None);
                if let Some(obj) = row.as_object_mut() {
                    obj.insert("error".into(), json!(msg));
                    if let Some(code) = status_from_error_message(e.message()) {
                        obj.insert("status_code".into(), json!(code));
                    }
                    // Prefer message from CliError display
                    obj.insert("http_error".into(), json!(true));
                }
                errors.push(row);
            }
            Err(e) => {
                // Distinguish panic vs cancel for agent diagnostics.
                let kind = if e.is_panic() {
                    "panic"
                } else if e.is_cancelled() {
                    "cancelled"
                } else {
                    "join"
                };
                errors.push(json!({ "error": format!("{kind}: {e}"), "http_error": true }));
            }
        }
    }
    // Drain aborted handles so JoinSet drop does not leave panic diagnostics unobserved.
    if cancel.is_cancelled() {
        while let Some(joined) = set.join_next().await {
            if let Err(e) = joined {
                if e.is_panic() {
                    errors.push(json!({ "error": format!("panic: {e}") }));
                }
            }
        }
    }
    Ok(json!({
        // NOT `ok`. The envelope already carries a top-level `ok` meaning "the
        // command ran", and this one means "every URL in the batch succeeded".
        // Both are defensible alone; sharing one name in one payload is not.
        // Measured before 0.1.9: a batch of 8 URLs with one 403 answered
        // `ok: true` at the top and `ok: false` here, so the documented agent
        // policy — check the exit code, validate `.ok`, read `.data` — passed
        // all three checks and never saw the partial failure. A 200-URL crawl
        // with 40 blocked presented itself as a success.
        "all_succeeded": errors.is_empty(),
        // Promoted alongside `error_count` so a caller can branch on partial
        // failure without counting an array itself.
        "partial_failure": !errors.is_empty() && !results.is_empty(),
        "count": results.len(),
        "error_count": errors.len(),
        "results": results,
        "errors": errors,
        "engine": "http",
        "concurrency": concurrency,
        "gate": "Arc<Semaphore>::acquire_owned",
        "robots_policy": robots.as_str(),
        "cancelled": cancel.is_cancelled(),
    }))
}
