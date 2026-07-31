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
        UiMessage::LocaleResolved => "Resolved UI locale",
        UiMessage::LocaleSource => "Resolution source",
        UiMessage::UrlAbsoluteHttp => "Pass an absolute http(s) URL with a host (about:blank and file:// only where documented)",
        UiMessage::TargetRefFromView => "Use a CSS selector or an @eN ref from view in the same process",
        UiMessage::NavigateFirst => "Navigate with goto first in the same process, or allow a blank page explicitly",
        UiMessage::JsonArrayObjects => "Pass a JSON array of objects, or NDJSON with one object per line",
        UiMessage::JsonObjectPayload => "Pass a single JSON object payload",
        UiMessage::RaiseSizeLimit => "Raise the byte ceiling via config set, or use a smaller input",
        UiMessage::RaiseTimeout => "Raise --timeout or --step-timeout, or reduce the work per step",
        UiMessage::ExternalBinaryPath => "Install the binary or set an absolute executable path via config set",
        UiMessage::LlmConfigRequired => "Set the LLM knobs: config set openrouter_api_key|llm_base_url|llm_model",
        UiMessage::RedisConfigRequired => "Start redis-server and set cache_redis_url, or: config set cache_backend sqlite",
        UiMessage::ConfigListKeys => "Run: browser-automation-cli config list-keys",
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
        UiMessage::ChromeLaunchFailed => "Check the Chrome install and Xvfb availability on Linux headed launches",
        UiMessage::StepFieldUnknown => "Check the allowed fields for this step cmd in schema run",
        UiMessage::XdgHomeRequired => "Ensure the home directory is available for this user",
        UiMessage::HeapCaptureFailed => "Ensure Chrome exposes HeapProfiler; re-run doctor and check event forwarders",
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
    }
}
