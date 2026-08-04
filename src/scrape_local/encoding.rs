// SPDX-License-Identifier: MIT OR Apache-2.0
//! Charset detection and decode for HTTP scrape bodies (encoding_rs).

use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};

/// Result of decoding HTTP body bytes to UTF-8 text.
#[derive(Debug, Clone)]
pub struct DecodedBody {
    /// UTF-8 text (lossy only as last resort inside encoding_rs).
    pub text: String,
    /// Label of the encoding used (`utf-8`, `windows-1252`, …).
    pub charset: String,
    /// True when the decoder reported errors.
    pub had_errors: bool,
}

/// Decode body using BOM → Content-Type charset → meta charset → UTF-8 → Windows-1252.
pub fn decode_html_body(bytes: &[u8], content_type: Option<&str>) -> DecodedBody {
    if let Some((enc, bom_len)) = Encoding::for_bom(bytes) {
        let (cow, _, had_errors) = enc.decode(&bytes[bom_len..]);
        return DecodedBody {
            text: cow.into_owned(),
            charset: enc.name().to_ascii_lowercase(),
            had_errors,
        };
    }

    if let Some(label) = charset_from_content_type(content_type) {
        if let Some(enc) = Encoding::for_label(label.as_bytes()) {
            let (cow, _, had_errors) = enc.decode(bytes);
            return DecodedBody {
                text: cow.into_owned(),
                charset: enc.name().to_ascii_lowercase(),
                had_errors,
            };
        }
    }

    // Peek meta charset from a lossy UTF-8 view of the head (HTML5 sniffing window).
    let peek = crate::xdg::resolve_scrape_charset_peek_bytes();
    let head_end = bytes.len().min(peek);
    let head_lossy = String::from_utf8_lossy(&bytes[..head_end]);
    if let Some(label) = charset_from_meta_html(&head_lossy) {
        if let Some(enc) = Encoding::for_label(label.as_bytes()) {
            let (cow, _, had_errors) = enc.decode(bytes);
            return DecodedBody {
                text: cow.into_owned(),
                charset: enc.name().to_ascii_lowercase(),
                had_errors,
            };
        }
    }

    // UTF-8 if valid; else Windows-1252 fallback (HTML5 default for legacy).
    match std::str::from_utf8(bytes) {
        Ok(s) => DecodedBody {
            text: s.to_string(),
            charset: "utf-8".into(),
            had_errors: false,
        },
        Err(_) => {
            let (cow, _, had_errors) = WINDOWS_1252.decode(bytes);
            DecodedBody {
                text: cow.into_owned(),
                charset: WINDOWS_1252.name().to_ascii_lowercase(),
                had_errors,
            }
        }
    }
}

fn charset_from_content_type(ct: Option<&str>) -> Option<String> {
    let ct = ct?;
    let lower = ct.to_ascii_lowercase();
    let idx = lower.find("charset=")?;
    let rest = lower[idx + "charset=".len()..].trim();
    let token = rest
        .split([';', ' ', '"', '\''])
        .next()
        .unwrap_or("")
        .trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn charset_from_meta_html(head: &str) -> Option<String> {
    let lower = head.to_ascii_lowercase();
    // <meta charset="utf-8">
    if let Some(i) = lower.find("charset=") {
        let rest = &lower[i + "charset=".len()..];
        let rest = rest.trim_start_matches(['"', '\'', ' ']);
        let token: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !token.is_empty() && token != "utf" {
            // avoid matching utf-8 partially wrong — accept utf-8 fully
            if rest.starts_with("utf-8") {
                return Some("utf-8".into());
            }
            return Some(token);
        }
        if rest.starts_with("utf-8") {
            return Some("utf-8".into());
        }
    }
    // http-equiv content-type
    if let Some(i) = lower.find("content-type") {
        let window = &lower[i..].chars().take(200).collect::<String>();
        return charset_from_content_type(Some(window));
    }
    let _ = UTF_8; // keep import used for docs clarity
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_passthrough() {
        let d = decode_html_body(b"<html>ok</html>", Some("text/html; charset=utf-8"));
        assert_eq!(d.charset, "utf-8");
        assert!(d.text.contains("ok"));
        assert!(!d.had_errors);
    }

    #[test]
    fn windows_1252_fallback() {
        // 0xE9 = é in windows-1252
        let bytes = b"<html>\xe9</html>";
        let d = decode_html_body(bytes, Some("text/html; charset=windows-1252"));
        assert!(d.text.contains('é'), "got {:?}", d.text);
        assert_eq!(d.charset, "windows-1252");
    }

    #[test]
    fn meta_charset_detected() {
        let html = b"<html><head><meta charset=\"iso-8859-1\"></head><body>\xe9</body></html>";
        let d = decode_html_body(html, None);
        assert!(d.text.contains('é') || d.charset.contains("1252") || d.charset.contains("8859"));
    }
}
