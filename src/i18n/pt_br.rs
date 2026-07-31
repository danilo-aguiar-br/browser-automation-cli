// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exhaustive Brazilian Portuguese (`pt-BR`) catalog — accents required, no catch-all.

use super::ui_message::UiMessage;

/// Translate `msg` to pt-BR. Match is exhaustive (compiler-enforced).
pub fn text(msg: UiMessage) -> &'static str {
    match msg {
        UiMessage::UsageSuggestion => "Confira --help e os argumentos obrigatórios",
        UiMessage::BrokenPipeSuggestion => {
            "Não pipe stdout para consumidor fechado; exit 141 é esperado"
        }
        UiMessage::UnavailableSuggestion => {
            "Instale Chrome/Chromium no PATH ou use: browser-automation-cli config set chrome_path <path>"
        }
        UiMessage::DataSuggestion => "Verifique robots.txt ou o payload JSON/NDJSON",
        UiMessage::BrowserSuggestion => {
            "Verifique URL e se o Chrome ainda está vivo no one-shot"
        }
        UiMessage::VisionRequired => "Passe --experimental-vision na mesma invocação",
        UiMessage::RobotsDual => {
            "Passe as duas flags juntas quando ignorar robots.txt de propósito"
        }
        UiMessage::CategoryMemory => {
            "Passe --category-memory (heap take/summary/close funcionam sem ops de grafo profundo)"
        }
        UiMessage::CategoryExtensions => "Passe --category-extensions na mesma invocação",
        UiMessage::ScreencastFlag => "Passe --experimental-screencast na mesma invocação",
        UiMessage::WebmcpFlag => "Passe --category-webmcp na mesma invocação",
        UiMessage::ThirdPartyFlag => "Passe --category-third-party na mesma invocação",
        UiMessage::CaptureNetwork => "Passe --capture-network antes de run/net",
        UiMessage::CaptureConsole => "Passe --capture-console antes de run/console",
        UiMessage::RunFailFast => {
            "Corrija o passo com falha; os passos seguintes não foram executados"
        }
        UiMessage::LighthouseMissing => {
            "Instale lighthouse ou: browser-automation-cli config set lighthouse_path <path>"
        }
        UiMessage::LighthouseTimeout => {
            "Aumente o timeout: browser-automation-cli config set lighthouse_timeout_secs <segundos>"
        }
        UiMessage::FfmpegTimeout => {
            "Aumente o timeout: browser-automation-cli config set ffmpeg_timeout_secs <segundos>"
        }
        UiMessage::BinaryUnsafeWindows => {
            "Use caminho de binário nativo (não .bat/.cmd/.ps1): config set lighthouse_path|ffmpeg_path <exe>"
        }
        UiMessage::SsrfBlocked => {
            "Use uma URL http(s) pública, ou: config set http_ssrf_mode allow_loopback|off"
        }
        UiMessage::HttpBodyTooLarge => {
            "Aumente scrape_max_body_bytes via config set, ou baixe um recurso menor"
        }
        UiMessage::HttpConnectTimeout => {
            "Aumente o timeout de connect: config set http_connect_timeout_secs <segundos>"
        }
        UiMessage::RedisHostBlocked => {
            "Use redis://127.0.0.1:6379 ou: config set redis_allow_remote true"
        }
        UiMessage::LocaleResolved => "Locale de UI resolvido",
        UiMessage::LocaleSource => "Fonte da resolução",
        UiMessage::UrlAbsoluteHttp => "Passe uma URL http(s) absoluta com host (about:blank e file:// só onde documentado)",
        UiMessage::TargetRefFromView => "Use um seletor CSS ou uma ref @eN de view no mesmo processo",
        UiMessage::NavigateFirst => "Navegue com goto antes no mesmo processo, ou permita página em branco explicitamente",
        UiMessage::JsonArrayObjects => "Passe um array JSON de objetos, ou NDJSON com um objeto por linha",
        UiMessage::JsonObjectPayload => "Passe um payload JSON de objeto único",
        UiMessage::RaiseSizeLimit => "Aumente o teto de bytes via config set, ou use uma entrada menor",
        UiMessage::RaiseTimeout => "Aumente --timeout ou --step-timeout, ou reduza o trabalho por passo",
        UiMessage::ExternalBinaryPath => "Instale o binário ou defina um caminho executável absoluto via config set",
        UiMessage::LlmConfigRequired => "Defina os knobs de LLM: config set openrouter_api_key|llm_base_url|llm_model",
        UiMessage::RedisConfigRequired => "Suba o redis-server e defina cache_redis_url, ou: config set cache_backend sqlite",
        UiMessage::ConfigListKeys => "Execute: browser-automation-cli config list-keys",
        UiMessage::UseListedValue => "Use um dos valores suportados informados na mensagem",
        UiMessage::FilePathInvalid => "Passe um caminho de arquivo regular existente (não um diretório)",
        UiMessage::HeapSnapshotInput => "Passe um caminho produzido por heap take (.heapsnapshot JSON) com node ou class id válido",
        UiMessage::ExtensionUnpackedDir => "Passe um diretório de extensão descompactada contendo manifest.json",
        UiMessage::ExtensionListFirst => "Rode extension list antes e passe um id de extensão carregada",
        UiMessage::RunScriptMultiStep => "Use run --script NDJSON para que passos dependentes dividam um processo",
        UiMessage::CdpKeyName => "Passe um nome de tecla CDP como Enter, Tab, Escape ou ArrowDown",
        UiMessage::DialogOpenRequired => "Dispare o diálogo antes com um press que abra alert/confirm/prompt",
        UiMessage::ConsoleAssertThreshold => "Corrija o ruído de console da página ou aumente o limite do assert",
        UiMessage::RetryAfterCancel => "Rode o comando de novo; a invocação anterior foi interrompida (exit 130)",
        UiMessage::WorkflowCycle => "Remova arestas depends_on circulares do manifesto",
        UiMessage::ChromeSearchPathsFormat => "Liste caminhos de descoberta separados pelo separador de caminho da plataforma",
        UiMessage::WebhookUnreachable => "Verifique se --webhook-url está acessível; destino é do operador",
        UiMessage::QrImageQuality => "Use um PNG/JPEG nítido de QR code com zona de silêncio",
        UiMessage::PdfInputInvalid => "Forneça um arquivo PDF real; gere um com print-pdf se precisar",
        UiMessage::SheetInputFormat => "Passe um arquivo .csv, .tsv ou .json com um array de objetos",
        UiMessage::ViewportSpecFormat => "Formato: WxHxDPR[,mobile][,touch][,landscape]",
        UiMessage::CommandsDiscovery => "Execute: browser-automation-cli commands --json para listar a superfície viva",
        UiMessage::SchemaCommandRequired => "Use: browser-automation-cli schema <cmd> ou schema --cmd <cmd>",
        UiMessage::ScrapeEngineChoice => "Use --engine http para baseline one-shot, ou --engine browser / parse para arquivos locais",
        UiMessage::ChromeLaunchFailed => "Verifique a instalação do Chrome e a disponibilidade de Xvfb em launches headed no Linux",
        UiMessage::StepFieldUnknown => "Confira os campos permitidos para este cmd de passo em schema run",
        UiMessage::XdgHomeRequired => "Garanta que o diretório home esteja disponível para este usuário",
        UiMessage::HeapCaptureFailed => "Garanta que o Chrome exponha HeapProfiler; rode doctor de novo e cheque os forwarders de evento",
        UiMessage::PathOutsideRoots => "Mantenha o caminho sob uma raiz permitida, acrescente uma com config set allowed_roots, ou passe --allow-outside-roots",
        UiMessage::MitmCapturePath => "Passe --capture-path <arquivo> para ler uma captura escrita por outra invocação",
        UiMessage::DragSameFrame => "Arraste dentro de um único frame, ou dirija o iframe como alvo próprio",
        UiMessage::DragDestinationRequired => "Passe --to @eN / CSS, ou --to-x N --to-y N",
        UiMessage::SubmitNeedsForm => "Passe o próprio <form> ou qualquer campo dentro dele",
        UiMessage::SubmitValidationFailed => "Preencha os campos obrigatórios, ou afrouxe a validação do formulário antes de enviar",
        UiMessage::IncludeCycle => "Remova o include que aponta de volta para um script ja incluido",
        UiMessage::IncludeDepth => "Achate o script ou reduza o aninhamento de includes",
        UiMessage::IncludePathRequired => "Use {\"cmd\":\"include\",\"path\":\"outro.jsonl\"}",
        UiMessage::AssertStepPath => "Use {\"cmd\":\"assert\",\"kind\":\"step\",\"path\":\"result\",\"equals\":\"OK\"}",
        UiMessage::AssertStepOrder => "Coloque o assert depois do passo cujo payload ele verifica",
        UiMessage::AssertStepOperator => "Use um entre equals, contains ou exists",
        UiMessage::AssertStepInspect => "Inspecione o payload do passo anterior com --json-steps e ajuste path/expected",
    }
}
