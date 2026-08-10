// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exhaustive English (`en`) catalog — no catch-all.

use super::ui_message::UiMessage;

/// Translate `msg` to English. Match is exhaustive (compiler-enforced).
pub fn text(msg: UiMessage) -> &'static str {
    match msg {
        UiMessage::UsageSuggestion => "Check --help and required arguments",
        UiMessage::BrokenPipeSuggestion => {
            "Do not pipe stdout to a closed consumer; exit 141 is expected"
        }
        UiMessage::UnavailableSuggestion => {
            "Install Chrome/Chromium on PATH or: browser-automation-cli config set chrome_path <path>"
        }
        UiMessage::DataSuggestion => "Check robots.txt or the JSON/NDJSON payload",
        UiMessage::BrowserSuggestion => {
            "Check the URL and whether Chrome stayed alive in this one-shot"
        }
        UiMessage::VisionRequired => "Pass --experimental-vision on the same invocation",
        UiMessage::RobotsDual => {
            "Pass both flags together when you intentionally skip robots.txt"
        }
        UiMessage::CategoryMemory => {
            "Pass --category-memory (heap take/summary/close work without deep graph ops)"
        }
        UiMessage::CategoryExtensions => "Pass --category-extensions on the same invocation",
        UiMessage::ScreencastFlag => "Pass --experimental-screencast on the same invocation",
        UiMessage::WebmcpFlag => "Pass --category-webmcp on the same invocation",
        UiMessage::ThirdPartyFlag => "Pass --category-third-party on the same invocation",
        UiMessage::CaptureNetwork => "Pass --capture-network before run/net",
        UiMessage::CaptureConsole => "Pass --capture-console before run/console",
        UiMessage::RunFailFast => "Fix the failing step; subsequent steps were not executed",
        UiMessage::LighthouseMissing => {
            "Install lighthouse or: browser-automation-cli config set lighthouse_path <path>"
        }
        UiMessage::LighthouseTimeout => {
            "Increase timeout: browser-automation-cli config set lighthouse_timeout_secs <seconds>"
        }
        UiMessage::FfmpegTimeout => {
            "Increase timeout: browser-automation-cli config set ffmpeg_timeout_secs <seconds>"
        }
        UiMessage::BinaryUnsafeWindows => {
            "Use a native binary path (not .bat/.cmd/.ps1): config set lighthouse_path|ffmpeg_path <exe>"
        }
        UiMessage::PerfTracePath => "Pass a trace file written by `perf stop --path`",
        UiMessage::VisionCoordinates => "Coordinates are page CSS pixels; pass --experimental-vision on the same invocation",
        UiMessage::EvalArgsUids => "Pass a JSON array of refs to --args, for example [\"@e1\",\"@e2\"]",
        UiMessage::ExtractLlmUsage => "Example: extract --llm --question 'sum' https://example.com",
        UiMessage::MitmBlockTarget => "Name a target, for example: mitm block --host example.com",
        UiMessage::Devtools3pListFirst => "Run devtools3p list --url <page> with --category-third-party to see the tool names",
        UiMessage::WebmcpInputJson => "Pass a JSON object to --input, with the tool input as key/value pairs",
        UiMessage::Devtools3pParamsJson => "Pass a JSON object to --params, with the tool parameters as key/value pairs",
        UiMessage::RunScriptFile => "Pass an existing NDJSON/JSONL or JSON-array file to --script",
        UiMessage::InitScriptJavascript => "Pass valid JavaScript to --init-script",
        UiMessage::RaiseStepTimeout => "Raise --step-timeout or --timeout, or split the script into fewer steps",
        UiMessage::WorkflowRunFirst => "Run workflow run first, or point at an existing journal with --journal",
        UiMessage::CookieJsonExample => "Pass the cookie array to --cookies-json; the global --json only selects the envelope format",
        UiMessage::GotoRunScript => "Use run --script steps.jsonl so goto and reload --init-script share one process",
        UiMessage::ConsoleCaptureRun => "Pass --capture-console on the same invocation, for example: --capture-console run --script audit.jsonl",
        UiMessage::PageSelectTarget => "Pass an index or --page-id, for example: page select 0",
        UiMessage::BlockedByWaf => {
            "A bot check answered instead of the page; see data.block_detection. \
             Retrying escalates toward a ban: use --engine browser, change egress with --proxy, or wait"
        }
        UiMessage::SsrfBlocked => {
            "Use a public http(s) URL, or: config set http_ssrf_mode allow_loopback|off"
        }
        UiMessage::HttpBodyTooLarge => {
            "Raise scrape_max_body_bytes via config set, or fetch a smaller resource"
        }
        UiMessage::HttpConnectTimeout => {
            "Raise connect timeout: config set http_connect_timeout_secs <seconds>"
        }
        UiMessage::RedisHostBlocked => {
            "Use redis://127.0.0.1:6379 or: config set redis_allow_remote true"
        }
        UiMessage::ProxyUrlInvalid => {
            "Use http://, https://, socks5:// or socks5h:// with host and port"
        }
        UiMessage::LocaleResolved => "Resolved UI locale",
        UiMessage::LocaleSource => "Resolution source",
        UiMessage::UrlAbsoluteHttp => "Pass an absolute http(s) URL with a host (about:blank and file:// only where documented)",
        UiMessage::TargetRefFromView => "Use a CSS selector or an @eN ref from view in the same process",
        UiMessage::NavigateFirst => "Navigate with goto first in the same process, or allow a blank page explicitly",
        UiMessage::JsonArrayObjects => "Pass a JSON array of objects, or NDJSON with one object per line",
        UiMessage::JsonObjectPayload => "Pass a single JSON object payload",
        UiMessage::RaiseSizeLimit => "Raise the byte ceiling via config set, or use a smaller input",
        UiMessage::RaiseTimeout => "Raise --timeout or --step-timeout, or reduce the work per step",
        UiMessage::AgentOpsFilterSyntax => {
            "Use key=value, key!=value or key~substring (dotted paths allowed)"
        }
        UiMessage::AgentOpsNoRows => {
            "This command has no list; use --fields to project fields instead"
        }
        UiMessage::AgentOpsManyRows => {
            "Narrow to one list with --fields <key>, then filter/sort/limit it"
        }
        UiMessage::AgentOpsOverBudget => {
            "Raise --max-output-bytes, or narrow the payload with --fields"
        }
        UiMessage::UrlsFileTooLarge => {
            "Split the list, or raise it with: config set max_urls_file_bytes <n>"
        }
        UiMessage::ExternalBinaryPath => "Install the binary or set an absolute executable path via config set",
        UiMessage::LlmConfigRequired => "Set the LLM knobs: config set openrouter_api_key|llm_base_url|llm_model",
        UiMessage::RedisConfigRequired => "Start redis-server and set cache_redis_url, or: config set cache_backend sqlite",
        UiMessage::ConfigListKeys => "Run: browser-automation-cli config list-keys",
        UiMessage::ConfigBoolValue => "Use true|false|1|0|yes|no|on|off (case-insensitive); config unset <key> restores the default",
        UiMessage::UseListedValue => "Use one of the supported values reported in the message",
        UiMessage::FilePathInvalid => "Pass an existing regular file path (not a directory)",
        UiMessage::HeapSnapshotInput => "Pass a path produced by heap take (.heapsnapshot JSON) with a valid node or class id",
        UiMessage::ExtensionUnpackedDir => "Pass an unpacked extension directory containing manifest.json",
        UiMessage::ExtensionListFirst => "Run extension list first and pass an id from a loaded extension",
        UiMessage::RunScriptMultiStep => "Use run --script NDJSON so dependent steps share one process",
        UiMessage::CdpKeyName => "Pass a CDP key name such as Enter, Tab, Escape, or ArrowDown",
        UiMessage::DialogOpenRequired => "Trigger the dialog first with a press that opens alert/confirm/prompt",
        UiMessage::ConsoleAssertThreshold => "Fix the page console noise or raise the assert threshold",
        UiMessage::RetryAfterCancel => "Re-run the command; the previous invocation was interrupted (exit 130)",
        UiMessage::WorkflowCycle => "Remove circular depends_on edges from the manifest",
        UiMessage::ChromeSearchPathsFormat => "List discovery paths separated by the platform path separator",
        UiMessage::WebhookUnreachable => "Check --webhook-url reachability; operator destination only",
        UiMessage::QrImageQuality => "Use a clear PNG/JPEG of a QR code with a quiet zone",
        UiMessage::PdfInputInvalid => "Provide a real PDF file; generate one with print-pdf if needed",
        UiMessage::SheetInputFormat => "Pass a .csv, .tsv, or .json file holding an array of objects",
        UiMessage::ViewportSpecFormat => "Format: WxHxDPR[,mobile][,touch][,landscape]",
        UiMessage::CommandsDiscovery => "Run: browser-automation-cli commands --json to list the live surface",
        UiMessage::SchemaCommandRequired => "Use: browser-automation-cli schema <cmd> or schema --cmd <cmd>",
        UiMessage::ScrapeEngineChoice => "Use --engine http for one-shot baselines, or --engine browser / parse for local files",
        UiMessage::ScrapeOpaqueContent => "Download the file first, then read it with parse; or use image / video / audio for that media type",
        UiMessage::ChromeLaunchFailed => "Check the Chrome install and Xvfb availability on Linux headed launches",
        UiMessage::StepFieldUnknown => "Check the allowed fields for this step cmd in schema run",
        UiMessage::XdgHomeRequired => "Ensure the home directory is available for this user",
        UiMessage::HeapCaptureFailed => "Ensure Chrome exposes HeapProfiler; re-run doctor and check event forwarders",
        UiMessage::PathIsProcessSubstitution => "Read the steps from stdin with run --script - instead: shell process substitution exposes the file as /proc/<pid>/fd/<n>, which no allowed root can contain",
        UiMessage::PathOutsideRoots => "Keep the path under an allowed root, add one with config set allowed_roots, or pass --allow-outside-roots",
        UiMessage::MitmCapturePath => "Pass --capture-path <file> to read a capture written by another invocation",
        UiMessage::DragSameFrame => "Drag within one frame, or drive the iframe as its own target",
        UiMessage::DragDestinationRequired => "Pass --to @eN / CSS, or --to-x N --to-y N",
        UiMessage::SubmitNeedsForm => "Pass the <form> itself or any field inside it",
        UiMessage::SubmitValidationFailed => "Fill the required fields, or relax the form validation before submitting",
        UiMessage::IncludeCycle => "Remove the include that points back into an already-included script",
        UiMessage::IncludeDepth => "Flatten the script or reduce include nesting",
        UiMessage::IncludePathRequired => "Use {\"cmd\":\"include\",\"path\":\"other.jsonl\"}",
        UiMessage::AssertStepPath => "Use {\"cmd\":\"assert\",\"kind\":\"step\",\"path\":\"result\",\"equals\":\"OK\"}",
        UiMessage::AssertStepOrder => "Place the assert after the step whose payload it checks",
        UiMessage::AssertStepOperator => "Use one of equals, contains or exists",
        UiMessage::AssertStepInspect => "Inspect the previous step payload with --json-steps and adjust path/expected",
        UiMessage::ImageTooLarge => {
            "Raise image_max_input_bytes or image_max_pixels via config set, or use a smaller image"
        }
        UiMessage::ImageMagicInvalid => {
            "Pass a real image file (png/jpeg/webp/gif); magic bytes are checked, not the extension"
        }
        UiMessage::ImageFeatureDisabled => {
            "Rebuild with the required Cargo feature (image-avif, image-heic, image-svg or media-manifest)"
        }
        UiMessage::ImageHeicEncodeUnavailable => {
            "Encode to png, jpeg, webp, gif or avif; no pure-Rust HEVC encoder exists, so HEIC is decode-only"
        }
        UiMessage::SvgRejected => {
            "Remove the DOCTYPE, script, event handler or external href, or raise svg_max_entities / svg_max_depth via config set"
        }
        UiMessage::VideoSiteExtractionRejected => {
            "Pass the direct media URL; extracting streams from site players is rejected by rule, not deferred"
        }
        UiMessage::VideoManifestNotAFile => {
            "Parse the manifest to pick a variant, then download that variant's direct URL"
        }
        UiMessage::VideoTooLarge => {
            "Raise video_max_input_bytes via config set, or use a smaller file"
        }
        UiMessage::VideoMagicInvalid => {
            "Pass a real video file (mp4/webm/mkv/…); magic bytes are checked, not the extension"
        }
        UiMessage::VideoFormatUnsupported => {
            "Use mp4, webm, mkv, mov, avi, or m4v as --format"
        }
        UiMessage::VideoCodecContainerMismatch => {
            "Pick codecs allowed for the output container (e.g. WebM: vp9+opus; no H.264 in WebM)"
        }
        UiMessage::FfmpegMissing => {
            "Install ffmpeg/ffprobe and: config set ffmpeg_path <absolute-path>"
        }
        UiMessage::FfmpegFailed => {
            "Check codecs/container, raise ffmpeg_timeout_secs, or pass explicit --video-codec/--audio-codec"
        }
        UiMessage::FfmpegIoFailed => {
            "Ensure input is readable and output path is writable (parent dir exists; not read-only); check permissions"
        }
        UiMessage::ImageFormatUnsupported => {
            "Use png, jpeg, webp or gif; AVIF is encode-only and HEIC/SVG need their Cargo feature"
        }
        UiMessage::AudioTooLarge => {
            "Raise audio_max_input_bytes via config set, or use a smaller file"
        }
        UiMessage::AudioMagicInvalid => {
            "Pass a real audio file (mp3/wav/flac/ogg/m4a/…); magic bytes are checked, not the extension"
        }
        UiMessage::AudioFormatUnsupported => {
            "Use mp3, m4a, aac, ogg, opus, flac, or wav as --format"
        }
        UiMessage::HttpStatusScrape => {
            "HTTP error page is not scraped as success; check status_code or use batch with --filter http_error=false"
        }
        UiMessage::MetaRobotsNoindex => {
            "Page declares noindex (meta or X-Robots-Tag); honor robots or set scrape_honor_meta_robots=false intentionally"
        }
        UiMessage::AudioLossyTranscode => {
            "Lossy→lossy recompress degrades quality; prefer lossless source or stream copy when possible"
        }
    }
}
