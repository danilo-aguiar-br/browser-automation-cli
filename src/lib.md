# browser-automation-cli

One-shot Chrome CDP automation library and CLI for AI agents.

Lifecycle is always **BORN → EXECUTE → FINALIZE → DIE** in a single process.
There is no daemon, no npm runtime, and no remote telemetry.

## Overview

- Parse argv with clap ([`cli`])
- Dispatch one command or a multi-step `run --script` session
- Launch system Chrome/Chromium through chromiumoxide CDP
- Always attempt FINALIZE (Browser.close, wait, kill fallback)

## Quick Start

```bash
cargo install --path . --locked
browser-automation-cli doctor --offline --quick --json
browser-automation-cli goto https://example.com --json
```

Library entry for embedding or tests:

```no_run
use std::process::ExitCode;

fn main() -> ExitCode {
    browser_automation_cli::run()
}
```

## Features

Cargo features (MVP always includes locale packs `en` + `pt-BR`):

| Feature | Purpose |
|---------|---------|
| `docs-mermaid` | Embed Mermaid diagrams in rustdoc via `aquamarine` (docs.rs / `cargo doc`) |
| `i18n-cjk` | Scaffold for zh-Hans / zh-Hant / ja / ko packs |
| `i18n-rtl` | Scaffold for ar / he (RTL) packs |
| `i18n-europe` | Scaffold for additional European packs |
| `i18n-full` | Enables all optional i18n scaffolds |
| `i18n-pseudo` | Pseudolocalization (dev only) |

Default builds omit `docs-mermaid` so daily `cargo build` / `cargo install` stay free of
the `aquamarine` → `proc-macro-error2` future-incompat warning. docs.rs enables all
features (`all-features = true`), so Mermaid still renders on published docs.
Optional CLI categories are process flags:

- `--category-memory` — deep heap tools
- `--category-extensions` — extension tools
- `--category-third-party` — third-party DevTools helpers
- `--category-webmcp` — webmcp tools
- `--experimental-vision` — coordinate click
- `--experimental-screencast` — screencast export (needs ffmpeg)

## Targets

Documented and tested for:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `aarch64-unknown-linux-musl`

Chrome automation is not supported on `wasm32-unknown-unknown`.

## MSRV

Minimum Supported Rust Version is **1.88.0** (`rust-version` in `Cargo.toml`).

## Graceful shutdown (one-shot)

Detect → signal → await, scoped for a **CLI one-shot** (not a long-lived server):

| Phase | Mechanism |
|-------|-----------|
| Detect | [`browser::shutdown_signal`] — SIGINT/SIGTERM (Unix), Ctrl-C/Break/Close (Windows) |
| Signal | [`lifecycle::Lifecycle`] `CancellationToken` → exit **130** (browser + I/O runtimes) |
| Await | `OneShotSession::shutdown` (Browser.close + wait ≤[`BROWSER_CLOSE_WAIT_SECS`](crate::xdg::policy::policy_u64(crate::xdg::policy::key::BROWSER_CLOSE_WAIT_SECS)) + kill); residual SIGTERM→grace→SIGKILL |
| Pipeline | SIGPIPE default + BrokenPipe → exit **141**; dual flush before DIE |
| Force | Second OS signal runs residual [`lifecycle::Lifecycle::finalize`] |

Daemon-only rules (TaskTracker fleets, SIGHUP reload, readiness probes, `sd_notify`)
are **N/A** by product law.

## Safety

- No remote telemetry is emitted by this crate (no OTEL/OTLP/Sentry)
- Local tracing: stderr by default; optional rotated JSON under XDG state
  (`log_to_file`); see [`tracing_local`]
- Unix paths may call `libc` for signal defaults and last-resort process kill
- Windows paths may use Job Objects (`win_job`) for residual-zero process trees

### docs.rs / rustdoc feature gates (nightly)

- `docs.rs` builds this crate with `--cfg docsrs` (see `[package.metadata.docs.rs]`)
- Under `docsrs`, the crate root enables `#![feature(doc_cfg)]` so
  `#[doc(cfg(...))]` and `#[cfg_attr(docsrs, doc(cfg(...)))]` render platform
  and feature badges on multi-target docs
- **`doc_auto_cfg` is not used**: as of the October 2025 rustdoc consolidation,
  automatic cfg labels live under `doc_cfg` only; enabling the removed
  `doc_auto_cfg` feature gate risks nightly docs.rs failures
- Stable `cargo doc` does not enable `docsrs`; platform items still compile via
  normal `#[cfg(unix)]` / `#[cfg(windows)]` without the experimental feature

## Error handling

Public errors use [`error::CliError`] with sysexits-style exit codes.
JSON agents should parse the envelope from [`envelope`].

## Examples

```no_run
use browser_automation_cli::error::{CliError, ErrorKind};
use browser_automation_cli::exit_code_for;

let err = CliError::new(ErrorKind::Unavailable, "chrome not found");
assert_eq!(exit_code_for(&err), 69);
```

## See also

- Crate README and `docs/HOW_TO_USE.md`
- `docs/schemas/` for JSON contracts
- `skill/browser-automation-cli-en/SKILL.md` for agent skill surface
- Local validation: `scripts/docs-check.sh` (HTML + optional rustdoc JSON; no CI/GHA)
