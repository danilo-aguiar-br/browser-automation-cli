[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Multiplataforma — browser-automation-cli

> Pare de reescrever automação de browser para cada SO host. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## A Dor Que Você Já Conhece
- Tooling de browser frequentemente assume um único layout de paths por SO
- Agentes locais falham quando a descoberta do Chrome é host-específica e não documentada
- Quoting de shell e separadores de path quebram wrappers frágeis
- Settings espalhados fora de flags e XDG `config` se multiplicam entre shells sem uma única fonte de verdade


## Matriz de Suporte

| Plataforma | Arch | Status | Notas |
|------------|------|--------|-------|
| Linux | x86_64 | primário | Paths comuns de Chromium e Google Chrome |
| Linux | aarch64 | suportado | exige Chrome ou Chromium local |
| macOS | x86_64 | suportado | descoberta de Chrome do sistema |
| macOS | aarch64 | suportado | descoberta de Chrome do sistema |
| Windows | x86_64 | suportado | helpers de processo específicos de Windows |
| Windows | aarch64 | compile-time | compile do source quando o target Rust estiver disponível |

- docs.rs documenta `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` e `aarch64-unknown-linux-musl`
- musl e Alpine são possibilidades de target em compile-time (`aarch64-unknown-linux-musl` e similares)
- Este repositório não envia artefatos prebuilt musl ou multi-arch por default
- Valide o binário no seu host com `doctor --json` após o install


## Cascata de Descoberta de Browser

Ordem de resolução (sem variáveis de ambiente de produto — lei do produto é **flags + XDG**):

1. XDG `chrome_path` (`config set chrome_path /caminho/absoluto`) se o arquivo for executável
2. Cache de browsers do produto sob XDG data (`browsers/`)
3. **Somente Windows:** `HKLM` depois `HKCU` `SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{chrome.exe|msedge.exe|brave.exe}` (descoberta via registro OS com `windows-sys`, não config de produto)
4. Nomes no `$PATH`: `google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`, `chrome`, `microsoft-edge`, `msedge`, `brave-browser`, …
5. Layouts absolutos conhecidos por SO (abaixo)
6. Caches locais Puppeteer / Playwright em `~/.cache/`

Override: `browser-automation-cli config set chrome_path /path/to/chrome`  
Diagnóstico: `doctor --offline --quick --json` reporta `path`, `sandbox`, `executable`, `version` (smoke `--version`), `windows_job_object` e `host_environment`.

### Paths conhecidos Linux
- `/usr/bin/google-chrome`, variantes beta/unstable, `chromium`, `chromium-browser`
- `/opt/google/chrome/chrome`, Edge em `/opt/microsoft/msedge/msedge`
- Snap: `/snap/bin/chromium` (emite **warn** de sandbox — prefira APT/RPM)
- Flatpak: `/var/lib/flatpak/exports/bin/com.google.Chrome` e user `~/.local/share/flatpak/…`

### Paths conhecidos macOS
- `/Applications/Google Chrome.app/…`, Beta, Canary, Chromium, Edge, Brave
- `~/Applications/Google Chrome.app/…` (install por usuário)

### Paths conhecidos Windows
- Registro **App Paths** (`chrome.exe` / `msedge.exe` / `brave.exe`) antes do walk de `$PATH`
- `%ProgramFiles%` / `%ProgramFiles(x86)%` / `%LOCALAPPDATA%` + Chrome / Beta / Canary / Edge / Brave
- Fallback hardcoded `C:\Program Files\…` só se as env vars faltarem
- Boot de console: code page UTF-8 **65001** + VT ANSI; Job Objects para residual Chrome
- Basenames reservados Windows (`CON`, `NUL`, `COM1`, …) rejeitados em **todos** os hosts

### Sandboxes Snap / Flatpak
- Detectados por prefixo de path e `$SNAP` / `$FLATPAK_ID`
- Doctor marca **warn** quando o sandbox restringe automação CDP


## Notas Linux
- Binários comuns incluem `chromium-browser`, `chromium` e `google-chrome`
- Rode `doctor` após o install do pacote para confirmar descoberta
- Sobrescreva a descoberta com `config set chrome_path /path/to/chrome` quando o PATH estiver confuso
- Headless é default para runs locais de agente
- Em Alpine ou outros hosts musl, faça cross-compile ou build nativo para o target musl
- Forneça um binário real de Chrome ou Chromium; a CLI não embute browser
- Containers adicionam `--no-sandbox` e `--disable-dev-shm-usage` quando root ou marcadores docker/podman/k8s estão presentes
- Higiene residual de disco (lei v0.1.5 ainda corrente na 0.1.7): BORN + FINALIZE scavenge Chromium tmp Singleton-only owned sob o temp do processo (comumente `/tmp/org.chromium.Chromium.*` e `/tmp/.org.chromium.Chromium.*`)
- Age floor do GC Singleton stale é **60s**; só dirs same-uid Singleton-only (ou vazios) sem holder vivo em `/proc` são apagados
- Markers CLI usam prefixo `browser-automation-cli-chrome-*` sob o temp do processo
- Prefixos temp de Chrome Flatpak do host **nunca** são apagados pelo GC residual do produto
- Inspecione com `doctor --offline --quick --json` → topo `residual` e check `residual_disk`


## Notas macOS
- Instale Google Chrome pelo canal oficial
- Prefira path completo do binário via XDG `chrome_path` só quando a descoberta por PATH falhar
- Apple Silicon e Intel usam descoberta de Chrome do sistema
- Conceda permissões de acessibilidade ou tela só se usar debug headed fora de agentes
- Universal binary / notarização são **ops de release** (não exigidos para build a partir do source)


## Notas Windows
- Use PowerShell ou cmd com quoting explícito em torno de URLs
- Prefira `--json` para evitar parsing de prosa dependente de locale
- Mantenha argv UTF-8 limpo; evite mojibake ao pipear por code pages legadas
- Quote paths com espaços: `"C:\Users\me\out.png"`
- Prefira `grab --path` com path completo em vez de depender do cwd
- Helpers de processo Windows ficam atrás de `cfg(windows)` e não mudam o contrato JSON
- Higiene residual de **processo** usa Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) para árvores Chrome morrerem com o processo da CLI
- Campos de relatório residual de disco (`residual` / `residual_disk`) permanecem disponíveis via doctor para diagnósticos de marker e temp


## Containers
- Instale Chrome ou Chromium na imagem antes de testes de runtime
- Forneça shared memory suficiente para o Chrome (`/dev/shm` ou equivalente)
- Mantenha expectativas de cleanup one-shot sob reinícios de orquestração
- Não assuma arquivo de settings de produto montado do host fora do XDG; use flags e mounts XDG se necessário
- Forma de exemplo: empacote `browser-automation-cli` mais Chromium, depois chame `doctor --json`
- Opcional: servidor Redis ao testar `cache_backend redis`; binário Lighthouse ou mock para auditorias
- Probe de host: `doctor --json` → `host_environment.container` / `.wsl` / `.ci` / `.termux`


## Probe de ambiente do host
- Módulo `platform::HostEnvironment` detecta WSL, container, CI, Termux, Flatpak, Snap
- Usado por doctor e flags de launch do Chrome
- Chaves de env de CI são só observabilidade — nunca settings de produto


## Suporte de Shell
- bash, zsh, fish e PowerShell podem spawnar o binário
- Completions são geradas via `completions <shell>`
- Shells de completion suportados: `bash`, `zsh`, `fish`, `elvish`, `powershell`
```bash
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
browser-automation-cli completions powershell
```


## Paths de Arquivo e XDG
- Resolva paths vivos com `browser-automation-cli config path --json`
- Inicialize o layout com `browser-automation-cli config init`
- Arquivo de config é o `config.toml` XDG no dir de config do produto
- `config path --json` inclui campos como `config_dir`, `data_dir`, `state_dir`, `mitm_ca_dir`, `mitm_capture_dir`, `workflow_dir`
- Campos relacionados também incluem `config_file`, `cache_dir`, `browsers_dir`, `sessions_dir`, `home_dir` e `layout`
- Artefatos seguem `--artifacts-dir` quando fornecido (flag ou chave de config)
- Cache, state, sessions e journals de workflow ficam sob árvores XDG locais do usuário
- Material de CA do MITM fica sob XDG data (`mitm/ca`); capturas sob XDG state (`mitm/`)
- Journals de workflow ficam sob XDG state (`workflows`)
- Chave de cifragem é definida com `config set encryption_key <value>`
- Descubra chaves vivas de config com `config list-keys --json` (inclui `dialog_settle_ms`; não fixe contagem como “16 chaves”)
- Settings de produto usam só flags e CLI XDG (`config path|init|show|set|get|list-keys`) — nunca variáveis de ambiente de produto
- Idioma das sugestões humanas: só `--lang` ou XDG `lang`
- Inventário completo de comandos (**69**) e padrões de agente: [docs/HOW_TO_USE.pt-BR.md](HOW_TO_USE.pt-BR.md)
- Cache Redis: `cache_backend redis` + `cache_redis_url redis://…` apenas (`rediss://` fail-closed)
- Logging de produto: `--verbose` / `--debug` / `-q` ou XDG `log_level`
- Cor: `config set color`; path do Chrome: `config set chrome_path`

## Superfície de agente v0.1.7 (compacta)

- Booleano **`dialog_settled`** após accept/dismiss real de diálogo (GAP-054); isolamento multi-aba via `Page::session_id` / `dialog_map_key`
- **`dialog_settle_ms`** só via XDG `config set` (flags + XDG; nunca env de produto)
- Chave pública **`wait_timeout_ms`** nos passos wait de run (GAP-053)
- Scrape `format`/`formats` em run sem monstro HTML (GAP-057)
- Select nativo `pick`/`select-option` despacha `input` e depois `change`, `via: native_select` (GAP-055)
- **Flags universais de envelope:** `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes` nos 69 comandos, idênticas em toda plataforma
- **`agent_ops`** aparece no envelope de sucesso somente quando uma dessas flags roda; `unresolved_paths` nomeia caminho que nenhuma linha carregava
- **`agent_ops` é omitido quando não há o que reportar:** flag que rodou e resolveu limpo deixa a forma do envelope intacta, em toda plataforma
- **`--select`/`--filter`/`--limit`/`--sort` NÃO são globais:** são flags por comando em scrape, crawl, map, search, batch-scrape e verbos `info` de mídia
- **Chaves XDG:** 176 documentadas em [CONFIGURATION.pt-BR.md](CONFIGURATION.pt-BR.md); descubra ao vivo com `config list-keys --json`
- **Encode do `grab`:** só png|jpeg|webp; AVIF removido (breaking)
- Inventário **69** inclui `submit` + `storage` + `image`+`video`+`audio`+`record`; lei residual-zero de disco da 0.1.5 ainda corrente
- GAP-021 parcial (fixtures unit LHR; e2e lighthouse mock SKIP); GAP-022 residual ~53 dups aceitos; GAP-023/024 intencionais em `parity_intentional_divergences.json`

## Inventário completo de agente (69)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é **67** (69 nomes de agente − 2 só-run).

## Performance por Target
- Desktop e servidores Linux são o alvo primário de otimização
- Cold start permanece limitado pelo Chrome em todo OS quando usa a engine browser
- Prefira `--engine http` em comandos estilo scrape quando um browser completo for desnecessário
- Validação local do mantenedor usa `cargo build --release`, Chrome do host e scripts e2e


## Agentes Validados por Plataforma
- Modo de integração em toda parte: subprocesso one-shot com `--json`
- Linux: Claude Code, Codex, Gemini CLI, Cursor, shell local, agentes de editor
- macOS: agentes shell locais e integrações de editor
- Windows: integrações shell e editor com quoting explícito
- Listas expandidas de agentes em [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md) são compatíveis via subprocesso; validação local com cargo e scripts e2e
