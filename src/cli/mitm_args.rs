// SPDX-License-Identifier: MIT OR Apache-2.0
//! The MITM flag group, split out of `GlobalOpts`.
//!
//! # Why its own module
//!
//! Same reason `agent_ops_args` gives, and the same mechanism: `global.rs` had
//! grown past the 300-line ceiling `scripts/filesize-check.sh` enforces, and the
//! MITM knobs are the largest group in it that shares one subject.
//!
//! `#[command(flatten)]` keeps the argv surface byte-identical — every flag
//! keeps its name, its `global = true` and its `MITM` help heading — so this is
//! a move inside Rust and not a change to the CLI contract.

use clap::{ArgAction, Args, ValueHint};

/// One-shot local MITM proxy options.
#[derive(Debug, Clone, Args)]
pub struct MitmArgs {
    /// Enable one-shot local MITM proxy and route Chrome through it (PRD §5E / GAP-019)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm: bool,

    /// Directory for MITM CA key+cert PEM (default: XDG data)
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help_heading = "MITM"
    )]
    pub mitm_ca_dir: Option<std::path::PathBuf>,

    /// Write HAR 1.2 to this path on FINALIZE when --mitm is active
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        value_hint = ValueHint::FilePath,
        help_heading = "MITM"
    )]
    pub mitm_har: Option<std::path::PathBuf>,

    /// Comma-separated hosts to decrypt (empty = all via proxy)
    #[arg(long, global = true, value_name = "HOSTS", help_heading = "MITM")]
    pub mitm_hosts: Option<String>,

    /// Restate the default: WebSocket frames are always captured under --mitm
    ///
    /// Kept because it reads as an intent, and passing it changes nothing —
    /// the same treatment `--mitm-redact-secrets` above already receives.
    ///
    /// Measured 2026-09-01: `proxy.rs` calls `.with_websocket_handler(handler)`
    /// unconditionally and `handler.rs` pushes every frame with no gate, so the
    /// capture has no off switch. The FIELD had exactly one occurrence in the
    /// tree — this declaration — which is the signature of a flag the parser
    /// accepts and the code never reads.
    ///
    /// Gating the handler on this flag was rejected deliberately: it would make
    /// omitting the flag SILENTLY STOP capture that works today, turning a
    /// documentation defect into a data-loss regression. The defect was the
    /// help text promising a choice that does not exist, and the help text is
    /// what changed.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_ws: bool,

    /// Max body bytes retained per exchange
    #[arg(long, global = true, value_name = "BYTES", help_heading = "MITM")]
    pub mitm_max_body_bytes: Option<usize>,

    /// Drop image/video/audio bodies from MITM capture
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_no_media_bodies: bool,

    /// Redact Authorization/Cookie secrets in MITM captures (already the default)
    ///
    /// Kept because it reads as an intent, and passing it changes nothing:
    /// redaction is on unless `--mitm-no-redact-secrets` turns it off.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_redact_secrets: bool,

    /// Keep Authorization/Cookie values readable in the MITM capture
    ///
    /// The capture is written to disk and read back by an agent, so masking is
    /// the default: forgetting the flag costs a missing header, while the
    /// opposite default would make forgetting it cost a leaked session cookie.
    /// Turn it off only when the secret itself is what you are debugging.
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_no_redact_secrets: bool,
}
