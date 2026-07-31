[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (maintainer notes)

- This project ships as a stable one-shot CLI
- The roadmap is intentionally short

## Near term (local quality)

- **v0.1.6 residual DoD achieved** (measured 2026-07-31): GAP-054 dialog settle + multi-tab (`dialog_settled` boolean; XDG `dialog_settle_ms`), GAP-055 native select, GAP-057 scrape format in run, GAP-053 `wait_timeout_ms`, residual-zero disk law from 0.1.5 still current
- Keep `scripts/*-check.sh` gates green on every audit pass (incl. `scripts/residual-check.sh` / `scripts/residual-stress.sh` for residual-zero disk)
- Optional confidence suite: re-run `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`, lighthouse unit LHR fixtures after large refactors
- Live agent inventory: **65** names via `commands --json` (includes `submit`, `storage`, `locale`, `man`)
- Product settings: flags + XDG `config` only (no product environment variables); discover keys via `config list-keys --json`
- **`grab` encode:** png|jpeg|webp only; AVIF removed (breaking, keep residual noted)
- Grow unit coverage for pure helpers (filter, JSON, residual ledger, `dialog_map_key`, `scores_from_lhr`)
- Optional: split large `commands` handler families when a new domain lands

## Intentional residuals (do not claim closed as full parity)

- **GAP-021 partial:** lighthouse parser confidence is unit fixtures (minimal + chrome-captured LHR); e2e mock remains **SKIP** — never claim full e2e lighthouse parser PASS
- **GAP-022 residual dups:** ~53 multi-version dependency duplicates measured; cheap prune exhausted; residual accepted
- **GAP-023 / GAP-024:** PRD wishlist flags/commands remain intentional divergences in `parity_intentional_divergences.json` — not full PRD parity
- **AVIF encode:** removed from `grab` (webp remains); document as intentional breaking residual of 0.1.6

## Full agent inventory (65)

Discover live: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Note: `pick` and `select-option` are multi-step inventory names used in `run` scripts; clap product subcommand count is 63.

## Explicitly out of scope

- Daemon / long-lived browser service
- Remote OpenTelemetry / SaaS dashboards
- MCP server embedding
- In-repo remote release orchestration / cargo-dist multi-arch matrix

## Profiling (on demand)

```bash
./scripts/profile-cdp.sh
# or: cargo flamegraph --bin browser-automation-cli -- goto about:blank
```

- Capture artefacts are not committed
- Use them locally to justify micro-opts
