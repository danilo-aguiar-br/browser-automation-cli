[English](HOW_TO_USE.md) | [Português Brasileiro](HOW_TO_USE.pt-BR.md)

# Como Usar — browser-automation-cli

> Instale uma vez, lance o Chrome uma vez, termine a tarefa do agente e saia limpo.

## Pré-requisitos
- Rust 1.88.0+ se for buildar do source
- Chrome ou Chromium no PATH
- ffmpeg opcional para export de screencast
- binário lighthouse opcional para auditorias

## Primeiro Comando em 60 Segundos
```bash
cargo install --path . --locked
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
```
- Doctor prova que a descoberta do Chrome funciona
- Goto navega em um processo one-shot fresco
- View imprime snapshot de acessibilidade com refs `@eN`

## Comandos Core
- Navegue com `goto`, `back`, `forward`, `reload`
- Faça snapshot com `view`
- Clique com `press @eN` ou seletores CSS
- Preencha com `write` e multi-campo `fill-form`
- Capture páginas com `grab out.png --full-page`
- Extraia body text com `scrape https://example.com`

## Multi-passo em Um Processo
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```
- Use `run` sempre que refs precisem sobreviver entre passos
- Launches de processo separado não compartilham refs `@eN`

## Padrões Avançados
- Capture network: `--capture-network net list --json`
- Capture console: `--capture-console console list --json`
- Emule device e network com `emulate`
- Heap profundo exige `--category-memory`
- Tools de extensão exigem `--category-extensions`
- Cliques por coordenada exigem `--experimental-vision`

## Configuração
- Prefira flags para chamadas pontuais de agente
- Prefira variáveis de ambiente para defaults de CI
- Mantenha a dual-flag de robots explícita ao contornar

## Subcomandos Não Cobertos Acima
- Use `browser-automation-cli commands --json` para o inventário vivo
- Use `browser-automation-cli schema --cmd <name> --json` para shapes de input
- Use `browser-automation-cli help <cmd>` para detalhe de flags

## Integração Com Agentes de IA
- Sempre peça `--json`
- Parseie apenas envelopes de stdout
- Trate stderr como diagnóstico
- Veja [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md) e [INTEGRATIONS.pt-BR.md](../INTEGRATIONS.pt-BR.md)
