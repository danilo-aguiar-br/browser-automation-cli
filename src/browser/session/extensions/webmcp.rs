// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// List WebMCP tools exposed by the active page/extension surface.
    pub async fn webmcp_list(&mut self) -> Result<Value, CliError> {
        self.pump_events().await;
        let expr = r#"(() => {
          const tools = [];
          // Declarative form-based tools (test harness / early WebMCP)
          document.querySelectorAll('form[toolname]').forEach((form) => {
            tools.push({
              name: form.getAttribute('toolname') || '',
              description: form.getAttribute('tooldescription') || '',
              source: 'form',
            });
          });
          // Future navigator surface (best-effort)
          try {
            if (navigator.modelContext && typeof navigator.modelContext.listTools === 'function') {
              // sync list not always available; ignore
            }
          } catch (_) {}
          if (window.__webmcpTools && Array.isArray(window.__webmcpTools)) {
            for (const t of window.__webmcpTools) {
              if (t && t.name) tools.push({ name: t.name, description: t.description || '', source: 'window' });
            }
          }
          return tools;
        })()"#;
        let result = self.eval(expr, None, Some("accept"), None).await?;
        let tools = result
            .get("result")
            .cloned()
            .or_else(|| result.get("value").cloned())
            .unwrap_or(result);
        let count = tools.as_array().map(|a| a.len()).unwrap_or(0);
        Ok(json!({
            "tools": tools,
            "count": count,
            "available": true,
            "note": "Requires Chrome with WebMCP/DevToolsWebMCPSupport for full surface; form[toolname] always listed",
        }))
    }

    /// Execute a WebMCP tool by name with JSON arguments.
    pub async fn webmcp_exec(
        &mut self,
        name: &str,
        input_json: Option<&str>,
    ) -> Result<Value, CliError> {
        let input = input_json.unwrap_or("{}");
        let parsed: Value =
            crate::json_util::parse_cli_json_value(input, "input").map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid input JSON: {}", e.message()),
                    crate::i18n::suggestion_key("webmcp_input_json", None),
                )
            })?;
        let name_js = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
        let input_js = parsed.to_string();
        let expr = format!(
            r#"(async () => {{
              const toolName = {name_js};
              const input = {input_js};
              // Form-based tools
              const form = document.querySelector('form[toolname="' + CSS.escape(toolName) + '"]')
                || document.querySelector('form[toolname="' + toolName + '"]');
              if (form) {{
                return await new Promise((resolve, reject) => {{
                  const handler = (event) => {{
                    event.preventDefault();
                    try {{
                      if (typeof event.respondWith === 'function') {{
                        // page may set respondWith on submit
                      }}
                    }} catch (_) {{}}
                  }};
                  form.addEventListener('submit', handler, {{ once: true }});
                  // Prefer page-defined onsubmit
                  if (typeof form.onsubmit === 'function') {{
                    const fake = {{
                      preventDefault() {{}},
                      respondWith(v) {{ resolve({{ status: 'Completed', output: v }}); }},
                    }};
                    try {{
                      form.onsubmit(fake);
                      setTimeout(() => resolve({{ status: 'Completed', output: null }}), 0);
                    }} catch (e) {{
                      reject(e);
                    }}
                    return;
                  }}
                  form.requestSubmit ? form.requestSubmit() : form.submit();
                  setTimeout(() => resolve({{ status: 'Completed', output: null, note: 'form submitted' }}), 50);
                }});
              }}
              if (window.__webmcpTools) {{
                const t = window.__webmcpTools.find((x) => x.name === toolName);
                if (t && typeof t.execute === 'function') {{
                  const out = await t.execute(input);
                  return {{ status: 'Completed', output: out }};
                }}
              }}
              throw new Error('Tool ' + toolName + ' not found');
            }})()"#
        );
        let result = self.eval(&expr, None, Some("accept"), None).await?;
        if result.get("exceptionDetails").is_some() {
            let msg = result
                .pointer("/exceptionDetails/exception/description")
                .or_else(|| result.pointer("/exceptionDetails/text"))
                .and_then(|v| v.as_str())
                .unwrap_or("tool not found");
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("webmcp exec {name}: {msg}"),
                "List tools first; page must expose form[toolname] or __webmcpTools",
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
