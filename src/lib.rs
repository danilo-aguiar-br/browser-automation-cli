// SPDX-License-Identifier: MIT OR Apache-2.0
//! Crate manual lives in `src/lib.md` so this file stays a module index.
//!
//! The include keeps rustdoc resolution at the crate root, so intra-doc
//! links inside the manual behave exactly as they did inline.
#![doc = include_str!("lib.md")]
// docs.rs / nightly: `doc_cfg` only (rules_rust_docsrs — no `doc_auto_cfg`).
#![cfg_attr(docsrs, feature(doc_cfg))]
// `serde_json::json!` recurses once per nesting level, and rustc's default
// ceiling of 128 is an arbitrary compiler guard, not a design limit. The XDG
// key catalog in `xdg::config_ops::keys` is one long array literal of shallow
// objects; it crossed the ceiling when the Wave 6 media knobs landed and failed
// the build with "recursion limit reached while expanding `json_internal!`".
//
// Raising the ceiling is the fix serde_json documents. The alternative —
// shredding one readable catalog into arbitrary chunks to appease a macro
// expander — trades legibility for nothing. Entries are still pushed rather
// than nested where that keeps expansion flat.
#![recursion_limit = "256"]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]
#![warn(rustdoc::redundant_explicit_links)]
// Document every `unsafe` block (rules: English + crates.io safety docs).
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::multiple_unsafe_ops_per_block)]
#![deny(unsafe_op_in_unsafe_fn)]
// Const / static hygiene (rules_rust_const_static_inicializacao).
// Deny: `static mut` refs + interior-mutable `const` (silent duplication bugs).
#![deny(static_mut_refs)]
#![deny(clippy::declare_interior_mutable_const)]
#![deny(clippy::borrow_interior_mutable_const)]
// Ownership / borrowing hygiene (rules_rust_ownership_borrowing_lifetimes).
#![warn(clippy::redundant_clone)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::ptr_arg)]
#![warn(clippy::implicit_clone)]
#![warn(clippy::unnecessary_to_owned)]
#![warn(clippy::cloned_instead_of_copied)]
#![warn(clippy::map_clone)]
#![warn(clippy::mut_mut)]
#![warn(clippy::needless_lifetimes)]

/// Chrome one-shot session: launch, actions, reap.
pub mod agent_context;
pub mod agent_ops;
/// One-shot local audio pipeline (probe/download/convert/trim; no Chrome).
pub mod audio_local;
/// Browser session lifecycle, CDP commands, and shutdown signal wiring.
pub mod browser;

/// Process-level browser policy: window mode, stealth, and egress.
pub mod browser_policy;
/// HTTP/parse cache under XDG (one-shot L1 + SQLite L2).
pub mod cache;
/// Declarative capability / precondition table for command gating.
pub mod capability;
/// Clap derive surface and global flags.
pub mod cli;
/// Injectable wall clock for deterministic tests.
pub mod clock;
/// ANSI color helpers for human stderr diagnostics.
pub mod color;
/// PRD command dispatch (meta paths and browser one-shot).
pub mod commands;
/// Bounded parallelism budget (`--max-concurrency`, Semaphore, Rayon, join_bounded).
pub mod concurrency;
/// Config surface (re-export of XDG; layout name for clap rules).
pub mod config;
/// Shared constants (schema version, product name).
pub mod constants;
/// Local install diagnostics (`doctor`).
pub mod doctor;
pub mod envelope;
/// Typed CLI errors and exit codes.
pub mod error;
/// JSON success/error envelopes for agents.
pub mod failure_dump;
/// One-shot filesystem path discovery (`find-paths`).
pub mod find_paths;
pub mod fs_roots;
/// Locale messaging helpers.
pub mod i18n;
/// One-shot local image pipeline (probe/convert/resize/download/exif; no Chrome).
pub mod image_local;
/// Install path helpers for doctor and packaging checks.
pub mod install;
/// Shared JSON / NDJSON helpers (BOM strip, size ceilings, compact encode).
pub mod json_util;
/// Cooperative cancel and FINALIZE ledger.
pub mod lifecycle;
/// Optional one-shot LLM HTTP extract (XDG key only).
pub mod llm_local;
/// Local MITM capture, CA, and HAR export (one-shot).
pub mod mitm_local;
/// Native CDP stack (browser, network, snapshot, heap).
pub mod native;
/// Network policy (SSRF, body caps, loopback addressing).
pub mod net;
/// Canonical stdout/stderr writers (BrokenPipe → 141, explicit flush).
pub mod output;
/// Cross-platform host helpers (PATH, console UTF-8/VT, sandbox, WSL/container).
pub mod platform;
/// One-shot QR encode/decode (no Chrome).
pub mod qr_local;
/// Owned residual path discovery (CLI marker + chromium tmp).
pub mod residual;
/// Named retry policies with backoff and jitter.
pub mod retry;
/// robots.txt policy enforcement.
pub mod robots;
/// Tokio runtime builders (budgeted multi-thread workers for CDP + HTTP fan-out).
pub mod runtime_util;
/// Local scrape/crawl/map/search/parse (HTTP + files; one-shot).
pub mod scrape_local;
/// Structural lint scan/rewrite one-shot (§5AC).
pub mod sg_local;
/// XLSX write-only path via rust_xlsxwriter (§5Z).
pub mod sheet_local;
/// Shared `std::sync` helpers (poison recovery; short critical sections).
pub mod sync_util;
/// Local tracing init (stderr + optional XDG rotated JSON; no remote export).
pub mod tracing_local;
/// Input and path validation helpers.
pub mod validation;
/// One-shot local video pipeline (probe/download/convert/to-mp3; no Chrome).
pub mod video_local;
/// Windows Job Object helpers (stubs on non-Windows).
pub mod win_job;
/// Workflow journal DAG (petgraph + SQLite), one-shot run/resume.
pub mod workflow_local;
/// XDG Base Directory paths and config file (no `.env` at runtime).
pub mod xdg;

#[cfg(test)]
pub mod test_utils;

mod entry;

pub use entry::{build_identity, command_factory_debug_assert, exit_code_for, run, run_from_args};
