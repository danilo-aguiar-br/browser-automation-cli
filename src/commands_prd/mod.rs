//! PRD command dispatch (paths leves + Camada A/B browser one-shot).
#![allow(missing_docs)]

mod meta;
mod run;

use std::path::Path;

use crate::browser::{
    block_on_browser_timeout, run_goto_with_robots, run_keys, run_press, run_scrape, run_type,
    run_view, run_write, CaptureOpts, OneShotSession,
};
use crate::cli::{
    AssertKind, Cli, Commands, CompletionShell, ConsoleAction, CookieAction, Devtools3pAction,
    DialogAction, ExtensionAction, GrabFormat, HeapAction, NetAction, PageAction, PerfAction,
    ScreencastAction, WebmcpAction,
};
use crate::envelope::{print_error_json, print_success_json};
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

/// Dispatch parsed CLI. Returns process exit code (0 success).
pub fn dispatch(cli: Cli, life: &Lifecycle) -> i32 {
    let json = cli.globals.json
        || matches!(
            &cli.command,
            Commands::Doctor { json: true, .. } | Commands::Commands { json: true }
        );
    let capture = CaptureOpts {
        console: cli.globals.capture_console,
        network: cli.globals.capture_network,
    };
    let timeout_secs = cli.globals.timeout;
    let artifacts = cli.globals.artifacts_dir.clone();
    let category_memory = cli.globals.category_memory;
    let category_extensions = cli.globals.category_extensions;
    let category_third_party = cli.globals.category_third_party;
    let category_webmcp = cli.globals.category_webmcp;
    let experimental_screencast = cli.globals.experimental_screencast;
    let experimental_vision = cli.globals.experimental_vision;
    let robots =
        match RobotsPolicy::from_flags(cli.globals.ignore_robots, cli.globals.i_accept_robots_risk)
        {
            Ok(p) => p,
            Err(e) => {
                life.finalize();
                return emit_err(&e, json);
            }
        };

    let code = match cli.command {
        Commands::Doctor {
            offline,
            quick,
            fix,
            json: doc_json,
        } => crate::doctor::run_doctor(crate::doctor::DoctorOptions {
            offline,
            quick,
            fix,
            json: json || doc_json,
        }),
        Commands::Commands { json: cmd_json } => match meta::list_commands(json || cmd_json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json || cmd_json),
        },
        Commands::Schema { cmd } => match meta::schema_for_cmd(&cmd, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Version => match handle_version(json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Goto {
            url,
            init_script,
            handle_before_unload,
            navigation_timeout_ms,
        } => {
            match handle_goto(
                life,
                &url,
                robots,
                capture,
                timeout_secs,
                json,
                init_script.as_deref(),
                handle_before_unload,
                navigation_timeout_ms,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::View { verbose, path } => {
            match handle_view(life, verbose, path.as_deref(), capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Press {
            target,
            dblclick,
            include_snapshot,
        } => {
            match handle_press(
                life,
                &target,
                dblclick,
                include_snapshot,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::ClickAt {
            x,
            y,
            dblclick,
            include_snapshot,
        } => {
            if !experimental_vision {
                let e = CliError::with_suggestion(
                    ErrorKind::Usage,
                    "click-at requires --experimental-vision",
                    "Pass --experimental-vision (or BROWSER_AUTOMATION_CLI_EXPERIMENTAL_VISION=1)",
                );
                emit_err(&e, json)
            } else {
                match handle_click_at(
                    life,
                    x,
                    y,
                    dblclick,
                    include_snapshot,
                    capture,
                    timeout_secs,
                    json,
                ) {
                    Ok(()) => 0,
                    Err(e) => emit_err(&e, json),
                }
            }
        }
        Commands::Write {
            target,
            value,
            include_snapshot,
        } => {
            match handle_write(
                life,
                &target,
                &value,
                include_snapshot,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Keys {
            key,
            include_snapshot,
        } => match handle_keys(life, &key, include_snapshot, capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Type {
            target,
            text,
            clear,
            submit,
            focus_only,
        } => match handle_type(
            life,
            target.as_deref(),
            &text,
            clear,
            submit.as_deref(),
            focus_only,
            capture,
            timeout_secs,
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Wait {
            ms,
            text,
            selector,
            state,
            wait_timeout_ms,
            include_snapshot,
        } => {
            match handle_wait(
                life,
                ms,
                &text,
                selector.as_deref(),
                state.as_deref(),
                wait_timeout_ms,
                include_snapshot,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Hover {
            target,
            include_snapshot,
        } => match handle_hover(life, &target, include_snapshot, capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Drag {
            from,
            to,
            include_snapshot,
        } => {
            match handle_drag(
                life,
                &from,
                &to,
                include_snapshot,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::FillForm {
            json: fields_json,
            include_snapshot,
        } => {
            match handle_fill_form(
                life,
                &fields_json,
                include_snapshot,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Upload {
            target,
            path,
            include_snapshot,
        } => {
            match handle_upload(
                life,
                &target,
                &path,
                include_snapshot,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Back => match handle_history(life, "back", capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Forward => match handle_history(life, "forward", capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Reload {
            ignore_cache,
            init_script,
            handle_before_unload,
        } => {
            match handle_reload(
                life,
                ignore_cache,
                init_script.as_deref(),
                handle_before_unload,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Eval {
            expression,
            args,
            dialog_action,
            file_path,
        } => {
            match handle_eval(
                life,
                &expression,
                args.as_deref(),
                dialog_action.as_deref(),
                file_path.as_deref(),
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Grab {
            path,
            format,
            full_page,
            quality,
            element,
        } => match handle_grab(
            life,
            path.as_deref(),
            format,
            full_page,
            quality,
            element.as_deref(),
            artifacts.as_deref(),
            capture,
            timeout_secs,
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Run { script } => {
            let flags = run::RunFlags {
                experimental_vision,
                experimental_screencast,
                category_memory,
            };
            match handle_run(life, &script, robots, capture, timeout_secs, json, flags) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Exec { args } => {
            // trailing_var_arg can capture global flags placed after `exec`;
            // peel them so agents can write: exec goto URL --json
            let (args, json_from_trail) = peel_trailing_globals(&args);
            let json = json || json_from_trail;
            let flags = run::RunFlags {
                experimental_vision,
                experimental_screencast,
                category_memory,
            };
            match handle_exec(life, &args, robots, capture, timeout_secs, json, flags) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Extract { target, attr } => {
            match handle_extract(life, &target, attr.as_deref(), capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Text { target } => {
            match handle_text(life, &target, capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Scroll {
            target,
            delta_x,
            delta_y,
        } => match handle_scroll(
            life,
            target.as_deref(),
            delta_x,
            delta_y,
            capture,
            timeout_secs,
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Cookie { action } => {
            match handle_cookie(life, action, capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Attr { target, name } => {
            match handle_attr(life, &target, &name, capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Assert { kind } => match handle_assert(life, kind, capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Console { action } => {
            match handle_console(life, action, capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Net { action } => match handle_net(life, action, capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Page { action } => match handle_page(life, action, capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Dialog { action } => {
            match handle_dialog(life, action, capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Scrape { url } => {
            match handle_scrape(life, &url, robots, capture, timeout_secs, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Emulate {
            user_agent,
            locale,
            timezone,
            offline,
            latitude,
            longitude,
            media,
            network_conditions,
            cpu_throttling_rate,
            color_scheme,
            extra_headers,
            viewport,
        } => match handle_emulate(
            life,
            user_agent.as_deref(),
            locale.as_deref(),
            timezone.as_deref(),
            offline,
            latitude,
            longitude,
            media.as_deref(),
            network_conditions.as_deref(),
            cpu_throttling_rate,
            color_scheme.as_deref(),
            extra_headers.as_deref(),
            viewport.as_deref(),
            capture,
            timeout_secs,
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Resize {
            width,
            height,
            scale,
            mobile,
        } => match handle_resize(
            life,
            width,
            height,
            scale,
            mobile,
            capture,
            timeout_secs,
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Perf { action } => match handle_perf(life, action, capture, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Lighthouse {
            url,
            out_dir,
            device,
            mode,
            lighthouse_path,
        } => match handle_lighthouse(
            &url,
            out_dir.as_deref(),
            &device,
            &mode,
            lighthouse_path.as_deref(),
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Screencast { action } => {
            if !experimental_screencast {
                emit_err(
                    &CliError::with_suggestion(
                        ErrorKind::Usage,
                        "screencast requires --experimental-screencast",
                        "Pass --experimental-screencast on the same invocation",
                    ),
                    json,
                )
            } else {
                match handle_screencast(life, action, capture, timeout_secs, json) {
                    Ok(()) => 0,
                    Err(e) => emit_err(&e, json),
                }
            }
        }
        Commands::Heap { action } => {
            let deep = !matches!(
                &action,
                HeapAction::Take { .. } | HeapAction::Summary { .. } | HeapAction::Close { .. }
            );
            if deep && !category_memory {
                emit_err(
                    &CliError::with_suggestion(
                        ErrorKind::Usage,
                        "deep heap tools require --category-memory",
                        "Pass --category-memory (heap take/summary/close work without deep graph ops)",
                    ),
                    json,
                )
            } else {
                match handle_heap(life, action, capture, timeout_secs, json) {
                    Ok(()) => 0,
                    Err(e) => emit_err(&e, json),
                }
            }
        }
        Commands::Extension { action } => {
            if !category_extensions {
                emit_err(
                    &CliError::with_suggestion(
                        ErrorKind::Usage,
                        "extension tools require --category-extensions",
                        "Pass --category-extensions on the same invocation",
                    ),
                    json,
                )
            } else {
                match handle_extension(life, action, capture, timeout_secs, json) {
                    Ok(()) => 0,
                    Err(e) => emit_err(&e, json),
                }
            }
        }
        Commands::Devtools3p { action } => {
            if !category_third_party {
                emit_err(
                    &CliError::with_suggestion(
                        ErrorKind::Usage,
                        "devtools3p requires --category-third-party",
                        "Pass --category-third-party on the same invocation",
                    ),
                    json,
                )
            } else {
                match handle_devtools3p(life, action, capture, timeout_secs, json) {
                    Ok(()) => 0,
                    Err(e) => emit_err(&e, json),
                }
            }
        }
        Commands::Webmcp { action } => {
            if !category_webmcp {
                emit_err(
                    &CliError::with_suggestion(
                        ErrorKind::Usage,
                        "webmcp requires --category-webmcp",
                        "Pass --category-webmcp on the same invocation",
                    ),
                    json,
                )
            } else {
                match handle_webmcp(life, action, capture, timeout_secs, json) {
                    Ok(()) => 0,
                    Err(e) => emit_err(&e, json),
                }
            }
        }
        Commands::Completions { shell } => match handle_completions(shell) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
    };

    life.finalize();
    code
}

fn handle_version(json: bool) -> Result<(), CliError> {
    let data = serde_json::json!({
        "name": "browser-automation-cli",
        "version": env!("CARGO_PKG_VERSION"),
    });
    emit_ok(data, json, |d| {
        println!(
            "{}",
            d.get("version").and_then(|v| v.as_str()).unwrap_or("")
        );
    })
}

fn emit_ok<F>(data: serde_json::Value, json: bool, text: F) -> Result<(), CliError>
where
    F: FnOnce(&serde_json::Value),
{
    if json {
        print_success_json(data)?;
    } else {
        text(&data);
    }
    Ok(())
}

fn emit_err(err: &CliError, json: bool) -> i32 {
    if json {
        let _ = print_error_json(err);
    } else {
        eprintln!("error: {err}");
        if let Some(s) = err.suggestion() {
            eprintln!("suggestion: {s}");
        }
    }
    err.exit_code() as i32
}

/// Peel known global flags mistakenly captured by `exec` trailing_var_arg.
fn peel_trailing_globals(args: &[String]) -> (Vec<String>, bool) {
    let mut json = false;
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        match a.as_str() {
            "--json" => json = true,
            other => out.push(other.to_string()),
        }
    }
    (out, json)
}

#[allow(clippy::too_many_arguments)]
fn handle_goto(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    init_script: Option<&str>,
    handle_before_unload: bool,
    navigation_timeout_ms: Option<u64>,
) -> Result<(), CliError> {
    let _ = (init_script, handle_before_unload, navigation_timeout_ms);
    // init_script / beforeunload applied inside session when multi-step run is used;
    // single-shot goto keeps robots path; flags accepted for tool-ref CLI parity.
    let data = block_on_browser_timeout(
        run_goto_with_robots(life, url, capture, robots),
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        let u = d.get("url").and_then(|v| v.as_str()).unwrap_or(url);
        let t = d.get("title").and_then(|v| v.as_str()).unwrap_or("");
        println!("ok url={u} title={t}");
    })
}

fn handle_view(
    life: &Lifecycle,
    verbose: bool,
    path: Option<&Path>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let path_owned = path.map(|p| p.to_path_buf());
    let data = block_on_browser_timeout(
        async move {
            let mut data = run_view(life, verbose, capture).await?;
            if let Some(p) = path_owned.as_ref() {
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            CliError::new(ErrorKind::Io, format!("view --path mkdir: {e}"))
                        })?;
                    }
                }
                let tree = data.get("tree").and_then(|v| v.as_str()).unwrap_or("");
                std::fs::write(p, tree.as_bytes())
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("view --path write: {e}")))?;
                if let Some(obj) = data.as_object_mut() {
                    obj.insert(
                        "path".to_string(),
                        serde_json::Value::String(p.display().to_string()),
                    );
                }
            }
            Ok(data)
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        if let Some(p) = d.get("path").and_then(|v| v.as_str()) {
            println!("ok view path={p}");
        } else if let Some(tree) = d.get("tree").and_then(|v| v.as_str()) {
            print!("{tree}");
            if !tree.ends_with('\n') {
                println!();
            }
        } else {
            println!("ok view");
        }
    })
}

fn handle_press(
    life: &Lifecycle,
    target: &str,
    dblclick: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        run_press(life, target, dblclick, include_snapshot, capture),
        timeout_secs,
    )?;
    emit_ok(data, json, |_| {
        println!("ok pressed={target} dblclick={dblclick}");
    })
}

fn handle_write(
    life: &Lifecycle,
    target: &str,
    value: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        run_write(life, target, value, include_snapshot, capture),
        timeout_secs,
    )?;
    emit_ok(data, json, |_| {
        println!("ok written={target} len={}", value.len());
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_click_at(
    life: &Lifecycle,
    x: f64,
    y: f64,
    dblclick: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = session.click_at(x, y, dblclick, include_snapshot).await;
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |_| {
        println!("ok click-at x={x} y={y} dbl={dblclick}")
    })
}

fn handle_keys(
    life: &Lifecycle,
    key: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data =
        block_on_browser_timeout(run_keys(life, key, include_snapshot, capture), timeout_secs)?;
    emit_ok(data, json, |_| println!("ok key={key}"))
}

#[allow(clippy::too_many_arguments)]
fn handle_type(
    life: &Lifecycle,
    target: Option<&str>,
    text: &str,
    clear: bool,
    submit: Option<&str>,
    focus_only: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    if target.is_none() && !focus_only {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "type requires a target or --focus-only",
            "Pass TARGET or --focus-only for the focused element",
        ));
    }
    let data = block_on_browser_timeout(
        run_type(life, target, text, clear, submit, focus_only, capture),
        timeout_secs,
    )?;
    let label = target.unwrap_or("(focused)");
    emit_ok(data, json, |_| {
        println!(
            "ok typed={label} len={} clear={clear} submit={submit:?} focus_only={focus_only}",
            text.len()
        );
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_wait(
    life: &Lifecycle,
    ms: u64,
    texts: &[String],
    selector: Option<&str>,
    state: Option<&str>,
    wait_timeout_ms: Option<u64>,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let texts_owned = texts.to_vec();
    let selector_owned = selector.map(|s| s.to_string());
    let state_owned = state.map(|s| s.to_string());
    // Prefer explicit wait_timeout_ms for text/selector; fall back to ms
    let wait_ms = wait_timeout_ms.or(if ms == 0 { None } else { Some(ms) });
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            // OR semantics: try each text until one succeeds (tool-ref wait_for array)
            let r = if texts_owned.is_empty() {
                session
                    .wait_for(
                        wait_ms,
                        None,
                        selector_owned.as_deref(),
                        state_owned.as_deref(),
                        include_snapshot,
                    )
                    .await
            } else {
                let mut last_err = None;
                let mut ok = None;
                for t in &texts_owned {
                    match session
                        .wait_for(
                            wait_ms,
                            Some(t.as_str()),
                            selector_owned.as_deref(),
                            state_owned.as_deref(),
                            false,
                        )
                        .await
                    {
                        Ok(v) => {
                            ok = Some(v);
                            break;
                        }
                        Err(e) => last_err = Some(e),
                    }
                }
                match ok {
                    Some(v) => {
                        if include_snapshot {
                            Ok(session
                                .attach_snapshot_if(true, v)
                                .await
                                .unwrap_or_else(|_| serde_json::json!({"waited": true})))
                        } else {
                            Ok(v)
                        }
                    }
                    None => Err(last_err.unwrap_or_else(|| {
                        CliError::new(ErrorKind::Browser, "wait: no text matched")
                    })),
                }
            };
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        println!("ok wait {}", d);
    })
}

fn with_session_blank<F, Fut>(
    life: &Lifecycle,
    capture: CaptureOpts,
    timeout_secs: u64,
    f: F,
) -> Result<serde_json::Value, CliError>
where
    F: FnOnce(OneShotSession) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(OneShotSession, serde_json::Value), CliError>> + Send,
{
    block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let (session, value) = f(session).await?;
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            Ok(value)
        },
        timeout_secs,
    )
}

fn handle_hover(
    life: &Lifecycle,
    target: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target = target.to_string();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.hover(&target, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| println!("ok hover"))
}

fn handle_drag(
    life: &Lifecycle,
    from: &str,
    to: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let from = from.to_string();
    let to = to.to_string();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.drag(&from, &to, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| println!("ok drag"))
}

fn handle_fill_form(
    life: &Lifecycle,
    fields_json: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let parsed: serde_json::Value = serde_json::from_str(fields_json).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("fill-form --json parse error: {e}"),
            r##"Pass JSON array: [{"target":"input","value":"x"}] or [{"uid":"@e1","value":"x"}]"##,
        )
    })?;
    let arr = parsed.as_array().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            "fill-form --json must be a JSON array",
            r##"[{"target":"input","value":"x"}]"##,
        )
    })?;
    let mut fields = Vec::new();
    for item in arr {
        let target = item
            .get("target")
            .or_else(|| item.get("uid"))
            .or_else(|| item.get("selector"))
            .or_else(|| item.get("ref"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CliError::new(
                    ErrorKind::Usage,
                    "fill-form field missing target/uid/selector/ref",
                )
            })?
            .to_string();
        // Normalize bare eN → @eN
        let target = if target.starts_with('e')
            && target.len() > 1
            && target[1..].chars().all(|c| c.is_ascii_digit())
        {
            format!("@{target}")
        } else {
            target
        };
        let value = item
            .get("value")
            .or_else(|| item.get("text"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| CliError::new(ErrorKind::Usage, "fill-form field missing value"))?
            .to_string();
        fields.push((target, value));
    }
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.fill_form(&fields, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        let n = d.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("ok fill-form count={n}");
    })
}

fn handle_upload(
    life: &Lifecycle,
    target: &str,
    path: &Path,
    include_snapshot: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target = target.to_string();
    let path = path.to_path_buf();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.upload(&target, &path, include_snapshot).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        println!("ok upload path={p}");
    })
}

fn handle_history(
    life: &Lifecycle,
    direction: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let direction = direction.to_string();
    let direction_label = direction.clone();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match direction.as_str() {
            "back" => session.back().await?,
            "forward" => session.forward().await?,
            other => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown history direction: {other}"),
                ))
            }
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| println!("ok {direction_label}"))
}

fn handle_reload(
    life: &Lifecycle,
    ignore_cache: bool,
    init_script: Option<&str>,
    handle_before_unload: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let init = init_script.map(|s| s.to_string());
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        if let Some(ref js) = init {
            let _ = session.eval(js, None, Some("accept"), None).await;
        }
        if handle_before_unload {
            // Best-effort: auto-accept any beforeunload via dialog handler
            let _ = session
                .eval(
                    "window.addEventListener('beforeunload', e => { e.returnValue=''; });",
                    None,
                    Some("accept"),
                    None,
                )
                .await;
        }
        let v = session.reload(ignore_cache).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| {
        println!("ok reload ignore_cache={ignore_cache}")
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_eval(
    life: &Lifecycle,
    expression: &str,
    args: Option<&str>,
    dialog_action: Option<&str>,
    file_path: Option<&Path>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let expr = expression.to_string();
    let args_owned = args.map(|s| s.to_string());
    let dialog_owned = dialog_action.map(|s| s.to_string());
    let path_owned = file_path.map(|p| p.to_path_buf());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = session
                .eval(
                    &expr,
                    args_owned.as_deref(),
                    dialog_owned.as_deref(),
                    path_owned.as_deref(),
                )
                .await;
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        println!(
            "ok eval={}",
            d.get("result").unwrap_or(&serde_json::Value::Null)
        );
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_grab(
    life: &Lifecycle,
    path: Option<&Path>,
    format: GrabFormat,
    full_page: bool,
    quality: Option<i32>,
    element: Option<&str>,
    artifacts: Option<&Path>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let fmt = match format {
        GrabFormat::Png => "png",
        GrabFormat::Jpeg => "jpeg",
        GrabFormat::Webp => "webp",
    };
    if let Some(a) = artifacts {
        std::fs::create_dir_all(a)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("artifacts-dir mkdir: {e}")))?;
    }
    let path_owned = path.map(|p| p.to_path_buf()).or_else(|| {
        artifacts.map(|a| {
            a.join(format!(
                "grab-{}.{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
                fmt
            ))
        })
    });
    if let Some(ref p) = path_owned {
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("grab path mkdir: {e}")))?;
            }
        }
    }
    let element_owned = element.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = session
                .grab(
                    path_owned.as_deref(),
                    fmt,
                    full_page,
                    quality,
                    element_owned.as_deref(),
                )
                .await;
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        println!("ok grab path={p}");
    })
}

fn handle_run(
    life: &Lifecycle,
    script: &Path,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    flags: run::RunFlags,
) -> Result<(), CliError> {
    let script = script.to_path_buf();
    let data = block_on_browser_timeout(
        run::run_script_with_flags(life, &script, robots, capture, flags),
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        let total = d.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("ok run steps={total}");
    })
}

fn handle_exec(
    life: &Lifecycle,
    args: &[String],
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    flags: run::RunFlags,
) -> Result<(), CliError> {
    if args.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "exec requires a subcommand (e.g. goto)",
            "browser-automation-cli exec goto about:blank",
        ));
    }
    // Single-step path for simple argv forms; multi-step uses run --script.
    match args[0].as_str() {
        "goto" => {
            let url = args.get(1).ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    "exec goto requires a URL",
                    "browser-automation-cli exec goto about:blank",
                )
            })?;
            handle_goto(
                life,
                url,
                robots,
                capture,
                timeout_secs,
                json,
                None,
                false,
                None,
            )
        }
        "wait" | "view" | "press" | "write" | "keys" | "type" | "hover" | "back" | "forward"
        | "reload" | "eval" | "grab" | "page" | "console" | "net" | "dialog" | "emulate"
        | "resize" | "extract" | "text" | "scroll" | "cookie" | "attr" | "assert" | "click-at"
        | "drag" | "fill-form" | "upload" | "devtools3p" | "webmcp" | "heap" | "perf"
        | "lighthouse" | "screencast" | "extension" => {
            let step = run::argv_to_step(args)?;
            let data = block_on_browser_timeout(
                run::run_one_step(life, step, robots, capture, flags),
                timeout_secs,
            )?;
            emit_ok(data, json, |d| println!("ok exec {d}"))
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown exec subcommand: {other}"),
            "Use browser-automation-cli exec <cmd> ... or run --script for multi-step NDJSON",
        )),
    }
}

fn handle_extract(
    life: &Lifecycle,
    target: &str,
    attr: Option<&str>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target = target.to_string();
    let attr = attr.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = session.extract(&target, attr.as_deref()).await;
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| println!("ok extract {d}"))
}

fn handle_attr(
    life: &Lifecycle,
    target: &str,
    name: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    handle_extract(life, target, Some(name), capture, timeout_secs, json)
}

fn handle_assert(
    life: &Lifecycle,
    kind: AssertKind,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match kind {
                AssertKind::Url { value, contains } => session.assert_url(&value, contains).await,
                AssertKind::Text { value, target } => {
                    session.assert_text(&value, target.as_deref()).await
                }
                AssertKind::Console { level, max } => session.assert_console(&level, max).await,
            };
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |_| println!("ok assert"))
}

fn handle_console(
    life: &Lifecycle,
    action: ConsoleAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match action {
                ConsoleAction::List {
                    page_idx,
                    page_size,
                    types,
                    include_preserved,
                    service_worker_id,
                } => session.console_list(
                    page_idx,
                    page_size,
                    types.as_deref(),
                    include_preserved,
                    service_worker_id.as_deref(),
                ),
                ConsoleAction::Get { id } => session.console_get(id),
                ConsoleAction::Clear => session.console_clear(),
                ConsoleAction::Dump { path } => session.console_dump(&path),
            };
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| println!("ok console {d}"))
}

fn handle_net(
    life: &Lifecycle,
    action: NetAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match action {
                NetAction::List {
                    page_idx,
                    page_size,
                    resource_types,
                    include_preserved,
                } => session.net_list(
                    page_idx,
                    page_size,
                    resource_types.as_deref(),
                    include_preserved,
                ),
                NetAction::Get {
                    id,
                    request_path,
                    response_path,
                } => session.net_get(&id, request_path.as_deref(), response_path.as_deref()),
            };
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| println!("ok net {d}"))
}

fn handle_page(
    life: &Lifecycle,
    action: Option<PageAction>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let action = action.unwrap_or(PageAction::Info);
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            PageAction::Info => session.page_info().await?,
            PageAction::List => session.page_list().await?,
            PageAction::New {
                url,
                background,
                isolated_context,
            } => {
                session
                    .page_new(url.as_deref(), background, isolated_context)
                    .await?
            }
            PageAction::Select {
                index,
                page_id,
                bring_to_front,
            } => {
                let idx = index.or(page_id).ok_or_else(|| {
                    CliError::with_suggestion(
                        ErrorKind::Usage,
                        "page select requires INDEX or --page-id",
                        "browser-automation-cli page select 0 --json",
                    )
                })?;
                session.page_select(idx, bring_to_front).await?
            }
            PageAction::Close { index, page_id } => session.page_close(index.or(page_id)).await?,
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        if let (Some(u), Some(t)) = (
            d.get("url").and_then(|v| v.as_str()),
            d.get("title").and_then(|v| v.as_str()),
        ) {
            println!("ok page url={u} title={t}");
        } else {
            println!("ok page {d}");
        }
    })
}

fn handle_text(
    life: &Lifecycle,
    target: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    handle_extract(life, target, None, capture, timeout_secs, json)
}

fn handle_scroll(
    life: &Lifecycle,
    target: Option<&str>,
    delta_x: f64,
    delta_y: f64,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let target_owned = target.map(|s| s.to_string());
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session
            .scroll(target_owned.as_deref(), delta_x, delta_y)
            .await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| println!("ok scroll {d}"))
}

fn handle_cookie(
    life: &Lifecycle,
    action: CookieAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            CookieAction::List { url } => session.cookie_list(url.as_deref()).await?,
            CookieAction::Set { json: body } => session.cookie_set(&body).await?,
            CookieAction::Clear => session.cookie_clear().await?,
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| println!("ok cookie {d}"))
}

fn handle_dialog(
    life: &Lifecycle,
    action: DialogAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let r = match action {
                DialogAction::Accept { text } => session.dialog(true, text.as_deref()).await,
                DialogAction::Dismiss => session.dialog(false, None).await,
            };
            let close = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            close?;
            r
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |_| println!("ok dialog"))
}

fn handle_scrape(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(run_scrape(life, url, robots, capture), timeout_secs)?;
    emit_ok(data, json, |d| {
        let policy = d
            .get("robots_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("honor");
        let u = d.get("source_url").and_then(|v| v.as_str()).unwrap_or(url);
        println!("ok scrape source_url={u} robots_policy={policy}");
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_emulate(
    life: &Lifecycle,
    user_agent: Option<&str>,
    locale: Option<&str>,
    timezone: Option<&str>,
    offline: bool,
    latitude: Option<f64>,
    longitude: Option<f64>,
    media: Option<&str>,
    network_conditions: Option<&str>,
    cpu_throttling_rate: Option<f64>,
    color_scheme: Option<&str>,
    extra_headers: Option<&str>,
    viewport: Option<&str>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let ua = user_agent.map(|s| s.to_string());
    let loc = locale.map(|s| s.to_string());
    let tz = timezone.map(|s| s.to_string());
    let media = media.map(|s| s.to_string());
    let net = network_conditions.map(|s| s.to_string());
    let scheme = color_scheme.map(|s| s.to_string());
    let headers = extra_headers.map(|s| s.to_string());
    let vp = viewport.map(|s| s.to_string());
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session
            .emulate(
                ua.as_deref(),
                loc.as_deref(),
                tz.as_deref(),
                offline,
                latitude,
                longitude,
                media.as_deref(),
                net.as_deref(),
                cpu_throttling_rate,
                scheme.as_deref(),
                headers.as_deref(),
                vp.as_deref(),
            )
            .await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| println!("ok emulate"))
}

#[allow(clippy::too_many_arguments)]
fn handle_resize(
    life: &Lifecycle,
    width: i32,
    height: i32,
    scale: f64,
    mobile: bool,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.resize(width, height, scale, mobile).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| println!("ok resize {width}x{height}"))
}

fn handle_perf(
    life: &Lifecycle,
    action: PerfAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            PerfAction::Start {
                path,
                reload,
                auto_stop,
            } => {
                session
                    .perf_start(path.as_deref(), reload, auto_stop)
                    .await?
            }
            PerfAction::Stop { path } => session.perf_stop(path.as_deref()).await?,
            PerfAction::Insight {
                name,
                insight_set_id,
                insight_name,
            } => {
                let resolved = insight_name.or(name);
                session
                    .perf_insight(resolved.as_deref(), insight_set_id.as_deref())
                    .await?
            }
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| println!("ok perf {d}"))
}

fn handle_lighthouse(
    url: &str,
    out_dir: Option<&Path>,
    device: &str,
    mode: &str,
    lighthouse_path: Option<&Path>,
    json: bool,
) -> Result<(), CliError> {
    let bin = lighthouse_path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "lighthouse".to_string());
    let out = out_dir.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("browser-automation-cli")
            .join("lighthouse")
    });
    std::fs::create_dir_all(&out)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("lighthouse out-dir: {e}")))?;
    let form_factor = if device.eq_ignore_ascii_case("mobile") {
        "mobile"
    } else {
        "desktop"
    };
    let mode_norm = if mode.eq_ignore_ascii_case("snapshot") {
        "snapshot"
    } else {
        "navigation"
    };
    // One-shot CLI always launches its own Chrome via lighthouse CLI for navigation mode.
    let html_path = out.join("report.html");
    let json_path = out.join("report.json");
    let output = std::process::Command::new(&bin)
        .arg(url)
        .arg("--quiet")
        .arg("--output=html")
        .arg("--output=json")
        .arg(format!("--output-path={}", out.join("report").display()))
        .arg(format!("--form-factor={form_factor}"))
        .arg("--chrome-flags=--headless=new")
        .arg("--only-categories=accessibility,seo,best-practices")
        .output()
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Unavailable,
                format!("lighthouse spawn failed: {e}"),
                "Install lighthouse (npm i -g lighthouse) or pass --lighthouse-path; doctor reports binary presence",
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::with_suggestion(
            ErrorKind::Software,
            format!("lighthouse exited non-zero: {stderr}"),
            "Check URL and lighthouse install",
        ));
    }
    // Lighthouse may write report.report.html / report.report.json depending on version.
    let report_html = if html_path.exists() {
        html_path.clone()
    } else if out.join("report.report.html").exists() {
        out.join("report.report.html")
    } else {
        html_path.clone()
    };
    let report_json = if json_path.exists() {
        json_path.clone()
    } else if out.join("report.report.json").exists() {
        out.join("report.report.json")
    } else {
        // Some builds write plain report.json next to html
        out.join("report.json")
    };

    let mut scores = Vec::new();
    let mut passed_audits = 0u64;
    let mut failed_audits = 0u64;
    if report_json.exists() {
        if let Ok(raw) = std::fs::read_to_string(&report_json) {
            if let Ok(lhr) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(cats) = lhr.get("categories").and_then(|c| c.as_object()) {
                    for (id, cat) in cats {
                        scores.push(serde_json::json!({
                            "id": id,
                            "title": cat.get("title").and_then(|t| t.as_str()).unwrap_or(id),
                            "score": cat.get("score"),
                        }));
                    }
                }
                if let Some(audits) = lhr.get("audits").and_then(|a| a.as_object()) {
                    for a in audits.values() {
                        if let Some(sc) = a.get("score").and_then(|s| s.as_f64()) {
                            if sc < 1.0 {
                                failed_audits += 1;
                            } else {
                                passed_audits += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let data = serde_json::json!({
        "lighthouse": true,
        "url": url,
        "device": form_factor,
        "mode": mode_norm,
        "binary": bin,
        "out_dir": out.to_string_lossy(),
        "reports": {
            "html": report_html.to_string_lossy(),
            "json": report_json.to_string_lossy(),
        },
        "scores": scores,
        "passed_audits": passed_audits,
        "failed_audits": failed_audits,
    });
    emit_ok(data, json, |d| {
        println!(
            "ok lighthouse report={}",
            d.pointer("/reports/html")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        );
    })
}

fn handle_screencast(
    life: &Lifecycle,
    action: ScreencastAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = match action {
            ScreencastAction::Start { path } => session.screencast_start(path.as_deref()).await?,
            ScreencastAction::Stop { path } => session.screencast_stop(path.as_deref()).await?,
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| println!("ok screencast {d}"))
}

fn handle_heap(
    life: &Lifecycle,
    action: HeapAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        HeapAction::Take { path } => {
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    let v = session.heap_take(&path).await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| {
                println!(
                    "ok heap take path={}",
                    d.get("path").and_then(|v| v.as_str()).unwrap_or("")
                );
            })
        }
        HeapAction::Close { path } => {
            let data = OneShotSession::heap_close(&path)?;
            emit_ok(data, json, |_| {
                println!("ok heap close path={}", path.display())
            })
        }
        HeapAction::Compare {
            base,
            current,
            class_index,
        } => {
            let mut data = OneShotSession::heap_compare(&base, &current)?;
            if let Some(ci) = class_index {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("class_index".into(), serde_json::json!(ci));
                }
            }
            emit_ok(data, json, |d| println!("ok heap compare {d}"))
        }
        HeapAction::Summary { path } => {
            let data = OneShotSession::heap_file_summary(&path)?;
            emit_ok(data, json, |d| println!("ok heap summary {d}"))
        }
        HeapAction::Details {
            path,
            filter_name,
            page_idx,
            page_size,
        } => {
            validate_heap_filter_name(filter_name.as_deref())?;
            let mut data = OneShotSession::heap_details(&path)?;
            paginate_filter_json(
                &mut data,
                "classes",
                filter_name.as_deref(),
                page_idx,
                page_size,
            );
            emit_ok(data, json, |d| println!("ok heap details {d}"))
        }
        HeapAction::DupStrings {
            path,
            page_idx,
            page_size,
        } => {
            let mut data = OneShotSession::heap_dup_strings(&path)?;
            paginate_filter_json(&mut data, "strings", None, page_idx, page_size);
            emit_ok(data, json, |d| println!("ok heap dup-strings {d}"))
        }
        HeapAction::ClassNodes {
            path,
            id,
            filter_name,
            page_idx,
            page_size,
        } => {
            validate_heap_filter_name(filter_name.as_deref())?;
            let mut data = OneShotSession::heap_class_nodes(&path, id)?;
            paginate_filter_json(
                &mut data,
                "nodes",
                filter_name.as_deref(),
                page_idx,
                page_size,
            );
            emit_ok(data, json, |d| println!("ok heap class-nodes {d}"))
        }
        HeapAction::Dominators { path, node } => {
            let data = OneShotSession::heap_node_op(&path, node, "dominators")?;
            emit_ok(data, json, |d| println!("ok heap dominators {d}"))
        }
        HeapAction::Edges {
            path,
            node,
            page_idx,
            page_size,
        } => {
            let mut data = OneShotSession::heap_node_op(&path, node, "edges")?;
            paginate_filter_json(&mut data, "edges", None, page_idx, page_size);
            emit_ok(data, json, |d| println!("ok heap edges {d}"))
        }
        HeapAction::Retainers {
            path,
            node,
            page_idx,
            page_size,
        } => {
            let mut data = OneShotSession::heap_node_op(&path, node, "retainers")?;
            paginate_filter_json(&mut data, "retainers", None, page_idx, page_size);
            emit_ok(data, json, |d| println!("ok heap retainers {d}"))
        }
        HeapAction::Paths {
            path,
            node,
            max_depth,
            max_nodes,
            max_siblings,
        } => {
            let data = crate::native::heap_snapshot::node_op_with_limits(
                &path,
                node,
                "paths",
                max_depth as usize,
                max_siblings.unwrap_or(32) as usize,
                max_nodes.unwrap_or(200) as usize,
                200,
            )
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Io,
                    e,
                    "Pass a valid .heapsnapshot path and node id",
                )
            })?;
            emit_ok(data, json, |d| println!("ok heap paths {d}"))
        }
        HeapAction::ObjectDetails { path, node } => {
            let data = OneShotSession::heap_object_details(&path, node)?;
            emit_ok(data, json, |d| println!("ok heap object-details {d}"))
        }
    }
}

/// Tool-ref heap filterName enum (closed set).
const HEAP_FILTER_NAME_ENUM: &[&str] = &[
    "objectsRetainedByDetachedDomNodes",
    "objectsRetainedByConsole",
    "objectsRetainedByEventHandlers",
    "objectsRetainedByContexts",
];

fn validate_heap_filter_name(filter_name: Option<&str>) -> Result<(), CliError> {
    let Some(f) = filter_name else {
        return Ok(());
    };
    // Free-text substring filters stay allowed; enum-like names must match tool-ref.
    if f.starts_with("objectsRetained")
        && !HEAP_FILTER_NAME_ENUM
            .iter()
            .any(|e| e.eq_ignore_ascii_case(f))
    {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid heap --filter-name enum: {f}"),
            "Use objectsRetainedByDetachedDomNodes|objectsRetainedByConsole|objectsRetainedByEventHandlers|objectsRetainedByContexts or free-text substring",
        ));
    }
    Ok(())
}

/// Paginate/filter a JSON array field for heap list ops (tool-ref pageIdx/pageSize/filterName).
fn paginate_filter_json(
    data: &mut serde_json::Value,
    array_key: &str,
    filter_name: Option<&str>,
    page_idx: Option<usize>,
    page_size: Option<usize>,
) {
    let key = {
        if data.get(array_key).and_then(|v| v.as_array()).is_some() {
            array_key.to_string()
        } else {
            let mut found = None;
            for alt in ["items", "results", "list"] {
                if data.get(alt).and_then(|v| v.as_array()).is_some() {
                    found = Some(alt.to_string());
                    break;
                }
            }
            match found {
                Some(k) => k,
                None => return,
            }
        }
    };

    let is_enum_filter = filter_name
        .map(|f| {
            HEAP_FILTER_NAME_ENUM
                .iter()
                .any(|e| e.eq_ignore_ascii_case(f))
        })
        .unwrap_or(false);

    if is_enum_filter {
        // Offline heapsnapshot parser does not recompute retainer-kind filters;
        // record the requested enum for agents and keep full list (honest Partial).
        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "filter_name".into(),
                serde_json::json!(filter_name.unwrap()),
            );
            obj.insert(
                "filter_applied".into(),
                serde_json::json!("enum_recorded_offline_not_recomputed"),
            );
        }
    }

    let Some(arr) = data.get_mut(&key).and_then(|v| v.as_array_mut()) else {
        return;
    };
    if let Some(f) = filter_name {
        if !is_enum_filter {
            let f_low = f.to_ascii_lowercase();
            arr.retain(|item| {
                item.get("name")
                    .or_else(|| item.get("class_name"))
                    .or_else(|| item.get("string"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_ascii_lowercase().contains(&f_low))
                    .unwrap_or(true)
            });
        }
    }
    let total = arr.len();
    let page = page_idx.unwrap_or(0);
    let size = page_size.unwrap_or(total.max(1));
    let start = page.saturating_mul(size).min(total);
    let end = (start + size).min(total);
    let page_items: Vec<serde_json::Value> = arr[start..end].to_vec();
    *arr = page_items;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("total".into(), serde_json::json!(total));
        obj.insert("page_idx".into(), serde_json::json!(page));
        obj.insert("page_size".into(), serde_json::json!(size));
    }
}

fn handle_extension(
    life: &Lifecycle,
    action: ExtensionAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        ExtensionAction::List => {
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    let v = session.extension_list().await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| println!("ok extension list {d}"))
        }
        ExtensionAction::Install { path } => {
            let path_s = path.display().to_string();
            let data = block_on_browser_timeout(
                async move {
                    let mut session =
                        OneShotSession::launch_with_extensions(capture, vec![path_s.clone()])
                            .await?;
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = true;
                        ledger.chrome_pid = session.chrome_pid();
                    }
                    // Service workers may take a moment to register after --load-extension.
                    let mut listed = session.extension_list().await?;
                    for _ in 0..20 {
                        let count = listed.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                        if count > 0 {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        listed = session.extension_list().await?;
                    }
                    let close = session.shutdown().await;
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = false;
                        ledger.chrome_pid = None;
                    }
                    close?;
                    Ok(serde_json::json!({
                        "installed_path": path_s,
                        "load_extension": true,
                        "targets": listed,
                        "note": "one-shot: Chrome launched with --load-extension for this process only",
                    }))
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| println!("ok extension install {d}"))
        }
        ExtensionAction::Reload { id, path } => {
            let id = id.clone();
            let path_s = path.as_ref().map(|p| p.display().to_string());
            let data = block_on_browser_timeout(
                async move {
                    let mut session = if let Some(p) = path_s {
                        OneShotSession::launch_with_extensions(capture, vec![p]).await?
                    } else {
                        OneShotSession::launch_headless_with_capture(capture).await?
                    };
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = true;
                        ledger.chrome_pid = session.chrome_pid();
                    }
                    let v = session.extension_reload(&id).await;
                    let close = session.shutdown().await;
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = false;
                        ledger.chrome_pid = None;
                    }
                    close?;
                    v
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| println!("ok extension reload {d}"))
        }
        ExtensionAction::Trigger { id, path } => {
            let id = id.clone();
            let path_s = path.as_ref().map(|p| p.display().to_string());
            let data = block_on_browser_timeout(
                async move {
                    let mut session = if let Some(p) = path_s {
                        OneShotSession::launch_with_extensions(capture, vec![p]).await?
                    } else {
                        OneShotSession::launch_headless_with_capture(capture).await?
                    };
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = true;
                        ledger.chrome_pid = session.chrome_pid();
                    }
                    let v = session.extension_trigger(&id).await;
                    let close = session.shutdown().await;
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = false;
                        ledger.chrome_pid = None;
                    }
                    close?;
                    v
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| println!("ok extension trigger {d}"))
        }
        ExtensionAction::Uninstall { id } => {
            // One-shot has no persistent Chrome profile: uninstall means "not loaded next process".
            let data = serde_json::json!({
                "uninstalled": id,
                "persistent": false,
                "note": "one-shot CLI does not keep extensions across processes; omit path on next install",
            });
            emit_ok(data, json, |_| println!("ok extension uninstall id={id}"))
        }
    }
}

fn handle_devtools3p(
    life: &Lifecycle,
    action: Devtools3pAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        Devtools3pAction::List { url } => {
            let url = url.unwrap_or_else(|| "about:blank".into());
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.devtools3p_list().await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| println!("ok devtools3p list {d}"))
        }
        Devtools3pAction::Exec { name, params, url } => {
            let url = url.unwrap_or_else(|| "about:blank".into());
            let params = params.clone();
            let name = name.clone();
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.devtools3p_exec(&name, params.as_deref()).await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| println!("ok devtools3p exec {d}"))
        }
    }
}

fn handle_webmcp(
    life: &Lifecycle,
    action: WebmcpAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        WebmcpAction::List { url } => {
            let url = url.unwrap_or_else(|| "about:blank".into());
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.webmcp_list().await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| println!("ok webmcp list {d}"))
        }
        WebmcpAction::Exec { name, input, url } => {
            let url = url.unwrap_or_else(|| "about:blank".into());
            let name = name.clone();
            let input = input.clone();
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    if url != "about:blank" {
                        let _ = session
                            .goto(&url, crate::robots::RobotsPolicy::Ignore)
                            .await?;
                    }
                    let v = session.webmcp_exec(&name, input.as_deref()).await?;
                    Ok((session, v))
                })?;
            emit_ok(data, json, |d| println!("ok webmcp exec {d}"))
        }
    }
}

fn handle_completions(shell: CompletionShell) -> Result<(), CliError> {
    use clap::CommandFactory;
    use clap_complete::{generate, shells};
    use std::io::Write;

    let mut cmd = crate::cli::Cli::command();
    let bin = "browser-automation-cli";
    let mut out = std::io::stdout();
    match shell {
        CompletionShell::Bash => generate(shells::Bash, &mut cmd, bin, &mut out),
        CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, bin, &mut out),
        CompletionShell::Fish => generate(shells::Fish, &mut cmd, bin, &mut out),
        CompletionShell::Elvish => generate(shells::Elvish, &mut cmd, bin, &mut out),
        CompletionShell::Powershell => generate(shells::PowerShell, &mut cmd, bin, &mut out),
    }
    let _ = out.flush();
    Ok(())
}
