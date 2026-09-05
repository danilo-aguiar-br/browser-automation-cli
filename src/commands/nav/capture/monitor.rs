// SPDX-License-Identifier: MIT OR Apache-2.0
//! Baseline hash change-detection artifact handler.

use crate::browser::block_on_browser_timeout;
use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

pub(crate) fn handle_monitor(
    action: crate::cli::MonitorAction,
    robots: RobotsPolicy,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    use sha2::{Digest, Sha256};
    match action {
        crate::cli::MonitorAction::Check {
            url,
            baseline,
            write_baseline,
            engine,
            diff_mode,
        } => {
            // Parsed before the fetch: a bad mode is an argv mistake, and
            // discovering it after a network round trip wastes the request.
            let mode = super::monitor_diff::DiffMode::parse(&diff_mode)?;
            // GAP-026, read axis. `--baseline` is operator argv and this command
            // read it with no root check while WRITING the same path through the
            // guarded `write_bytes_sync` below — the write was contained and the
            // read was not, in the same function, on the same path.
            //
            // The leak was not just the read: the previous file's contents come
            // back to the caller in `previous_hash`, and through `build_diff` in
            // `diff` / `added` / `removed`.
            //
            // Bounded HERE and not in `monitor_diff`: this is where the operator
            // path enters, `content_path` derives the sidecar from this same
            // value, and `build_diff` / `record_content` have no other caller.
            // A second check down there could only ever agree with this one or
            // drift from it.
            //
            // Placed before the fetch for the reason the note above gives: a
            // path that will be refused should not cost a network round trip.
            let baseline = crate::fs_roots::ensure_read_allowed(&baseline)?;
            let engine_l = engine.to_ascii_lowercase();
            let text = if engine_l == "browser" {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "monitor check --engine browser is reserved; use http for baseline hash",
                    crate::i18n::suggestion_key("scrape_engine_choice", None),
                ));
            } else {
                // The cache bypass is NOT a preference here, so it is not a flag.
                //
                // This command answers "did this page change". Served from the
                // response cache it hashed a stored copy, compared it with the
                // baseline it came from, and reported `changed: false` for a
                // page that had changed — with `ok: true`, exit 0 and
                // `diff_available: true` asserting the empty diff was reliable.
                //
                // Measured: same fixture, same URL, content replaced on disk.
                // With the sqlite backend the hash matched the baseline; with
                // `cache_backend = memory` the same command saw the change. The
                // only variable was the cache, so the cache is the defect.
                //
                // There is deliberately no flag to switch this off. A flag would
                // only ever be a way to ask this command for a false negative.
                let opts = crate::scrape_local::ScrapeOpts {
                    format: crate::scrape_local::ScrapeFormat::Text,
                    engine: "http".into(),
                    no_cache: true,
                    ..Default::default()
                };
                let page = block_on_browser_timeout(
                    crate::scrape_local::scrape_http(&url, robots, &opts),
                    timeout_secs,
                )?;
                page.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            let hash = hex::encode(hasher.finalize());
            let baseline_exists = baseline.exists();
            let (changed, previous_hash) = if baseline_exists {
                // Outer CLI path (no Tokio worker) — sync helper is correct (PAR-83).
                let prev = std::fs::read_to_string(&baseline).map_err(|e| {
                    CliError::new(
                        ErrorKind::Io,
                        format!("read baseline {}: {e}", baseline.display()),
                    )
                })?;
                let prev = prev.trim().to_string();
                (prev != hash, Some(prev))
            } else {
                (true, None)
            };
            if write_baseline || !baseline_exists {
                crate::concurrency::write_bytes_sync(&baseline, format!("{hash}\n").as_bytes())
                    .map_err(|e| {
                        CliError::new(
                            ErrorKind::Io,
                            format!("write baseline {}: {e}", baseline.display()),
                        )
                    })?;
            }
            let diff = super::monitor_diff::build_diff(mode, &baseline, &text);
            // Recorded AFTER the diff is built, so this run compares against
            // the previous body rather than against itself.
            if mode.wants_content() {
                super::monitor_diff::record_content(&baseline, &text);
            }
            let mut data = serde_json::json!({
                "url": url,
                "baseline": baseline.display().to_string(),
                "hash": hash,
                "previous_hash": previous_hash,
                "changed": changed,
                "baseline_written": write_baseline || !baseline_exists,
                "engine": "http",
            });
            if let (Some(obj), false) = (data.as_object_mut(), diff.is_null()) {
                obj.insert("diff".into(), diff);
            }
            emit_ok(data, json, |d| {
                let ch = d.get("changed").and_then(|v| v.as_bool()).unwrap_or(false);
                crate::output::writeln_stdout(format!("ok monitor check changed={ch}"))?;
                Ok(())
            })
        }
    }
}
