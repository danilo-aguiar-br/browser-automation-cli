// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenshot public types and options.
use serde::Serialize;

#[derive(Debug, Clone)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct RawAnnotation {
    pub ref_id: String,
    pub number: u64,
    pub role: String,
    pub name: Option<String>,
    pub rect: Rect,
}

/// Rectangle of an annotation, in whole image pixels.
///
/// Integer, unlike the float `Rect` used while measuring: this is what goes
/// into the envelope, and a fractional pixel means nothing to a consumer
/// drawing on the saved image.
#[derive(Debug, Clone, Serialize)]
pub struct AnnotationBox {
    /// Left edge, pixels from the image origin.
    pub x: i64,
    /// Top edge, pixels from the image origin.
    pub y: i64,
    /// Width in pixels.
    pub width: i64,
    /// Height in pixels.
    pub height: i64,
}

/// One numbered overlay drawn on an annotated screenshot.
///
/// Annotations are what let an agent connect a pixel it can see to a `@eN` it
/// can act on, so every field here exists to make that link readable.
#[derive(Debug, Clone)]
pub struct ScreenshotAnnotation {
    /// Ref this overlay points at, so the agent can `press` it afterwards.
    pub ref_id: String,
    /// Badge number printed on the image, matching the legend.
    pub number: u64,
    /// Accessibility role, for the legend entry.
    pub role: String,
    /// Accessible name, when the element has one.
    pub name: Option<String>,
    /// Where the badge sits. Trailing underscore because `box` is a Rust keyword.
    pub box_: AnnotationBox,
}

/// Outcome of a capture: where it landed and what is on it.
#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    /// Path the image was written to.
    pub path: String,
    /// Same image inline, base64-encoded, for a consumer that will not read disk.
    pub base64: String,
    /// Overlays drawn on the image. Empty unless annotation was requested.
    pub annotations: Vec<ScreenshotAnnotation>,
}

/// What to capture and how to encode it.
#[derive(Debug, Clone)]
pub struct ScreenshotOptions {
    /// Element to clip to. `None` captures the viewport or the full page.
    pub selector: Option<String>,
    /// Destination file. `None` lets the caller derive one under `output_dir`.
    pub path: Option<String>,
    /// Capture the whole scrollable document instead of the viewport.
    pub full_page: bool,
    /// Encoding: `png`, `jpeg` or `webp`.
    pub format: String,
    /// Lossy quality 0..=100. Ignored by `png`, which is lossless.
    pub quality: Option<i32>,
    /// Draw numbered overlays over the snapshot refs.
    pub annotate: bool,
    /// Directory used when `path` is absent.
    pub output_dir: Option<String>,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            selector: None,
            path: None,
            full_page: false,
            format: "png".to_string(),
            quality: None,
            annotate: false,
            output_dir: None,
        }
    }
}

impl Serialize for ScreenshotAnnotation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("ScreenshotAnnotation", 5)?;
        state.serialize_field("ref", &self.ref_id)?;
        state.serialize_field("number", &self.number)?;
        state.serialize_field("role", &self.role)?;
        if let Some(name) = &self.name {
            state.serialize_field("name", name)?;
        }
        state.serialize_field("box", &self.box_)?;
        state.end()
    }
}
