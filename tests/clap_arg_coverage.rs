//! Argument coverage / silent-discard guards (rules_rust_cli_com_clap).
//!
//! 1. Every top-level subcommand has help that clap can render.
//! 2. Global flags that must affect parse are actually present on the command tree.
//! 3. Payload flags renamed to avoid shadowing (`fields-json`, `cookies-json`, `detailed`)
//!    parse into the expected struct fields.
//! 4. Missing required args yield usage errors (exit-class 2 via clap).

use clap::{CommandFactory, Parser};

use browser_automation_cli::cli::{on_clap_stack, Cli, Commands};

#[test]
fn every_top_level_subcommand_renders_help() {
    on_clap_stack(|| {
        let cmd = Cli::command();
        for sub in cmd.get_subcommands() {
            let name = sub.get_name().to_string();
            let mut owned = sub.clone();
            let mut buf = Vec::new();
            owned.write_long_help(&mut buf).unwrap_or_else(|e| {
                panic!("help failed for subcommand {name}: {e}");
            });
            assert!(!buf.is_empty(), "empty help for subcommand {name}");
        }
    });
}

#[test]
fn required_global_flags_exist_on_command_tree() {
    on_clap_stack(|| {
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
    });
}

#[test]
fn fill_form_fields_json_parses_into_payload_field() {
    on_clap_stack(|| {
        let cli = Cli::try_parse_from([
            "browser-automation-cli",
            "fill-form",
            "--fields-json",
            r#"[{"target":"@e1","value":"x"}]"#,
        ])
        .expect("parse fill-form");
        match cli.command {
            Commands::FillForm(a) => {
                assert!(
                    a.fields_json.contains("@e1"),
                    "fields-json not consumed: {}",
                    a.fields_json
                );
            }
            other => panic!("expected FillForm, got {other:?}"),
        }
    });
}

#[test]
fn cookie_set_cookies_json_parses() {
    on_clap_stack(|| {
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
    });
}

#[test]
fn view_detailed_maps_to_verbose_field() {
    on_clap_stack(|| {
        let cli = Cli::try_parse_from(["browser-automation-cli", "view", "--detailed"])
            .expect("parse view --detailed");
        match cli.command {
            Commands::View(a) => assert!(a.verbose, "--detailed must set verbose field"),
            other => panic!("expected View, got {other:?}"),
        }
    });
}

#[test]
fn shadowing_payload_json_long_rejected_for_fill_form() {
    on_clap_stack(|| {
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
    });
}

#[test]
fn man_and_completions_parse() {
    on_clap_stack(|| {
        let man = Cli::try_parse_from(["browser-automation-cli", "man"]).expect("man");
        assert!(matches!(man.command, Commands::Man { .. }));
        let comp = Cli::try_parse_from(["browser-automation-cli", "completions", "bash"])
            .expect("completions");
        assert!(matches!(comp.command, Commands::Completions { .. }));
    });
}

#[test]
fn build_identity_has_version() {
    on_clap_stack(|| {
        let id = browser_automation_cli::build_identity();
        assert_eq!(id["name"], "browser-automation-cli");
        assert!(id["version"].as_str().is_some_and(|v| !v.is_empty()));
        assert!(id.get("git_sha").is_some());
        assert!(id.get("build_timestamp").is_some());
    });
}

/// D-04: sample additional Args→variant paths (silent discard smoke).
#[test]
fn more_subcommand_args_bind() {
    on_clap_stack(|| {
        let goto =
            Cli::try_parse_from(["browser-automation-cli", "goto", "about:blank"]).expect("goto");
        assert!(matches!(goto.command, Commands::Goto(_)));

        let doctor =
            Cli::try_parse_from(["browser-automation-cli", "doctor", "--offline", "--quick"])
                .expect("doctor");
        match doctor.command {
            Commands::Doctor(a) => assert!(a.offline && a.quick),
            other => panic!("expected Doctor, got {other:?}"),
        }

        let schema =
            Cli::try_parse_from(["browser-automation-cli", "schema", "run"]).expect("schema");
        match schema.command {
            Commands::Schema(a) => {
                let resolved = a.cmd_positional.or(a.cmd);
                assert_eq!(resolved.as_deref(), Some("run"));
            }
            other => panic!("expected Schema, got {other:?}"),
        }

        let plain = Cli::try_parse_from(["browser-automation-cli", "--plain", "version"])
            .expect("plain version");
        assert!(plain.globals.plain);
    });
}

/// Text recognition is excised from the product surface.
///
/// The agent consuming this CLI reads images natively, so an in-process OCR
/// stage is redundant middleware that spends tokens and drags an external C
/// binary into a rust-native tool. This gate is what stops it coming back.
///
/// It also retires BUG-CLAP-LANG-COLLISION at the source: `--ocr-lang` was the
/// only local flag that ever shadowed the global `--lang` (UI locale), and the
/// `#[arg(id = ...)]` workaround it needed is gone with the action.
#[test]
fn image_exposes_no_text_recognition_action() {
    on_clap_stack(|| {
        let cmd = Cli::command();
        let image = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "image")
            .expect("image subcommand");

        let actions: Vec<&str> = image
            .get_subcommands()
            .map(clap::Command::get_name)
            .collect();
        assert!(
            !actions.contains(&"ocr"),
            "image must not expose an ocr action: {actions:?}"
        );
        assert_eq!(
            actions,
            ["info", "convert", "resize", "download", "exif"],
            "image action set drifted"
        );

        // No surviving flag may re-open the pack under another spelling.
        for action in image.get_subcommands() {
            let longs: Vec<&str> = action
                .get_arguments()
                .filter(|a| !a.is_global_set())
                .filter_map(clap::Arg::get_long)
                .collect();
            assert!(
                !longs.contains(&"ocr-lang") && !longs.contains(&"engine"),
                "image {} still carries an OCR flag: {longs:?}",
                action.get_name()
            );
        }
    });
}

/// `image ocr` must fail as an unknown action, not silently resolve elsewhere.
#[test]
fn image_ocr_argv_is_rejected() {
    on_clap_stack(|| {
        let err = Cli::try_parse_from([
            "browser-automation-cli",
            "image",
            "ocr",
            "--path",
            "/tmp/a.png",
        ])
        .expect_err("image ocr must not parse");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    });
}
