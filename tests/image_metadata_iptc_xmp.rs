// SPDX-License-Identifier: MIT OR Apache-2.0
//! IPTC IIM and XMP readers (GAP-IMG-097), plus the SVG sanitiser's threat
//! gate (GAP-IMG-092).
//!
//! Fixtures are assembled byte by byte so each test states exactly which wire
//! structure it exercises.

use browser_automation_cli::image_local;

/// Wrap a payload in a JPEG `APP13` Photoshop segment inside a minimal JPEG.
fn jpeg_with_app13(iim: &[u8]) -> Vec<u8> {
    let mut blocks = Vec::new();
    blocks.extend_from_slice(b"8BIM");
    blocks.extend_from_slice(&0x0404u16.to_be_bytes());
    blocks.push(0); // empty pascal name…
    blocks.push(0); // …padded to even
    blocks.extend_from_slice(&(iim.len() as u32).to_be_bytes());
    blocks.extend_from_slice(iim);
    if iim.len() % 2 == 1 {
        blocks.push(0);
    }

    let mut body = Vec::new();
    body.extend_from_slice(b"Photoshop 3.0\0");
    body.extend_from_slice(&blocks);

    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&[0xFF, 0xED]);
    out.extend_from_slice(&((body.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// Build one IIM dataset record in the application record (record 2).
fn iim_record(dataset: u8, value: &str) -> Vec<u8> {
    let mut v = vec![0x1C, 0x02, dataset];
    v.extend_from_slice(&(value.len() as u16).to_be_bytes());
    v.extend_from_slice(value.as_bytes());
    v
}

/// Wrap an XMP packet in a JPEG `APP1` segment inside a minimal JPEG.
fn jpeg_with_xmp(packet: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"http://ns.adobe.com/xap/1.0/\0");
    body.extend_from_slice(packet.as_bytes());

    let mut out = vec![0xFF, 0xD8];
    out.extend_from_slice(&[0xFF, 0xE1]);
    out.extend_from_slice(&((body.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

// ── IPTC ───────────────────────────────────────────────────────────────────

#[test]
fn iptc_reads_named_application_datasets() {
    let mut iim = iim_record(5, "Sunset over the bay");
    iim.extend(iim_record(80, "A. Photographer"));
    iim.extend(iim_record(116, "(c) 2026 Example"));
    let jpeg = jpeg_with_app13(&iim);

    let map = image_local::read_iptc_map(&jpeg).expect("iptc");
    assert_eq!(
        map.get("ObjectName").map(String::as_str),
        Some("Sunset over the bay")
    );
    assert_eq!(
        map.get("Byline").map(String::as_str),
        Some("A. Photographer")
    );
    assert_eq!(
        map.get("CopyrightNotice").map(String::as_str),
        Some("(c) 2026 Example")
    );
}

#[test]
fn iptc_joins_repeatable_datasets_instead_of_overwriting() {
    let mut iim = iim_record(25, "coast");
    iim.extend(iim_record(25, "dusk"));
    iim.extend(iim_record(25, "long-exposure"));
    let map = image_local::read_iptc_map(&jpeg_with_app13(&iim)).unwrap();
    let kw = map.get("Keywords").expect("Keywords present");
    assert!(kw.contains("coast") && kw.contains("dusk") && kw.contains("long-exposure"));
}

#[test]
fn iptc_surfaces_unknown_datasets_rather_than_dropping_them() {
    let map = image_local::read_iptc_map(&jpeg_with_app13(&iim_record(199, "custom"))).unwrap();
    assert_eq!(map.get("2:199").map(String::as_str), Some("custom"));
}

#[test]
fn iptc_returns_empty_when_there_is_no_app13() {
    let plain = [vec![0xFFu8, 0xD8, 0xFF, 0xE0], vec![0xFF, 0xD9]].concat();
    assert!(image_local::read_iptc_map(&plain).unwrap().is_empty());
}

#[test]
fn iptc_does_not_panic_on_a_truncated_block() {
    let mut jpeg = jpeg_with_app13(&iim_record(5, "Title"));
    jpeg.truncate(jpeg.len() - 6);
    // Best-effort parse: never a panic, never an error that sinks `image info`.
    let _ = image_local::read_iptc_map(&jpeg).unwrap();
}

// ── XMP ────────────────────────────────────────────────────────────────────

const XMP_PACKET: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" dc:format="image/jpeg">
      <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Harbour at dawn</rdf:li></rdf:Alt></dc:title>
      <dc:subject><rdf:Bag><rdf:li>boats</rdf:li><rdf:li>water</rdf:li></rdf:Bag></dc:subject>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

#[test]
fn xmp_packet_is_found_in_a_jpeg_app1_segment() {
    let jpeg = jpeg_with_xmp(XMP_PACKET);
    let packet = image_local::extract_xmp_packet(&jpeg).expect("packet");
    assert!(packet.starts_with(b"<x:xmpmeta"));
}

#[test]
fn xmp_flattens_element_and_attribute_properties() {
    let map = image_local::read_xmp_map(&jpeg_with_xmp(XMP_PACKET)).expect("xmp");
    assert_eq!(
        map.get("dc:title").map(String::as_str),
        Some("Harbour at dawn")
    );
    // Attribute-form properties are as common as element-form ones.
    assert_eq!(map.get("dc:format").map(String::as_str), Some("image/jpeg"));
}

#[test]
fn xmp_collapses_rdf_containers_into_one_joined_value() {
    let map = image_local::read_xmp_map(&jpeg_with_xmp(XMP_PACKET)).unwrap();
    let subject = map.get("dc:subject").expect("dc:subject");
    assert!(subject.contains("boats") && subject.contains("water"));
}

#[test]
fn xmp_never_exposes_xmlns_declarations_as_metadata() {
    let map = image_local::read_xmp_map(&jpeg_with_xmp(XMP_PACKET)).unwrap();
    assert!(
        map.keys()
            .all(|k| !k.starts_with("xmlns") && !k.starts_with("xml:")),
        "namespace declarations and xml:* reserved attributes are scaffolding, not metadata"
    );
}

#[test]
fn xmp_returns_empty_when_no_packet_is_present() {
    let plain = [vec![0xFFu8, 0xD8, 0xFF, 0xE0], vec![0xFF, 0xD9]].concat();
    assert!(image_local::read_xmp_map(&plain).unwrap().is_empty());
}

// ── SVG sanitiser threat gate ──────────────────────────────────────────────

fn assert_rejected(svg: &str, needle: &str) {
    let err =
        image_local::sanitize_svg(svg.as_bytes()).expect_err("sanitiser must reject this document");
    let msg = err.message().to_ascii_lowercase();
    assert!(
        msg.contains(needle),
        "expected {needle:?} in rejection, got: {}",
        err.message()
    );
}

#[test]
fn svg_rejects_billion_laughs_entity_expansion() {
    assert_rejected(
        r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "xx"><!ENTITY b "&a;&a;">]><svg xmlns="http://www.w3.org/2000/svg">&b;</svg>"#,
        "entit",
    );
}

#[test]
fn svg_rejects_a_bare_doctype_when_entities_are_disallowed() {
    assert_rejected(
        r#"<!DOCTYPE svg SYSTEM "http://evil.test/x.dtd"><svg xmlns="http://www.w3.org/2000/svg"/>"#,
        "doctype",
    );
}

#[test]
fn svg_rejects_embedded_script() {
    assert_rejected(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><script>fetch('http://evil.test')</script></svg>"#,
        "script",
    );
}

#[test]
fn svg_rejects_event_handler_attributes() {
    assert_rejected(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><rect onload="alert(1)" width="1" height="1"/></svg>"#,
        "event-handler",
    );
}

#[test]
fn svg_rejects_external_href_ssrf() {
    assert_rejected(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><image href="http://169.254.169.254/latest/meta-data"/></svg>"#,
        "http:",
    );
}

#[test]
fn svg_rejects_local_file_href() {
    assert_rejected(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><image href="file:///etc/passwd"/></svg>"#,
        "file:",
    );
}

#[test]
fn svg_rejects_foreign_object() {
    assert_rejected(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><foreignObject><b>x</b></foreignObject></svg>"#,
        "foreignobject",
    );
}

#[test]
fn svg_rejects_pathological_nesting_depth() {
    let depth = 400; // above DEFAULT_SVG_MAX_DEPTH
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    svg.push_str(&"<g>".repeat(depth));
    svg.push_str(&"</g>".repeat(depth));
    svg.push_str("</svg>");
    assert_rejected(&svg, "depth");
}

#[test]
fn svg_allows_a_self_contained_data_uri() {
    // `data:` cannot reach the network or the filesystem, so it stays legal.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><image href="data:image/png;base64,iVBORw0KGgo="/></svg>"#;
    assert!(image_local::sanitize_svg(svg.as_bytes()).is_ok());
}

#[test]
fn svg_does_not_flag_ordinary_words_containing_on() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><title>only one</title><rect width="1" height="1"/></svg>"#;
    assert!(
        image_local::sanitize_svg(svg.as_bytes()).is_ok(),
        "the on* detector must not fire on prose"
    );
}
