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
mod scrape_args;

use crate::cli::Commands;

pub(crate) use ctx::{result_code, DispatchCtx};

/// Route one parsed command. Caller must `life.finalize()` after return.
///
/// Owned clap fields are borrowed into leaf handlers (`&str` / `&Path` / `&[T]`)
/// so dispatch never needlessly takes ownership of values it only reads.
pub(crate) fn route(cmd: Commands, ctx: &DispatchCtx<'_>) -> i32 {
    match cmd {
        Commands::Doctor(a) => meta::doctor(ctx, a.offline, a.quick, a.fix),
        Commands::Commands { detail } => meta::commands(ctx, detail),
        Commands::Schema(a) => meta::schema(ctx, a.cmd.as_deref(), a.cmd_positional.as_deref()),
        Commands::Version => meta::version(ctx),
        Commands::Locale => meta::locale(ctx),
        Commands::Goto(a) => browser::goto(
            ctx,
            &a.url,
            a.init_script.as_deref(),
            a.handle_before_unload,
            a.navigation_timeout_ms,
        ),
        Commands::View(a) => browser::view(ctx, a.verbose, a.path.as_deref(), a.allow_empty),
        Commands::Press(a) => browser::press(ctx, &a.target, a.dblclick, a.include_snapshot),
        Commands::ClickAt(a) => browser::click_at(ctx, a.x, a.y, a.dblclick, a.include_snapshot),
        Commands::Write(a) => browser::write(ctx, &a.target, &a.value, a.include_snapshot),
        Commands::Keys(a) => browser::keys(ctx, &a.key, a.include_snapshot),
        Commands::Type(a) => browser::type_text(
            ctx,
            a.target.as_deref(),
            &a.text,
            a.clear,
            a.submit.as_deref(),
            a.focus_only,
            a.include_snapshot,
        ),
        Commands::Wait(a) => browser::wait(
            ctx,
            a.ms,
            &a.text,
            a.selector.as_deref(),
            a.state.as_deref(),
            a.wait_timeout_ms,
            a.network_idle_ms,
            a.min_count,
            a.dom_stable_ms,
            a.include_snapshot,
        ),
        Commands::Hover(a) => browser::hover(ctx, &a.target, a.include_snapshot),
        Commands::Drag(a) => browser::drag(
            ctx,
            &a.from,
            a.to.as_deref(),
            a.to_x,
            a.to_y,
            &a.anchor,
            a.synthetic_payload.as_deref(),
            a.include_snapshot,
        ),
        Commands::Submit(a) => browser::submit(ctx, &a.target, a.timeout_ms, a.include_snapshot),
        Commands::FillForm(a) => browser::fill_form(ctx, &a.fields_json, a.include_snapshot),
        Commands::Upload(a) => browser::upload(ctx, &a.target, &a.path, a.include_snapshot),
        Commands::Back => browser::back(ctx),
        Commands::Forward => browser::forward(ctx),
        Commands::Reload(a) => browser::reload(
            ctx,
            a.ignore_cache,
            a.init_script.as_deref(),
            a.handle_before_unload,
        ),
        Commands::Eval(a) => browser::eval(
            ctx,
            &a.expression,
            a.args.as_deref(),
            a.dialog_action.as_deref(),
            a.file_path.as_deref(),
            a.service_worker_id.as_deref(),
            a.typed,
        ),
        Commands::Grab(a) => browser::grab(
            ctx,
            a.path.as_deref(),
            a.format,
            a.full_page,
            a.quality,
            a.element.as_deref(),
            a.include_base64,
        ),
        Commands::PrintPdf { path, url } => {
            browser::print_pdf(ctx, path.as_deref(), url.as_deref())
        }
        Commands::Monitor { action } => run_exec::monitor(ctx, action),
        Commands::Run { script } => run_exec::run(ctx, &script),
        Commands::Exec { args } => run_exec::exec(ctx, &args),
        Commands::Record(a) => ops::record(ctx, &a.url, &a.path, a.seconds, a.max_events),
        Commands::Extract(a) => browser::extract(
            ctx,
            &a.target,
            a.attr.as_deref(),
            a.llm,
            a.question.as_deref(),
            a.schema_json.as_deref(),
        ),
        Commands::Text { target } => browser::text(ctx, &target),
        Commands::Scroll(a) => browser::scroll(
            ctx,
            a.target.as_deref(),
            a.delta_x,
            a.delta_y,
            a.to_x,
            a.to_y,
            a.include_snapshot,
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
        Commands::Scrape(a) => scrape::scrape(
            ctx,
            &a.url,
            &a.format,
            &a.engine,
            a.only_main_content,
            a.webhook_url.as_deref(),
            a.select.as_deref(),
            a.max_text_chars,
            &a.include_selector,
            &a.exclude_selector,
            a.redact_pii,
            a.with_content_hash,
            a.schema_json.as_deref(),
            a.question.as_deref(),
            &a.header,
            a.wait_ms,
            &a.attribute_selector,
            &a.attribute_name,
            &a.action,
            a.no_cache,
        ),
        Commands::BatchScrape(a) => scrape::batch_scrape(
            ctx,
            &a.urls_file,
            &a.format,
            a.concurrency,
            &a.engine,
            a.select.as_deref(),
            a.max_text_chars,
            a.filter.as_deref(),
            &a.output_mode,
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
            a.dedup_similar,
            &a.include_selector,
            &a.exclude_selector,
            a.redact_pii,
            a.with_content_hash,
        ),
        Commands::Crawl(a) => scrape::crawl(
            ctx,
            &a.url,
            a.limit,
            a.max_depth,
            &a.format,
            a.same_host,
            &a.engine,
            a.select.as_deref(),
            a.max_text_chars,
            a.filter.as_deref(),
            &a.output_mode,
            &a.include_path,
            &a.exclude_path,
            a.use_sitemap,
            a.ignore_query_params,
            a.follow_rel_next,
            a.dedup_similar,
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
            a.redact_pii,
            a.with_content_hash,
            &a.include_selector,
            &a.exclude_selector,
            a.dry_run,
        ),
        Commands::Map(a) => scrape::map(
            ctx,
            &a.url,
            a.limit,
            if a.sitemap_only { 0 } else { a.max_depth },
            a.select.as_deref(),
            &a.include_path,
            &a.exclude_path,
            if a.sitemap_only {
                Some(true)
            } else {
                a.use_sitemap
            },
            a.search.as_deref(),
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
        ),
        Commands::Search(a) => scrape::search(
            ctx,
            &a.query,
            a.limit,
            a.select.as_deref(),
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
        ),
        Commands::Parse { path, redact_pii } => scrape::parse(ctx, &path, redact_pii),
        Commands::Qr { action } => scrape::qr(ctx, action),
        Commands::Image { action } => scrape::image(ctx, action),
        Commands::Video { action } => scrape::video(ctx, action),
        Commands::Audio { action } => scrape::audio(ctx, action),
        Commands::FindPaths(a) => scrape::find_paths(
            ctx,
            a.pattern.as_deref(),
            &a.paths,
            a.extension.as_deref(),
            a.hidden,
            a.no_ignore,
            a.max_depth,
            a.entry_type.as_deref(),
            a.limit,
            a.glob.as_deref(),
        ),
        Commands::SgScan { paths, limit } => scrape::sg_scan(ctx, &paths, limit),
        Commands::SgRewrite { paths, apply } => scrape::sg_rewrite(ctx, &paths, apply),
        Commands::SheetWrite(a) => scrape::sheet_write(ctx, &a.input, &a.out, &a.sheet),
        Commands::Mitm { action } => ops::mitm(ctx, action),
        Commands::Workflow { action } => ops::workflow(ctx, action),
        Commands::Config { action } => ops::config(ctx, action),
        Commands::Emulate(a) => browser::emulate(
            ctx,
            a.user_agent.as_deref(),
            a.locale.as_deref(),
            a.timezone.as_deref(),
            a.offline,
            a.latitude,
            a.longitude,
            a.media.as_deref(),
            a.network_conditions.as_deref(),
            a.cpu_throttling_rate,
            a.color_scheme.as_deref(),
            a.extra_headers.as_deref(),
            a.viewport.as_deref(),
        ),
        Commands::Resize(a) => browser::resize(ctx, a.width, a.height, a.scale, a.mobile),
        Commands::Perf { action } => ops::perf(ctx, action),
        Commands::Lighthouse(a) => ops::lighthouse(
            ctx,
            &a.url,
            a.out_dir.as_deref(),
            &a.device,
            &a.mode,
            a.lighthouse_path.as_deref(),
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
