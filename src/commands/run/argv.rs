// SPDX-License-Identifier: MIT OR Apache-2.0
//! `exec` argv → single NDJSON step + unit tests.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

/// Map flat `exec` argv into a single NDJSON step object for the shared dispatcher.
///
/// Rejects unknown fields that look like silent discards (agent-first).
pub fn argv_to_step(args: &[String]) -> Result<Value, CliError> {
    if args.is_empty() {
        return Err(CliError::new(ErrorKind::Usage, "exec argv empty"));
    }
    let cmd = args[0].as_str();
    let mut step = json!({ "cmd": cmd });
    let obj = step.as_object_mut().ok_or_else(|| {
        CliError::new(ErrorKind::Software, "exec step object construction failed")
    })?;
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        if let Some(key) = a.strip_prefix("--") {
            let key = key.replace('-', "_");
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                let val = &args[i + 1];
                if val == "true" || val == "false" {
                    obj.insert(key, json!(val == "true"));
                } else if let Ok(n) = val.parse::<u64>() {
                    obj.insert(key, json!(n));
                } else if let Ok(n) = val.parse::<f64>() {
                    obj.insert(key, json!(n));
                } else {
                    obj.insert(key, json!(val));
                }
                i += 2;
            } else {
                obj.insert(key, json!(true));
                i += 1;
            }
        } else {
            // positional fallbacks by cmd
            match cmd {
                "goto" if !obj.contains_key("url") => {
                    obj.insert("url".into(), json!(a));
                }
                "press" | "write" | "hover" | "type" | "extract" | "attr" | "upload"
                    if !obj.contains_key("target") =>
                {
                    obj.insert("target".into(), json!(a));
                }
                "write" | "type" if obj.contains_key("target") && !obj.contains_key("value") => {
                    obj.insert("value".into(), json!(a));
                    obj.insert("text".into(), json!(a));
                }
                "keys" if !obj.contains_key("key") => {
                    obj.insert("key".into(), json!(a));
                }
                "eval" if !obj.contains_key("expression") => {
                    obj.insert("expression".into(), json!(a));
                }
                "wait" if !obj.contains_key("ms") => {
                    if let Ok(n) = a.parse::<u64>() {
                        obj.insert("ms".into(), json!(n));
                    }
                }
                // CLI flag is --detailed; JSON/tool-ref key remains verbose.
                "view" if a == "verbose" || a == "detailed" || a == "--detailed" => {
                    obj.insert("verbose".into(), json!(true));
                }
                "net" | "page" | "console" | "dialog" | "perf" | "heap" | "extension"
                | "devtools3p" | "webmcp" | "screencast"
                    if !obj.contains_key("action") =>
                {
                    obj.insert("action".into(), json!(a));
                }
                "net"
                    if obj.get("action").and_then(|v| v.as_str()) == Some("get")
                        && !obj.contains_key("id") =>
                {
                    obj.insert("id".into(), json!(a));
                }
                _ => {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("unrecognized exec argument: {a}"),
                        crate::i18n::suggestion_key("run_script_multi_step", None),
                    ));
                }
            }
            i += 1;
        }
    }
    Ok(step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_to_step_goto_and_wait_flags() {
        let step = argv_to_step(&[
            "wait".into(),
            "--ms".into(),
            "250".into(),
            "--state".into(),
            "networkidle".into(),
            "--include-snapshot".into(),
        ])
        .unwrap();
        assert_eq!(step["cmd"], "wait");
        assert_eq!(step["ms"], 250);
        assert_eq!(step["state"], "networkidle");
        assert_eq!(step["include_snapshot"], true);
    }

    #[test]
    fn argv_to_step_net_get_id() {
        let step = argv_to_step(&["net".into(), "get".into(), "req-abc".into()]).unwrap();
        assert_eq!(step["cmd"], "net");
        assert_eq!(step["action"], "get");
        assert_eq!(step["id"], "req-abc");
    }

    #[test]
    fn argv_to_step_press_target() {
        let step = argv_to_step(&[
            "press".into(),
            "#btn".into(),
            "--dblclick".into(),
            "--include-snapshot".into(),
        ])
        .unwrap();
        assert_eq!(step["target"], "#btn");
        assert_eq!(step["dblclick"], true);
        assert_eq!(step["include_snapshot"], true);
    }
}
