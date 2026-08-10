// SPDX-License-Identifier: MIT OR Apache-2.0
//! scrape, view, attach_snapshot_if

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::snapshot::{self, SnapshotOptions};

use super::super::OneShotSession;
use crate::browser::helpers::tree_to_at_refs;

impl OneShotSession {
    /// Navigate the origin root once before the target, when `--warmup` is on.
    ///
    /// # Why failure here is not failure of the run
    ///
    /// Same contract as the HTTP engine's warm-up: this is a preparation, not
    /// a result. A root that 404s, redirects away, or is blocked by robots
    /// does not change what the caller asked for, and failing the scrape over
    /// a preparatory navigation would break working runs on sites whose root
    /// simply is not interesting. The error is dropped to stderr-level tracing
    /// and the target navigation proceeds.
    async fn warm_origin_via_browser(&mut self, target: &str, robots: crate::robots::RobotsPolicy) {
        if !crate::browser_policy::warmup_enabled() {
            return;
        }
        let Some(root) = crate::browser_policy::warmup_url()
            .map(str::to_string)
            .or_else(|| crate::scrape_local::origin_root_of(target))
        else {
            return;
        };
        if root == target {
            return;
        }
        if let Err(e) = self.goto(&root, robots).await {
            tracing::warn!(
                target: "browser_automation_cli::scrape",
                root = %root,
                error = %e,
                "browser warm-up navigation failed; continuing to the target"
            );
        }
    }

    /// Extract page content in the requested formats (GAP-057 parity with top-level scrape).
    ///
    /// Empty `formats` defaults to `text` (agent-first: no HTML dump). Multi-format
    /// requests share one CDP HTML fetch and [`crate::scrape_local::build_formats_map`].
    /// Scrape with optional settle wait after navigation (`wait_ms`).
    pub async fn scrape_with_wait(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
        formats: &[&str],
        wait_ms: u64,
    ) -> Result<Value, CliError> {
        let formats: Vec<&str> = if formats.is_empty() {
            vec!["text"]
        } else {
            formats.to_vec()
        };
        // The warm-up used to exist only on the HTTP engine, which is the
        // engine that needs it least: a challenge that cares about "no
        // session, straight to the interior" is the same challenge that makes
        // a caller reach for the browser engine in the first place. Here the
        // cookies land in Chrome's own jar, so the effect is stronger than on
        // the HTTP path — and it is the same opt-in flag.
        self.warm_origin_via_browser(url, robots).await;
        let nav = self.goto(url, robots).await?;
        if wait_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
        }
        self.scrape_after_nav(url, robots, &formats, nav).await
    }

    /// Extract page content in the requested formats (empty formats → text).
    pub async fn scrape(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
        formats: &[&str],
    ) -> Result<Value, CliError> {
        self.scrape_with_wait(url, robots, formats, 0).await
    }

    /// Shared HTML extract path after navigation (and optional wait).
    async fn scrape_after_nav(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
        formats: &[&str],
        nav: Value,
    ) -> Result<Value, CliError> {
        let formats: Vec<&str> = if formats.is_empty() {
            vec!["text"]
        } else {
            formats.to_vec()
        };
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

        // G1 on the engine the audit calls the most valuable asset.
        //
        // Detection shipped on `--engine http` only, so the SAME bot check
        // exited 6 through one engine and returned `ok: true` through the other.
        // That inverted the product's own advice: when a WAF is in front it
        // tells agents to switch to `--engine browser`, which moved them from
        // the engine that reports blocks to the engine that stayed silent.
        //
        // The check belongs HERE and not in the command layer: this function
        // builds and returns the finished payload, so `html` never reaches the
        // caller and a check up there reads an empty string and never fires.
        // Put the assertion where the evidence is.
        //
        // THREE SIGNALS: body, final URL and title. The URL and the title cost
        // nothing here because `nav` already carried both, and a challenge that
        // auto-navigates rewrites exactly those two while leaving a body this
        // check would otherwise call clean.
        //
        // What stays out of reach is the transport: a rendered DOM carries no
        // `cf-ray` header and no `Set-Cookie` jar, so a generic challenge stays
        // `waf: "generic"` here while the http engine can name the vendor. Same
        // exit code and same payload shape; strictly less attribution, because
        // strictly less evidence exists.
        //
        // The line above said "BODY SIGNALS ONLY" for one round after the call
        // already passed three arguments. Corrected 2026-08-10.
        let title_s = title.as_str().unwrap_or_default();
        if let Some(hit) = crate::scrape_local::detect_in_page(&html_s, &source, title_s) {
            return Err(CliError::with_suggestion(
                ErrorKind::Blocked,
                format!(
                    "{} served a bot check for {source} (signal {} in {})",
                    hit.waf,
                    hit.signal,
                    hit.phase.as_str()
                ),
                hit.suggestion(),
            )
            .with_data(json!({ "block_detection": hit.to_json() })));
        }

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

        let base_opts = crate::scrape_local::ScrapeOpts {
            format: crate::scrape_local::ScrapeFormat::Html,
            only_main_content: false,
            engine: "browser".into(),
            ..Default::default()
        };
        let formats_out = crate::scrape_local::build_formats_map(
            &source, 200, &html_s, &formats, &base_opts, "browser", robots,
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
