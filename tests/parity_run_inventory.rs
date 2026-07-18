//! GAP-017: every browser-side top-level command is either in `run` dispatch or intentional exclude.

use browser_automation_cli::commands_prd::run::{
    INTENTIONAL_RUN_EXCLUDE, RUN_DISPATCHED_CMDS,
};

/// Browser-side commands that agents may expect inside multi-step `run`.
const BROWSER_SIDE_TOP_LEVEL: &[&str] = &[
    "goto",
    "view",
    "press",
    "write",
    "keys",
    "type",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "upload",
    "back",
    "forward",
    "reload",
    "eval",
    "grab",
    "print-pdf",
    "extract",
    "text",
    "scroll",
    "cookie",
    "attr",
    "assert",
    "console",
    "net",
    "page",
    "dialog",
    "scrape",
    "emulate",
    "resize",
    "perf",
    "lighthouse",
    "screencast",
    "heap",
    "extension",
    "click-at",
];

#[test]
fn print_pdf_is_dispatched_in_run() {
    assert!(
        RUN_DISPATCHED_CMDS.contains(&"print-pdf") || RUN_DISPATCHED_CMDS.contains(&"print_pdf"),
        "GAP-001: print-pdf must be in RUN_DISPATCHED_CMDS"
    );
}

#[test]
fn browser_side_top_level_covered_by_run_or_exclude() {
    let dispatched: std::collections::BTreeSet<&str> =
        RUN_DISPATCHED_CMDS.iter().copied().collect();
    let excluded: std::collections::BTreeSet<&str> = INTENTIONAL_RUN_EXCLUDE
        .iter()
        .map(|(c, _)| *c)
        .collect();
    let mut missing = Vec::new();
    for cmd in BROWSER_SIDE_TOP_LEVEL {
        let underscored = cmd.replace('-', "_");
        let ok = dispatched.contains(cmd)
            || dispatched.contains(underscored.as_str())
            || excluded.contains(cmd)
            || excluded.iter().any(|e| e.starts_with(cmd));
        if !ok {
            missing.push(*cmd);
        }
    }
    assert!(
        missing.is_empty(),
        "browser-side cmds missing from run dispatch and intentional exclude: {missing:?}"
    );
}

#[test]
fn intentional_exclude_has_reasons() {
    for (cmd, reason) in INTENTIONAL_RUN_EXCLUDE {
        assert!(!cmd.is_empty());
        assert!(
            !reason.is_empty(),
            "exclude {cmd} must document a reason"
        );
    }
}
