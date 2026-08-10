// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed human-facing message keys (`UiMessage`) with exhaustive per-locale match.
//!
//! # Module map
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `ftl_ids` | variant → stable FTL id |
//! | `mapping` | legacy suggestion / error-kind keys → variant |

use super::ui_locale::UiLocale;

mod ftl_ids;
mod mapping;

pub use ftl_ids::ftl_id;

/// Every human-facing UI string key for this binary.
///
/// Technical `error.message` fields and tracing targets stay English literals
/// outside this enum (agent contract). Variants are named in English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UiMessage {
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
    /// Lighthouse CLI wall-clock timeout.
    LighthouseTimeout,
    /// ffmpeg encode wall-clock timeout.
    FfmpegTimeout,
    /// Rejected Windows script host binary (.bat/.cmd/.ps1).
    BinaryUnsafeWindows,
    /// Coordinates are page CSS pixels; pass --experimental-vision on the same invocation
    VisionCoordinates,
    /// The `--args` payload must be a JSON array of element refs.
    EvalArgsUids,
    /// Usage example for `extract --llm`.
    ExtractLlmUsage,
    /// Name a target, for example: mitm block --host example.com
    MitmBlockTarget,
    /// Discover third-party tool names with `devtools3p list` first.
    Devtools3pListFirst,
    /// Pass a JSON object to --input, for example {\"key\":\"value\"}
    WebmcpInputJson,
    /// Pass a JSON object to --params, for example {\"key\":\"value\"}
    Devtools3pParamsJson,
    /// Pass an existing NDJSON/JSONL or JSON-array file to --script
    RunScriptFile,
    /// Pass valid JavaScript to --init-script
    InitScriptJavascript,
    /// Raise --step-timeout or --timeout, or split the script into fewer steps
    RaiseStepTimeout,
    /// Run workflow run first, or point at an existing journal with --journal
    WorkflowRunFirst,
    /// Pass the cookie array to --cookies-json; the global --json only selects the envelope format
    CookieJsonExample,
    /// Use run --script steps.jsonl so goto and reload --init-script share one process
    GotoRunScript,
    /// Pass --capture-console on the same invocation, for example: --capture-console run --script audit.jsonl
    ConsoleCaptureRun,
    /// Pass an index or --page-id, for example: page select 0
    PageSelectTarget,
    /// A WAF or bot-check served a challenge instead of the requested content.
    BlockedByWaf,
    /// Offline perf insight needs a trace file this product wrote.
    PerfTracePath,
    /// SSRF policy blocked a URL host.
    SsrfBlocked,
    /// HTTP response body exceeded configured max.
    HttpBodyTooLarge,
    /// HTTP connect phase timed out.
    HttpConnectTimeout,
    /// Redis host rejected (non-loopback without override).
    RedisHostBlocked,
    /// Egress proxy URL could not be parsed.
    ProxyUrlInvalid,
    /// `locale` subcommand: resolved label.
    LocaleResolved,
    /// `locale` subcommand: source label.
    LocaleSource,
    /// Pass an absolute http(s) URL with a host (about:blank and file:// only where documented)
    UrlAbsoluteHttp,
    /// Use a CSS selector or an @eN ref from view in the same process
    TargetRefFromView,
    /// Navigate with goto first in the same process, or allow a blank page explicitly
    NavigateFirst,
    /// Pass a JSON array of objects, or NDJSON with one object per line
    JsonArrayObjects,
    /// Pass a single JSON object payload
    JsonObjectPayload,
    /// Raise the byte ceiling via config set, or use a smaller input
    RaiseSizeLimit,
    /// Raise --timeout or --step-timeout, or reduce the work per step
    RaiseTimeout,
    /// Filter grammar for --filter: key=value, key!=value or key~substring
    AgentOpsFilterSyntax,
    /// Row operations need a list this command does not have
    AgentOpsNoRows,
    /// More than one list in data: narrow with --fields first
    AgentOpsManyRows,
    /// The byte ceiling cannot be met even with zero rows
    AgentOpsOverBudget,
    /// The --urls-file list is larger than the configured ceiling
    UrlsFileTooLarge,
    /// Install the binary or set an absolute executable path via config set
    ExternalBinaryPath,
    /// Set the LLM knobs: config set openrouter_api_key|llm_base_url|llm_model
    LlmConfigRequired,
    /// Start redis-server and set cache_redis_url, or: config set cache_backend sqlite
    RedisConfigRequired,
    /// Run: browser-automation-cli config list-keys
    ConfigListKeys,
    /// Use true|false|1|0|yes|no|on|off (case-insensitive); config unset `<key>` restores the default
    ConfigBoolValue,
    /// Use one of the supported values reported in the message
    UseListedValue,
    /// Pass an existing regular file path (not a directory)
    FilePathInvalid,
    /// Pass a path produced by heap take (.heapsnapshot JSON) with a valid node or class id
    HeapSnapshotInput,
    /// Pass an unpacked extension directory containing manifest.json
    ExtensionUnpackedDir,
    /// Run extension list first and pass an id from a loaded extension
    ExtensionListFirst,
    /// Use run --script NDJSON so dependent steps share one process
    RunScriptMultiStep,
    /// Pass a CDP key name such as Enter, Tab, Escape, or ArrowDown
    CdpKeyName,
    /// Trigger the dialog first with a press that opens alert/confirm/prompt
    DialogOpenRequired,
    /// Fix the page console noise or raise the assert threshold
    ConsoleAssertThreshold,
    /// Re-run the command; the previous invocation was interrupted (exit 130)
    RetryAfterCancel,
    /// Remove circular depends_on edges from the manifest
    WorkflowCycle,
    /// List discovery paths separated by the platform path separator
    ChromeSearchPathsFormat,
    /// Check --webhook-url reachability; operator destination only
    WebhookUnreachable,
    /// Use a clear PNG/JPEG of a QR code with a quiet zone
    QrImageQuality,
    /// Provide a real PDF file; generate one with print-pdf if needed
    PdfInputInvalid,
    /// Pass a .csv, .tsv, or .json file holding an array of objects
    SheetInputFormat,
    /// Format: `WxHxDPR[,mobile][,touch][,landscape]`
    ViewportSpecFormat,
    /// Run: browser-automation-cli commands --json to list the live surface
    CommandsDiscovery,
    /// Use: `browser-automation-cli schema <cmd>` or `schema --cmd <cmd>`
    SchemaCommandRequired,
    /// Use --engine http for one-shot baselines, or --engine browser / parse for local files
    ScrapeEngineChoice,
    /// Download the file and read it with parse, or pick a command for this media type
    ScrapeOpaqueContent,
    /// Check the Chrome install and Xvfb availability on Linux headed launches
    ChromeLaunchFailed,
    /// Check the allowed fields for this step cmd in schema run
    StepFieldUnknown,
    /// Ensure the home directory is available for this user
    XdgHomeRequired,
    /// Ensure Chrome exposes HeapProfiler; re-run doctor and check event forwarders
    HeapCaptureFailed,
    /// Read the steps from stdin with `run --script -` instead: shell process substitution exposes the file as `/proc/<pid>/fd/<n>`, which no allowed root can contain
    PathIsProcessSubstitution,
    /// Keep the path under an allowed root, add one with config set allowed_roots, or pass --allow-outside-roots
    PathOutsideRoots,
    /// Pass `--capture-path <file>` to read a capture written by another invocation
    MitmCapturePath,
    /// Drag within one frame, or drive the iframe as its own target
    DragSameFrame,
    /// Pass --to @eN / CSS, or --to-x N --to-y N
    DragDestinationRequired,
    /// Pass the `<form>` itself or any field inside it
    SubmitNeedsForm,
    /// Fill the required fields, or relax the form validation before submitting
    SubmitValidationFailed,
    /// Remove the include that points back into an already-included script
    IncludeCycle,
    /// Flatten the script or reduce include nesting
    IncludeDepth,
    /// Use {"cmd":"include","path":"other.jsonl"}
    IncludePathRequired,
    /// Use {"cmd":"assert","kind":"step","path":"result","equals":"OK"}
    AssertStepPath,
    /// Place the assert after the step whose payload it checks
    AssertStepOrder,
    /// Use one of equals, contains or exists
    AssertStepOperator,
    /// Inspect the previous step payload with --json-steps and adjust path/expected
    AssertStepInspect,
    /// Image input exceeds image_max_input_bytes or image_max_pixels
    ImageTooLarge,
    /// Image magic bytes invalid or unsupported.
    ImageMagicInvalid,
    /// Codec or parser is gated behind a Cargo feature that is off in this build.
    ImageFeatureDisabled,
    /// HEIC encode requested; no pure-Rust HEVC encoder exists.
    ImageHeicEncodeUnavailable,
    /// SVG source refused by the sanitiser (entities, depth, script, href).
    SvgRejected,
    /// Video input exceeds XDG video_max_input_bytes.
    VideoTooLarge,
    /// Video magic bytes invalid or unsupported.
    VideoMagicInvalid,
    /// Site-player stream extraction requested; rejected by product rule.
    VideoSiteExtractionRejected,
    /// An HLS/DASH manifest was fetched where a media file was expected.
    VideoManifestNotAFile,
    /// Video output format unsupported.
    VideoFormatUnsupported,
    /// Codec incompatible with target container.
    VideoCodecContainerMismatch,
    /// ffmpeg/ffprobe binary missing.
    FfmpegMissing,
    /// ffmpeg convert/trim/to-mp3/thumbnail process failed.
    FfmpegFailed,
    /// ffmpeg failed due to path I/O (permission, read-only, not permitted).
    FfmpegIoFailed,
    /// Image format detected but not supported in this build
    ImageFormatUnsupported,
    /// Audio input exceeds XDG audio_max_input_bytes.
    AudioTooLarge,
    /// Audio magic bytes invalid or unsupported.
    AudioMagicInvalid,
    /// Audio output format unsupported.
    AudioFormatUnsupported,
    /// Lossy→lossy recompress warning (suggestion only).
    AudioLossyTranscode,
    /// HTTP scrape returned 4xx/5xx (not success body).
    HttpStatusScrape,
    /// Page blocked by meta robots / X-Robots-Tag noindex.
    MetaRobotsNoindex,
}

impl UiMessage {
    /// All variants (for parity / exhaustiveness tests).
    pub const ALL: &'static [UiMessage] = &[
        UiMessage::UsageSuggestion,
        UiMessage::BrokenPipeSuggestion,
        UiMessage::UnavailableSuggestion,
        UiMessage::DataSuggestion,
        UiMessage::BrowserSuggestion,
        UiMessage::VisionRequired,
        UiMessage::RobotsDual,
        UiMessage::CategoryMemory,
        UiMessage::CategoryExtensions,
        UiMessage::ScreencastFlag,
        UiMessage::WebmcpFlag,
        UiMessage::ThirdPartyFlag,
        UiMessage::CaptureNetwork,
        UiMessage::CaptureConsole,
        UiMessage::RunFailFast,
        UiMessage::LighthouseMissing,
        UiMessage::LighthouseTimeout,
        UiMessage::FfmpegTimeout,
        UiMessage::BinaryUnsafeWindows,
        UiMessage::VisionCoordinates,
        UiMessage::EvalArgsUids,
        UiMessage::ExtractLlmUsage,
        UiMessage::MitmBlockTarget,
        UiMessage::Devtools3pListFirst,
        UiMessage::WebmcpInputJson,
        UiMessage::Devtools3pParamsJson,
        UiMessage::RunScriptFile,
        UiMessage::InitScriptJavascript,
        UiMessage::RaiseStepTimeout,
        UiMessage::WorkflowRunFirst,
        UiMessage::CookieJsonExample,
        UiMessage::GotoRunScript,
        UiMessage::ConsoleCaptureRun,
        UiMessage::PageSelectTarget,
        UiMessage::BlockedByWaf,
        UiMessage::PerfTracePath,
        UiMessage::SsrfBlocked,
        UiMessage::HttpBodyTooLarge,
        UiMessage::HttpConnectTimeout,
        UiMessage::ProxyUrlInvalid,
        UiMessage::RedisHostBlocked,
        UiMessage::LocaleResolved,
        UiMessage::LocaleSource,
        UiMessage::UrlAbsoluteHttp,
        UiMessage::TargetRefFromView,
        UiMessage::NavigateFirst,
        UiMessage::JsonArrayObjects,
        UiMessage::JsonObjectPayload,
        UiMessage::RaiseSizeLimit,
        UiMessage::RaiseTimeout,
        UiMessage::AgentOpsFilterSyntax,
        UiMessage::AgentOpsNoRows,
        UiMessage::AgentOpsManyRows,
        UiMessage::AgentOpsOverBudget,
        UiMessage::UrlsFileTooLarge,
        UiMessage::ExternalBinaryPath,
        UiMessage::LlmConfigRequired,
        UiMessage::RedisConfigRequired,
        UiMessage::ConfigListKeys,
        UiMessage::ConfigBoolValue,
        UiMessage::UseListedValue,
        UiMessage::FilePathInvalid,
        UiMessage::HeapSnapshotInput,
        UiMessage::ExtensionUnpackedDir,
        UiMessage::ExtensionListFirst,
        UiMessage::RunScriptMultiStep,
        UiMessage::CdpKeyName,
        UiMessage::DialogOpenRequired,
        UiMessage::ConsoleAssertThreshold,
        UiMessage::RetryAfterCancel,
        UiMessage::WorkflowCycle,
        UiMessage::ChromeSearchPathsFormat,
        UiMessage::WebhookUnreachable,
        UiMessage::QrImageQuality,
        UiMessage::PdfInputInvalid,
        UiMessage::SheetInputFormat,
        UiMessage::ViewportSpecFormat,
        UiMessage::CommandsDiscovery,
        UiMessage::SchemaCommandRequired,
        UiMessage::ScrapeEngineChoice,
        UiMessage::ScrapeOpaqueContent,
        UiMessage::ChromeLaunchFailed,
        UiMessage::StepFieldUnknown,
        UiMessage::XdgHomeRequired,
        UiMessage::HeapCaptureFailed,
        UiMessage::PathIsProcessSubstitution,
        UiMessage::PathOutsideRoots,
        UiMessage::MitmCapturePath,
        UiMessage::DragSameFrame,
        UiMessage::DragDestinationRequired,
        UiMessage::SubmitNeedsForm,
        UiMessage::SubmitValidationFailed,
        UiMessage::IncludeCycle,
        UiMessage::IncludeDepth,
        UiMessage::IncludePathRequired,
        UiMessage::AssertStepPath,
        UiMessage::AssertStepOrder,
        UiMessage::AssertStepOperator,
        UiMessage::AssertStepInspect,
        UiMessage::ImageTooLarge,
        UiMessage::ImageMagicInvalid,
        UiMessage::ImageFeatureDisabled,
        UiMessage::ImageHeicEncodeUnavailable,
        UiMessage::SvgRejected,
        UiMessage::VideoTooLarge,
        UiMessage::VideoMagicInvalid,
        UiMessage::VideoSiteExtractionRejected,
        UiMessage::VideoManifestNotAFile,
        UiMessage::VideoFormatUnsupported,
        UiMessage::VideoCodecContainerMismatch,
        UiMessage::FfmpegMissing,
        UiMessage::FfmpegFailed,
        UiMessage::FfmpegIoFailed,
        UiMessage::ImageFormatUnsupported,
        UiMessage::AudioTooLarge,
        UiMessage::AudioMagicInvalid,
        UiMessage::AudioFormatUnsupported,
        UiMessage::AudioLossyTranscode,
        UiMessage::HttpStatusScrape,
        UiMessage::MetaRobotsNoindex,
    ];

    /// Resolve text for an explicit UI locale (no process global). Preferred in tests.
    pub fn text(self, ui_locale: UiLocale) -> &'static str {
        match ui_locale {
            UiLocale::En => super::en::text(self),
            UiLocale::PtBr => super::pt_br::text(self),
        }
    }
}
