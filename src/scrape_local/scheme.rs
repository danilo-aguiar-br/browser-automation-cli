// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTTP engine URL scheme validation.

// GAP-046 lote 3 pending: wire-type docs land per file; the allow is removed
// as each file is documented, so it can never silence a new module.
#![allow(missing_docs)]
use url::Url;

use crate::error::{CliError, ErrorKind};

pub fn reject_non_http_scheme_for_http_engine(url: &str) -> Result<(), CliError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "empty URL for scrape --engine http",
            crate::i18n::suggestion_key("url_absolute_http", None),
        ));
    }
    // Bare local path (not a URL).
    if !trimmed.contains("://") {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("HTTP engine cannot fetch local path: {trimmed}"),
            crate::i18n::suggestion_key("scrape_engine_choice", None),
        ));
    }
    match Url::parse(trimmed) {
        Ok(u) => match u.scheme() {
            "http" | "https" => Ok(()),
            "file" => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("HTTP engine cannot fetch file:// URL: {trimmed}"),
                crate::i18n::suggestion_key("scrape_engine_choice", None),
            )),
            other => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("HTTP engine does not support scheme `{other}`"),
                crate::i18n::suggestion_key("url_absolute_http", None),
            )),
        },
        Err(e) => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid URL for HTTP scrape: {e}"),
            crate::i18n::suggestion_key("url_absolute_http", None),
        )),
    }
}
