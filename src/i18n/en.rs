// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exhaustive English (`en`) catalog — no catch-all.

use super::mensagem::Mensagem;

/// Translate `msg` to English. Match is exhaustive (compiler-enforced).
pub fn texto(msg: Mensagem) -> &'static str {
    match msg {
        Mensagem::UsageSuggestion => "Check --help and required arguments",
        Mensagem::BrokenPipeSuggestion => {
            "Do not pipe stdout to a closed consumer; exit 141 is expected"
        }
        Mensagem::UnavailableSuggestion => {
            "Install Chrome/Chromium on PATH or: browser-automation-cli config set chrome_path <path>"
        }
        Mensagem::DataSuggestion => "Check robots.txt or the JSON/NDJSON payload",
        Mensagem::BrowserSuggestion => {
            "Check the URL and whether Chrome stayed alive in this one-shot"
        }
        Mensagem::VisionRequired => "Pass --experimental-vision on the same invocation",
        Mensagem::RobotsDual => {
            "Pass both flags together when you intentionally skip robots.txt"
        }
        Mensagem::CategoryMemory => {
            "Pass --category-memory (heap take/summary/close work without deep graph ops)"
        }
        Mensagem::CategoryExtensions => "Pass --category-extensions on the same invocation",
        Mensagem::ScreencastFlag => "Pass --experimental-screencast on the same invocation",
        Mensagem::WebmcpFlag => "Pass --category-webmcp on the same invocation",
        Mensagem::ThirdPartyFlag => "Pass --category-third-party on the same invocation",
        Mensagem::CaptureNetwork => "Pass --capture-network before run/net",
        Mensagem::CaptureConsole => "Pass --capture-console before run/console",
        Mensagem::RunFailFast => "Fix the failing step; subsequent steps were not executed",
        Mensagem::LighthouseMissing => {
            "Install lighthouse or: browser-automation-cli config set lighthouse_path <path>"
        }
        Mensagem::LocaleResolved => "Resolved UI locale",
        Mensagem::LocaleSource => "Resolution source",
    }
}
