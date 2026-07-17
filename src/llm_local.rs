//! One-shot optional LLM HTTP extract (XDG key only; no product env vars).
//!
//! Uses an OpenAI-compatible chat completions endpoint configured via XDG
//! (`openrouter_api_key`, `llm_base_url`, `llm_model`). No telemetry.

use std::time::Duration;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Default OpenAI-compatible base URL (path ends before `/chat/completions`).
pub const DEFAULT_LLM_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Default model id when XDG `llm_model` is unset.
pub const DEFAULT_LLM_MODEL: &str = "openai/gpt-4o-mini";

/// Resolve API key from XDG only.
pub fn require_api_key() -> Result<String, CliError> {
    xdg::openrouter_api_key().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            "LLM extract requires XDG openrouter_api_key",
            "Run: browser-automation-cli config set openrouter_api_key <key>",
        )
    })
}

/// Base URL from XDG or default constant.
pub fn base_url() -> String {
    xdg::llm_base_url().unwrap_or_else(|| DEFAULT_LLM_BASE_URL.to_string())
}

/// Model from XDG or default constant.
pub fn model() -> String {
    xdg::llm_model().unwrap_or_else(|| DEFAULT_LLM_MODEL.to_string())
}

/// Call chat completions with retry/backoff (one-shot; no daemon).
pub fn chat_completion(
    system: &str,
    user: &str,
    schema_hint: Option<&str>,
) -> Result<Value, CliError> {
    let key = require_api_key()?;
    let model = model();
    let base = base_url().trim_end_matches('/').to_string();
    let url = format!("{base}/chat/completions");

    let mut user_content = user.to_string();
    if let Some(schema) = schema_hint {
        user_content.push_str("\n\nRespond with JSON matching this schema:\n");
        user_content.push_str(schema);
    }

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user_content }
        ],
        "temperature": 0.2,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("browser-automation-cli/0.1.2")
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("llm client: {e}")))?;

    let mut last_err = String::from("llm request failed");
    let delays_ms = [200u64, 500, 1200];
    for (attempt, delay) in delays_ms.iter().enumerate() {
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send();
        match resp {
            Ok(r) if r.status().is_success() => {
                let v: Value = r.json().map_err(|e| {
                    CliError::new(ErrorKind::Data, format!("llm response json: {e}"))
                })?;
                let answer = v
                    .pointer("/choices/0/message/content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                return Ok(json!({
                    "llm": true,
                    "model": model,
                    "base_url": base,
                    "answer": answer,
                    "raw": v,
                    "attempt": attempt + 1,
                }));
            }
            Ok(r) => {
                last_err = format!("llm HTTP {}", r.status());
                if r.status().as_u16() < 500 && r.status().as_u16() != 429 {
                    break;
                }
            }
            Err(e) => last_err = format!("llm: {e}"),
        }
        std::thread::sleep(Duration::from_millis(*delay));
    }
    Err(CliError::with_suggestion(
        ErrorKind::Unavailable,
        last_err,
        "Check XDG openrouter_api_key, llm_base_url, llm_model and network reachability",
    ))
}

/// Build extract+LLM payload from free text and optional question/schema.
pub fn extract_with_llm(
    source_text: &str,
    question: Option<&str>,
    schema_json: Option<&str>,
) -> Result<Value, CliError> {
    let q = question.unwrap_or("Summarize the key facts from the content.");
    let system =
        "You are a careful extraction assistant for a local CLI. Answer concisely. No telemetry.";
    let user = format!("Question: {q}\n\nContent:\n{source_text}");
    let mut out = chat_completion(system, &user, schema_json)?;
    out["question"] = json!(q);
    out["source_chars"] = json!(source_text.chars().count());
    if let Some(s) = schema_json {
        if let Ok(parsed) =
            serde_json::from_str::<Value>(out.get("answer").and_then(|a| a.as_str()).unwrap_or(""))
        {
            out["json"] = parsed;
        }
        out["schema_requested"] = json!(true);
        let _ = s;
    }
    Ok(out)
}
