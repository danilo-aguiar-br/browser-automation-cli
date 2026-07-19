// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed human-facing message keys (`Mensagem`) with exhaustive per-locale match.

use super::idioma::Idioma;

/// Stable FTL / catalog id for a [`Mensagem`] variant.
pub fn ftl_id(msg: Mensagem) -> &'static str {
    match msg {
        Mensagem::UsageSuggestion => "usage-suggestion",
        Mensagem::BrokenPipeSuggestion => "broken-pipe-suggestion",
        Mensagem::UnavailableSuggestion => "unavailable-suggestion",
        Mensagem::DataSuggestion => "data-suggestion",
        Mensagem::BrowserSuggestion => "browser-suggestion",
        Mensagem::VisionRequired => "vision-required",
        Mensagem::RobotsDual => "robots-dual",
        Mensagem::CategoryMemory => "category-memory",
        Mensagem::CategoryExtensions => "category-extensions",
        Mensagem::ScreencastFlag => "screencast-flag",
        Mensagem::WebmcpFlag => "webmcp-flag",
        Mensagem::ThirdPartyFlag => "third-party-flag",
        Mensagem::CaptureNetwork => "capture-network",
        Mensagem::CaptureConsole => "capture-console",
        Mensagem::RunFailFast => "run-fail-fast",
        Mensagem::LighthouseMissing => "lighthouse-missing",
        Mensagem::LocaleResolved => "locale-resolved",
        Mensagem::LocaleSource => "locale-source",
    }
}

/// Every human-facing UI string key for this binary.
///
/// Technical `error.message` fields and tracing targets stay English literals
/// outside this enum (agent contract). Variants are named in English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Mensagem {
    /// Usage / argv errors — human suggestion.
    UsageSuggestion,
    /// Broken pipe (exit 141) — human suggestion.
    BrokenPipeSuggestion,
    /// Chrome unavailable — human suggestion.
    UnavailableSuggestion,
    /// Data / robots / payload — human suggestion.
    DataSuggestion,
    /// Browser session failure — human suggestion.
    BrowserSuggestion,
    /// Vision experimental flag required.
    VisionRequired,
    /// Dual robots ignore flags required.
    RobotsDual,
    /// Memory category flag required.
    CategoryMemory,
    /// Extensions category flag required.
    CategoryExtensions,
    /// Screencast experimental flag required.
    ScreencastFlag,
    /// WebMCP category flag required.
    WebmcpFlag,
    /// Third-party category flag required.
    ThirdPartyFlag,
    /// Network capture flag required before net tools.
    CaptureNetwork,
    /// Console capture flag required before console tools.
    CaptureConsole,
    /// Run script failed fast.
    RunFailFast,
    /// Lighthouse binary missing.
    LighthouseMissing,
    /// `locale` subcommand: resolved label.
    LocaleResolved,
    /// `locale` subcommand: source label.
    LocaleSource,
}

impl Mensagem {
    /// All variants (for parity / exhaustiveness tests).
    pub const ALL: &'static [Mensagem] = &[
        Mensagem::UsageSuggestion,
        Mensagem::BrokenPipeSuggestion,
        Mensagem::UnavailableSuggestion,
        Mensagem::DataSuggestion,
        Mensagem::BrowserSuggestion,
        Mensagem::VisionRequired,
        Mensagem::RobotsDual,
        Mensagem::CategoryMemory,
        Mensagem::CategoryExtensions,
        Mensagem::ScreencastFlag,
        Mensagem::WebmcpFlag,
        Mensagem::ThirdPartyFlag,
        Mensagem::CaptureNetwork,
        Mensagem::CaptureConsole,
        Mensagem::RunFailFast,
        Mensagem::LighthouseMissing,
        Mensagem::LocaleResolved,
        Mensagem::LocaleSource,
    ];

    /// Resolve text for an explicit idioma (no process global). Preferred in tests.
    pub fn texto(self, idioma: Idioma) -> &'static str {
        match idioma {
            Idioma::En => super::en::texto(self),
            Idioma::PtBr => super::pt_br::texto(self),
        }
    }

    /// Map a legacy suggestion catalog key to a message (or usage fallback).
    pub fn from_suggestion_key(key: &str) -> Mensagem {
        match key {
            "vision_required" => Mensagem::VisionRequired,
            "robots_dual" => Mensagem::RobotsDual,
            "category_memory" => Mensagem::CategoryMemory,
            "category_extensions" => Mensagem::CategoryExtensions,
            "screencast_flag" => Mensagem::ScreencastFlag,
            "webmcp_flag" => Mensagem::WebmcpFlag,
            "third_party_flag" => Mensagem::ThirdPartyFlag,
            "capture_network" => Mensagem::CaptureNetwork,
            "capture_console" => Mensagem::CaptureConsole,
            "run_fail_fast" => Mensagem::RunFailFast,
            "lighthouse_missing" => Mensagem::LighthouseMissing,
            _ => Mensagem::UsageSuggestion,
        }
    }

    /// Map error kind string to a human suggestion message.
    pub fn from_error_kind(kind: &str) -> Option<Mensagem> {
        match kind {
            "usage" => Some(Mensagem::UsageSuggestion),
            "broken-pipe" => Some(Mensagem::BrokenPipeSuggestion),
            "unavailable" => Some(Mensagem::UnavailableSuggestion),
            "data" => Some(Mensagem::DataSuggestion),
            "browser" => Some(Mensagem::BrowserSuggestion),
            _ => None,
        }
    }
}
