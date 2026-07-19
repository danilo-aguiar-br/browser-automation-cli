# English (neutral `en`) — human-facing suggestions only.
# Machine JSON `error.message` and tracing stay English and are not FTL keys.
# Identifiers are English; keep keys in sync with `Mensagem` in src/i18n/mensagem.rs.

usage-suggestion = Check --help and required arguments
broken-pipe-suggestion = Do not pipe stdout to a closed consumer; exit 141 is expected
unavailable-suggestion = Install Chrome/Chromium on PATH or: browser-automation-cli config set chrome_path <path>
data-suggestion = Check robots.txt or the JSON/NDJSON payload
browser-suggestion = Check the URL and whether Chrome stayed alive in this one-shot
vision-required = Pass --experimental-vision on the same invocation
robots-dual = Pass both flags together when you intentionally skip robots.txt
category-memory = Pass --category-memory (heap take/summary/close work without deep graph ops)
category-extensions = Pass --category-extensions on the same invocation
screencast-flag = Pass --experimental-screencast on the same invocation
webmcp-flag = Pass --category-webmcp on the same invocation
third-party-flag = Pass --category-third-party on the same invocation
capture-network = Pass --capture-network before run/net
capture-console = Pass --capture-console before run/console
run-fail-fast = Fix the failing step; subsequent steps were not executed
lighthouse-missing = Install lighthouse or: browser-automation-cli config set lighthouse_path <path>
locale-resolved = Resolved UI locale
locale-source = Resolution source
