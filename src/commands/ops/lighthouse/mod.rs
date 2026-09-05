// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lighthouse audit via the external CLI.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | resolve | binary discovery (XDG, PATH) and spawn safety |
//! | report | report JSON to envelope payload |

mod report;
mod resolve;

pub(crate) use report::lighthouse_to_value;
// Used only by the unit tests below; `cargo fix --lib` cannot see that target
// and would otherwise drop the re-export as unused.
#[cfg(test)]
use crate::error::ErrorKind;
#[cfg(test)]
use resolve::resolve_lighthouse_binary;

use crate::commands::common::emit_ok;
use crate::error::CliError;
use std::path::Path;

pub(crate) fn handle_lighthouse(
    url: &str,
    out_dir: Option<&Path>,
    device: &str,
    mode: &str,
    lighthouse_path: Option<&Path>,
    json: bool,
) -> Result<(), CliError> {
    let data = lighthouse_to_value(url, out_dir, device, mode, lighthouse_path)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok lighthouse report={}",
            d.pointer("/reports/html")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        ))?;
        Ok(())
    })
}

#[cfg(test)]
mod lighthouse_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mock_lighthouse_parses_scores() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mock = root.join("scripts/mock-lighthouse.sh");
        if !mock.is_file() {
            crate::test_utils::skip_unit_test(
                "lighthouse_mock",
                "scripts/mock-lighthouse.sh missing.",
            );
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&mock, std::fs::Permissions::from_mode(0o755));
        }
        let out = tempfile::tempdir().expect("tmp");
        let v = lighthouse_to_value(
            "https://example.com",
            Some(out.path()),
            "desktop",
            "navigation",
            Some(&mock),
        )
        .expect("mock lighthouse");
        assert_eq!(
            v.get("binary_source").and_then(|s| s.as_str()),
            Some("mock")
        );
        assert_eq!(
            v.get("binary_present").and_then(|b| b.as_bool()),
            Some(true)
        );
        let scores = v
            .get("scores")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(!scores.is_empty(), "expected scores from mock LHR, got {v}");
    }

    #[test]
    fn resolve_missing_is_unavailable() {
        let err =
            resolve_lighthouse_binary(Some(Path::new("/no/such/lighthouse-bin-xyz"))).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Usage);
    }
}
