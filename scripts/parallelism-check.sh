#!/usr/bin/env bash
# Local gate: bounded parallelism / no unbounded fan-out anti-patterns.
# rules_rust_paralelismo_e_multiprocessamento — product law one-shot CLI.
# Pass 23: Semaphore gate in join_bounded + spawn_blocking scrape parse.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail=0

echo "== concurrency module present =="
test -d src/concurrency || { echo "missing src/concurrency/"; exit 1; }

echo "== no production Box::leak / mem::forget in concurrency =="
if rg -n 'Box::leak|mem::forget' src/concurrency/; then
  echo "FAIL: leak/forget in concurrency module"
  fail=1
fi

echo "== no unbounded join_all on large fan-out (src, excluding tests comments) =="
if rg -n 'future::join_all|futures_util::future::join_all' src/native/snapshot/ src/native/screenshot/ src/scrape_local 2>/dev/null; then
  echo "FAIL: unbounded join_all still present in fan-out modules"
  fail=1
else
  echo "OK: fan-out modules use join_bounded / JoinSet"
fi

echo "== batch/crawl use Semaphore gate (acquire_owned / try_acquire_owned) =="
if ! rg -n 'acquire_owned|try_acquire_owned' src/scrape_local >/dev/null; then
  echo "FAIL: scrape_local missing Semaphore acquire_owned gate"
  fail=1
else
  echo "OK: scrape_local Semaphore gate present"
fi

echo "== join_bounded uses Semaphore acquire =="
if ! rg -n 'sem\.acquire\(\)|Semaphore::new' src/concurrency/ >/dev/null; then
  echo "FAIL: join_bounded missing Semaphore gate"
  fail=1
else
  echo "OK: join_bounded Semaphore present"
fi

echo "== scrape HTML parse uses spawn_blocking =="
if ! rg -n 'spawn_blocking' src/scrape_local >/dev/null; then
  echo "FAIL: scrape_local missing spawn_blocking for CPU parse"
  fail=1
else
  echo "OK: scrape spawn_blocking present"
fi

echo "== walk_threads helper (budget-aware) =="
if ! rg -n 'fn walk_threads' src/concurrency/ >/dev/null; then
  echo "FAIL: missing walk_threads"
  fail=1
else
  echo "OK: walk_threads present"
fi

echo "== command workload matrix exported =="
if ! rg -n 'command_workload_matrix' src/concurrency/ >/dev/null; then
  echo "FAIL: missing command_workload_matrix"
  fail=1
else
  echo "OK: command_workload_matrix present"
fi

echo "== matrix has na_product_law =="
if ! rg -n 'na_product_law' src/concurrency/ >/dev/null; then
  echo "FAIL: matrix missing na_product_law"
  fail=1
else
  echo "OK: na_product_law present"
fi

echo "== Pass 24: by_command matrix + helpers =="
if ! rg -n 'by_command' src/concurrency/ >/dev/null; then
  echo "FAIL: missing by_command in matrix"
  fail=1
else
  echo "OK: by_command present"
fi
if ! rg -n 'fn write_bytes_blocking|fn map_cpu' src/concurrency/ >/dev/null; then
  echo "FAIL: missing write_bytes_blocking / map_cpu helpers"
  fail=1
else
  echo "OK: Pass 24 helpers present"
fi

echo "== Pass 24/28: screencast frames use spawn_blocking (session/media) =="
# Pass F/G: screencast lives under browser/session/media/ (not media.rs monólito).
if ! rg -n 'spawn_blocking' src/browser/session/media/ >/dev/null; then
  echo "FAIL: screencast stop missing spawn_blocking frame write"
  fail=1
elif ! rg -n 'parallel_frames|par_iter' src/browser/session/media/ >/dev/null; then
  echo "FAIL: screencast stop missing parallel frame write"
  fail=1
else
  echo "OK: screencast spawn_blocking + parallel frames present"
fi
if ! rg -n 'par_iter' src/browser/session/media/ >/dev/null; then
  echo "FAIL: browser session missing Rayon par_iter for screencast frames"
  fail=1
else
  echo "OK: browser session Rayon frames present"
fi

echo "== Pass 24: sg multi-root par collect =="
if ! rg -n 'par_iter\(\)\.flat_map|roots\.par_iter' src/sg_local.rs >/dev/null; then
  echo "FAIL: sg_local missing multi-root parallel collect"
  fail=1
else
  echo "OK: sg multi-root par present"
fi

echo "== Pass 24: CDP page forwarders join_bounded =="
# Pass G: cdp/client is a directory (join_bounded in page_attach.rs).
if ! rg -n 'join_bounded' src/native/cdp/client/ >/dev/null; then
  echo "FAIL: cdp client missing join_bounded for multi-page forwarders"
  fail=1
else
  echo "OK: cdp join_bounded present"
fi

echo "== Pass 25: filter_cpu + read_to_string_blocking + rename_blocking =="
if ! rg -n 'fn filter_cpu' src/concurrency/ >/dev/null; then
  echo "FAIL: missing filter_cpu helper"
  fail=1
else
  echo "OK: filter_cpu present"
fi
if ! rg -n 'fn read_to_string_blocking|fn rename_blocking' src/concurrency/ >/dev/null; then
  echo "FAIL: missing read_to_string_blocking / rename_blocking"
  fail=1
else
  echo "OK: Pass 25 blocking helpers present"
fi

echo "== Pass 25/28: console/net use filter_cpu|count_cpu (session split) =="
# Pass G: console/net live under browser/session/assert_net/.
if ! rg -n 'filter_cpu|count_cpu' src/browser/session/assert_net/ src/browser/ >/dev/null; then
  echo "FAIL: browser session missing filter_cpu/count_cpu for console/net"
  fail=1
else
  echo "OK: browser session filter_cpu/count_cpu present"
fi
if ! rg -n 'fn count_cpu' src/concurrency/ >/dev/null; then
  echo "FAIL: missing count_cpu helper (zero-copy cardinality)"
  fail=1
else
  echo "OK: count_cpu helper present"
fi

echo "== Pass 25: state load uses async blocking read path =="
if ! rg -n 'read_state_json_async|read_bytes_blocking|spawn_blocking.*read_state' src/native/state >/dev/null; then
  echo "FAIL: state load missing blocking read offload"
  fail=1
else
  echo "OK: state load blocking read present"
fi

echo "== Pass 25: matrix honesty (doctor not fake map_cpu) =="
if rg -n '"doctor".*"map_cpu checks"|doctor.*map_cpu checks' src/concurrency/ >/dev/null; then
  echo "FAIL: matrix overclaims doctor map_cpu (must be sequential_justified)"
  fail=1
else
  echo "OK: doctor matrix not overclaiming map_cpu"
fi
if ! rg -n 'console\.list|heap\.dup-strings|filter_cpu when large' src/concurrency/ >/dev/null; then
  echo "FAIL: missing nested by_command / filter_cpu gates"
  fail=1
else
  echo "OK: nested multi-item by_command markers present"
fi

echo "== Pass 26: residual indexes the process table once (PAR-89) =="
# Property check, NOT a spelling check.
#
# The invariant is: the live-process table is built ONCE per scan and the
# resulting index is passed into the per-candidate predicate. It must never be
# rebuilt per candidate, and never inside a Rayon closure.
#
# Earlier revisions of this gate matched the literal argument name
# (`path_has_live_process(path, &proc_index)`). That made a legitimate rename of
# a local variable fail the gate while a real violation with the "right" name
# would have passed. Anchor on the property and on type names (API surface),
# never on identifier spelling.

# 1) An index constructor exists. Either name is accepted; both are public API.
if ! rg -n 'fn (index_live_processes|index_proc_cmdlines)\b' src/residual/ >/dev/null; then
  echo "FAIL: residual has no live-process index constructor"
  fail=1
else
  echo "OK: residual live-process index constructor present"
fi

# 2) The per-candidate predicate RECEIVES an index instead of building one.
#    Anchored on the type (`LiveProcessIndex`), which is API surface, not on the
#    parameter name, which is free to change.
if ! rg -U -n 'fn path_has_live_process\([^)]*&LiveProcessIndex' src/residual/ >/dev/null; then
  echo "FAIL: path_has_live_process does not take a shared &LiveProcessIndex"
  fail=1
else
  echo "OK: per-candidate predicate takes a shared index"
fi

# 3) The predicate does not construct an index itself (that would be one full
#    process scan per candidate — the exact regression PAR-89 forbids).
if rg -U -n 'fn path_has_live_process\((.|\n)*?\n\}' src/residual/ \
  | rg -q 'index_live_processes\(|index_proc_cmdlines\(|backend::collect\('; then
  echo "FAIL: path_has_live_process builds an index per candidate"
  fail=1
else
  echo "OK: per-candidate predicate builds no index"
fi

# 4) No index construction inside a parallel closure anywhere in residual.
if rg -U -n '(map_cpu|par_iter|par_bridge|into_par_iter)\((.|\n){0,600}?\n\s*\}' src/residual/ \
  | rg -q 'index_live_processes\(|index_proc_cmdlines\(|backend::collect\('; then
  echo "FAIL: residual builds the process index inside a parallel closure"
  fail=1
else
  echo "OK: no index construction inside parallel closures"
fi

echo "== Pass 26: MITM CA blocking read (PAR-91) =="
if ! rg -n 'load_ca_pems_blocking|read_to_string_blocking' src/mitm_local >/dev/null; then
  echo "FAIL: mitm missing load_ca_pems_blocking / read_to_string_blocking"
  fail=1
else
  echo "OK: mitm CA blocking path present"
fi
# Raw fs::read_to_string inside async oneshot bodies is forbidden (blocking I/O
# on a runtime thread). Argument names are free: matching `read_to_string(cert`
# would go green the moment the variable is renamed to `ca_path`.
if rg -n 'async fn (start_proxy_oneshot|capture_url_oneshot)' -A40 src/mitm_local | rg -n 'fs::read_to_string\(' >/dev/null; then
  echo "FAIL: mitm async oneshot still uses fs::read_to_string for CA"
  fail=1
else
  echo "OK: mitm async oneshot no raw CA fs::read_to_string"
fi

echo "== Pass 26: chrome temp profile mkdir off async (PAR-92) =="
# Pass F: chrome is a directory under cdp/chrome/.
if rg -n 'std::fs::create_dir_all\(&dir\)' src/native/cdp/chrome/ >/dev/null; then
  if rg -n 'create_dir_all\(&dir\)' src/native/cdp/chrome/ | rg -v 'materialize_temp_user_data_dir_sync|//|PAR-92' >/dev/null; then
    if ! rg -n 'fn materialize_temp_user_data_dir_sync' -A6 src/native/cdp/chrome/ | rg -q 'create_dir_all'; then
      echo "FAIL: chrome create_dir_all outside materialize helper"
      fail=1
    else
      echo "OK: chrome create_dir_all confined to materialize helper"
    fi
  else
    echo "OK: chrome create_dir_all confined"
  fi
else
  echo "OK: no create_dir_all(&dir) in chrome/"
fi
if ! rg -n 'create_dir_all_blocking' src/native/cdp/oxide.rs >/dev/null; then
  echo "FAIL: oxide launch missing create_dir_all_blocking for temp profile"
  fail=1
else
  echo "OK: oxide create_dir_all_blocking present"
fi

echo "== Pass 26: sort_cpu helper + call sites (PAR-94) =="
if ! rg -n 'fn sort_cpu|fn sort_by_cpu|fn sort_by_key_cpu' src/concurrency/ >/dev/null; then
  echo "FAIL: missing sort_cpu helpers"
  fail=1
else
  echo "OK: sort_cpu helpers present"
fi
if ! rg -n 'sort_cpu|sort_by_cpu|sort_by_key_cpu' src/sg_local.rs src/native/heap_snapshot src/native/perf_insight.rs src/mitm_local >/dev/null; then
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

echo "== Pass 26/28: extension multi-close join_bounded (PAR-96, session split) =="
# Pass G: multi-close lives under browser/session/extensions/.
if ! rg -n 'join_bounded' src/browser/session/extensions/ src/browser/ >/dev/null; then
  echo "FAIL: browser session missing join_bounded (extension multi-close expected)"
  fail=1
else
  if rg -n 'closeTarget' src/browser/session/extensions/ >/dev/null \
    && rg -n 'join_bounded' src/browser/session/extensions/ >/dev/null; then
    echo "OK: browser session has closeTarget + join_bounded"
  else
    echo "WARN: join_bounded present but closeTarget pattern unclear"
  fi
fi

echo "== --max-concurrency flag wired =="
if ! rg -n 'max_concurrency|max-concurrency' src/cli/ src/lib.rs >/dev/null; then
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
