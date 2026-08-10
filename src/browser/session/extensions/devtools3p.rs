// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// List third-party DevTools protocol endpoints/tools.
    pub async fn devtools3p_list(&mut self) -> Result<Value, CliError> {
        self.pump_events().await;
        let expr = r#"(() => {
          return new Promise((resolve) => {
            if (!window.__dtmcp) window.__dtmcp = {};
            window.__dtmcp.toolGroups = [];
            const groups = [];
            const event = new CustomEvent('devtoolstooldiscovery');
            event.respondWith = (toolGroup) => {
              if (!toolGroup || typeof toolGroup.name !== 'string' || !Array.isArray(toolGroup.tools)) {
                return;
              }
              const tools = [];
              for (const tool of toolGroup.tools) {
                if (!tool || typeof tool.name !== 'string') continue;
                tools.push({
                  name: tool.name,
                  description: typeof tool.description === 'string' ? tool.description : '',
                  inputSchema: tool.inputSchema || {},
                });
              }
              const g = {
                name: toolGroup.name,
                description: typeof toolGroup.description === 'string' ? toolGroup.description : '',
                tools,
              };
              groups.push(g);
              window.__dtmcp.toolGroups.push({
                name: g.name,
                description: g.description,
                tools: toolGroup.tools,
              });
              if (!window.__dtmcp.executeTool) {
                window.__dtmcp.executeTool = async (toolName, args) => {
                  for (const group of (window.__dtmcp.toolGroups || [])) {
                    const t = (group.tools || []).find((x) => x.name === toolName);
                    if (t && typeof t.execute === 'function') {
                      return await t.execute(args || {});
                    }
                  }
                  throw new Error('Tool ' + toolName + ' not found');
                };
              }
            };
            window.dispatchEvent(event);
            setTimeout(() => resolve(groups), 0);
          });
        })()"#;
        let result = self.eval(expr, None, Some("accept"), None).await?;
        let groups = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        let tools_flat: Vec<Value> = groups
            .as_array()
            .map(|arr| {
                arr.iter()
                    .flat_map(|g| {
                        g.get("tools")
                            .and_then(|t| t.as_array())
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(json!({
            "groups": groups,
            "tools": tools_flat,
            "count": tools_flat.len(),
            "available": true,
        }))
    }

    /// Execute a third-party DevTools tool by id.
    pub async fn devtools3p_exec(
        &mut self,
        name: &str,
        params_json: Option<&str>,
    ) -> Result<Value, CliError> {
        let _ = self.devtools3p_list().await?;
        let params = params_json.unwrap_or("{}");
        // Validate JSON object
        let parsed: Value =
            crate::json_util::parse_cli_json_value(params, "params").map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid params JSON: {}", e.message()),
                    crate::i18n::suggestion_key("devtools3p_params_json", None),
                )
            })?;
        if !parsed.is_object() {
            // Same swap as the cookie path had: the catalog string was serving
            // as the machine-facing message. `error.message` is the half agents
            // match on and stays English; the localized half is the suggestion.
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "devtools3p params must be a JSON object",
                crate::i18n::suggestion_key("devtools3p_params_json", None),
            ));
        }
        let name_js = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
        let params_js = parsed.to_string();
        let expr = format!(
            r#"(async () => {{
              if (!window.__dtmcp || typeof window.__dtmcp.executeTool !== 'function') {{
                throw new Error('No third-party tools discovered on page');
              }}
              const out = await window.__dtmcp.executeTool({name_js}, {params_js});
              try {{ return JSON.parse(JSON.stringify(out)); }} catch (_) {{ return String(out); }}
            }})()"#
        );
        let result = self.eval(&expr, None, Some("accept"), None).await?;
        if result.get("exceptionDetails").is_some() {
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("devtools3p exec {name} failed"),
                crate::i18n::suggestion_key("devtools3p_list_first", None),
            ));
        }
        let value = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        Ok(json!({
            "name": name,
            "result": value,
            "ok": true,
        }))
    }
}
