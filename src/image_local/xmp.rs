// SPDX-License-Identifier: MIT OR Apache-2.0
//! XMP packet extraction and flattening (GAP-IMG-097).
//!
//! Written over `quick-xml`, which the crate already depends on. The Adobe
//! `xmp_toolkit` crate is FFI over the C++ XMP SDK and is therefore rejected by
//! the project's no-C-toolchain rule.
//!
//! # Carriers
//!
//! | Container | Location |
//! |-----------|----------|
//! | JPEG | `APP1` segment prefixed `http://ns.adobe.com/xap/1.0/\0` |
//! | PNG | `iTXt` chunk with keyword `XML:com.adobe.xmp` |
//! | ISOBMFF (HEIF/AVIF/MP4) | `uuid` box with the XMP UUID |
//!
//! A raw `<?xpacket …?>` scan is used as the last resort so a packet embedded
//! in a container this build does not model is still found.

use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::CliError;

/// Max bytes kept per XMP value in an envelope (mirrors the EXIF cap).
const MAX_VALUE_BYTES: usize = 256;

/// Max XMP packet bytes parsed; packets larger than this are almost always a
/// serialised edit history rather than metadata an agent needs.
const MAX_PACKET_BYTES: usize = 512 * 1024;

/// JPEG `APP1` XMP signature.
const JPEG_XMP_SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

/// ISOBMFF `uuid` box identifier for an XMP payload.
const ISOBMFF_XMP_UUID: [u8; 16] = [
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];

fn be16(b: &[u8], at: usize) -> Option<usize> {
    Some(((*b.get(at)? as usize) << 8) | (*b.get(at + 1)? as usize))
}

fn be32(b: &[u8], at: usize) -> Option<usize> {
    let mut v: usize = 0;
    for i in 0..4 {
        v = (v << 8) | (*b.get(at + i)? as usize);
    }
    Some(v)
}

fn from_jpeg(bytes: &[u8]) -> Option<&[u8]> {
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
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
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
        if marker == 0xE1 {
            if let Some(rest) = bytes[body_start..body_end].strip_prefix(JPEG_XMP_SIG) {
                return Some(rest);
            }
        }
        i = body_end;
    }
    None
}

fn from_png(bytes: &[u8]) -> Option<&[u8]> {
    const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    const KEYWORD: &[u8] = b"XML:com.adobe.xmp\0";
    if !bytes.starts_with(&PNG_SIG) {
        return None;
    }
    let mut i = PNG_SIG.len();
    while i + 8 <= bytes.len() {
        let len = be32(bytes, i)?;
        let kind = bytes.get(i + 4..i + 8)?;
        let data_at = i + 8;
        let data_end = data_at.checked_add(len)?;
        if data_end > bytes.len() {
            return None;
        }
        if kind == b"iTXt" {
            let data = &bytes[data_at..data_end];
            if let Some(rest) = data.strip_prefix(KEYWORD) {
                // iTXt: compression flag, compression method, language tag\0,
                // translated keyword\0, then the text. Only uncompressed
                // (flag 0) packets are read; a compressed one is skipped
                // rather than guessed at.
                if rest.first() == Some(&0) {
                    let after_flags = rest.get(2..)?;
                    let mut nulls = after_flags.splitn(3, |&b| b == 0);
                    nulls.next()?;
                    nulls.next()?;
                    return nulls.next();
                }
            }
        }
        // chunk = len + type + data + crc32
        i = data_end + 4;
    }
    None
}

fn from_isobmff(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return None;
    }
    let mut i = 0usize;
    while i + 8 <= bytes.len() {
        let size = be32(bytes, i)?;
        let kind = bytes.get(i + 4..i + 8)?;
        // size 0 means "to end of file"; size 1 means a 64-bit largesize field,
        // which this scanner does not follow.
        if size < 8 {
            return None;
        }
        let end = i.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        if kind == b"uuid" && bytes.get(i + 8..i + 24) == Some(&ISOBMFF_XMP_UUID[..]) {
            return bytes.get(i + 24..end);
        }
        i = end;
    }
    None
}

fn from_raw_scan(bytes: &[u8]) -> Option<&[u8]> {
    const BEGIN: &[u8] = b"<?xpacket begin";
    const RDF_END: &[u8] = b"</x:xmpmeta>";
    let start = bytes
        .windows(BEGIN.len())
        .position(|w| w == BEGIN)
        .or_else(|| {
            bytes
                .windows(b"<x:xmpmeta".len())
                .position(|w| w == b"<x:xmpmeta")
        })?;
    let tail = &bytes[start..];
    let end = tail
        .windows(RDF_END.len())
        .position(|w| w == RDF_END)
        .map(|p| p + RDF_END.len())
        .unwrap_or(tail.len());
    Some(&tail[..end])
}

/// Extract the raw XMP packet from any container this build understands.
#[must_use]
pub fn extract_packet(bytes: &[u8]) -> Option<&[u8]> {
    from_jpeg(bytes)
        .or_else(|| from_png(bytes))
        .or_else(|| from_isobmff(bytes))
        .or_else(|| from_raw_scan(bytes))
}

/// True for XML plumbing that is not metadata: namespace declarations, the RDF
/// subject reference, and `xml:*` reserved attributes such as `xml:lang`.
fn is_scaffolding(key: &str) -> bool {
    key.starts_with("xmlns") || key.starts_with("xml:") || key == "rdf:about"
}

fn truncate(text: &mut String) {
    if text.len() > MAX_VALUE_BYTES {
        text.truncate(
            (0..=MAX_VALUE_BYTES)
                .rev()
                .find(|&n| text.is_char_boundary(n))
                .unwrap_or(0),
        );
        text.push('…');
    }
}

/// Flatten an XMP packet into a `prefix:local -> value` map.
///
/// RDF containers (`rdf:Bag`, `rdf:Seq`, `rdf:Alt`) collapse into a
/// `"; "`-joined value under the owning property name, which is what an agent
/// projecting `--select xmp` actually wants.
fn flatten(packet: &[u8]) -> BTreeMap<String, String> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut reader = Reader::from_reader(packet);
    let config = reader.config_mut();
    config.trim_text(true);
    // The packet is untrusted metadata, not a document to resolve: never let
    // the parser chase an external or general entity.
    config.check_end_names = false;

    let mut stack: Vec<String> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                // Attribute-form properties (`<rdf:Description dc:title="x"/>`)
                // are as common as element-form ones.
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    if is_scaffolding(&key) {
                        continue;
                    }
                    let mut val = String::from_utf8_lossy(&attr.value).trim().to_string();
                    if val.is_empty() {
                        continue;
                    }
                    truncate(&mut val);
                    map.entry(key).or_insert(val);
                }
                stack.push(name);
            }
            Ok(Event::Empty(e)) => {
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    if is_scaffolding(&key) {
                        continue;
                    }
                    let mut val = String::from_utf8_lossy(&attr.value).trim().to_string();
                    if val.is_empty() {
                        continue;
                    }
                    truncate(&mut val);
                    map.entry(key).or_insert(val);
                }
            }
            Ok(Event::Text(t)) => {
                // quick-xml 0.41: `xml10_content` decodes and resolves only the
                // five predefined XML entities, so a custom `<!ENTITY>` cannot
                // expand here even if one slipped past the sanitiser.
                let text = t.xml10_content().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }
                // Skip RDF scaffolding to name the value after its property.
                let owner = stack
                    .iter()
                    .rev()
                    .find(|n| !n.starts_with("rdf:") && !n.starts_with("x:"));
                let Some(owner) = owner else { continue };
                let key = owner.clone();
                let mut text = text;
                truncate(&mut text);
                map.entry(key)
                    .and_modify(|existing| {
                        existing.push_str("; ");
                        existing.push_str(&text);
                    })
                    .or_insert(text);
            }
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Eof) | Err(_) => break,
            Ok(_) => {}
        }
        buf.clear();
    }
    map
}

/// Read a flattened XMP map from image bytes. Empty map when none is present.
pub fn read_xmp_map(bytes: &[u8]) -> Result<BTreeMap<String, String>, CliError> {
    let Some(packet) = extract_packet(bytes) else {
        return Ok(BTreeMap::new());
    };
    if packet.len() > MAX_PACKET_BYTES {
        tracing::debug!(
            packet_bytes = packet.len(),
            "xmp packet over cap; skipping parse"
        );
        return Ok(BTreeMap::new());
    }
    Ok(flatten(packet))
}
