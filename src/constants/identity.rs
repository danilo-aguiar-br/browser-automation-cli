// SPDX-License-Identifier: MIT OR Apache-2.0
//! Product identity constants: user agent, temp prefixes, endpoints, wire version.

/// Product HTTP User-Agent for robots, scrape, and optional LLM/webhook clients.
///
/// Single source of truth (`&'static str`, zero runtime alloc). Built from
/// `CARGO_PKG_NAME` / `VERSION` / `HOMEPAGE` so rename/release stay in sync.
pub const HTTP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+",
    env!("CARGO_PKG_HOMEPAGE"),
    "; local-scrape)"
);

/// Temp-file name prefix for atomic xlsx writes (dotfile under parent dir).
pub const XLSX_TMP_NAME_PREFIX: &str = concat!(".", env!("CARGO_PKG_NAME"), "-xlsx-");

/// Executable name of this product, taken from the manifest rather than typed.
///
/// Used by residual classification to tell an owning CLI process apart from the
/// browser it launched. Derived from `CARGO_PKG_NAME` so a rename cannot leave a
/// stale literal behind in the process-table heuristics.
pub const PRODUCT_BIN_NAME: &str = env!("CARGO_PKG_NAME");

/// Default HTML search endpoint template base (query appended as `?q=`).
///
/// Override with `config set search_base_url <url>`. Used only when XDG is unset.
pub const DEFAULT_SEARCH_BASE_URL: &str = "https://html.duckduckgo.com/html/";

/// Wire envelope / agent contract schema version (stdout JSON).
///
/// Named constant (anti-hardcode). Not operator-tunable — bump only with a
/// documented breaking envelope change.
pub const ENVELOPE_SCHEMA_VERSION: u32 = 1;

/// Loopback-only bind host (product law: never `0.0.0.0`).
///
/// Alias used by MITM, Lightpanda serve, and local probes (DRY).
pub const LOOPBACK_HOST: &str = "127.0.0.1";
