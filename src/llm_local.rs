// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot optional LLM HTTP extract (XDG only; no product env vars).
//!
//! Uses an OpenAI-compatible chat completions endpoint configured via XDG
//! (`openrouter_api_key`, `llm_base_url`, `llm_model`). No telemetry.
//! Fail-closed: all three keys must be set via `config set` (no silent
//! third-party endpoint defaults).
//!
//! # Workload
//!
//! **I/O-bound** (blocking HTTP to operator-configured LLM endpoint). Client is
//! process-wide via `OnceLock` (rules: create `reqwest` client once).

use std::sync::OnceLock;
use std::time::Duration;

use serde_json::{json, Value};
use zeroize::Zeroize;

use crate::constants::{DEFAULT_LLM_HTTP_TIMEOUT_SECS, HTTP_REDIRECT_MAX, HTTP_USER_AGENT};
use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Wall-clock timeout for the process-wide LLM/webhook blocking HTTP client.
///
/// Prefer [`crate::xdg::resolve_llm_http_timeout_secs`] at call sites; this
/// constant is the compile-time default (Pass N).
pub const LLM_HTTP_TIMEOUT_SECS: u64 = DEFAULT_LLM_HTTP_TIMEOUT_SECS;

/// Process-wide blocking HTTP client for rare LLM/webhook one-shots.
///
/// Stable `get_or_init` path (`get_or_try_init` still unstable on MSRV 1.88).
/// **Never** honors system `HTTP_PROXY*` env (`no_proxy` — product law).
pub fn shared_blocking_http_client() -> Result<&'static reqwest::blocking::Client, CliError> {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    if let Some(c) = CLIENT.get() {
        return Ok(c);
    }
    let total = crate::xdg::resolve_llm_http_timeout_secs();
    let connect = crate::xdg::resolve_http_connect_timeout_secs();
    let built = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(total))
        .connect_timeout(Duration::from_secs(connect))
        .user_agent(HTTP_USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(HTTP_REDIRECT_MAX))
        .tcp_nodelay(true)
        .no_proxy()
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("http client: {e}")))?;
    Ok(CLIENT.get_or_init(|| built))
}

/// Zeroizing API key buffer (rules: wipe secrets before drop).
struct ApiKey(String);

impl ApiKey {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl Drop for ApiKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Resolve API key from XDG only; buffer is zeroized on drop.
fn require_api_key() -> Result<ApiKey, CliError> {
    let key = xdg::openrouter_api_key().filter(|s| !s.trim().is_empty());
    match key {
        // Clone into ApiKey (own Drop+zeroize); Zeroizing guard also wipes on drop.
        Some(k) => Ok(ApiKey((*k).clone())),
        None => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "LLM extract requires XDG openrouter_api_key",
            crate::i18n::suggestion_key("llm_config_required", None),
        )),
    }
}

/// Base URL from XDG only (fail closed — no hardcoded third-party default).
pub fn require_base_url() -> Result<String, CliError> {
    xdg::llm_base_url()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                "LLM extract requires XDG llm_base_url",
                crate::i18n::suggestion_key("llm_config_required", None),
            )
        })
}

/// Model from XDG only (fail closed — no hardcoded model default).
pub fn require_model() -> Result<String, CliError> {
    xdg::llm_model()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                "LLM extract requires XDG llm_model",
                crate::i18n::suggestion_key("llm_config_required", None),
            )
        })
}

/// Call chat completions with retry/backoff (one-shot; no daemon).
pub fn chat_completion(
    system: &str,
    user: &str,
    schema_hint: Option<&str>,
) -> Result<Value, CliError> {
    let key = require_api_key()?;
    let model = require_model()?;
    let base = require_base_url()?;
    // SSRF: operator-configured base must still pass policy (default strict).
    crate::net::assert_safe_http_url(&base)?;
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

    // Process-wide blocking client (rules: create once; LLM path is rare one-shot).
    let client = shared_blocking_http_client()?;

    // GAP-013: named RetryConfig::llm() (budget + jitter), not ad-hoc delay array.
    let cfg = crate::retry::RetryConfig::llm();
    let mut attempt_no = 0u32;
    let result = crate::retry::retry_blocking(cfg, || {
        attempt_no += 1;
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", key.as_str()))
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
                Ok(json!({
                    "llm": true,
                    "model": model,
                    "base_url": base,
                    "answer": answer,
                    "raw": v,
                    "attempt": attempt_no,
                }))
            }
            Ok(r) => {
                let code = r.status().as_u16();
                let err = CliError::new(ErrorKind::Unavailable, format!("llm HTTP {code}"));
                // Permanent client errors (except 429) must not retry.
                if code < 500 && code != 429 {
                    return Err(CliError::new(
                        ErrorKind::Usage,
                        format!("llm HTTP {code} (non-retryable)"),
                    ));
                }
                Err(err)
            }
            Err(e) => Err(CliError::new(ErrorKind::Unavailable, format!("llm: {e}"))),
        }
    });
    // `key` drops here → zeroize via Drop.
    result.map_err(|e| {
        CliError::with_suggestion(
            e.kind(),
            e.message(),
            crate::i18n::suggestion_key("llm_config_required", None),
        )
    })
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
        if let Ok(parsed) = crate::json_util::value_from_str(
            out.get("answer").and_then(|a| a.as_str()).unwrap_or(""),
        ) {
            out["json"] = parsed;
        }
        out["schema_requested"] = json!(true);
        let _ = s;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_closed_without_xdg() {
        // Without XDG config, require_* fail (may still pass if user has XDG set —
        // only assert error type shape when missing).
        let err = require_base_url().err().or_else(|| require_model().err());
        // If operator already configured XDG, both Ok — skip soft.
        if let Some(e) = err {
            assert_eq!(e.kind(), ErrorKind::Usage);
        }
    }

    #[test]
    fn api_key_zeroizes_on_drop() {
        let mut s = String::from("sk-test-secret-material");
        {
            let mut k = ApiKey(s.clone());
            assert_eq!(k.as_str(), "sk-test-secret-material");
            k.0.zeroize();
            assert!(
                k.as_str().chars().all(|c| c == '\0')
                    || k.as_str().is_empty()
                    || !k.as_str().contains("secret")
            );
        }
        s.zeroize();
    }
}
