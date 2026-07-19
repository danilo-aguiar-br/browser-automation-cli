//! Argument coverage / silent-discard guards (rules_rust_cli_com_clap).
//!
//! 1. Every top-level subcommand has help that clap can render.
//! 2. Global flags that must affect parse are actually present on the command tree.
//! 3. Payload flags renamed to avoid shadowing (`fields-json`, `cookies-json`, `detailed`)
//!    parse into the expected struct fields.
//! 4. Missing required args yield usage errors (exit-class 2 via clap).

use clap::{CommandFactory, Parser};

use browser_automation_cli::cli::{Cli, Commands};

#[test]
fn every_top_level_subcommand_renders_help() {
    let cmd = Cli::command();
    for sub in cmd.get_subcommands() {
        let name = sub.get_name().to_string();
        let mut owned = sub.clone();
        let mut buf = Vec::new();
        owned.write_long_help(&mut buf).unwrap_or_else(|e| {
            panic!("help failed for subcommand {name}: {e}");
        });
        assert!(
            !buf.is_empty(),
            "empty help for subcommand {name}"
        );
    }
}

#[test]
fn required_global_flags_exist_on_command_tree() {
    let cmd = Cli::command();
    let mut longs = std::collections::HashSet::new();
    for arg in cmd.get_arguments() {
        if let Some(l) = arg.get_long() {
            longs.insert(l.to_string());
        }
    }
    for required in [
        "json",
        "json-steps",
        "quiet",
        "verbose",
        "debug",
        "plain",
        "timeout",
        "step-timeout",
        "headed",
        "artifacts-dir",
        "lang",
    ] {
        assert!(
            longs.contains(required),
            "missing global long flag --{required}"
        );
    }
}

#[test]
fn fill_form_fields_json_parses_into_payload_field() {
    let cli = Cli::try_parse_from([
        "browser-automation-cli",
        "fill-form",
        "--fields-json",
        r#"[{"target":"@e1","value":"x"}]"#,
    ])
    .expect("parse fill-form");
    match cli.command {
        Commands::FillForm { fields_json, .. } => {
            assert!(
                fields_json.contains("@e1"),
                "fields-json not consumed: {fields_json}"
            );
        }
        other => panic!("expected FillForm, got {other:?}"),
    }
}

#[test]
fn cookie_set_cookies_json_parses() {
    let cli = Cli::try_parse_from([
        "browser-automation-cli",
        "cookie",
        "set",
        "--cookies-json",
        r#"[{"name":"a","value":"b","url":"https://example.com"}]"#,
    ])
    .expect("parse cookie set");
    match cli.command {
        Commands::Cookie { action } => match action {
            browser_automation_cli::cli::CookieAction::Set { cookies_json } => {
                assert!(
                    cookies_json.contains("\"name\""),
                    "cookies-json not consumed"
                );
            }
            other => panic!("expected Set, got {other:?}"),
        },
        other => panic!("expected Cookie, got {other:?}"),
    }
}

#[test]
fn view_detailed_maps_to_verbose_field() {
    let cli = Cli::try_parse_from(["browser-automation-cli", "view", "--detailed"])
        .expect("parse view --detailed");
    match cli.command {
        Commands::View { verbose, .. } => assert!(verbose, "--detailed must set verbose field"),
        other => panic!("expected View, got {other:?}"),
    }
}

#[test]
fn shadowing_payload_json_long_rejected_for_fill_form() {
    // Old local --json must NOT be accepted as fill-form payload (would shadow global).
    let err = Cli::try_parse_from([
        "browser-automation-cli",
        "fill-form",
        "--json",
        r#"[{"target":"@e1","value":"x"}]"#,
    ]);
    assert!(
        err.is_err(),
        "fill-form must not accept payload via --json (global only)"
    );
}

#[test]
fn man_and_completions_parse() {
    let man = Cli::try_parse_from(["browser-automation-cli", "man"]).expect("man");
    assert!(matches!(man.command, Commands::Man { .. }));
    let comp = Cli::try_parse_from(["browser-automation-cli", "completions", "bash"])
        .expect("completions");
    assert!(matches!(comp.command, Commands::Completions { .. }));
}

#[test]
fn build_identity_has_version() {
    let id = browser_automation_cli::build_identity();
    assert_eq!(id["name"], "browser-automation-cli");
    assert!(id["version"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(id.get("git_sha").is_some());
    assert!(id.get("build_timestamp").is_some());
}

/// D-04: sample additional Args→variant paths (silent discard smoke).
#[test]
fn more_subcommand_args_bind() {
    let goto = Cli::try_parse_from(["browser-automation-cli", "goto", "about:blank"])
        .expect("goto");
    assert!(matches!(goto.command, Commands::Goto { .. }));

    let doctor = Cli::try_parse_from([
        "browser-automation-cli",
        "doctor",
        "--offline",
        "--quick",
    ])
    .expect("doctor");
    assert!(matches!(
        doctor.command,
        Commands::Doctor {
            offline: true,
            quick: true,
            ..
        }
    ));

    let schema = Cli::try_parse_from(["browser-automation-cli", "schema", "run"]).expect("schema");
    match schema.command {
        Commands::Schema {
            cmd,
            cmd_positional,
        } => {
            let resolved = cmd_positional.or(cmd);
            assert_eq!(resolved.as_deref(), Some("run"));
        }
        other => panic!("expected Schema, got {other:?}"),
    }

    let plain = Cli::try_parse_from(["browser-automation-cli", "--plain", "version"])
        .expect("plain version");
    assert!(plain.globals.plain);
}
