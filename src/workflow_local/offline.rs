// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline (non-browser) workflow step execution.

use std::path::Path;

use serde_json::{json, Value};

use super::types::WorkflowStep;
use crate::error::{CliError, ErrorKind};

pub(crate) fn execute_offline_step(step: &WorkflowStep) -> Result<Value, CliError> {
    match step.cmd.as_str() {
        "noop" | "echo" => Ok(json!({
            "cmd": step.cmd,
            "args": step.args,
            "ok": true,
        })),
        "parse" => {
            let path = step
                .args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "parse step needs args.path"))?;
            crate::scrape_local::parse_file(Path::new(path))
        }
        "scrape" => {
            // Offline workflow cannot launch browser without lifecycle; require engine=http.
            let url = step
                .args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "scrape step needs args.url"))?;
            let fmt = step
                .args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("text");
            let opts = crate::scrape_local::ScrapeOpts {
                format: crate::scrape_local::ScrapeFormat::parse(fmt)?,
                engine: "http".into(),
                only_main_content: step
                    .args
                    .get("only_main_content")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                ..Default::default()
            };
            // Block on async HTTP scrape (current_thread I/O runtime).
            let robots = crate::robots::RobotsPolicy::Honor;
            crate::runtime_util::block_on_io(crate::scrape_local::scrape_http(url, robots, &opts))
        }
        "batch-scrape" | "batch_scrape" => {
            let path = step
                .args
                .get("urls_file")
                .or_else(|| step.args.get("urls-file"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CliError::new(ErrorKind::Usage, "batch-scrape needs args.urls_file")
                })?;
            let urls = crate::scrape_local::read_urls_file(Path::new(path))?;
            let opts = crate::scrape_local::ScrapeOpts {
                format: crate::scrape_local::ScrapeFormat::Text,
                engine: "http".into(),
                ..Default::default()
            };
            crate::runtime_util::block_on_io(crate::scrape_local::batch_scrape_http(
                &urls,
                crate::robots::RobotsPolicy::Honor,
                &opts,
                2,
            ))
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("workflow offline step unsupported cmd: {other}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )),
    }
}
