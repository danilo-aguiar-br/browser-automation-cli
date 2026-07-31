// SPDX-License-Identifier: MIT OR Apache-2.0
//! scrape, view, attach_snapshot_if

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::snapshot::{self, SnapshotOptions};

use super::super::OneShotSession;
use crate::browser::helpers::tree_to_at_refs;

impl OneShotSession {
    /// Extract page content in the requested formats (GAP-057 parity with top-level scrape).
    ///
    /// Empty `formats` defaults to `text` (agent-first: no HTML dump). Multi-format
    /// requests share one CDP HTML fetch and [`crate::scrape_local::build_formats_map`].
    pub async fn scrape(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
        formats: &[&str],
    ) -> Result<Value, CliError> {
        let formats: Vec<&str> = if formats.is_empty() {
            vec!["text"]
        } else {
            formats.to_vec()
        };
        let nav = self.goto(url, robots).await?;
        let source = nav
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or(url)
            .to_string();
        let title = nav
            .get("title")
            .cloned()
            .unwrap_or(Value::String(String::new()));

        let html_val = self
            .eval(
                "String(document.documentElement ? document.documentElement.outerHTML : '')",
                None,
                Some("accept"),
                None,
            )
            .await
            .unwrap_or_else(|_| json!({"result": ""}));
        let html_s = match html_val.get("result") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        };

        if formats.len() == 1 {
            let fmt = crate::scrape_local::ScrapeFormat::parse(formats[0])?;
            if html_s.is_empty() {
                let text_val = self
                    .eval(
                        "String((document.body && document.body.innerText) || '')",
                        None,
                        Some("accept"),
                        None,
                    )
                    .await
                    .unwrap_or_else(|_| json!({"result": ""}));
                let text_s = match text_val.get("result") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => String::new(),
                };
                return Ok(json!({
                    "source_url": source,
                    "title": title,
                    "robots_policy": robots.as_str(),
                    "text": text_s,
                    "format": format!("{fmt:?}").to_ascii_lowercase(),
                    "engine": "browser",
                }));
            }
            let opts = crate::scrape_local::ScrapeOpts {
                format: fmt,
                only_main_content: false,
                engine: "browser".into(),
                ..Default::default()
            };
            let mut payload =
                crate::scrape_local::build_scrape_payload(&source, 200, &html_s, &opts, robots);
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("title".into(), title);
            }
            return Ok(payload);
        }

        let formats_out = crate::scrape_local::build_formats_map(
            &source, 200, &html_s, &formats, false, "browser", robots,
        )?;
        Ok(json!({
            "source_url": source,
            "title": title,
            "engine": "browser",
            "formats": formats_out,
            "format_list": formats,
            "robots_policy": robots.as_str(),
        }))
    }

    /// Accessibility tree with agent-facing `@eN` refs.
    pub async fn view(&mut self, verbose: bool) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self.session_id()?;

        let options = SnapshotOptions {
            interactive: false,
            compact: !verbose,
            ..SnapshotOptions::default()
        };

        self.ref_map.clear();
        let tree = snapshot::take_snapshot(
            &self.manager.client,
            &session_id,
            &options,
            &mut self.ref_map,
            None,
            &self.iframe_sessions,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("view/snapshot failed: {e}"),
                crate::i18n::suggestion_key("navigate_first", None),
            )
        })?;

        let tree_at = tree_to_at_refs(&tree);
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();

        let entries = self.ref_map.entries_sorted();
        let ref_count = entries.len();
        // GAP-034 pillar 2: `@eN` is an ordinal minted by THIS snapshot and dies
        // with the process. The role+name pair belongs to the element, so it is
        // emitted alongside as a locator the caller can write down and reuse.
        let pairs: Vec<(String, String)> = entries
            .iter()
            .map(|(_, entry)| (entry.role.clone(), entry.name.clone()))
            .collect();
        let locators = crate::native::element::assign_locators(&pairs);
        let refs: serde_json::Map<String, Value> = entries
            .into_iter()
            .zip(locators)
            .map(|((ref_id, entry), locator)| {
                let key = format!("@{ref_id}");
                (
                    key,
                    json!({
                        "role": entry.role,
                        "name": entry.name,
                        "id": ref_id,
                        "locator": locator.to_wire(),
                    }),
                )
            })
            .collect();

        Ok(json!({
            "tree": tree_at,
            "url": url,
            "title": title,
            "refs": refs,
            "ref_count": ref_count,
        }))
    }

    /// Optionally attach a slim accessibility snapshot to a JSON result.
    pub(crate) async fn attach_snapshot_if(
        &mut self,
        include: bool,
        mut data: Value,
    ) -> Result<Value, CliError> {
        if !include {
            return Ok(data);
        }
        let snap = self.view(false).await?;
        if let Some(obj) = data.as_object_mut() {
            obj.insert("snapshot".to_string(), snap);
            obj.insert("include_snapshot".to_string(), json!(true));
        }
        Ok(data)
    }
}
