// SPDX-License-Identifier: MIT OR Apache-2.0
//! EXIF read via kamadak-exif (pure Rust; GPS omitted by default).

use std::collections::BTreeMap;
use std::io::Cursor;

use crate::error::{CliError, ErrorKind};

/// Tags treated as GPS / location (stripped unless `include_gps`).
const GPS_TAG_NAMES: &[&str] = &[
    "GPSLatitude",
    "GPSLongitude",
    "GPSAltitude",
    "GPSLatitudeRef",
    "GPSLongitudeRef",
    "GPSAltitudeRef",
    "GPSTimeStamp",
    "GPSDateStamp",
    "GPSImgDirection",
    "GPSImgDirectionRef",
    "GPSProcessingMethod",
    "GPSAreaInformation",
    "GPSDestLatitude",
    "GPSDestLongitude",
    "GPSVersionID",
    "GPSSatellites",
    "GPSStatus",
    "GPSMeasureMode",
    "GPSDOP",
    "GPSSpeed",
    "GPSSpeedRef",
    "GPSTrack",
    "GPSTrackRef",
    "GPSMapDatum",
    "GPSDestLatitudeRef",
    "GPSDestLongitudeRef",
    "GPSDestBearing",
    "GPSDestBearingRef",
    "GPSDestDistance",
    "GPSDestDistanceRef",
    "GPSDifferential",
    "GPSHPositioningError",
];

/// Read a compact EXIF tag map from image bytes. Empty map if no EXIF.
pub fn read_exif_map(
    bytes: &[u8],
    include_gps: bool,
) -> Result<BTreeMap<String, String>, CliError> {
    let mut cursor = Cursor::new(bytes);
    let exifreader = exif::Reader::new();
    let exif = match exifreader.read_from_container(&mut cursor) {
        Ok(e) => e,
        Err(exif::Error::NotFound(_)) => return Ok(BTreeMap::new()),
        Err(e) => {
            // Non-fatal for info: many PNGs have no EXIF.
            tracing::debug!(error = %e, "exif parse skipped");
            return Ok(BTreeMap::new());
        }
    };
    let mut map = BTreeMap::new();
    for f in exif.fields() {
        let tag = f.tag.to_string();
        if !include_gps && is_gps_tag(&tag) {
            continue;
        }
        // Prefer display value; cap length for agent envelopes.
        let mut val = f.display_value().with_unit(&exif).to_string();
        if val.len() > 256 {
            val.truncate(256);
            val.push('…');
        }
        map.insert(tag, val);
    }
    if !include_gps && map.keys().any(|k| is_gps_tag(k)) {
        // belt-and-suspenders
        map.retain(|k, _| !is_gps_tag(k));
    }
    let _ = ErrorKind::Data; // keep import used if empty path changes
    Ok(map)
}

fn is_gps_tag(name: &str) -> bool {
    GPS_TAG_NAMES.contains(&name) || name.starts_with("GPS")
}
