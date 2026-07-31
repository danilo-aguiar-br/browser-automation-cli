// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exhaustive CLI command routing (Tier-3 SRP family split).
//!
//! # Module map
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `mod` | `DispatchCtx`, `result_code`, thin exhaustive `route` |
//! | `gates` | experimental / category feature gates |
//! | `meta` | Doctor, Commands, Schema, Version, Locale |
//! | `browser` | nav/interact/page/emulate arms |
//! | `scrape` | scrape/crawl/search/sg/sheet |
//! | `run_exec` | Run, Exec, Monitor |
//! | `ops` | mitm/workflow/config/perf/gated tools |

mod browser;
mod ctx;
mod gates;
mod meta;
mod ops;
mod run_exec;
mod scrape;

use crate::cli::Commands;

pub(crate) use ctx::{result_code, DispatchCtx};

/// Route one parsed command. Caller must `life.finalize()` after return.
///
/// Owned clap fields are borrowed into leaf handlers (`&str` / `&Path` / `&[T]`)
/// so dispatch never needlessly takes ownership of values it only reads.
pub(crate) fn route(cmd: Commands, ctx: &DispatchCtx<'_>) -> i32 {
    match cmd {
        Commands::Doctor {
            offline,
            quick,
            fix,
        } => meta::doctor(ctx, offline, quick, fix),
        Commands::Commands { detail } => meta::commands(ctx, detail),
        Commands::Schema {
            cmd,
            cmd_positional,
        } => meta::schema(ctx, cmd.as_deref(), cmd_positional.as_deref()),
        Commands::Version => meta::version(ctx),
        Commands::Locale => meta::locale(ctx),
        Commands::Goto {
            url,
            init_script,
            handle_before_unload,
            navigation_timeout_ms,
        } => browser::goto(
            ctx,
            &url,
            init_script.as_deref(),
            handle_before_unload,
            navigation_timeout_ms,
        ),
        Commands::View {
            verbose,
            path,
            allow_empty,
        } => browser::view(ctx, verbose, path.as_deref(), allow_empty),
        Commands::Press {
            target,
            dblclick,
            include_snapshot,
        } => browser::press(ctx, &target, dblclick, include_snapshot),
        Commands::ClickAt {
            x,
            y,
            dblclick,
            include_snapshot,
        } => browser::click_at(ctx, x, y, dblclick, include_snapshot),
        Commands::Write {
            target,
            value,
            include_snapshot,
        } => browser::write(ctx, &target, &value, include_snapshot),
        Commands::Keys {
            key,
            include_snapshot,
        } => browser::keys(ctx, &key, include_snapshot),
        Commands::Type {
            target,
            text,
            clear,
            submit,
            focus_only,
            include_snapshot,
        } => browser::type_text(
            ctx,
            target.as_deref(),
            &text,
            clear,
            submit.as_deref(),
            focus_only,
            include_snapshot,
        ),
        Commands::Wait {
            ms,
            text,
            selector,
            state,
            wait_timeout_ms,
            network_idle_ms,
            min_count,
            dom_stable_ms,
            include_snapshot,
        } => browser::wait(
            ctx,
            ms,
            &text,
            selector.as_deref(),
            state.as_deref(),
            wait_timeout_ms,
            network_idle_ms,
            min_count,
            dom_stable_ms,
            include_snapshot,
        ),
        Commands::Hover {
            target,
            include_snapshot,
        } => browser::hover(ctx, &target, include_snapshot),
        Commands::Drag {
            from,
            to,
            to_x,
            to_y,
            anchor,
            synthetic_payload,
            include_snapshot,
        } => browser::drag(
            ctx,
            &from,
            to.as_deref(),
            to_x,
            to_y,
            &anchor,
            synthetic_payload.as_deref(),
            include_snapshot,
        ),
        Commands::Submit {
            target,
            timeout_ms,
            include_snapshot,
        } => browser::submit(ctx, &target, timeout_ms, include_snapshot),
        Commands::FillForm {
            fields_json,
            include_snapshot,
        } => browser::fill_form(ctx, &fields_json, include_snapshot),
        Commands::Upload {
            target,
            path,
            include_snapshot,
        } => browser::upload(ctx, &target, &path, include_snapshot),
        Commands::Back => browser::back(ctx),
        Commands::Forward => browser::forward(ctx),
        Commands::Reload {
            ignore_cache,
            init_script,
            handle_before_unload,
        } => browser::reload(
            ctx,
            ignore_cache,
            init_script.as_deref(),
            handle_before_unload,
        ),
        Commands::Eval {
            expression,
            args,
            dialog_action,
            file_path,
            service_worker_id,
            typed,
        } => browser::eval(
            ctx,
            &expression,
            args.as_deref(),
            dialog_action.as_deref(),
            file_path.as_deref(),
            service_worker_id.as_deref(),
            typed,
        ),
        Commands::Grab {
            path,
            format,
            full_page,
            quality,
            element,
        } => browser::grab(
            ctx,
            path.as_deref(),
            format,
            full_page,
            quality,
            element.as_deref(),
        ),
        Commands::PrintPdf { path, url } => {
            browser::print_pdf(ctx, path.as_deref(), url.as_deref())
        }
        Commands::Monitor { action } => run_exec::monitor(ctx, action),
        Commands::Run { script } => run_exec::run(ctx, &script),
        Commands::Exec { args } => run_exec::exec(ctx, &args),
        Commands::Extract {
            target,
            attr,
            llm,
            question,
            schema_json,
        } => browser::extract(
            ctx,
            &target,
            attr.as_deref(),
            llm,
            question.as_deref(),
            schema_json.as_deref(),
        ),
        Commands::Text { target } => browser::text(ctx, &target),
        Commands::Scroll {
            target,
            delta_x,
            delta_y,
            to_x,
            to_y,
            include_snapshot,
        } => browser::scroll(
            ctx,
            target.as_deref(),
            delta_x,
            delta_y,
            to_x,
            to_y,
            include_snapshot,
        ),
        Commands::Storage { action } => result_code(
            crate::commands::nav::handle_storage(
                ctx.life,
                action,
                ctx.robots,
                ctx.capture,
                ctx.timeout_secs,
                ctx.json,
            ),
            ctx.json,
        ),
        Commands::Cookie { action } => browser::cookie(ctx, action),
        Commands::Attr { target, name } => browser::attr(ctx, &target, &name),
        Commands::Assert { kind } => browser::assert_cmd(ctx, kind),
        Commands::Console { action } => browser::console(ctx, action),
        Commands::Net { action } => browser::net(ctx, action),
        Commands::Page { action } => browser::page(ctx, action),
        Commands::Dialog { action } => browser::dialog(ctx, action),
        Commands::Scrape {
            url,
            format,
            engine,
            only_main_content,
            webhook_url,
        } => scrape::scrape(
            ctx,
            &url,
            &format,
            &engine,
            only_main_content,
            webhook_url.as_deref(),
        ),
        Commands::BatchScrape {
            urls_file,
            format,
            concurrency,
            engine,
        } => scrape::batch_scrape(ctx, &urls_file, &format, concurrency, &engine),
        Commands::Crawl {
            url,
            limit,
            max_depth,
            format,
            same_host,
            engine,
        } => scrape::crawl(ctx, &url, limit, max_depth, &format, same_host, &engine),
        Commands::Map {
            url,
            limit,
            max_depth,
        } => scrape::map(ctx, &url, limit, max_depth),
        Commands::Search { query, limit } => scrape::search(ctx, &query, limit),
        Commands::Parse { path, redact_pii } => scrape::parse(ctx, &path, redact_pii),
        Commands::Qr { action } => scrape::qr(ctx, action),
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
        } => scrape::find_paths(
            ctx,
            pattern.as_deref(),
            &paths,
            extension.as_deref(),
            hidden,
            no_ignore,
            max_depth,
            entry_type.as_deref(),
            limit,
            glob.as_deref(),
        ),
        Commands::SgScan { paths, limit } => scrape::sg_scan(ctx, &paths, limit),
        Commands::SgRewrite { paths, apply } => scrape::sg_rewrite(ctx, &paths, apply),
        Commands::SheetWrite { input, out, sheet } => {
            scrape::sheet_write(ctx, &input, &out, &sheet)
        }
        Commands::Mitm { action } => ops::mitm(ctx, action),
        Commands::Workflow { action } => ops::workflow(ctx, action),
        Commands::Config { action } => ops::config(ctx, action),
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
        } => browser::emulate(
            ctx,
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
        ),
        Commands::Resize {
            width,
            height,
            scale,
            mobile,
        } => browser::resize(ctx, width, height, scale, mobile),
        Commands::Perf { action } => ops::perf(ctx, action),
        Commands::Lighthouse {
            url,
            out_dir,
            device,
            mode,
            lighthouse_path,
        } => ops::lighthouse(
            ctx,
            &url,
            out_dir.as_deref(),
            &device,
            &mode,
            lighthouse_path.as_deref(),
        ),
        Commands::Screencast { action } => ops::screencast(ctx, action),
        Commands::Heap { action } => ops::heap(ctx, action),
        Commands::Extension { action } => ops::extension(ctx, action),
        Commands::Devtools3p { action } => ops::devtools3p(ctx, action),
        Commands::Webmcp { action } => ops::webmcp(ctx, action),
        Commands::Completions { shell } => ops::completions(ctx, shell),
        Commands::Man { out } => ops::man(ctx, out.as_deref()),
    }
}
