[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (notas do mantenedor)

- Este projeto entrega uma CLI one-shot estável
- O roadmap é intencionalmente curto

## Curto prazo (qualidade local)

- Manter gates `scripts/*-check.sh` verdes em cada passagem de auditoria (incl. `scripts/residual-check.sh` / `scripts/residual-stress.sh` para residual-zero de disco)
- Residual-zero processo + disco é lei de produto desde a v0.1.5 (RES-01…12 fechados: GC Singleton em BORN, dual scavenge em FINALIZE, doctor `residual_disk` + JSON `residual`)
- Inventário vivo de agentes: 63 nomes via `commands --json` (inclui `locale` e `man`)
- Settings de produto: só flags + XDG `config` (sem variáveis de ambiente de produto)
- Crescer cobertura unitária de helpers puros (filter, JSON, residual ledger)
- Opcional: fatiar famílias grandes de handlers em `commands_prd` quando um domínio novo entrar

## Explicitamente fora de escopo

- Daemon / serviço de browser de longa duração
- OpenTelemetry remoto / dashboards SaaS
- Embedding de servidor MCP
- GitHub Actions in-repo / matriz cargo-dist de release

## Profiling (sob demanda)

```bash
./scripts/profile-cdp.sh
# or: cargo flamegraph --bin browser-automation-cli -- goto about:blank
```

- Artefatos de captura não são commitados
- Use-os localmente para justificar micro-opts
