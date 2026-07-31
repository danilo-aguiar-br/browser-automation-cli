// SPDX-License-Identifier: MIT OR Apache-2.0
//! Workload matrix report builders and permit resolution.

use std::sync::atomic::Ordering;

use super::super::budget::{
    compute_auto_budget, cpu_count, effective_limit, free_ram_mb, HARD_CAP, MIN_CONCURRENCY,
    OVERRIDE, RAM_PER_IO_TASK_MB,
};
use super::super::pool::{browser_worker_threads, CPU_MAP_THRESHOLD};
use super::rows::command_rows;

/// Snapshot of concurrency budget for doctor / `--json` diagnostics.
pub fn budget_report() -> serde_json::Value {
    serde_json::json!({
        "effective": effective_limit(),
        "override": OVERRIDE.load(Ordering::Relaxed),
        "auto": compute_auto_budget(),
        "cpus": cpu_count(),
        "free_ram_mb": free_ram_mb(),
        "ram_per_io_task_mb": RAM_PER_IO_TASK_MB,
        "hard_cap": HARD_CAP,
        "cpu_map_threshold": CPU_MAP_THRESHOLD,
        "browser_workers": browser_worker_threads(),
        "formula": "min(cpus, (free_ram_mb*50%)/64, 64); --max-concurrency overrides",
        "workload_default": "I/O-bound (CDP/HTTP) + CPU-bound (sg scan via rayon)",
        "local_available_permits_note": "host-local only; no remote OTel of permits (product law)",
        "commands": command_workload_matrix(),
    })
}

/// Entry helper for [`command_by_command_matrix`] (avoids deep `json!` recursion).
fn cmd_entry(class: &str, gate: Option<&str>, reason: Option<&str>) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    m.insert("class".into(), serde_json::Value::String(class.into()));
    if let Some(g) = gate {
        m.insert("gate".into(), serde_json::Value::String(g.into()));
    }
    if let Some(r) = reason {
        m.insert("reason".into(), serde_json::Value::String(r.into()));
    }
    serde_json::Value::Object(m)
}

fn command_by_command_matrix() -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (name, class, gate, reason) in command_rows() {
        map.insert((*name).into(), cmd_entry(class, *gate, *reason));
    }
    serde_json::Value::Object(map)
}

/// Per-command parallelism posture (agent discovery / doctor).
///
/// Every multi-item command either fans out under [`effective_limit`] or has an
/// explicit sequential justification (single CDP session, single-writer journal,
/// atomic rewrite apply, or cost ≪ coordination overhead).
///
/// Pass 24–25: `by_command` maps each top-level CLI command **and** multi-item
/// nested subcommands so agents never treat sequential single-act paths as
/// forgotten parallelism. Gates must match code (PAR-73 honesty).
pub fn command_workload_matrix() -> serde_json::Value {
    let mut root = serde_json::Map::new();
    root.insert(
        "parallel_io".into(),
        serde_json::json!([
            "batch-scrape --engine http (JoinSet+Semaphore; parse via spawn_blocking)",
            "crawl --engine http (bounded frontier; discovery under same permit)",
            "map / search (HTTP; crawl/scrape under budget)",
            "view/snapshot multi-ref CDP resolve (join_bounded+Semaphore)",
            "grab multi-target rect resolve (join_bounded+Semaphore)",
            "find-paths (WalkBuilder.threads=walk_threads; multi-root par)",
            "robots shared client keep-alive under batch",
            "network sanitize multi-page (join_bounded CDP navigate)",
            "browser multi-target attach (join_bounded_ordered)",
            "cdp page forwarders multi-page (join_bounded)",
            "screencast stop frames (spawn_blocking+rayon decode/write)"
        ]),
    );
    root.insert(
        "parallel_cpu".into(),
        serde_json::json!([
            "sg-scan (rayon par_iter; multi-root collect par)",
            "sg-rewrite dry-run (rayon par_iter); --apply sequential",
            "heap score/filter independent passes (map_cpu); idom/RPO sequential",
            "mitm domains/apis filter when items >= CPU_MAP_THRESHOLD",
            "console/net list filter_cpu when buffer >= CPU_MAP_THRESHOLD",
            "residual multi-candidate scavenge when >= threshold",
            "perf_insight top-level events map_cpu when large"
        ]),
    );
    root.insert(
        "sequential_justified".into(),
        serde_json::json!({
            "batch-scrape --engine browser": "single residual Chrome / one Page (N-129); use --engine http for fan-out",
            "crawl --engine browser": "single CDP Page session (N-129)",
            "run / exec multi-step": "ordered script semantics + fail-fast (N-134)",
            "workflow run/resume": "SQLite journal single-writer (N-130); fan-out inside steps",
            "fill-form": "DOM focus/state must be sequential on one Page (N-135)",
            "sg-rewrite --apply": "atomic writers must not race the same tree (N-136)",
            "sheet-write": "single workbook writer (rust_xlsxwriter not Sync) (N-137)",
            "qr encode/decode": "single payload; multi-grid decode rare and cheap",
            "goto/press/type/click/keys/…": "single interactive act (N-138); cost ≪ spawn",
            "doctor/commands/schema/version/locale": "meta; doctor probes cheap — sequential (N-144); cost ≪ Rayon",
            "lighthouse": "single external subprocess (N-140)",
            "mitm start/capture": "one proxy task JoinHandle awaited; not multi-URL fan-out",
            "llm local": "single request; OnceLock HTTP client (N-139)",
            "residual FINALIZE": "map_cpu when candidates large; else cost ≪ overhead",
            "install chrome discovery": "few version dirs; sequential OK (N-cost)",
            "state list/clear/clean": "few session files; cost ≪ Rayon (PAR-72)",
            "state load multi-origin": "single CDP session navigate — sequential (N-143)",
            "cache get/put": "single key; Mutex short critical section no .await",
            "parse spreadsheet multi-sheet": "calamine Reader not Sync — sequential (PAR-59)",
            "type char-a-char": "ordered input semantics (N-141)",
            "snapshot tree build": "parent/child links ordered — sequential (N-145)"
        }),
    );
    root.insert("by_command".into(), command_by_command_matrix());
    root.insert(
        "bound_everywhere".into(),
        serde_json::Value::String(
            "JoinSet+Arc<Semaphore>::acquire_owned | join_bounded | WalkBuilder.threads(walk_threads) | rayon pool sized to budget | map_cpu/filter_cpu/sort_cpu threshold".into(),
        ),
    );
    root.insert(
        "cancel".into(),
        serde_json::Value::String(
            "Lifecycle CancellationToken checked mid batch/crawl acquire".into(),
        ),
    );
    root.insert(
        "helpers".into(),
        serde_json::json!([
            "write_bytes_blocking",
            "write_bytes_sync",
            "create_dir_all_blocking",
            "read_bytes_blocking",
            "read_to_string_blocking",
            "rename_blocking",
            "map_cpu",
            "filter_cpu",
            "count_cpu",
            "sort_cpu",
            "sort_by_cpu",
            "sort_by_key_cpu",
            "join_bounded"
        ]),
    );
    root.insert(
        "na_product_law".into(),
        serde_json::json!([
            "multi-process Chrome pool (N-129/N-154)",
            "workflow multi-writer SQLite (N-130/N-155)",
            "systemd-run MemoryMax default (N-121)",
            "remote OTel available_permits (N-124)",
            "loom GHA (N-122)",
            "state multi-origin parallel same session (N-143)",
            "heap idom/RPO blind par_iter (N-142/N-152)",
            "doctor cheap probes Rayon (N-144/N-156)",
            "snapshot tree build blind par (N-145/N-153)",
            "DOM single-act parallel (N-151)",
            "mitm block/allow rules sync CLI (N-158)"
        ]),
    );
    serde_json::Value::Object(root)
}

/// Resolve permits for a fan-out: `0` → process budget; else clamp to hard cap.
pub fn resolve_permits(requested: usize) -> usize {
    if requested == 0 {
        effective_limit()
    } else {
        requested.clamp(MIN_CONCURRENCY, HARD_CAP)
    }
}
