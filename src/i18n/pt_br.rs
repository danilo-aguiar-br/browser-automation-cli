// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exhaustive Brazilian Portuguese (`pt-BR`) catalog — accents required, no catch-all.

use super::mensagem::Mensagem;

/// Translate `msg` to pt-BR. Match is exhaustive (compiler-enforced).
pub fn texto(msg: Mensagem) -> &'static str {
    match msg {
        Mensagem::UsageSuggestion => "Confira --help e os argumentos obrigatórios",
        Mensagem::BrokenPipeSuggestion => {
            "Não pipe stdout para consumidor fechado; exit 141 é esperado"
        }
        Mensagem::UnavailableSuggestion => {
            "Instale Chrome/Chromium no PATH ou use: browser-automation-cli config set chrome_path <path>"
        }
        Mensagem::DataSuggestion => "Verifique robots.txt ou o payload JSON/NDJSON",
        Mensagem::BrowserSuggestion => {
            "Verifique URL e se o Chrome ainda está vivo no one-shot"
        }
        Mensagem::VisionRequired => "Passe --experimental-vision na mesma invocação",
        Mensagem::RobotsDual => {
            "Passe as duas flags juntas quando ignorar robots.txt de propósito"
        }
        Mensagem::CategoryMemory => {
            "Passe --category-memory (heap take/summary/close funcionam sem ops de grafo profundo)"
        }
        Mensagem::CategoryExtensions => "Passe --category-extensions na mesma invocação",
        Mensagem::ScreencastFlag => "Passe --experimental-screencast na mesma invocação",
        Mensagem::WebmcpFlag => "Passe --category-webmcp na mesma invocação",
        Mensagem::ThirdPartyFlag => "Passe --category-third-party na mesma invocação",
        Mensagem::CaptureNetwork => "Passe --capture-network antes de run/net",
        Mensagem::CaptureConsole => "Passe --capture-console antes de run/console",
        Mensagem::RunFailFast => {
            "Corrija o passo com falha; os passos seguintes não foram executados"
        }
        Mensagem::LighthouseMissing => {
            "Instale lighthouse ou: browser-automation-cli config set lighthouse_path <path>"
        }
        Mensagem::LocaleResolved => "Locale de UI resolvido",
        Mensagem::LocaleSource => "Fonte da resolução",
    }
}
