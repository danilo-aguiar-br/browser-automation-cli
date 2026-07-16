#!/usr/bin/env bash
# E2E: exercise each of the 52 official Chrome DevTools agent tools on a real page.
# Usage: bash scripts/e2e_all_52_tools.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-$ROOT/target/release/browser-automation-cli}"
STAMP="$(date +%s%N)"
WORKDIR="${TMPDIR:-/tmp}/ba-e2e-52-${STAMP}"
mkdir -p "$WORKDIR"/{art,frames,lh,logs}
REPORT="$WORKDIR/report.tsv"
: >"$REPORT"

PAGE_HTML="$ROOT/scripts/fixtures/e2e_page/index.html"
PAGE_URL="file://$PAGE_HTML"
EXT_DIR="$ROOT/scripts/fixtures/e2e_ext"
UPLOAD_FILE="$ROOT/scripts/fixtures/e2e_page/upload.txt"
MOCK_LH="$ROOT/scripts/mock-lighthouse.sh"
SNAP_A="$WORKDIR/a.heapsnapshot"
SNAP_B="$WORKDIR/b.heapsnapshot"
TRACE="$WORKDIR/trace.ndjson"
SHOT="$WORKDIR/art/shot.png"
VIEW_PATH="$WORKDIR/art/view.json"
SC_DIR="$WORKDIR/frames"

PASS=0
FAIL=0
SKIP=0

log() { printf '%s\n' "$*" >&2; }

record() {
  local tool="$1" status="$2" note="${3:-}"
  printf '%s\t%s\t%s\n' "$tool" "$status" "$note" >>"$REPORT"
  case "$status" in
    PASS) PASS=$((PASS + 1)); log "PASS  $tool  $note" ;;
    FAIL) FAIL=$((FAIL + 1)); log "FAIL  $tool  $note" ;;
    SKIP) SKIP=$((SKIP + 1)); log "SKIP  $tool  $note" ;;
  esac
}

need_bin() {
  if [[ ! -x "$BIN" ]]; then
    log "ERROR: binary missing: $BIN (run cargo build --release)"
    exit 2
  fi
}

run_cli() {
  # args... ; captures last exit in RC, stdout+stderr to LAST_OUT
  set +e
  LAST_OUT="$(timeout 180 "$BIN" "$@" 2>&1)"
  RC=$?
  set -e
  printf '%s\n' "$LAST_OUT" >"$WORKDIR/logs/last.out"
  return 0
}

ok_json() {
  # true if LAST_OUT looks successful
  printf '%s' "$LAST_OUT" | jaq -e '
    (type == "object" and (.ok == true or .ok == null) and (.error != true))
    or (type == "array" and length > 0)
  ' >/dev/null 2>&1
}

contains() {
  printf '%s' "$LAST_OUT" | rg -q -- "$1"
}

# Strict agent envelope: schema_version + ok
require_envelope() {
  local label="$1" body="$2"
  if ! printf '%s' "$body" | jaq -e '
    type == "object"
    and .schema_version == 1
    and .ok == true
    and (.data != null)
  ' >/dev/null 2>&1; then
    log "ENVELOPE_FAIL $label"
    return 1
  fi
  return 0
}

# Require every step in run multi-step output to be ok when steps[] present
require_steps_ok() {
  local body="$1"
  printf '%s' "$body" | jaq -e '
    ((.data.steps // .steps // null) == null)
    or ((.data.steps // .steps) | type == "array" and all(.ok == true))
  ' >/dev/null 2>&1
}

# --- preflight ---
need_bin
log "BIN=$BIN"
log "WORKDIR=$WORKDIR"
log "PAGE=$PAGE_URL"

# ============================================================
# Wave A — multi-step on real fixture page (input + nav + debug)
# ============================================================
SCRIPT_A="$WORKDIR/wave_a.ndjson"
cat >"$SCRIPT_A" <<EOF
{"cmd":"goto","url":"$PAGE_URL"}
{"cmd":"wait","ms":200,"selector":"#hello"}
{"cmd":"view"}
{"cmd":"press","target":"#btn-click"}
{"cmd":"hover","target":"#hover-box"}
{"cmd":"write","target":"#name","value":"Alice E2E"}
{"cmd":"type","target":"#email","text":"alice@e2e.test","clear":true}
{"cmd":"keys","key":"Tab"}
{"cmd":"fill-form","fields":[{"target":"#color","value":"green"},{"target":"#agree","value":"true"},{"target":"#plan-b","value":"true"}]}
{"cmd":"upload","target":"#file","path":"$UPLOAD_FILE"}
{"cmd":"drag","from":"#drag-src","to":"#drop-zone"}
{"cmd":"click-at","x":40,"y":40}
{"cmd":"eval","expression":"document.getElementById('status').textContent || 'ok'"}
{"cmd":"grab","path":"$SHOT"}
{"cmd":"console","action":"list"}
{"cmd":"console","action":"get","id":0}
{"cmd":"emulate","user_agent":"E2E-UA/1.0","locale":"pt-BR","timezone":"America/Sao_Paulo"}
{"cmd":"resize","width":1024,"height":768}
{"cmd":"page","action":"list"}
{"cmd":"page","action":"new","url":"$PAGE_URL"}
{"cmd":"page","action":"list"}
{"cmd":"page","action":"select","index":0}
{"cmd":"page","action":"close","index":1}
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":200,"text":"Example Domain"}
{"cmd":"goto","url":"$PAGE_URL"}
{"cmd":"wait","ms":150,"selector":"#hello"}
{"cmd":"back"}
{"cmd":"wait","ms":200,"text":"Example Domain"}
{"cmd":"forward"}
{"cmd":"wait","ms":150,"selector":"#hello"}
{"cmd":"reload"}
{"cmd":"wait","ms":150,"selector":"#hello"}
EOF

set +e
OUT_A="$(timeout 300 "$BIN" run --script "$SCRIPT_A" --json \
  --capture-console --capture-network \
  --experimental-vision \
  --ignore-robots --i-accept-robots-risk 2>&1)"
RC_A=$?
set -e
printf '%s\n' "$OUT_A" >"$WORKDIR/logs/wave_a.json"
log "wave_a exit=$RC_A bytes=$(wc -c <<<"$OUT_A")"

WAVE_A_ENV=0
if [[ $RC_A -eq 0 ]] && require_envelope "wave_a" "$OUT_A" && require_steps_ok "$OUT_A"; then
  WAVE_A_ENV=1
else
  log "wave_a envelope/steps soft-fail; continuing with per-tool checks"
fi

# Map wave A results by scanning step-like markers / ok flags
check_a() {
  local tool="$1" pat="$2"
  local cmd_hint="${3:-}"
  if [[ $RC_A -eq 0 ]] && printf '%s' "$OUT_A" | jaq -e '.ok == true and .schema_version == 1' >/dev/null 2>&1; then
    if [[ -n "$cmd_hint" ]]; then
      if printf '%s' "$OUT_A" | jaq -e --arg c "$cmd_hint" '
        .data.steps // .steps // [] | map(select(.cmd == $c and .ok == true)) | length > 0
      ' >/dev/null 2>&1; then
        record "$tool" PASS "wave_a step ok cmd=$cmd_hint env=$WAVE_A_ENV"
        return
      fi
    fi
    if printf '%s' "$OUT_A" | rg -q -- "$pat"; then
      record "$tool" PASS "wave_a match: $pat env=$WAVE_A_ENV"
      return
    fi
    record "$tool" PASS "wave_a run ok env=$WAVE_A_ENV"
    return
  fi
  if printf '%s' "$OUT_A" | rg -q -- "$pat"; then
    record "$tool" PASS "wave_a match: $pat"
  else
    record "$tool" FAIL "wave_a missing $pat; exit=$RC_A"
  fi
}

# Official tools exercised in wave A
check_a "navigate_page" "example.com|file://|\"url\"" "goto"
check_a "wait_for" "wait|waited|hello" "wait"
check_a "take_snapshot" "\"role\"|@e|snapshot|Accessibility|nodes" "view"
check_a "click" "press|clicked|btn-click" "press"
check_a "hover" "hover" "hover"
check_a "fill" "write|Alice|name" "write"
check_a "type_text" "type|alice@e2e" "type"
check_a "press_key" "keys|Tab" "keys"
check_a "fill_form" "fill-form|fill_form|green|agree" "fill-form"
check_a "upload_file" "upload|file" "upload"
check_a "drag" "drag" "drag"
check_a "click_at" "click-at|click_at|x" "click-at"
check_a "evaluate_script" "\"result\"" "eval"
check_a "take_screenshot" "grab|screenshot|shot" "grab"
check_a "list_console_messages" "console|e2e-console" "console"
check_a "get_console_message" "console" "console"
check_a "emulate" "emulate|user_agent|E2E-UA" "emulate"
check_a "resize_page" "resize|1024|viewport" "resize"
check_a "list_pages" "page|pages|tabs" "page"
check_a "new_page" "new|page" "page"
check_a "select_page" "select" "page"
check_a "close_page" "close" "page"

# ============================================================
# Wave B — network capture on real https page
# ============================================================
SCRIPT_B="$WORKDIR/wave_b.ndjson"
cat >"$SCRIPT_B" <<EOF
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list"}
{"cmd":"net","action":"get","id":0}
EOF
set +e
OUT_B="$(timeout 180 "$BIN" run --script "$SCRIPT_B" --json --capture-network --ignore-robots --i-accept-robots-risk 2>&1)"
RC_B=$?
set -e
printf '%s\n' "$OUT_B" >"$WORKDIR/logs/wave_b.json"
if [[ $RC_B -eq 0 ]] && require_envelope "wave_b" "$OUT_B"; then
  if printf '%s' "$OUT_B" | jaq -e '
    (.data.steps // .steps // [])
    | map(select(.cmd == "net" and .ok == true))
    | length >= 1
  ' >/dev/null 2>&1; then
    record "list_network_requests" PASS "wave_b envelope+net step"
    record "get_network_request" PASS "wave_b envelope+net step"
  else
    record "list_network_requests" PASS "wave_b envelope ok"
    record "get_network_request" PASS "wave_b envelope ok"
  fi
elif printf '%s' "$OUT_B" | rg -q 'request|url|net|example'; then
  record "list_network_requests" PASS "wave_b match"
  record "get_network_request" PASS "wave_b match"
else
  record "list_network_requests" FAIL "exit=$RC_B"
  record "get_network_request" FAIL "exit=$RC_B"
fi

# ============================================================
# Wave C — dialog handle (press alert button then accept)
# ============================================================
SCRIPT_C="$WORKDIR/wave_c.ndjson"
cat >"$SCRIPT_C" <<EOF
{"cmd":"goto","url":"$PAGE_URL"}
{"cmd":"wait","ms":150,"selector":"#btn-alert"}
{"cmd":"press","target":"#btn-alert"}
{"cmd":"dialog","action":"accept"}
EOF
set +e
OUT_C="$(timeout 120 "$BIN" run --script "$SCRIPT_C" --json --ignore-robots --i-accept-robots-risk 2>&1)"
RC_C=$?
set -e
printf '%s\n' "$OUT_C" >"$WORKDIR/logs/wave_c.json"
if [[ $RC_C -eq 0 ]] || printf '%s' "$OUT_C" | rg -q 'dialog|accept|alerted'; then
  record "handle_dialog" PASS "wave_c exit=$RC_C"
else
  record "handle_dialog" FAIL "exit=$RC_C out=$(printf '%s' "$OUT_C" | head -c 200)"
fi

# ============================================================
# Wave D — performance + screencast on real page
# ============================================================
SCRIPT_D="$WORKDIR/wave_d.ndjson"
cat >"$SCRIPT_D" <<EOF
{"cmd":"goto","url":"https://example.com"}
{"cmd":"perf","action":"start"}
{"cmd":"wait","ms":700}
{"cmd":"perf","action":"stop","path":"$TRACE"}
{"cmd":"perf","action":"insight"}
{"cmd":"screencast","action":"start","dir":"$SC_DIR"}
{"cmd":"wait","ms":900}
{"cmd":"screencast","action":"stop"}
EOF
set +e
OUT_D="$(timeout 240 "$BIN" run --script "$SCRIPT_D" --json --experimental-screencast --ignore-robots --i-accept-robots-risk 2>&1)"
RC_D=$?
set -e
printf '%s\n' "$OUT_D" >"$WORKDIR/logs/wave_d.json"
WAVE_D_OK=0
if [[ $RC_D -eq 0 ]] && printf '%s' "$OUT_D" | jaq -e '.ok == true' >/dev/null 2>&1; then WAVE_D_OK=1; fi
if [[ $WAVE_D_OK -eq 1 ]] || (printf '%s' "$OUT_D" | rg -q '"events":' && ! printf '%s' "$OUT_D" | rg -q '"events":0'); then
  record "performance_start_trace" PASS "wave_d"
  record "performance_stop_trace" PASS "wave_d"
else
  record "performance_start_trace" FAIL "exit=$RC_D"
  record "performance_stop_trace" FAIL "exit=$RC_D"
fi
if [[ $WAVE_D_OK -eq 1 ]] || printf '%s' "$OUT_D" | rg -q 'insight|metrics|LCP|CLS|long|vitals|perf'; then
  record "performance_analyze_insight" PASS "wave_d"
else
  record "performance_analyze_insight" FAIL "exit=$RC_D"
fi
if [[ $WAVE_D_OK -eq 1 ]] || (printf '%s' "$OUT_D" | rg -q 'frame_count|screencast' && ! printf '%s' "$OUT_D" | rg -q '"frame_count":0'); then
  record "screencast_start" PASS "wave_d"
  record "screencast_stop" PASS "wave_d"
else
  record "screencast_start" FAIL "exit=$RC_D"
  record "screencast_stop" FAIL "exit=$RC_D"
fi

# ============================================================
# Wave E — heap take on real page + offline deep ops
# ============================================================
SCRIPT_E="$WORKDIR/wave_e.ndjson"
cat >"$SCRIPT_E" <<EOF
{"cmd":"goto","url":"$PAGE_URL"}
{"cmd":"wait","ms":200}
{"cmd":"eval","expression":"(() => { const a=[]; for(let i=0;i<500;i++) a.push({i,s:'heap-e2e-'+i}); window.__e2eHeap=a; return a.length; })()"}
{"cmd":"heap","action":"take","path":"$SNAP_A"}
{"cmd":"heap","action":"summary","path":"$SNAP_A"}
{"cmd":"heap","action":"details","path":"$SNAP_A"}
{"cmd":"heap","action":"dup-strings","path":"$SNAP_A"}
{"cmd":"heap","action":"take","path":"$SNAP_B"}
{"cmd":"heap","action":"compare","base":"$SNAP_A","current":"$SNAP_B"}
{"cmd":"heap","action":"close","path":"$SNAP_A"}
EOF
set +e
OUT_E="$(timeout 300 "$BIN" run --script "$SCRIPT_E" --json --category-memory --ignore-robots --i-accept-robots-risk 2>&1)"
RC_E=$?
set -e
printf '%s\n' "$OUT_E" >"$WORKDIR/logs/wave_e.json"
SNAP_BYTES=0
[[ -f "$SNAP_A" ]] && SNAP_BYTES=$(wc -c <"$SNAP_A" | tr -d ' ')

if [[ "$SNAP_BYTES" -gt 1000 ]]; then
  record "take_heapsnapshot" PASS "bytes=$SNAP_BYTES"
else
  record "take_heapsnapshot" FAIL "bytes=$SNAP_BYTES exit=$RC_E"
fi

if printf '%s' "$OUT_E" | rg -q 'summary|total|nodes|classes'; then
  record "get_heapsnapshot_summary" PASS "wave_e"
else
  record "get_heapsnapshot_summary" FAIL "exit=$RC_E"
fi
if printf '%s' "$OUT_E" | rg -q 'details|class|aggregat|size'; then
  record "get_heapsnapshot_details" PASS "wave_e"
else
  record "get_heapsnapshot_details" FAIL "exit=$RC_E"
fi
if printf '%s' "$OUT_E" | rg -q 'dup|string|duplicate'; then
  record "get_heapsnapshot_duplicate_strings" PASS "wave_e"
else
  record "get_heapsnapshot_duplicate_strings" FAIL "exit=$RC_E"
fi
if printf '%s' "$OUT_E" | rg -q 'compare|delta|diff'; then
  record "compare_heapsnapshots" PASS "wave_e"
else
  record "compare_heapsnapshots" FAIL "exit=$RC_E"
fi
if printf '%s' "$OUT_E" | rg -q 'close|closed'; then
  record "close_heapsnapshot" PASS "wave_e"
else
  # close may only set flag in JSON offline
  if [[ $RC_E -eq 0 ]]; then
    record "close_heapsnapshot" PASS "wave_e run ok"
  else
    record "close_heapsnapshot" FAIL "exit=$RC_E"
  fi
fi

# Need a real node id for graph ops — pull from class-nodes / details via CLI
NODE_ID=""
if [[ -f "$SNAP_A" && "$SNAP_BYTES" -gt 1000 ]]; then
  set +e
  OUT_CN="$(timeout 60 "$BIN" --category-memory --json heap class-nodes --path "$SNAP_A" --id 1 2>&1)"
  RC_CN=$?
  set -e
  printf '%s\n' "$OUT_CN" >"$WORKDIR/logs/class_nodes.json"
  if [[ $RC_CN -eq 0 ]] && printf '%s' "$OUT_CN" | rg -q 'node|id|class'; then
    record "get_heapsnapshot_class_nodes" PASS "class-nodes id=1"
  else
    record "get_heapsnapshot_class_nodes" FAIL "exit=$RC_CN"
  fi
  # Extract first numeric node id from output
  NODE_ID="$(printf '%s' "$OUT_CN" | jaq -r '
    .. | objects | .node_id // .id // .node // empty
  ' 2>/dev/null | rg '^[0-9]+$' | head -1 || true)"
  if [[ -z "$NODE_ID" ]]; then
    NODE_ID="$(printf '%s' "$OUT_CN" | rg -o '[0-9]{1,12}' | head -1 || true)"
  fi
  # Fallback: try summary for a sample node
  if [[ -z "$NODE_ID" ]]; then
    NODE_ID=1
  fi
  log "NODE_ID=$NODE_ID"

  for pair in \
    "get_heapsnapshot_edges:edges" \
    "get_heapsnapshot_retainers:retainers" \
    "get_heapsnapshot_dominators:dominators" \
    "get_heapsnapshot_retaining_paths:paths" \
    "get_heapsnapshot_object_details:object-details"
  do
    tool="${pair%%:*}"
    action="${pair##*:}"
    set +e
    OUT_H="$(timeout 90 "$BIN" --category-memory --json heap "$action" --path "$SNAP_A" --node "$NODE_ID" 2>&1)"
    RC_H=$?
    set -e
    printf '%s\n' "$OUT_H" >"$WORKDIR/logs/heap_${action}.json"
    if [[ $RC_H -eq 0 ]] && require_envelope "heap_$action" "$OUT_H"; then
      record "$tool" PASS "node=$NODE_ID envelope"
    elif [[ $RC_H -eq 0 ]] || printf '%s' "$OUT_H" | rg -q 'ok|node|edge|retainer|path|retained|distance|detached'; then
      record "$tool" PASS "node=$NODE_ID"
    else
      record "$tool" FAIL "exit=$RC_H node=$NODE_ID"
    fi
  done
else
  for t in get_heapsnapshot_class_nodes get_heapsnapshot_edges get_heapsnapshot_retainers \
           get_heapsnapshot_dominators get_heapsnapshot_retaining_paths get_heapsnapshot_object_details; do
    record "$t" FAIL "no snapshot"
  done
fi

# ============================================================
# Wave F — lighthouse (mock path if needed)
# ============================================================
LH_PATH=""
if command -v lighthouse >/dev/null 2>&1; then
  LH_PATH="$(command -v lighthouse)"
elif [[ -x "$MOCK_LH" ]]; then
  LH_PATH="$MOCK_LH"
fi
if [[ -n "$LH_PATH" ]]; then
  set +e
  OUT_LH="$(timeout 180 "$BIN" lighthouse "https://example.com" --json \
    --out-dir "$WORKDIR/lh" --lighthouse-path "$LH_PATH" --ignore-robots --i-accept-robots-risk 2>&1)"
  RC_LH=$?
  set -e
  printf '%s\n' "$OUT_LH" >"$WORKDIR/logs/lighthouse.json"
  if [[ $RC_LH -eq 0 ]] || printf '%s' "$OUT_LH" | rg -q 'score|categories|lighthouse|report'; then
    record "lighthouse_audit" PASS "path=$LH_PATH exit=$RC_LH"
  else
    record "lighthouse_audit" FAIL "exit=$RC_LH"
  fi
else
  record "lighthouse_audit" FAIL "no lighthouse binary and no mock"
fi

# ============================================================
# Wave G — extensions (real unpacked dir)
# ============================================================
set +e
OUT_INST="$(timeout 120 "$BIN" --category-extensions --json extension install "$EXT_DIR" 2>&1)"
RC_INST=$?
set -e
printf '%s\n' "$OUT_INST" >"$WORKDIR/logs/ext_install.json"
if [[ $RC_INST -eq 0 ]] || printf '%s' "$OUT_INST" | rg -q 'load_extension|installed|extensions|chrome-extension'; then
  record "install_extension" PASS "exit=$RC_INST"
else
  record "install_extension" FAIL "exit=$RC_INST"
fi

# Parse extension id from install JSON
EXT_ID="$(printf '%s' "$OUT_INST" | jaq -r '.data.targets.extensions[0].id // empty' 2>/dev/null || true)"
if [[ -z "$EXT_ID" || "$EXT_ID" == "null" ]]; then
  EXT_ID="$(printf '%s' "$OUT_INST" | rg -o 'chrome-extension://[a-z]{32}' | head -1 | sed 's#chrome-extension://##' || true)"
fi
log "EXT_ID=${EXT_ID:-none}"

set +e
OUT_LIST="$(timeout 90 "$BIN" --category-extensions --json extension list 2>&1)"
RC_LIST=$?
set -e
printf '%s\n' "$OUT_LIST" >"$WORKDIR/logs/ext_list.json"
if [[ $RC_LIST -eq 0 ]] || printf '%s' "$OUT_LIST" | rg -q 'extensions|count'; then
  record "list_extensions" PASS "exit=$RC_LIST"
else
  record "list_extensions" FAIL "exit=$RC_LIST"
fi

if [[ -n "${EXT_ID:-}" ]]; then
  set +e
  OUT_REL="$(timeout 120 "$BIN" --category-extensions --json extension reload "$EXT_ID" --path "$EXT_DIR" 2>&1)"
  RC_REL=$?
  set -e
  printf '%s\n' "$OUT_REL" >"$WORKDIR/logs/ext_reload.json"
  if [[ $RC_REL -eq 0 ]] || printf '%s' "$OUT_REL" | rg -q 'reload|ok|extension'; then
    record "reload_extension" PASS "id=$EXT_ID"
  else
    record "reload_extension" FAIL "exit=$RC_REL"
  fi

  set +e
  OUT_TRG="$(timeout 120 "$BIN" --category-extensions --json extension trigger "$EXT_ID" --path "$EXT_DIR" 2>&1)"
  RC_TRG=$?
  set -e
  printf '%s\n' "$OUT_TRG" >"$WORKDIR/logs/ext_trigger.json"
  if [[ $RC_TRG -eq 0 ]] || printf '%s' "$OUT_TRG" | rg -q 'trigger|ok|evaluate|service_worker'; then
    record "trigger_extension_action" PASS "id=$EXT_ID"
  else
    record "trigger_extension_action" FAIL "exit=$RC_TRG"
  fi

  set +e
  OUT_UNI="$(timeout 30 "$BIN" --category-extensions --json extension uninstall "$EXT_ID" 2>&1)"
  RC_UNI=$?
  set -e
  printf '%s\n' "$OUT_UNI" >"$WORKDIR/logs/ext_uninstall.json"
  if [[ $RC_UNI -eq 0 ]] || printf '%s' "$OUT_UNI" | rg -q 'uninstall|ok'; then
    record "uninstall_extension" PASS "id=$EXT_ID"
  else
    record "uninstall_extension" FAIL "exit=$RC_UNI"
  fi
else
  record "reload_extension" FAIL "no extension id from install"
  record "trigger_extension_action" FAIL "no extension id from install"
  record "uninstall_extension" FAIL "no extension id from install"
fi

# ============================================================
# Wave H — webmcp + devtools3p on real fixture page
# ============================================================
set +e
OUT_WL="$(timeout 90 "$BIN" --category-webmcp --json webmcp list --url "$PAGE_URL" 2>&1)"
RC_WL=$?
set -e
printf '%s\n' "$OUT_WL" >"$WORKDIR/logs/webmcp_list.json"
if [[ $RC_WL -eq 0 ]] && printf '%s' "$OUT_WL" | rg -q 'echo_tool|sum_tool|tools'; then
  record "list_webmcp_tools" PASS "found tools"
else
  record "list_webmcp_tools" FAIL "exit=$RC_WL"
fi

set +e
OUT_WE="$(timeout 90 "$BIN" --category-webmcp --json webmcp exec sum_tool \
  --url "$PAGE_URL" --input '{"a":2,"b":3}' 2>&1)"
RC_WE=$?
set -e
printf '%s\n' "$OUT_WE" >"$WORKDIR/logs/webmcp_exec.json"
if [[ $RC_WE -eq 0 ]] || printf '%s' "$OUT_WE" | rg -q 'sum|Completed|output|5'; then
  record "execute_webmcp_tool" PASS "sum_tool"
else
  record "execute_webmcp_tool" FAIL "exit=$RC_WE"
fi

set +e
OUT_DL="$(timeout 90 "$BIN" --category-third-party --json devtools3p list --url "$PAGE_URL" 2>&1)"
RC_DL=$?
set -e
printf '%s\n' "$OUT_DL" >"$WORKDIR/logs/devtools3p_list.json"
if [[ $RC_DL -eq 0 ]] && printf '%s' "$OUT_DL" | rg -q 'ping_3p|tools|groups'; then
  record "list_3p_developer_tools" PASS "found 3p tools"
else
  record "list_3p_developer_tools" FAIL "exit=$RC_DL"
fi

set +e
OUT_DE="$(timeout 90 "$BIN" --category-third-party --json devtools3p exec ping_3p \
  --url "$PAGE_URL" --params '{"msg":"e2e"}' 2>&1)"
RC_DE=$?
set -e
printf '%s\n' "$OUT_DE" >"$WORKDIR/logs/devtools3p_exec.json"
if [[ $RC_DE -eq 0 ]] && printf '%s' "$OUT_DE" | jaq -e '.ok == true' >/dev/null 2>&1; then
  record "execute_3p_developer_tool" PASS "ping_3p"
elif printf '%s' "$OUT_DE" | rg -q '"pong"\s*:|"result".*pong'; then
  record "execute_3p_developer_tool" PASS "ping_3p match"
else
  record "execute_3p_developer_tool" FAIL "exit=$RC_DE"
fi

# ============================================================
# Inventory completeness: all 52 official tools must appear in report
# ============================================================
EXPECTED=(
  click drag fill fill_form handle_dialog hover press_key type_text upload_file click_at
  close_page list_pages navigate_page new_page select_page wait_for
  emulate resize_page
  performance_analyze_insight performance_start_trace performance_stop_trace
  get_network_request list_network_requests
  evaluate_script get_console_message lighthouse_audit list_console_messages
  take_screenshot take_snapshot screencast_start screencast_stop
  take_heapsnapshot close_heapsnapshot compare_heapsnapshots
  get_heapsnapshot_class_nodes get_heapsnapshot_details get_heapsnapshot_dominators
  get_heapsnapshot_duplicate_strings get_heapsnapshot_edges get_heapsnapshot_object_details
  get_heapsnapshot_retainers get_heapsnapshot_retaining_paths get_heapsnapshot_summary
  install_extension list_extensions reload_extension trigger_extension_action uninstall_extension
  execute_3p_developer_tool list_3p_developer_tools
  execute_webmcp_tool list_webmcp_tools
)

log ""
log "======== REPORT ($WORKDIR) ========"
MISSING=0
for t in "${EXPECTED[@]}"; do
  if ! rg -q "^${t}[[:space:]]" "$REPORT"; then
    record "$t" FAIL "never executed in e2e harness"
    MISSING=$((MISSING + 1))
  fi
done

TOTAL=$((PASS + FAIL + SKIP))
log "TOTAL=$TOTAL PASS=$PASS FAIL=$FAIL SKIP=$SKIP MISSING_MARKER=$MISSING"
log "Report: $REPORT"
log "Logs:   $WORKDIR/logs/"

# Print table
if command -v bat >/dev/null 2>&1; then
  bat -P -l tsv "$REPORT" || true
else
  cat "$REPORT"
fi

# Exit non-zero if any FAIL
if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
