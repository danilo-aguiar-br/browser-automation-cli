// SPDX-License-Identifier: MIT OR Apache-2.0
//! PRD command dispatch (path-level + Layer A/B browser one-shot).
//!
//! # Workload
//!
//! **Mista (orchestration):** dispatch is sequential per invocation (one CLI
//! process). Multi-item fan-out lives inside callees (`scrape_local` JoinSet,
//! `sg`/`find-paths` Rayon, CDP `join_bounded`). Browser batch/crawl stays
//! sequential under product law (N-129). Large disk dumps from handlers use
//! [`crate::concurrency::write_bytes_blocking`] (under `block_on`) or
//! [`crate::concurrency::write_bytes_sync`] (outer CLI thread).
//!
//! # Module map (Tier-3 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `mod` (this file) | Build `DispatchCtx`, call `dispatch::route`, finalize |
//! | `dispatch` | Thin exhaustive `route` + family modules (meta/browser/scrape/run_exec/ops) + gates |
//! | `common` | emit helpers, version/locale |
//! | `nav` | browser navigation/interaction handlers |
//! | `scrape` | local scrape/crawl/search/sg/sheet handlers |
//! | `ops` | mitm/workflow/config/perf/heap/completions/man |
//! | `meta` | agent inventory (`commands`, `schema`) |
//! | [`run`] | multi-step script engine (`run` / `exec`) |
//!
//! New **command families** should land in sibling modules and be wired from
//! `dispatch`, rather than growing unrelated helpers in this file.
#![allow(missing_docs)] // clap/handler surface; agent UX is `--help` + skills (D-02/D-11)

pub(crate) mod common;
mod dispatch;
mod local_audio;
mod local_video;
mod meta;
mod nav;
mod ops;
pub mod run;
mod scrape;

use crate::browser::CaptureOpts;
use crate::cli::Cli;
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

use common::emit_err;
use dispatch::{route, DispatchCtx};

/// Dispatch parsed CLI. Returns process exit code (0 success).
pub fn dispatch(cli: Cli, life: &Lifecycle) -> i32 {
    let json = cli.globals.json;
    let capture = CaptureOpts {
        console: cli.globals.capture_console,
        network: cli.globals.capture_network,
    };
    let timeout_secs = crate::config::resolve_global_timeout(cli.globals.timeout);
    let step_timeout_secs =
        crate::config::resolve_step_timeout(cli.globals.step_timeout, timeout_secs);
    crate::fs_roots::set_allow_outside_roots(cli.globals.allow_outside_roots);
    crate::failure_dump::configure(
        cli.globals.dump_on_failure,
        cli.globals.artifacts_dir.clone(),
    );
    let robots =
        match RobotsPolicy::from_flags(cli.globals.ignore_robots, cli.globals.i_accept_robots_risk)
        {
            Ok(p) => p,
            Err(e) => {
                life.finalize();
                return emit_err(&e, json);
            }
        };

    let ctx = DispatchCtx {
        life,
        json,
        capture,
        timeout_secs,
        step_timeout_secs,
        robots,
        category_memory: cli.globals.category_memory,
        category_extensions: cli.globals.category_extensions,
        category_third_party: cli.globals.category_third_party,
        category_webmcp: cli.globals.category_webmcp,
        experimental_screencast: cli.globals.experimental_screencast,
        experimental_vision: cli.globals.experimental_vision,
        json_steps: cli.globals.json_steps,
        artifacts: cli.globals.artifacts_dir.clone(),
    };

    let code = route(cli.command, &ctx);
    life.finalize();
    code
}
