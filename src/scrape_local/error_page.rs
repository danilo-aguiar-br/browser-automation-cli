// SPDX-License-Identifier: MIT OR Apache-2.0
//! Structured failure rows for batch/crawl collections (honest error pages).

use serde_json::{json, Map, Value};

/// Parse HTTP status from a scrape error message like `HTTP 404 for https://…`.
pub fn status_from_error_message(msg: &str) -> Option<u16> {
    let rest = msg.strip_prefix("HTTP ")?;
    let code: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    code.parse().ok()
}

/// Build a structured http_error page object for batch/crawl honesty.
pub fn http_error_page(url: &str, err_msg: &str, depth: Option<usize>) -> Value {
    let mut m = Map::new();
    m.insert("source_url".into(), json!(url));
    m.insert("http_error".into(), json!(true));
    m.insert("error".into(), json!(err_msg));
    if let Some(code) = status_from_error_message(err_msg) {
        m.insert("status_code".into(), json!(code));
    }
    if let Some(d) = depth {
        m.insert("depth".into(), json!(d));
    }
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parse() {
        assert_eq!(
            status_from_error_message("HTTP 404 for https://x"),
            Some(404)
        );
        assert_eq!(status_from_error_message("other"), None);
    }
}
