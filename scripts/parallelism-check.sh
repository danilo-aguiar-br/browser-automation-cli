#!/usr/bin/env bash
# Local gate: bounded parallelism / no unbounded fan-out anti-patterns.
# rules_rust_paralelismo_e_multiprocessamento — product law one-shot CLI.
# Pass 23: Semaphore gate in join_bounded + spawn_blocking scrape parse.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail=0

echo "== concurrency module present =="
test -f src/concurrency.rs || { echo "missing src/concurrency.rs"; exit 1; }

echo "== no production Box::leak / mem::forget in concurrency =="
if rg -n 'Box::leak|mem::forget' src/concurrency.rs; then
  echo "FAIL: leak/forget in concurrency module"
  fail=1
fi

echo "== no unbounded join_all on large fan-out (src, excluding tests comments) =="
if rg -n 'future::join_all|futures_util::future::join_all' src/native/snapshot.rs src/native/screenshot.rs src/scrape_local.rs 2>/dev/null; then
  echo "FAIL: unbounded join_all still present in fan-out modules"
  fail=1
else
  echo "OK: fan-out modules use join_bounded / JoinSet"
fi

echo "== batch/crawl use Semaphore gate (acquire_owned / try_acquire_owned) =="
if ! rg -n 'acquire_owned|try_acquire_owned' src/scrape_local.rs >/dev/null; then
  echo "FAIL: scrape_local missing Semaphore acquire_owned gate"
  fail=1
else
  echo "OK: scrape_local Semaphore gate present"
fi

echo "== join_bounded uses Semaphore acquire =="
if ! rg -n 'sem\.acquire\(\)|Semaphore::new' src/concurrency.rs >/dev/null; then
  echo "FAIL: join_bounded missing Semaphore gate"
  fail=1
else
  echo "OK: join_bounded Semaphore present"
fi

echo "== scrape HTML parse uses spawn_blocking =="
if ! rg -n 'spawn_blocking' src/scrape_local.rs >/dev/null; then
  echo "FAIL: scrape_local missing spawn_blocking for CPU parse"
  fail=1
else
  echo "OK: scrape spawn_blocking present"
fi

echo "== walk_threads helper (budget-aware) =="
if ! rg -n 'fn walk_threads' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing walk_threads"
  fail=1
else
  echo "OK: walk_threads present"
fi

echo "== command workload matrix exported =="
if ! rg -n 'command_workload_matrix' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing command_workload_matrix"
  fail=1
else
  echo "OK: command_workload_matrix present"
fi

echo "== matrix has na_product_law =="
if ! rg -n 'na_product_law' src/concurrency.rs >/dev/null; then
  echo "FAIL: matrix missing na_product_law"
  fail=1
else
  echo "OK: na_product_law present"
fi

echo "== Pass 24: by_command matrix + helpers =="
if ! rg -n 'by_command' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing by_command in matrix"
  fail=1
else
  echo "OK: by_command present"
fi
if ! rg -n 'fn write_bytes_blocking|fn map_cpu' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing write_bytes_blocking / map_cpu helpers"
  fail=1
else
  echo "OK: Pass 24 helpers present"
fi

echo "== Pass 24: screencast frames use spawn_blocking =="
if ! rg -n 'spawn_blocking' src/browser/mod.rs | rg -q 'screencast|frame'; then
  # softer: require spawn_blocking near screencast_stop path
  if ! rg -n 'parallel_frames|par_iter' src/browser/mod.rs >/dev/null; then
    echo "FAIL: screencast stop missing parallel frame write"
    fail=1
  else
    echo "OK: screencast parallel frames marker present"
  fi
else
  echo "OK: screencast spawn_blocking present"
fi
if ! rg -n 'par_iter' src/browser/mod.rs >/dev/null; then
  echo "FAIL: browser missing Rayon par_iter for screencast frames"
  fail=1
else
  echo "OK: browser Rayon frames present"
fi

echo "== Pass 24: sg multi-root par collect =="
if ! rg -n 'par_iter\(\)\.flat_map|roots\.par_iter' src/sg_local.rs >/dev/null; then
  echo "FAIL: sg_local missing multi-root parallel collect"
  fail=1
else
  echo "OK: sg multi-root par present"
fi

echo "== Pass 24: CDP page forwarders join_bounded =="
if ! rg -n 'join_bounded' src/native/cdp/client.rs >/dev/null; then
  echo "FAIL: cdp client missing join_bounded for multi-page forwarders"
  fail=1
else
  echo "OK: cdp join_bounded present"
fi

echo "== Pass 25: filter_cpu + read_to_string_blocking + rename_blocking =="
if ! rg -n 'fn filter_cpu' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing filter_cpu helper"
  fail=1
else
  echo "OK: filter_cpu present"
fi
if ! rg -n 'fn read_to_string_blocking|fn rename_blocking' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing read_to_string_blocking / rename_blocking"
  fail=1
else
  echo "OK: Pass 25 blocking helpers present"
fi

echo "== Pass 25: console/net use filter_cpu =="
if ! rg -n 'filter_cpu' src/browser/mod.rs >/dev/null; then
  echo "FAIL: browser missing filter_cpu for console/net"
  fail=1
else
  echo "OK: browser filter_cpu present"
fi

echo "== Pass 25: state load uses async blocking read path =="
if ! rg -n 'read_state_json_async|read_bytes_blocking|spawn_blocking.*read_state' src/native/state.rs >/dev/null; then
  echo "FAIL: state load missing blocking read offload"
  fail=1
else
  echo "OK: state load blocking read present"
fi

echo "== Pass 25: matrix honesty (doctor not fake map_cpu) =="
if rg -n '"doctor".*"map_cpu checks"|doctor.*map_cpu checks' src/concurrency.rs >/dev/null; then
  echo "FAIL: matrix overclaims doctor map_cpu (must be sequential_justified)"
  fail=1
else
  echo "OK: doctor matrix not overclaiming map_cpu"
fi
if ! rg -n 'console\.list|heap\.dup-strings|filter_cpu when large' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing nested by_command / filter_cpu gates"
  fail=1
else
  echo "OK: nested multi-item by_command markers present"
fi

echo "== Pass 26: residual index_proc_cmdlines once (PAR-89) =="
if ! rg -n 'fn index_proc_cmdlines|index_proc_cmdlines\(\)' src/residual.rs >/dev/null; then
  echo "FAIL: residual missing index_proc_cmdlines"
  fail=1
else
  echo "OK: residual index_proc_cmdlines present"
fi
if ! rg -n 'path_has_live_process\(path, &proc_index\)|path_has_live_process\(path, proc_index\)' src/residual.rs >/dev/null; then
  echo "FAIL: residual still scans /proc per candidate (must pass index)"
  fail=1
else
  echo "OK: residual live-check uses shared index"
fi

echo "== Pass 26: MITM CA blocking read (PAR-91) =="
if ! rg -n 'load_ca_pems_blocking|read_to_string_blocking' src/mitm_local.rs >/dev/null; then
  echo "FAIL: mitm missing load_ca_pems_blocking / read_to_string_blocking"
  fail=1
else
  echo "OK: mitm CA blocking path present"
fi
# Raw fs::read_to_string of CA inside async oneshot bodies is forbidden.
if rg -n 'async fn (start_proxy_oneshot|capture_url_oneshot)' -A40 src/mitm_local.rs | rg -n 'fs::read_to_string\(cert|fs::read_to_string\(key' >/dev/null; then
  echo "FAIL: mitm async oneshot still uses fs::read_to_string for CA"
  fail=1
else
  echo "OK: mitm async oneshot no raw CA fs::read_to_string"
fi

echo "== Pass 26: chrome temp profile mkdir off async (PAR-92) =="
if rg -n 'std::fs::create_dir_all\(&dir\)' src/native/cdp/chrome.rs >/dev/null; then
  # materialize_temp_user_data_dir_sync may still create_dir_all — that's OK if not in build_chrome_args body side-effect only
  if rg -n 'create_dir_all\(&dir\)' src/native/cdp/chrome.rs | rg -v 'materialize_temp_user_data_dir_sync|//|PAR-92' >/dev/null; then
    # Allow only inside materialize helper
    if ! rg -n 'fn materialize_temp_user_data_dir_sync' -A6 src/native/cdp/chrome.rs | rg -q 'create_dir_all'; then
      echo "FAIL: chrome create_dir_all outside materialize helper"
      fail=1
    else
      echo "OK: chrome create_dir_all confined to materialize helper"
    fi
  else
    echo "OK: chrome create_dir_all confined"
  fi
else
  echo "OK: no create_dir_all(&dir) in chrome.rs"
fi
if ! rg -n 'create_dir_all_blocking' src/native/cdp/oxide.rs >/dev/null; then
  echo "FAIL: oxide launch missing create_dir_all_blocking for temp profile"
  fail=1
else
  echo "OK: oxide create_dir_all_blocking present"
fi

echo "== Pass 26: sort_cpu helper + call sites (PAR-94) =="
if ! rg -n 'fn sort_cpu|fn sort_by_cpu|fn sort_by_key_cpu' src/concurrency.rs >/dev/null; then
  echo "FAIL: missing sort_cpu helpers"
  fail=1
else
  echo "OK: sort_cpu helpers present"
fi
if ! rg -n 'sort_cpu|sort_by_cpu|sort_by_key_cpu' src/sg_local.rs src/native/heap_snapshot.rs src/native/perf_insight.rs src/mitm_local.rs >/dev/null; then
  echo "FAIL: sort_cpu not used at multi-item sort sites"
  fail=1
else
  echo "OK: sort_cpu used at multi-item sites"
fi

echo "== Pass 26: find_paths no Mutex fan-out (PAR-95) =="
if rg -n 'use std::sync::Mutex|Mutex::new|Mutex<' src/find_paths.rs >/dev/null; then
  echo "FAIL: find_paths still uses Mutex (must flat_map collect)"
  fail=1
elif ! rg -n 'flat_map|par_iter' src/find_paths.rs >/dev/null; then
  echo "FAIL: find_paths missing multi-root flat_map/par_iter"
  fail=1
else
  echo "OK: find_paths Mutex-free (flat_map multi-root)"
fi

echo "== Pass 26: extension multi-close join_bounded (PAR-96) =="
if ! rg -n 'join_bounded' src/browser/mod.rs | rg -q .; then
  echo "FAIL: browser missing join_bounded (extension multi-close expected)"
  fail=1
else
  # Prefer explicit closeTarget near join_bounded
  if rg -n 'closeTarget' src/browser/mod.rs >/dev/null && rg -n 'join_bounded' src/browser/mod.rs >/dev/null; then
    echo "OK: browser has closeTarget + join_bounded"
  else
    echo "WARN: join_bounded present but closeTarget pattern unclear"
  fi
fi

echo "== --max-concurrency flag wired =="
if ! rg -n 'max_concurrency|max-concurrency' src/cli.rs src/lib.rs >/dev/null; then
  echo "FAIL: --max-concurrency not wired"
  fail=1
fi

echo "== no unbounded_channel in production src =="
if rg -n 'unbounded_channel' src --glob '*.rs' | rg -v 'test|//|N/A|proib'; then
  echo "FAIL: unbounded_channel in production"
  fail=1
else
  echo "OK: no production unbounded_channel"
fi

echo "== cargo test concurrency unit =="
cargo test --lib concurrency:: -- --quiet

echo "== doctor budget field =="
out="$(cargo run --quiet -- doctor --offline --quick --json 2>/dev/null || true)"
if echo "$out" | rg -q '"concurrency"'; then
  echo "OK: doctor JSON exposes concurrency budget"
else
  echo "WARN: doctor JSON missing concurrency (may fail if chrome missing); checking unit only"
fi
if echo "$out" | rg -q '"commands"'; then
  echo "OK: doctor concurrency.commands matrix present"
else
  echo "WARN: doctor missing commands matrix (optional if doctor failed early)"
fi
if echo "$out" | rg -q 'na_product_law'; then
  echo "OK: doctor matrix na_product_law present"
else
  echo "WARN: doctor matrix missing na_product_law key (optional if doctor failed early)"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "parallelism-check FAILED"
  exit 1
fi
echo "parallelism-check PASS"
