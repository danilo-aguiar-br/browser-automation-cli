// SPDX-License-Identifier: MIT OR Apache-2.0
//! In-memory console/error event tracker (bounded ring).
use serde_json::{json, Value};

/// One `console.*` call observed on the page.
#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    /// Console method that produced it: `log`, `warn`, `error`, `debug`, `info`, …
    ///
    /// Serialized as `type` in the envelope, matching the CDP field name.
    pub level: String,
    /// Message text, already flattened from the call arguments.
    pub text: String,
    /// Original arguments, kept structured so a consumer can read an object the
    /// page logged instead of parsing it back out of `text`. Often empty.
    pub args: Vec<Value>,
}

/// One uncaught page error.
#[derive(Debug, Clone)]
pub struct ErrorEntry {
    /// Error text as reported by the runtime.
    pub text: String,
    /// Script URL the error came from, when the runtime named one.
    pub url: Option<String>,
    /// Line of the error location. CDP counts from 0.
    pub line: Option<i64>,
    /// Column of the error location. CDP counts from 0.
    pub column: Option<i64>,
}

/// Bounded in-memory buffer of console and error events for this invocation.
///
/// A page can log without limit, so both buffers are capped and drop from the
/// FRONT when full: the newest events are the ones an agent is asking about,
/// and an unbounded buffer would let a noisy page decide this process's memory.
///
/// The buffer lives and dies with the process. Nothing is captured before
/// `--capture-console` attaches the forwarder.
pub struct EventTracker {
    /// Console calls in arrival order, oldest first.
    pub console_entries: Vec<ConsoleEntry>,
    /// Uncaught errors in arrival order, oldest first.
    pub error_entries: Vec<ErrorEntry>,
    /// Cap applied to EACH buffer independently, from XDG policy.
    pub max_entries: usize,
}

impl Default for EventTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl EventTracker {
    /// Empty tracker with the cap resolved from XDG policy.
    pub fn new() -> Self {
        Self {
            console_entries: Vec::new(),
            error_entries: Vec::new(),
            max_entries: crate::xdg::policy::policy_usize(
                crate::xdg::policy::key::EVENT_TRACKER_MAX_ENTRIES,
            ),
        }
    }

    /// Record a console call, evicting the oldest entry when the cap is reached.
    pub fn add_console(&mut self, level: &str, text: &str, args: Vec<Value>) {
        if self.console_entries.len() >= self.max_entries {
            self.console_entries.remove(0);
        }
        self.console_entries.push(ConsoleEntry {
            level: level.to_string(),
            text: text.to_string(),
            args,
        });
    }

    /// Record a page error, evicting the oldest entry when the cap is reached.
    pub fn add_error(
        &mut self,
        text: &str,
        url: Option<&str>,
        line: Option<i64>,
        col: Option<i64>,
    ) {
        if self.error_entries.len() >= self.max_entries {
            self.error_entries.remove(0);
        }
        self.error_entries.push(ErrorEntry {
            text: text.to_string(),
            url: url.map(String::from),
            line,
            column: col,
        });
    }

    /// Drop buffered console entries, leaving errors untouched.
    pub fn clear_console(&mut self) {
        self.console_entries.clear();
    }

    /// Console buffer as the `{ "messages": [...] }` envelope payload.
    ///
    /// `args` is omitted when empty rather than emitted as `[]`, which keeps the
    /// common entry small in a payload an agent pays tokens for.
    pub fn get_console_json(&self) -> Value {
        let messages: Vec<Value> = self
            .console_entries
            .iter()
            .map(|e| {
                let mut msg = json!({ "type": e.level, "text": e.text });
                if !e.args.is_empty() {
                    if let Some(obj) = msg.as_object_mut() {
                        obj.insert("args".to_string(), Value::Array(e.args.clone()));
                    }
                }
                msg
            })
            .collect();
        json!({ "messages": messages })
    }

    /// Error buffer as the `{ "errors": [...] }` envelope payload.
    pub fn get_errors_json(&self) -> Value {
        let entries: Vec<Value> = self
            .error_entries
            .iter()
            .map(|e| {
                json!({
                    "text": e.text,
                    "url": e.url,
                    "line": e.line,
                    "column": e.column,
                })
            })
            .collect();
        json!({ "errors": entries })
    }
}
