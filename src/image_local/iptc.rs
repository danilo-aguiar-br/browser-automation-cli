// SPDX-License-Identifier: MIT OR Apache-2.0
//! IPTC IIM reader (GAP-IMG-097), written from scratch over the Photoshop 8BIM
//! block carried in a JPEG `APP13` segment.
//!
//! No crate is used here on purpose. The only published option, `gamut-iptc`,
//! has a few hundred downloads and no production track record, while the IIM
//! wire format itself is a fixed 5-byte record header that has not moved since
//! the 1990s. Parsing it directly is smaller than the dependency.
//!
//! # Format
//!
//! ```text
//! APP13 segment  : FF ED <len:u16be> "Photoshop 3.0\0" <8BIM blocks>
//! 8BIM block     : "8BIM" <id:u16be> <pascal name, padded even> <len:u32be> <data, padded even>
//! IIM dataset    : 1C <record:u8> <dataset:u8> <len:u16be> <value>
//! ```
//!
//! Only block id `0x0404` (`IptcNaa`) carries IIM datasets; every other 8BIM
//! block is skipped.

use std::collections::BTreeMap;

use crate::error::CliError;

/// Max bytes kept per IPTC value in an envelope (mirrors the EXIF cap).
const MAX_VALUE_BYTES: usize = 256;

/// Photoshop resource id whose payload is an IIM dataset stream.
const BLOCK_ID_IPTC_NAA: u16 = 0x0404;

/// IIM application record; the only record with agent-relevant metadata.
const RECORD_APPLICATION: u8 = 2;

/// Human names for the IIM application-record datasets we surface.
///
/// Unlisted datasets are emitted as `2:NN` so nothing is silently dropped.
fn dataset_name(dataset: u8) -> Option<&'static str> {
    Some(match dataset {
        5 => "ObjectName",
        7 => "EditStatus",
        10 => "Urgency",
        15 => "Category",
        20 => "SupplementalCategories",
        25 => "Keywords",
        40 => "SpecialInstructions",
        55 => "DateCreated",
        60 => "TimeCreated",
        80 => "Byline",
        85 => "BylineTitle",
        90 => "City",
        92 => "SubLocation",
        95 => "ProvinceState",
        100 => "CountryCode",
        101 => "CountryName",
        103 => "OriginalTransmissionReference",
        105 => "Headline",
        110 => "Credit",
        115 => "Source",
        116 => "CopyrightNotice",
        118 => "Contact",
        120 => "Caption",
        122 => "WriterEditor",
        _ => return None,
    })
}

fn be16(b: &[u8], at: usize) -> Option<usize> {
    let hi = *b.get(at)? as usize;
    let lo = *b.get(at + 1)? as usize;
    Some((hi << 8) | lo)
}

fn be32(b: &[u8], at: usize) -> Option<usize> {
    let mut v: usize = 0;
    for i in 0..4 {
        v = (v << 8) | (*b.get(at + i)? as usize);
    }
    Some(v)
}

/// Locate the Photoshop `APP13` payload inside a JPEG byte stream.
///
/// Returns the bytes that follow the `Photoshop 3.0\0` signature.
fn find_app13_photoshop(bytes: &[u8]) -> Option<&[u8]> {
    const SIGNATURE: &[u8] = b"Photoshop 3.0\0";
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2usize;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        // Standalone markers carry no length payload.
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        // Start of scan: entropy-coded data follows, no more APP segments.
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        let seg_len = be16(bytes, i + 2)?;
        if seg_len < 2 {
            return None;
        }
        let body_start = i + 4;
        let body_end = body_start.checked_add(seg_len - 2)?;
        if body_end > bytes.len() {
            return None;
        }
        if marker == 0xED {
            let body = &bytes[body_start..body_end];
            if let Some(rest) = body.strip_prefix(SIGNATURE) {
                return Some(rest);
            }
        }
        i = body_end;
    }
    None
}

/// Walk 8BIM blocks and return the concatenated `IptcNaa` payloads.
fn collect_iptc_blocks(mut blocks: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while blocks.len() >= 12 && blocks.starts_with(b"8BIM") {
        let Some(id) = be16(blocks, 4) else { break };
        let name_len = blocks[6] as usize;
        // Pascal name is padded so that the length byte + name is even.
        let name_field = if name_len % 2 == 0 {
            name_len + 2
        } else {
            name_len + 1
        };
        let len_at = 6 + name_field;
        let Some(data_len) = be32(blocks, len_at) else {
            break;
        };
        let data_at = len_at + 4;
        let Some(data_end) = data_at.checked_add(data_len) else {
            break;
        };
        if data_end > blocks.len() {
            break;
        }
        if id as u16 == BLOCK_ID_IPTC_NAA {
            out.extend_from_slice(&blocks[data_at..data_end]);
        }
        // Data is padded to an even length.
        let advance = data_end + (data_len % 2);
        if advance <= data_at {
            break;
        }
        blocks = blocks.get(advance..).unwrap_or(&[]);
    }
    out
}

fn push_value(map: &mut BTreeMap<String, String>, key: String, raw: &[u8]) {
    let mut text = String::from_utf8_lossy(raw).trim().to_string();
    if text.is_empty() {
        return;
    }
    if text.len() > MAX_VALUE_BYTES {
        text.truncate(
            (0..=MAX_VALUE_BYTES)
                .rev()
                .find(|&n| text.is_char_boundary(n))
                .unwrap_or(0),
        );
        text.push('…');
    }
    map.entry(key)
        .and_modify(|existing| {
            // Repeatable datasets (Keywords, SupplementalCategories) legitimately
            // appear many times; join instead of letting the last one win.
            existing.push_str("; ");
            existing.push_str(&text);
        })
        .or_insert(text);
}

/// Parse an IIM dataset stream into a `name -> value` map.
fn parse_iim(data: &[u8]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut i = 0usize;
    while i + 5 <= data.len() {
        if data[i] != 0x1C {
            i += 1;
            continue;
        }
        let record = data[i + 1];
        let dataset = data[i + 2];
        let Some(len) = be16(data, i + 3) else { break };
        let value_at = i + 5;
        // The extended form (high bit set) encodes a long value length; agent
        // envelopes truncate anyway, so it is skipped rather than mis-parsed.
        if len & 0x8000 != 0 {
            break;
        }
        let Some(value_end) = value_at.checked_add(len) else {
            break;
        };
        if value_end > data.len() {
            break;
        }
        if record == RECORD_APPLICATION {
            let key = dataset_name(dataset)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{record}:{dataset}"));
            push_value(&mut map, key, &data[value_at..value_end]);
        }
        i = value_end;
    }
    map
}

/// Read IPTC IIM tags from image bytes. Empty map when the image carries none.
///
/// Never fails on malformed metadata: a truncated 8BIM block yields whatever
/// was parsed before the truncation, matching the EXIF reader's behaviour.
pub fn read_iptc_map(bytes: &[u8]) -> Result<BTreeMap<String, String>, CliError> {
    let Some(blocks) = find_app13_photoshop(bytes) else {
        return Ok(BTreeMap::new());
    };
    let iim = collect_iptc_blocks(blocks);
    if iim.is_empty() {
        return Ok(BTreeMap::new());
    }
    Ok(parse_iim(&iim))
}
