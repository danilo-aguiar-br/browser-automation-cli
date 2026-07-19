//! Zero silent shadowing of global flags by local subcommand flags
//! (`rules_rust_cli_com_clap` — Armadilha Shadowing Silencioso).
//!
//! Enumerates clap `Command` tree: every global long/short must not collide
//! with a non-global argument on any subcommand.

use std::collections::HashSet;

use clap::CommandFactory;

use browser_automation_cli::cli::Cli;

fn collect_globals(cmd: &clap::Command) -> (HashSet<String>, HashSet<char>) {
    let mut longs = HashSet::new();
    let mut shorts = HashSet::new();
    for arg in cmd.get_arguments() {
        if !arg.is_global_set() {
            continue;
        }
        if let Some(l) = arg.get_long() {
            longs.insert(l.to_string());
        }
        if let Some(aliases) = arg.get_all_aliases() {
            for alias in aliases {
                longs.insert(alias.to_string());
            }
        }
        if let Some(s) = arg.get_short() {
            shorts.insert(s);
        }
        if let Some(ss) = arg.get_all_short_aliases() {
            for s in ss {
                shorts.insert(s);
            }
        }
    }
    (longs, shorts)
}

fn check_local_arg(
    path: &str,
    arg: &clap::Arg,
    global_longs: &HashSet<String>,
    global_shorts: &HashSet<char>,
    collisions: &mut Vec<String>,
) {
    if arg.is_global_set() {
        return;
    }
    if let Some(l) = arg.get_long() {
        if global_longs.contains(l) {
            collisions.push(format!("{path}: local --{l} shadows global --{l}"));
        }
    }
    if let Some(aliases) = arg.get_all_aliases() {
        for a in aliases {
            if global_longs.contains(a) {
                collisions.push(format!("{path}: local alias --{a} shadows global --{a}"));
            }
        }
    }
    if let Some(s) = arg.get_short() {
        if global_shorts.contains(&s) {
            collisions.push(format!("{path}: local -{s} shadows global -{s}"));
        }
    }
    if let Some(ss) = arg.get_all_short_aliases() {
        for s in ss {
            if global_shorts.contains(&s) {
                collisions.push(format!(
                    "{path}: local short alias -{s} shadows global -{s}"
                ));
            }
        }
    }
}

fn walk_locals(
    cmd: &clap::Command,
    path: &str,
    global_longs: &HashSet<String>,
    global_shorts: &HashSet<char>,
    collisions: &mut Vec<String>,
) {
    for arg in cmd.get_arguments() {
        check_local_arg(path, arg, global_longs, global_shorts, collisions);
    }
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        let child = if path.is_empty() {
            name.to_string()
        } else {
            format!("{path}/{name}")
        };
        walk_locals(sub, &child, global_longs, global_shorts, collisions);
    }
}

#[test]
fn no_global_flag_shadowed_by_local_subcommand_flags() {
    let cmd = Cli::command();
    let (global_longs, global_shorts) = collect_globals(&cmd);
    assert!(
        global_longs.contains("json"),
        "expected global --json in GlobalOpts"
    );
    assert!(
        global_longs.contains("verbose"),
        "expected global --verbose in GlobalOpts"
    );

    let mut collisions = Vec::new();
    walk_locals(&cmd, "", &global_longs, &global_shorts, &mut collisions);

    assert!(
        collisions.is_empty(),
        "global/local flag collisions detected:\n{}",
        collisions.join("\n")
    );
}

#[test]
fn clap_debug_assert_still_passes() {
    browser_automation_cli::command_factory_debug_assert();
}
