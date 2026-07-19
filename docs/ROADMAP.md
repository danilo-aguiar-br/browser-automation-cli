[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (maintainer notes)

- This project ships as a stable one-shot CLI
- The roadmap is intentionally short

## Near term (local quality)

- Keep `scripts/*-check.sh` gates green on every audit pass (incl. `scripts/residual-check.sh` / `scripts/residual-stress.sh` for residual-zero disk)
- Residual-zero process + disk is product law as of v0.1.5 (RES-01…12 closed: BORN Singleton GC, FINALIZE dual scavenge, doctor `residual_disk` + JSON `residual`)
- Live agent inventory: 63 names via `commands --json` (includes `locale` and `man`)
- Product settings: flags + XDG `config` only (no product environment variables)
- Grow unit coverage for pure helpers (filter, JSON, residual ledger)
- Optional: split large `commands_prd` handler families when a new domain lands

## Explicitly out of scope

- Daemon / long-lived browser service
- Remote OpenTelemetry / SaaS dashboards
- MCP server embedding
- In-repo GitHub Actions / cargo-dist release matrix

## Profiling (on demand)

```bash
./scripts/profile-cdp.sh
# or: cargo flamegraph --bin browser-automation-cli -- goto about:blank
```

- Capture artefacts are not committed
- Use them locally to justify micro-opts
