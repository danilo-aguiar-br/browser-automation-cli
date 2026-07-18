//! PRD command dispatch (paths leves + Layer A/B browser one-shot).
#![allow(missing_docs)]

mod meta;
mod run;

use std::path::Path;

use crate::browser::{
    block_on_browser_timeout, run_keys, run_press, run_scrape, run_type, run_view, run_write,
    CaptureOpts, OneShotSession,
};
use crate::cli::{
    AssertKind, Cli, Commands, CompletionShell, ConfigAction, ConsoleAction, CookieAction,
    Devtools3pAction, DialogAction, ExtensionAction, GrabFormat, HeapAction, MitmAction, NetAction,
    PageAction, PerfAction, ScreencastAction, WebmcpAction, WorkflowAction,
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
                    crate::i18n::suggestion_key("vision_required", None),
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
            service_worker_id,
        } => {
            match handle_eval(
                life,
                &expression,
                args.as_deref(),
                dialog_action.as_deref(),
                file_path.as_deref(),
                service_worker_id.as_deref(),
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
        Commands::PrintPdf { path, url } => {
            match handle_print_pdf(
                life,
                path.as_deref(),
                url.as_deref(),
                robots,
                capture,
                timeout_secs,
                json,
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Monitor { action } => match handle_monitor(action, robots, timeout_secs, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Run { script } => {
            let flags = run::RunFlags::from_globals(
                experimental_vision,
                experimental_screencast,
                category_memory,
                category_extensions,
                category_third_party,
                category_webmcp,
            );
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
            let flags = run::RunFlags::from_globals(
                experimental_vision,
                experimental_screencast,
                category_memory,
                category_extensions,
                category_third_party,
                category_webmcp,
            );
            match handle_exec(life, &args, robots, capture, timeout_secs, json, flags) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Extract {
            target,
            attr,
            llm,
            question,
            schema_json,
        } => {
            match handle_extract(
                life,
                &target,
                attr.as_deref(),
                llm,
                question.as_deref(),
                schema_json.as_deref(),
                capture,
                timeout_secs,
                json,
            ) {
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
        Commands::Scrape {
            url,
            format,
            engine,
            only_main_content,
            webhook_url,
        } => {
            match handle_scrape(
                life,
                &url,
                robots,
                capture,
                timeout_secs,
                json,
                &format,
                &engine,
                only_main_content,
                webhook_url.as_deref(),
            ) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::BatchScrape {
            urls_file,
            format,
            concurrency,
        } => match handle_batch_scrape(&urls_file, robots, &format, concurrency, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Crawl {
            url,
            limit,
            max_depth,
            format,
            same_host,
        } => match handle_crawl(&url, robots, limit, max_depth, &format, same_host, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Map {
            url,
            limit,
            max_depth,
        } => match handle_map(&url, robots, limit, max_depth, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Search { query, limit } => match handle_search(&query, robots, limit, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Parse { path, redact_pii } => match handle_parse(&path, redact_pii, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Qr { action } => match handle_qr(action, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::FindPaths {
            pattern,
            paths,
            extension,
            hidden,
            no_ignore,
            max_depth,
            entry_type,
            limit,
            glob,
        } => match handle_find_paths(
            pattern.as_deref(),
            &paths,
            extension.as_deref(),
            hidden,
            no_ignore,
            max_depth,
            entry_type.as_deref(),
            limit,
            glob.as_deref(),
            json,
        ) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::SgScan { paths, limit } => match handle_sg_scan(&paths, limit, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::SgRewrite { paths, apply } => match handle_sg_rewrite(&paths, apply, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::SheetWrite { input, out, sheet } => {
            match handle_sheet_write(&input, &out, &sheet, json) {
                Ok(()) => 0,
                Err(e) => emit_err(&e, json),
            }
        }
        Commands::Mitm { action } => match handle_mitm(action, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Workflow { action } => match handle_workflow(action, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
        Commands::Config { action } => match handle_config(action, json) {
            Ok(()) => 0,
            Err(e) => emit_err(&e, json),
        },
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
    let localized = crate::i18n::localize_error_suggestion(err);
    if json {
        let _ = print_error_json(&localized);
    } else {
        eprintln!("error: {localized}");
        if let Some(s) = localized.suggestion() {
            eprintln!("suggestion: {s}");
        }
    }
    localized.exit_code() as i32
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
    let init = init_script.map(|s| s.to_string());
    let url_owned = url.to_string();
    let data = block_on_browser_timeout(
        crate::browser::run_goto_with_options(
            life,
            &url_owned,
            capture,
            robots,
            init.as_deref(),
            handle_before_unload,
            navigation_timeout_ms,
        ),
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
    // Single-shot reload without a prior URL cannot apply init_script meaningfully.
    // Require multi-step `run` (session already on a document) OR reject blank-only.
    if init_script.is_some() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "reload --init-script requires multi-step `run` with a prior goto in the same process",
            "Use: browser-automation-cli run --script steps.jsonl  (goto then reload --init-script …)",
        ));
    }
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        // GAP-A009/A005/A006: CDP Page.reload + dialog pump; no preventDefault inject.
        let v = session
            .reload_with_options(ignore_cache, None, handle_before_unload)
            .await?;
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
    service_worker_id: Option<&str>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let expr = expression.to_string();
    let args_owned = args.map(|s| s.to_string());
    let dialog_owned = dialog_action.map(|s| s.to_string());
    let path_owned = file_path.map(|p| p.to_path_buf());
    let sw_owned = service_worker_id.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
                if let Some(dir) = session.temp_user_data_dir() {
                    ledger.profile_dir = Some(dir);
                }
            }
            let r = if let Some(ref sw) = sw_owned {
                session.eval_service_worker(sw, &expr).await
            } else {
                let _ = session
                    .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                    .await?;
                session
                    .eval(
                        &expr,
                        args_owned.as_deref(),
                        dialog_owned.as_deref(),
                        path_owned.as_deref(),
                    )
                    .await
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

fn handle_print_pdf(
    life: &Lifecycle,
    path: Option<&Path>,
    url: Option<&str>,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let path = path.map(|p| p.to_path_buf());
    let url = url.map(|s| s.to_string());
    let data = block_on_browser_timeout(
        async {
            let mut session =
                crate::browser::OneShotSession::launch_headless_with_capture(capture).await?;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = true;
                ledger.chrome_pid = session.chrome_pid();
            }
            if let Some(u) = url.as_deref() {
                let _ = session.goto(u, robots).await?;
            }
            let out = session.print_pdf(path.as_deref()).await;
            let _ = session.shutdown().await;
            if let Ok(mut ledger) = life.ledger.lock() {
                ledger.chrome_launched = false;
                ledger.chrome_pid = None;
            }
            out
        },
        timeout_secs,
    )?;
    emit_ok(data, json, |d| {
        let p = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        println!("ok print-pdf path={p}");
    })
}

fn handle_monitor(
    action: crate::cli::MonitorAction,
    robots: RobotsPolicy,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    use sha2::{Digest, Sha256};
    match action {
        crate::cli::MonitorAction::Check {
            url,
            baseline,
            write_baseline,
            engine,
        } => {
            let engine_l = engine.to_ascii_lowercase();
            let text = if engine_l == "browser" {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "monitor check --engine browser is reserved; use http for baseline hash",
                    "Pass --engine http (default) for one-shot baseline compare",
                ));
            } else {
                let opts = crate::scrape_local::ScrapeOpts {
                    format: crate::scrape_local::ScrapeFormat::Text,
                    engine: "http".into(),
                    ..Default::default()
                };
                let page = block_on_browser_timeout(
                    crate::scrape_local::scrape_http(&url, robots, &opts),
                    timeout_secs,
                )?;
                page.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            let mut hasher = Sha256::new();
            hasher.update(text.as_bytes());
            let hash = hex::encode(hasher.finalize());
            let baseline_exists = baseline.exists();
            let (changed, previous_hash) = if baseline_exists {
                let prev = std::fs::read_to_string(&baseline).map_err(|e| {
                    CliError::new(
                        ErrorKind::Io,
                        format!("read baseline {}: {e}", baseline.display()),
                    )
                })?;
                let prev = prev.trim().to_string();
                (prev != hash, Some(prev))
            } else {
                (true, None)
            };
            if write_baseline || !baseline_exists {
                if let Some(parent) = baseline.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                std::fs::write(&baseline, format!("{hash}\n")).map_err(|e| {
                    CliError::new(
                        ErrorKind::Io,
                        format!("write baseline {}: {e}", baseline.display()),
                    )
                })?;
            }
            let data = serde_json::json!({
                "url": url,
                "baseline": baseline.display().to_string(),
                "hash": hash,
                "previous_hash": previous_hash,
                "changed": changed,
                "baseline_written": write_baseline || !baseline_exists,
                "engine": "http",
            });
            emit_ok(data, json, |d| {
                let ch = d.get("changed").and_then(|v| v.as_bool()).unwrap_or(false);
                println!("ok monitor check changed={ch}");
            })
        }
    }
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
    // Fail-fast payload: ok:false with partial steps (still non-zero exit).
    if data.get("ok") == Some(&serde_json::json!(false)) {
        let kind = data
            .pointer("/error/kind")
            .and_then(|v| v.as_str())
            .unwrap_or("data");
        let message = data
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("run fail-fast")
            .to_string();
        let suggestion = data
            .pointer("/error/suggestion")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| crate::i18n::suggestion_key("run_fail_fast", None))
            .to_string();
        let err_kind = match kind {
            "usage" => ErrorKind::Usage,
            "unavailable" => ErrorKind::Unavailable,
            "browser" => ErrorKind::Browser,
            "timeout" => ErrorKind::Timeout,
            "data" => ErrorKind::Data,
            _ => ErrorKind::Software,
        };
        let partial = serde_json::json!({
            "total": data.get("total"),
            "failed_index": data.get("failed_index"),
            "failed_cmd": data.get("failed_cmd"),
            "steps": data.get("steps"),
        });
        return Err(CliError::with_suggestion(err_kind, message, suggestion).with_data(partial));
    }
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

#[allow(clippy::too_many_arguments)]
fn handle_extract(
    life: &Lifecycle,
    target: &str,
    attr: Option<&str>,
    llm: bool,
    question: Option<&str>,
    schema_json: Option<&std::path::Path>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    if llm {
        return handle_extract_llm(target, question, schema_json, json);
    }
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

fn handle_extract_llm(
    target: &str,
    question: Option<&str>,
    schema_json: Option<&std::path::Path>,
    json: bool,
) -> Result<(), CliError> {
    let schema_body = match schema_json {
        Some(p) => Some(std::fs::read_to_string(p).map_err(|e| {
            CliError::new(
                ErrorKind::Io,
                format!("read schema-json {}: {e}", p.display()),
            )
        })?),
        None => None,
    };
    let source_text = if target.starts_with("http://") || target.starts_with("https://") {
        let opts = crate::scrape_local::ScrapeOpts {
            format: crate::scrape_local::ScrapeFormat::Text,
            only_main_content: true,
            engine: "http".into(),
            max_body_bytes: 2_000_000,
        };
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| CliError::new(ErrorKind::Software, format!("runtime: {e}")))?;
        let data = rt.block_on(crate::scrape_local::scrape_http(
            target,
            crate::robots::RobotsPolicy::Honor,
            &opts,
        ))?;
        data.get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else if Path::new(target).is_file() {
        let parsed = crate::scrape_local::parse_file(Path::new(target))?;
        parsed
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "extract --llm target must be http(s) URL or local file path",
            "Example: browser-automation-cli --json extract --llm --question 'sum' https://example.com",
        ));
    };
    if source_text.trim().is_empty() {
        return Err(CliError::new(
            ErrorKind::Data,
            "extract --llm: empty source text",
        ));
    }
    let data = crate::llm_local::extract_with_llm(&source_text, question, schema_body.as_deref())?;
    emit_ok(data, json, |d| println!("ok extract-llm {d}"))
}

fn handle_attr(
    life: &Lifecycle,
    target: &str,
    name: &str,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    handle_extract(
        life,
        target,
        Some(name),
        false,
        None,
        None,
        capture,
        timeout_secs,
        json,
    )
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
            PageAction::TabId => {
                let tab = session.active_tab_id_string().ok_or_else(|| {
                    CliError::with_suggestion(
                        ErrorKind::Browser,
                        "no active tab id",
                        "Open a page first (goto / page new)",
                    )
                })?;
                serde_json::json!({
                    "tab_id": tab,
                    "tool": "get_tab_id",
                })
            }
        };
        Ok((session, v))
    })?;
    emit_ok(data, json, |d| {
        if let Some(tab) = d.get("tab_id").and_then(|v| v.as_str()) {
            println!("ok page tab-id={tab}");
        } else if let (Some(u), Some(t)) = (
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
    handle_extract(
        life,
        target,
        None,
        false,
        None,
        None,
        capture,
        timeout_secs,
        json,
    )
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

#[allow(clippy::too_many_arguments)]
fn post_webhook(webhook_url: &str, data: &serde_json::Value) -> Result<(), CliError> {
    // One-shot operator webhook; no product telemetry.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("webhook client: {e}")))?;
    let mut last_err = String::new();
    for attempt in 0..3u32 {
        match client.post(webhook_url).json(data).send() {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            Ok(resp) => {
                last_err = format!("webhook HTTP {}", resp.status());
            }
            Err(e) => last_err = format!("webhook: {e}"),
        }
        if attempt < 2 {
            std::thread::sleep(std::time::Duration::from_millis(50 * (1 << attempt)));
        }
    }
    Err(CliError::with_suggestion(
        ErrorKind::Unavailable,
        last_err,
        "Check --webhook-url reachability; operator destination only",
    ))
}

#[allow(clippy::too_many_arguments)]
fn handle_scrape(
    life: &Lifecycle,
    url: &str,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    format: &str,
    engine: &str,
    only_main_content: bool,
    webhook_url: Option<&str>,
) -> Result<(), CliError> {
    let fmt = crate::scrape_local::ScrapeFormat::parse(format)?;
    let engine_l = engine.to_ascii_lowercase();
    if engine_l == "http" {
        let opts = crate::scrape_local::ScrapeOpts {
            format: fmt,
            only_main_content,
            engine: "http".into(),
            ..Default::default()
        };
        let data = block_on_browser_timeout(
            crate::scrape_local::scrape_http(url, robots, &opts),
            timeout_secs,
        )?;
        if let Some(wh) = webhook_url {
            post_webhook(wh, &data)?;
        }
        return emit_ok(data, json, |d| {
            let u = d.get("source_url").and_then(|v| v.as_str()).unwrap_or(url);
            println!("ok scrape engine=http source_url={u}");
        });
    }
    // browser engine: CDP scrape always includes outerHTML for multi-format payload.
    let data = block_on_browser_timeout(run_scrape(life, url, robots, capture), timeout_secs)?;
    let html = data
        .get("html")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source = data
        .get("source_url")
        .and_then(|v| v.as_str())
        .unwrap_or(url)
        .to_string();
    let data = if html.is_empty() {
        // Fallback: still surface text-only with format marker so agents see format intent.
        let mut d = data;
        if let Some(obj) = d.as_object_mut() {
            obj.insert(
                "format".into(),
                serde_json::json!(format!("{:?}", fmt).to_ascii_lowercase()),
            );
            obj.insert("engine".into(), serde_json::json!("browser"));
        }
        d
    } else {
        let opts = crate::scrape_local::ScrapeOpts {
            format: fmt,
            only_main_content,
            engine: "browser".into(),
            ..Default::default()
        };
        crate::scrape_local::build_scrape_payload(&source, 200, &html, &opts, robots)
    };
    if let Some(wh) = webhook_url {
        post_webhook(wh, &data)?;
    }
    emit_ok(data, json, |d| {
        let policy = d
            .get("robots_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("honor");
        let u = d.get("source_url").and_then(|v| v.as_str()).unwrap_or(url);
        println!("ok scrape source_url={u} robots_policy={policy}");
    })
}

fn handle_batch_scrape(
    urls_file: &Path,
    robots: RobotsPolicy,
    format: &str,
    concurrency: usize,
    json: bool,
) -> Result<(), CliError> {
    let urls = crate::scrape_local::read_urls_file(urls_file)?;
    let opts = crate::scrape_local::ScrapeOpts {
        format: crate::scrape_local::ScrapeFormat::parse(format)?,
        engine: "http".into(),
        ..Default::default()
    };
    let data = block_on_browser_timeout(
        crate::scrape_local::batch_scrape_http(&urls, robots, &opts, concurrency),
        0,
    )?;
    emit_ok(data, json, |d| {
        println!(
            "ok batch-scrape count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_crawl(
    url: &str,
    robots: RobotsPolicy,
    limit: usize,
    max_depth: usize,
    format: &str,
    same_host: bool,
    json: bool,
) -> Result<(), CliError> {
    let opts = crate::scrape_local::ScrapeOpts {
        format: crate::scrape_local::ScrapeFormat::parse(format)?,
        engine: "http".into(),
        ..Default::default()
    };
    let data = block_on_browser_timeout(
        crate::scrape_local::crawl_http(url, robots, &opts, limit, max_depth, same_host),
        0,
    )?;
    emit_ok(data, json, |d| {
        println!(
            "ok crawl count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_map(
    url: &str,
    robots: RobotsPolicy,
    limit: usize,
    max_depth: usize,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(
        crate::scrape_local::map_http(url, robots, limit, max_depth),
        0,
    )?;
    emit_ok(data, json, |d| {
        println!(
            "ok map count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_search(
    query: &str,
    robots: RobotsPolicy,
    limit: usize,
    json: bool,
) -> Result<(), CliError> {
    let data = block_on_browser_timeout(crate::scrape_local::search_http(query, robots, limit), 0)?;
    emit_ok(data, json, |d| {
        println!(
            "ok search count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_parse(path: &Path, redact_pii: bool, json: bool) -> Result<(), CliError> {
    let data = crate::scrape_local::parse_file_opts(path, redact_pii)?;
    emit_ok(data, json, |d| {
        println!(
            "ok parse path={}",
            d.get("path").and_then(|v| v.as_str()).unwrap_or("")
        );
    })
}

fn handle_qr(action: crate::cli::QrAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        crate::cli::QrAction::Encode { text, format, path } => {
            crate::qr_local::encode(&text, &format, path.as_deref())?
        }
        crate::cli::QrAction::Decode { path } => crate::qr_local::decode(&path)?,
    };
    emit_ok(data, json, |d| println!("ok qr {d}"))
}

#[allow(clippy::too_many_arguments)]
fn handle_find_paths(
    pattern: Option<&str>,
    paths: &[String],
    extension: Option<&str>,
    hidden: bool,
    no_ignore: bool,
    max_depth: Option<usize>,
    entry_type: Option<&str>,
    limit: usize,
    glob: Option<&str>,
    json: bool,
) -> Result<(), CliError> {
    let opts = crate::find_paths::FindPathsOpts {
        pattern: pattern.unwrap_or("").to_string(),
        roots: crate::find_paths::roots_from(paths),
        extension: extension.map(|s| s.to_string()),
        hidden,
        no_ignore,
        max_depth,
        entry_type: entry_type.map(|s| s.to_string()),
        limit,
        glob: glob.map(|s| s.to_string()),
    };
    let data = crate::find_paths::find_paths(&opts)?;
    emit_ok(data, json, |d| {
        println!(
            "ok find-paths count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_sg_scan(paths: &[String], limit: usize, json: bool) -> Result<(), CliError> {
    let roots: Vec<std::path::PathBuf> = if paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        paths.iter().map(std::path::PathBuf::from).collect()
    };
    let data = crate::sg_local::sg_scan(&roots, limit)?;
    emit_ok(data, json, |d| {
        println!(
            "ok sg-scan count={}",
            d.get("count").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_sg_rewrite(paths: &[String], apply: bool, json: bool) -> Result<(), CliError> {
    let roots: Vec<std::path::PathBuf> = if paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        paths.iter().map(std::path::PathBuf::from).collect()
    };
    let data = crate::sg_local::sg_rewrite(&roots, apply)?;
    emit_ok(data, json, |d| {
        println!(
            "ok sg-rewrite apply={} planned={}",
            d.get("apply").and_then(|v| v.as_bool()).unwrap_or(false),
            d.get("planned").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_sheet_write(
    input: &std::path::Path,
    out: &std::path::Path,
    sheet: &str,
    json: bool,
) -> Result<(), CliError> {
    let data = crate::sheet_local::sheet_write(input, out, sheet)?;
    emit_ok(data, json, |d| {
        println!(
            "ok sheet-write path={} rows={}",
            d.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            d.get("rows").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    })
}

fn handle_mitm(action: MitmAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        MitmAction::Status => crate::mitm_local::status()?,
        MitmAction::List { host, limit } => crate::mitm_local::list(host.as_deref(), limit)?,
        MitmAction::Get { id } => crate::mitm_local::get(id)?,
        MitmAction::Har { out } => crate::mitm_local::export_har(&out)?,
        MitmAction::Export { format, out } => {
            if format.eq_ignore_ascii_case("har") {
                crate::mitm_local::export_har(&out)?
            } else {
                let path = crate::mitm_local::default_capture_path()?;
                let cap = crate::mitm_local::MitmCapture::load(&path, true)?;
                let body = serde_json::to_vec_pretty(&serde_json::json!({
                    "count": cap.items.len(),
                    "items": cap.items,
                }))
                .map_err(|e| CliError::new(ErrorKind::Data, format!("export: {e}")))?;
                std::fs::write(&out, body)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("export write: {e}")))?;
                serde_json::json!({
                    "path": out.display().to_string(),
                    "format": format,
                    "count": cap.items.len(),
                })
            }
        }
        MitmAction::Domains => crate::mitm_local::domains()?,
        MitmAction::Apis { kind } => crate::mitm_local::apis(kind.as_deref())?,
        MitmAction::InitCa => crate::mitm_local::ensure_ca()?,
        MitmAction::Start { seconds } => {
            crate::browser::block_on_browser(crate::mitm_local::start_proxy_oneshot(seconds))?
        }
    };
    emit_ok(data, json, |d| println!("ok mitm {d}"))
}

fn handle_workflow(action: WorkflowAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        WorkflowAction::Run { manifest, journal } => {
            crate::workflow_local::workflow_run(&manifest, journal.as_deref())?
        }
        WorkflowAction::Resume { manifest, journal } => {
            crate::workflow_local::workflow_resume(&manifest, journal.as_deref())?
        }
        WorkflowAction::Status { journal, name } => {
            crate::workflow_local::workflow_status(journal.as_deref(), name.as_deref())?
        }
    };
    emit_ok(data, json, |d| println!("ok workflow {d}"))
}

fn handle_config(action: ConfigAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        ConfigAction::Path => crate::xdg::paths_snapshot()?,
        ConfigAction::Init => crate::xdg::init_layout()?,
        ConfigAction::Show => crate::xdg::config_get(None)?,
        ConfigAction::Set { key, value } => crate::xdg::config_set(&key, &value)?,
        ConfigAction::Get { key } => crate::xdg::config_get(key.as_deref())?,
        ConfigAction::ListKeys => crate::xdg::config_list_keys()?,
    };
    emit_ok(data, json, |d| println!("ok config {d}"))
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

/// Where the lighthouse binary was resolved from (agent-honest; GAP-A010 / LH-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LighthouseSource {
    /// Explicit `--lighthouse-path` flag.
    Flag,
    /// XDG `config set lighthouse_path`.
    Xdg,
    /// Found on PATH via which-equivalent.
    Path,
    /// Local e2e mock script (`mock-lighthouse`).
    Mock,
}

impl LighthouseSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Xdg => "xdg",
            Self::Path => "path",
            Self::Mock => "mock",
        }
    }
}

/// Resolve lighthouse binary: flag → XDG → PATH (rules processos_externos).
pub(crate) fn resolve_lighthouse_binary(
    cli_path: Option<&Path>,
) -> Result<(std::path::PathBuf, LighthouseSource), CliError> {
    if let Some(p) = cli_path {
        if p.is_file() {
            let source = if p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("mock-lighthouse"))
            {
                LighthouseSource::Mock
            } else {
                LighthouseSource::Flag
            };
            return Ok((p.to_path_buf(), source));
        }
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("lighthouse path not found: {}", p.display()),
            "Pass an absolute executable path to --lighthouse-path",
        ));
    }
    if let Some(xdg) = crate::xdg::lighthouse_path_from_config().filter(|s| !s.is_empty()) {
        let p = Path::new(&xdg);
        if p.is_file() {
            let source = if xdg.contains("mock-lighthouse") {
                LighthouseSource::Mock
            } else {
                LighthouseSource::Xdg
            };
            return Ok((p.to_path_buf(), source));
        }
    }
    if let Some(p) = which_lighthouse() {
        return Ok((Path::new(&p).to_path_buf(), LighthouseSource::Path));
    }
    Err(CliError::with_suggestion(
        ErrorKind::Unavailable,
        "lighthouse binary not found on PATH or XDG lighthouse_path",
        crate::i18n::suggestion_key("lighthouse_missing", None),
    ))
}

fn which_lighthouse() -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("lighthouse");
            if candidate.is_file() {
                return Some(candidate.display().to_string());
            }
            #[cfg(windows)]
            {
                let candidate = dir.join("lighthouse.cmd");
                if candidate.is_file() {
                    return Some(candidate.display().to_string());
                }
            }
        }
        None
    })
}

/// Run lighthouse binary and return envelope data (shared by CLI and `run` scripts).
pub(crate) fn lighthouse_to_value(
    url: &str,
    out_dir: Option<&Path>,
    device: &str,
    mode: &str,
    lighthouse_path: Option<&Path>,
) -> Result<serde_json::Value, CliError> {
    let (bin_path, binary_source) = resolve_lighthouse_binary(lighthouse_path)?;
    let bin = bin_path.display().to_string();
    let out = out_dir.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        crate::xdg::cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("browser-automation-cli"))
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
    } else if mode.eq_ignore_ascii_case("navigation") || mode.is_empty() {
        "navigation"
    } else {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unsupported lighthouse mode: {mode}"),
            "Use --mode navigation or --mode snapshot",
        ));
    };
    // Map mode to real Lighthouse CLI args (GAP-006). Snapshot uses gather-mode.
    let html_path = out.join("report.html");
    let json_path = out.join("report.json");
    let mut cmd = std::process::Command::new(&bin);
    cmd.arg(url)
        .arg("--quiet")
        .arg("--output=html")
        .arg("--output=json")
        .arg(format!("--output-path={}", out.join("report").display()))
        .arg(format!("--form-factor={form_factor}"))
        .arg("--chrome-flags=--headless=new")
        .arg("--only-categories=accessibility,seo,best-practices");
    if mode_norm == "snapshot" {
        // Lighthouse user-flows / gather-mode snapshot (when supported by binary).
        cmd.arg("--gather-mode=snapshot");
    }
    let output = cmd.output().map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Unavailable,
            format!("lighthouse spawn failed: {e}"),
            crate::i18n::suggestion_key("lighthouse_missing", None),
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

    Ok(serde_json::json!({
        "lighthouse": true,
        "url": url,
        "device": form_factor,
        "mode": mode_norm,
        "binary": bin,
        "binary_source": binary_source.as_str(),
        "binary_present": true,
        "out_dir": out.to_string_lossy(),
        "reports": {
            "html": report_html.to_string_lossy(),
            "json": report_json.to_string_lossy(),
        },
        "scores": scores,
        "passed_audits": passed_audits,
        "failed_audits": failed_audits,
    }))
}

fn handle_lighthouse(
    url: &str,
    out_dir: Option<&Path>,
    device: &str,
    mode: &str,
    lighthouse_path: Option<&Path>,
    json: bool,
) -> Result<(), CliError> {
    let data = lighthouse_to_value(url, out_dir, device, mode, lighthouse_path)?;
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
            let id = id.clone();
            let id_print = id.clone();
            // Prefer in-process unload when a session can be opened; otherwise honest metadata.
            let data = block_on_browser_timeout(
                async move {
                    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = true;
                        ledger.chrome_pid = session.chrome_pid();
                        if let Some(dir) = session.temp_user_data_dir() {
                            ledger.profile_dir = Some(dir);
                        }
                    }
                    let v = session.extension_uninstall(&id).await;
                    let close = session.shutdown().await;
                    if let Ok(mut ledger) = life.ledger.lock() {
                        ledger.chrome_launched = false;
                        ledger.chrome_pid = None;
                        ledger.profile_dir = None;
                    }
                    close?;
                    v
                },
                timeout_secs,
            )?;
            emit_ok(data, json, |d| {
                let effect = d.get("effect").and_then(|v| v.as_str()).unwrap_or("?");
                println!("ok extension uninstall id={id_print} effect={effect}")
            })
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

#[cfg(test)]
mod lighthouse_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mock_lighthouse_parses_scores() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mock = root.join("scripts/mock-lighthouse.sh");
        if !mock.is_file() {
            eprintln!("skip: mock-lighthouse.sh missing");
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&mock, std::fs::Permissions::from_mode(0o755));
        }
        let out = tempfile::tempdir().expect("tmp");
        let v = lighthouse_to_value(
            "https://example.com",
            Some(out.path()),
            "desktop",
            "navigation",
            Some(&mock),
        )
        .expect("mock lighthouse");
        assert_eq!(
            v.get("binary_source").and_then(|s| s.as_str()),
            Some("mock")
        );
        assert_eq!(
            v.get("binary_present").and_then(|b| b.as_bool()),
            Some(true)
        );
        let scores = v
            .get("scores")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(!scores.is_empty(), "expected scores from mock LHR, got {v}");
    }

    #[test]
    fn resolve_missing_is_unavailable() {
        let err =
            resolve_lighthouse_binary(Some(Path::new("/no/such/lighthouse-bin-xyz"))).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Usage);
    }
}
