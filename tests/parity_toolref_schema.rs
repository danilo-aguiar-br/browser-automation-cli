//! Gate: tool-ref optional/required flags must appear in CLI help for §5C parity.
//!
//! Does not require Chrome. Fails closed when a semantic flag is omitted.

use assert_cmd::cargo::cargo_bin_cmd;

fn help(args: &[&str]) -> String {
    let assert = cargo_bin_cmd!("browser-automation-cli")
        .args(args)
        .assert()
        .success();
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

#[test]
fn input_include_snapshot_on_all_toolref_tools() {
    for (cmd, needles) in [
        (&["hover", "--help"][..], &["include-snapshot"][..]),
        (&["drag", "--help"][..], &["include-snapshot"][..]),
        (&["keys", "--help"][..], &["include-snapshot"][..]),
        (&["upload", "--help"][..], &["include-snapshot"][..]),
        (&["fill-form", "--help"][..], &["include-snapshot"][..]),
        (&["press", "--help"][..], &["include-snapshot"][..]),
        (&["write", "--help"][..], &["include-snapshot"][..]),
        (&["click-at", "--help"][..], &["include-snapshot"][..]),
    ] {
        let h = help(cmd);
        for n in needles {
            assert!(h.contains(n), "{} --help missing `{n}`:\n{h}", cmd[0]);
        }
    }
}

#[test]
fn type_supports_focus_only_and_submit() {
    let h = help(&["type", "--help"]);
    assert!(
        h.contains("focus-only"),
        "type --help missing focus-only:\n{h}"
    );
    assert!(h.contains("submit"), "type --help missing submit:\n{h}");
}

#[test]
fn wait_supports_multi_text_and_timeout() {
    let h = help(&["wait", "--help"]);
    assert!(h.contains("text"), "wait --help missing text:\n{h}");
    assert!(
        h.contains("wait-timeout-ms") || h.contains("wait_timeout"),
        "wait --help missing wait-timeout-ms:\n{h}"
    );
}

#[test]
fn net_list_has_pagination_and_filters() {
    let h = help(&["net", "list", "--help"]);
    for n in [
        "page-idx",
        "page-size",
        "resource-types",
        "include-preserved",
    ] {
        assert!(h.contains(n), "net list --help missing `{n}`:\n{h}");
    }
}

#[test]
fn console_list_has_pagination_and_filters() {
    let h = help(&["console", "list", "--help"]);
    for n in ["page-idx", "page-size", "types", "include-preserved"] {
        assert!(h.contains(n), "console list --help missing `{n}`:\n{h}");
    }
}

#[test]
fn eval_has_function_args_dialog_filepath() {
    let h = help(&["eval", "--help"]);
    for n in ["args", "dialog-action", "file-path"] {
        assert!(h.contains(n), "eval --help missing `{n}`:\n{h}");
    }
}

#[test]
fn perf_has_autostop_and_insight_set() {
    let start = help(&["perf", "start", "--help"]);
    assert!(
        start.contains("auto-stop"),
        "perf start --help missing auto-stop:\n{start}"
    );
    let insight = help(&["perf", "insight", "--help"]);
    assert!(
        insight.contains("insight-set-id"),
        "perf insight --help missing insight-set-id:\n{insight}"
    );
}

#[test]
fn screencast_stop_accepts_path() {
    let h = help(&["screencast", "stop", "--help"]);
    assert!(
        h.contains("path"),
        "screencast stop --help missing path:\n{h}"
    );
}

#[test]
fn page_new_and_select_toolref_flags() {
    let new = help(&["page", "new", "--help"]);
    assert!(
        new.contains("background"),
        "page new missing background:\n{new}"
    );
    assert!(
        new.contains("isolated-context"),
        "page new missing isolated-context:\n{new}"
    );
    let sel = help(&["page", "select", "--help"]);
    assert!(
        sel.contains("bring-to-front"),
        "page select missing bring-to-front:\n{sel}"
    );
}

#[test]
fn heap_list_ops_have_pagination_flags() {
    let details = help(&["heap", "details", "--help"]);
    for n in ["filter-name", "page-idx", "page-size"] {
        assert!(
            details.contains(n),
            "heap details missing `{n}`:\n{details}"
        );
    }
    let paths = help(&["heap", "paths", "--help"]);
    assert!(
        paths.contains("max-depth"),
        "heap paths missing max-depth:\n{paths}"
    );
    assert!(
        paths.contains("max-nodes"),
        "heap paths missing max-nodes:\n{paths}"
    );
}

#[test]
fn goto_and_reload_nav_toolref_flags() {
    let g = help(&["goto", "--help"]);
    assert!(g.contains("init-script"), "goto missing init-script:\n{g}");
    assert!(
        g.contains("handle-before-unload"),
        "goto missing handle-before-unload:\n{g}"
    );
    let r = help(&["reload", "--help"]);
    assert!(
        r.contains("init-script"),
        "reload missing init-script:\n{r}"
    );
}
