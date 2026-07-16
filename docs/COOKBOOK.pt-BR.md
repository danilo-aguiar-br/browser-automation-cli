[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Receitas práticas com comandos prontos para trabalho one-shot de browser.

## Nota de Latência
- O launch do Chrome domina o cold start
- Prefira um script `run` a vários launches separados quando os passos compartilham estado

## Referência de Defaults
- Timeout global default é `0` e significa sem orçamento wall-clock até ser setado
- Step timeout default é `0` e herda o timeout global
- Headless é o default salvo `--headed`
- JSON fica off até `--json` ou `BROWSER_AUTOMATION_CLI_JSON`

## Como Diagnosticar Saúde do Install
```bash
browser-automation-cli doctor --offline --quick --json
```

## Como Abrir uma Página e Fazer Snapshot
```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
```

## Como Clicar e Preencher em Um Processo
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
{"cmd":"write","target":"input","value":"hello"}
{"cmd":"press","target":"button"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```

## Como Capturar Screenshot Full-page
```bash
browser-automation-cli --timeout 60 --json grab /tmp/page.png --full-page
```

## Como Listar Requests de Rede
```bash
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/nav.jsonl
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
```
- Flags de captura devem aplicar ao processo que navega

## Como Avaliar JavaScript
```bash
browser-automation-cli --json eval 'document.title'
```

## Como Emular Viewport Mobile
```bash
browser-automation-cli --json emulate --device "iPhone 12"
browser-automation-cli --json resize --width 390 --height 844
```

## Como Rodar Auditoria Lighthouse
```bash
browser-automation-cli --timeout 180 --json lighthouse https://example.com
```

## Como Inspecionar Heap Snapshots
```bash
browser-automation-cli --category-memory --json heap summary --path snap.heapsnapshot
```

## Como Gerar Completions de Shell
```bash
browser-automation-cli completions bash
```

## Como Descobrir Schemas de Comando
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
```
