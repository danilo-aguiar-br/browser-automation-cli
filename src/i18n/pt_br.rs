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
        UiMessage::AgentOpsFilterSyntax => {
            "Use key=value, key!=value ou key~substring (caminhos pontilhados permitidos)"
        }
        UiMessage::AgentOpsNoRows => {
            "Este comando não tem lista; use --fields para projetar campos"
        }
        UiMessage::AgentOpsManyRows => {
            "Estreite para uma lista com --fields <chave>, depois filtre/ordene/limite"
        }
        UiMessage::AgentOpsOverBudget => {
            "Aumente --max-output-bytes, ou estreite o payload com --fields"
        }
        UiMessage::UrlsFileTooLarge => {
            "Divida a lista, ou aumente com: config set max_urls_file_bytes <n>"
        }
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
        UiMessage::PathIsProcessSubstitution => "Leia os passos do stdin com run --script - : process substitution do shell expõe o arquivo como /proc/<pid>/fd/<n>, que nenhuma raiz permitida pode conter",
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
        UiMessage::ImageTooLarge => {
            "Aumente image_max_input_bytes ou image_max_pixels via config set, ou use uma imagem menor"
        }
        UiMessage::ImageMagicInvalid => {
            "Passe um arquivo de imagem real (png/jpeg/webp/gif); magic bytes são verificados, não a extensão"
        }
        UiMessage::ImageFeatureDisabled => {
            "Recompile com a feature Cargo necessária (image-avif, image-heic, image-svg ou media-manifest)"
        }
        UiMessage::ImageHeicEncodeUnavailable => {
            "Converta para png, jpeg, webp, gif ou avif; não existe encoder HEVC puro-Rust, então HEIC é somente decode"
        }
        UiMessage::SvgRejected => {
            "Remova o DOCTYPE, script, handler de evento ou href externo, ou aumente svg_max_entities / svg_max_depth via config set"
        }
        UiMessage::VideoSiteExtractionRejected => {
            "Passe a URL direta da mídia; extrair stream de player de site é rejeitado por regra, não adiado"
        }
        UiMessage::VideoManifestNotAFile => {
            "Parseie o manifesto para escolher uma variante e baixe a URL direta dessa variante"
        }
        UiMessage::VideoTooLarge => {
            "Aumente video_max_input_bytes via config set, ou use um arquivo menor"
        }
        UiMessage::VideoMagicInvalid => {
            "Passe um arquivo de vídeo real (mp4/webm/mkv/…); magic bytes são verificados, não a extensão"
        }
        UiMessage::VideoFormatUnsupported => {
            "Use mp4, webm, mkv, mov, avi ou m4v como --format"
        }
        UiMessage::VideoCodecContainerMismatch => {
            "Escolha codecs permitidos no container de saída (ex.: WebM: vp9+opus; sem H.264 em WebM)"
        }
        UiMessage::FfmpegMissing => {
            "Instale ffmpeg/ffprobe e: config set ffmpeg_path <caminho-absoluto>"
        }
        UiMessage::FfmpegFailed => {
            "Verifique codecs/container, aumente ffmpeg_timeout_secs, ou passe --video-codec/--audio-codec"
        }
        UiMessage::FfmpegIoFailed => {
            "Garanta path de entrada legível e de saída gravável (diretório pai existe; não read-only); verifique permissões"
        }
        UiMessage::ImageFormatUnsupported => {
            "Use png, jpeg, webp ou gif; AVIF é somente encode e HEIC/SVG exigem sua feature Cargo"
        }
        UiMessage::AudioTooLarge => {
            "Aumente audio_max_input_bytes via config set, ou use um arquivo menor"
        }
        UiMessage::AudioMagicInvalid => {
            "Passe um arquivo de áudio real (mp3/wav/flac/ogg/m4a/…); magic bytes são verificados, não a extensão"
        }
        UiMessage::AudioFormatUnsupported => {
            "Use mp3, m4a, aac, ogg, opus, flac ou wav como --format"
        }
        UiMessage::HttpStatusScrape => {
            "Página HTTP de erro não é scrape de sucesso; veja status_code ou use batch com --filter http_error=false"
        }
        UiMessage::MetaRobotsNoindex => {
            "Página declara noindex (meta ou X-Robots-Tag); honre robots ou defina scrape_honor_meta_robots=false de propósito"
        }
        UiMessage::AudioLossyTranscode => {
            "Recompressão lossy→lossy degrada qualidade; prefira fonte lossless ou stream copy quando possível"
        }
    }
}
