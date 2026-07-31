// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for concurrency budget and helpers.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering, Ordering as AtomOrd};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[test]
fn auto_budget_is_bounded() {
    let b = compute_auto_budget();
    assert!(b >= MIN_CONCURRENCY);
    assert!(b <= HARD_CAP);
}

#[test]
fn install_override_clamps() {
    install_limit(0);
    assert_eq!(OVERRIDE.load(Ordering::Relaxed), 0);
    install_limit(1);
    assert_eq!(effective_limit(), 1);
    install_limit(9999);
    assert_eq!(effective_limit(), HARD_CAP);
    // Reset for other tests in the same process.
    install_limit(0);
}

#[test]
fn browser_workers_at_least_two() {
    install_limit(1);
    assert!(browser_worker_threads() >= 2);
    install_limit(0);
}

#[tokio::test]
async fn join_bounded_respects_peak() {
    let peak = Arc::new(AtomicUsize::new(0));
    let current = Arc::new(AtomicUsize::new(0));
    let limit = 3usize;
    let mut futs = Vec::new();
    for _ in 0..12 {
        let peak = Arc::clone(&peak);
        let current = Arc::clone(&current);
        futs.push(async move {
            let n = current.fetch_add(1, AtomOrd::SeqCst) + 1;
            peak.fetch_max(n, AtomOrd::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
            current.fetch_sub(1, AtomOrd::SeqCst);
            1u32
        });
    }
    let out = join_bounded(futs, limit).await;
    assert_eq!(out.len(), 12);
    assert!(
        peak.load(AtomOrd::SeqCst) <= limit,
        "peak {} exceeded limit {}",
        peak.load(AtomOrd::SeqCst),
        limit
    );
}

#[tokio::test]
async fn join_bounded_ordered_preserves_order() {
    let futs: Vec<_> = (0..8u32)
        .map(|i| async move {
            tokio::time::sleep(std::time::Duration::from_millis((8 - i) as u64)).await;
            i
        })
        .collect();
    let out = join_bounded_ordered(futs, 4).await;
    assert_eq!(out, (0..8).collect::<Vec<_>>());
}

#[test]
fn semaphore_has_effective_permits() {
    install_limit(4);
    let s = io_semaphore();
    assert_eq!(s.available_permits(), 4);
    install_limit(0);
}

#[test]
fn resolve_permits_zero_is_effective() {
    install_limit(3);
    assert_eq!(resolve_permits(0), 3);
    assert_eq!(resolve_permits(2), 2);
    assert_eq!(resolve_permits(9999), HARD_CAP);
    install_limit(0);
}

#[test]
fn command_matrix_lists_parallel_and_sequential() {
    let m = command_workload_matrix();
    assert!(m.get("parallel_io").and_then(|v| v.as_array()).is_some());
    assert!(m
        .get("sequential_justified")
        .and_then(|v| v.as_object())
        .is_some());
}

#[test]
#[cfg(target_os = "linux")]
fn free_ram_linux_reads_meminfo() {
    // On a normal Linux host MemAvailable is present; CI containers too.
    let mb = free_ram_mb();
    assert!(mb.is_some(), "expected MemAvailable/MemFree on Linux");
    assert!(mb.unwrap() > 0);
}

#[tokio::test]
async fn semaphore_permit_returns_after_panic_in_joinset() {
    // Rule checklist: panic in task must not permanently leak permits.
    let limit = 2usize;
    let sem = Arc::new(Semaphore::new(limit));
    let mut set: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
    for i in 0..4 {
        let permit = Arc::clone(&sem).acquire_owned().await.expect("sem open");
        set.spawn(async move {
            let _permit = permit;
            if i == 1 {
                panic!("intentional concurrency panic");
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        });
    }
    let mut panics = 0u32;
    while let Some(joined) = set.join_next().await {
        if let Err(e) = joined {
            if e.is_panic() {
                panics += 1;
            }
        }
    }
    assert_eq!(panics, 1);
    assert_eq!(
        sem.available_permits(),
        limit,
        "all permits must return after JoinSet drain (incl. panic tasks)"
    );
}

#[test]
fn walk_threads_never_exceeds_hard_cap_or_cpus() {
    // Pure bound: walk threads ≤ min(HARD_CAP, cpus) regardless of override races.
    let w = walk_threads();
    assert!(w >= MIN_CONCURRENCY);
    assert!(w <= HARD_CAP);
    assert!(w <= cpu_count().max(MIN_CONCURRENCY));
}

#[test]
fn command_matrix_has_na_and_cancel() {
    let m = command_workload_matrix();
    assert!(m.get("na_product_law").and_then(|v| v.as_array()).is_some());
    assert!(m.get("cancel").and_then(|v| v.as_str()).is_some());
    let seq = m
        .get("sequential_justified")
        .and_then(|v| v.as_object())
        .expect("seq");
    assert!(seq.contains_key("lighthouse"));
    assert!(seq.contains_key("mitm start/capture"));
}

#[test]
fn map_cpu_sequential_below_threshold() {
    let items: Vec<u32> = (0..10).collect();
    let out = map_cpu(&items, |x| x * 2);
    assert_eq!(out, (0..10).map(|x| x * 2).collect::<Vec<_>>());
}

#[test]
fn map_cpu_parallel_above_threshold() {
    let items: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 8)).collect();
    let out = map_cpu(&items, |x| x.saturating_add(1));
    assert_eq!(out.len(), items.len());
    assert_eq!(out[0], 1);
    assert_eq!(out[items.len() - 1], items[items.len() - 1] + 1);
}

#[test]
fn sort_cpu_orders_small_and_large() {
    // PAR-99: threshold path + parallel path both produce sorted output.
    let mut small = vec![3, 1, 2];
    sort_cpu(&mut small);
    assert_eq!(small, vec![1, 2, 3]);
    let mut large: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 16)).rev().collect();
    sort_cpu(&mut large);
    assert!(large.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(large.first().copied(), Some(0));
}

#[test]
fn sort_by_key_cpu_reverse_counts() {
    let mut items = vec![("a", 2u64), ("b", 9u64), ("c", 1u64)];
    sort_by_key_cpu(&mut items, |b| std::cmp::Reverse(b.1));
    assert_eq!(items[0].1, 9);
    assert_eq!(items[2].1, 1);
}

#[test]
fn matrix_residual_mentions_index_proc() {
    let m = command_workload_matrix();
    let by = m.get("by_command").and_then(|v| v.as_object()).expect("by");
    let residual = by.get("residual").expect("residual");
    let gate = residual.get("gate").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        gate.contains("index_proc") || gate.contains("map_cpu"),
        "residual gate must document index-once scavenge: {gate}"
    );
    let helpers = m
        .get("helpers")
        .and_then(|v| v.as_array())
        .expect("helpers");
    let has_sort = helpers.iter().any(|h| h.as_str() == Some("sort_cpu"));
    assert!(has_sort, "helpers must list sort_cpu");
}

#[test]
fn by_command_covers_inventory_minimum() {
    let m = command_workload_matrix();
    let by = m
        .get("by_command")
        .and_then(|v| v.as_object())
        .expect("by_command");
    for key in [
        "doctor",
        "goto",
        "view",
        "batch-scrape",
        "crawl",
        "find-paths",
        "sg-scan",
        "screencast",
        "heap",
        "workflow",
        "run",
        "mitm",
        "state",
        "residual",
        "lighthouse",
        "grab",
        "map",
        "search",
        "console",
        "net",
        // Pass 25 nested multi-item
        "console.list",
        "console.dump",
        "net.list",
        "net.get",
        "heap.dup-strings",
        "mitm.domains",
        "state.load",
        "perf.insight",
        "screencast.stop",
    ] {
        assert!(by.contains_key(key), "missing by_command entry: {key}");
    }
    assert!(m.get("helpers").and_then(|v| v.as_array()).is_some());
}

#[test]
fn matrix_honesty_doctor_not_fake_map_cpu() {
    // PAR-73: doctor must not claim map_cpu when probes are sequential.
    let m = command_workload_matrix();
    let by = m
        .get("by_command")
        .and_then(|v| v.as_object())
        .expect("by_command");
    let doctor = by
        .get("doctor")
        .and_then(|v| v.as_object())
        .expect("doctor");
    assert_eq!(
        doctor.get("class").and_then(|v| v.as_str()),
        Some("sequential_justified")
    );
    assert!(
        doctor.get("gate").is_none(),
        "doctor must not claim a parallel gate"
    );
    let helpers = m
        .get("helpers")
        .and_then(|v| v.as_array())
        .expect("helpers");
    let helper_names: Vec<&str> = helpers.iter().filter_map(|v| v.as_str()).collect();
    assert!(helper_names.contains(&"filter_cpu"));
    assert!(helper_names.contains(&"count_cpu"));
    assert!(helper_names.contains(&"read_to_string_blocking"));
    assert!(helper_names.contains(&"rename_blocking"));
}

#[test]
fn filter_cpu_sequential_below_threshold() {
    let items: Vec<u32> = (0..10).collect();
    let out = filter_cpu(items, |x| x % 2 == 0);
    assert_eq!(out, vec![0, 2, 4, 6, 8]);
}

#[test]
fn filter_cpu_parallel_above_threshold() {
    let items: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 16)).collect();
    let out = filter_cpu(items.clone(), |x| x % 2 == 0);
    assert_eq!(out.len(), items.len() / 2);
    assert_eq!(out[0], 0);
}

#[test]
fn count_cpu_matches_filter_len_without_clone() {
    let items: Vec<u32> = (0..10).collect();
    assert_eq!(count_cpu(&items, |x| x % 2 == 0), 5);
    let large: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 16)).collect();
    assert_eq!(
        count_cpu(&large, |x| x % 2 == 0),
        filter_cpu(large.clone(), |x| x % 2 == 0).len()
    );
}

#[tokio::test]
async fn write_bytes_blocking_roundtrip() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("par24.bin");
    write_bytes_blocking(path.clone(), b"pass24".to_vec())
        .await
        .expect("write");
    let got = read_bytes_blocking(path).await.expect("read");
    assert_eq!(got, b"pass24");
}

#[tokio::test]
async fn read_to_string_and_rename_blocking_roundtrip() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("a.txt");
    let path2 = dir.path().join("b.txt");
    write_bytes_blocking(path.clone(), b"pass25".to_vec())
        .await
        .expect("write");
    let s = read_to_string_blocking(path.clone())
        .await
        .expect("read str");
    assert_eq!(s, "pass25");
    rename_blocking(path, path2.clone()).await.expect("rename");
    let s2 = read_to_string_blocking(path2).await.expect("read2");
    assert_eq!(s2, "pass25");
}
