[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Multiplataforma — browser-automation-cli


- Pare de reescrever automação de browser para cada SO host
- Ciclo de vida: BORN EXECUTE FINALIZE DIE


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
- A ordem de resolução nunca lê variáveis de ambiente de produto, porque a lei do produto é só flags mais XDG
- O passo 1 é o XDG `chrome_path` (`config set chrome_path /caminho/absoluto`) quando o arquivo é executável
- O passo 2 é o cache de browsers do produto sob XDG data (`browsers/`)
- O passo 3 roda só no Windows: `HKLM` depois `HKCU` `SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{chrome.exe|msedge.exe|brave.exe}` (descoberta via registro do SO com `windows-sys`, não config de produto)
- O passo 4 são os nomes no `$PATH` `google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`, `chrome`, `microsoft-edge`, `msedge`, `brave-browser` e similares
- O passo 5 são os layouts absolutos conhecidos por SO listados abaixo
- O passo 6 são os caches locais de Puppeteer e Playwright em `~/.cache/`
- Sobrescreva a cascata com `browser-automation-cli config set chrome_path /path/to/chrome`
- Desfaça a sobrescrita com `browser-automation-cli config unset chrome_path`
- `browser-automation-cli doctor --offline --quick --json` reporta `path`, `sandbox`, `executable`, `version` (smoke `--version`), `windows_job_object` e `host_environment`

### Paths conhecidos Linux
- `/usr/bin/google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`
- `/opt/google/chrome/chrome`, `/opt/google/chrome/google-chrome`
- `/usr/bin/microsoft-edge`, `/opt/microsoft/msedge/msedge`
- Snap: `/snap/bin/chromium` (emite warn de sandbox — prefira APT/RPM)
- Exports Flatpak: `/var/lib/flatpak/exports/bin/com.google.Chrome`, `org.chromium.Chromium`, usuário `~/.local/share/flatpak/exports/bin/…`

### Paths conhecidos macOS
- `/Applications/Google Chrome.app/…`, Beta, Canary
- `/Applications/Chromium.app/…`, `Microsoft Edge.app`, `Brave Browser.app`
- `~/Applications/Google Chrome.app/…` (install por usuário)

### Paths conhecidos Windows
- Registro App Paths para `chrome.exe` / `msedge.exe` / `brave.exe` (antes do walk de `$PATH`)
- `%ProgramFiles%`, `%ProgramFiles(x86)%` e `%LOCALAPPDATA%` são unidos a `Google\Chrome\Application\chrome.exe`
- As mesmas raízes são unidas a `Google\Chrome Beta\…` e `Google\Chrome SxS\…` (Canary)
- As mesmas raízes são unidas a `Microsoft\Edge\Application\msedge.exe` e `BraveSoftware\Brave-Browser\Application\brave.exe`
- Fallback hardcoded `C:\Program Files\…` só como último recurso quando faltam as variáveis de path do SO
- Boot de console: code page UTF-8 65001 + `ENABLE_VIRTUAL_TERMINAL_PROCESSING` para ANSI
- Árvores residuais do Chrome: Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`)

### Sandboxes Snap / Flatpak
- Detectados por prefixo de path (`/snap/`, `/var/lib/flatpak/`, `~/.var/app/`) e `$SNAP` / `$FLATPAK_ID`
- O status do doctor vira warn quando o sandbox restringe
- Prefira pacotes do sistema
- CDP + user-data-dir temporário quebram com frequência sob confinamento


## Notas Linux
- Binários comuns incluem `chromium-browser`, `chromium` e `google-chrome`
- Rode `doctor` após o install do pacote para confirmar descoberta
- Sobrescreva a descoberta com `config set chrome_path /path/to/chrome` quando o PATH estiver confuso
- `browser_mode auto` é headless, a menos que o host seja Linux com `Xvfb` no PATH e sem `--no-xvfb`, como diz [Anti-Detecção Entre Plataformas](#anti-detecção-entre-plataformas)
- Em Alpine ou outros hosts musl, faça cross-compile ou build nativo para o target musl
- Forneça um binário real de Chrome ou Chromium
- A CLI não embute browser
- Containers acrescentam ao Chrome `--no-sandbox` e `--disable-dev-shm-usage` quando root ou marcadores docker/podman/k8s estão presentes
- Higiene residual de disco (lei v0.1.5 ainda corrente na 0.2.0): BORN + FINALIZE scavenge Chromium tmp Singleton-only owned sob o temp do processo (comumente `/tmp/org.chromium.Chromium.*` e `/tmp/.org.chromium.Chromium.*`)
- Age floor do GC Singleton stale é 60s
- Só dirs same-uid Singleton-only (ou vazios) sem holder vivo em `/proc` são apagados
- Markers CLI usam prefixo `browser-automation-cli-chrome-*` sob o temp do processo
- Prefixos temp de Chrome Flatpak do host nunca são apagados pelo GC residual do produto
- Inspecione com `doctor --offline --quick --json` → topo `residual` e check `residual_disk`

### Launch headed sob Wayland e o Xvfb privado
- Um launch headed no Linux desenha num display Xvfb privado quando o binário `Xvfb` está no PATH e `--no-xvfb` está ausente
- `Xvfb` é pacote do host que a CLI nunca instala, e o `doctor` o reporta com a dica de install da distribuição
- Sem `Xvfb` o launch avisa, continua headed no display atual, e `display_backend` informa `host`
- Com o display privado de pé, `display_backend` informa `xvfb`
- `display_backend` segue o display que o launch realmente usou desde a 0.2.0, então uma execução cujo Xvfb falhou informa `host`
- Uma execução headed com extensão também sobe o display privado desde a 0.2.0
- Numa sessão Wayland o Chromium escolhe Wayland a partir de `XDG_SESSION_TYPE` quando nenhum `--ozone-platform` é passado
- Remover só `WAYLAND_DISPLAY` não impede isso, porque o Chromium então acha `$XDG_RUNTIME_DIR/wayland-0` sozinho
- Medido no Fedora 44 com Chromium 152, isso deixava a janela no compositor real enquanto o Xvfb não desenhava nada
- Por isso a CLI acrescenta `--ozone-platform=x11` quando o display privado subiu, e também remove `WAYLAND_DISPLAY` do ambiente do Chrome
- O pino só entra quando o Xvfb realmente subiu, porque forçar X11 sem servidor X falha com `Missing X server or $DISPLAY`
- Uma plataforma já presente nos argumentos do Chrome é mantida
- `--no-xvfb` não sobe display privado, não acrescenta pino, e deixa o Chrome usar o display atual, Wayland inclusive
- `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY` e `XDG_RUNTIME_DIR` são fatos do host que o Chromium lê, nunca configuração de produto
- Execuções headed simultâneas ganham cada uma o próprio display, buscado a partir de `:99` por 32 números
- Uma execução só é dona de um display quando `/tmp/.X{n}-lock` nomeia o pid do próprio Xvfb
- Um servidor que perde a corrida por um número sai, e o launch tenta de novo no próximo número livre
- O lock e o socket só são removidos no encerramento quando o lock nomeia o Xvfb desta execução
- Um lock deixado por processo morto é reaproveitado, porque uma CLI morta com `SIGKILL` mata o próprio Xvfb do mesmo jeito
- O Xvfb privado para com `SIGTERM` e uma carência de 2 segundos antes do `SIGKILL`, então o servidor remove o próprio lock e o socket
- Um servidor que não sobe em 10 segundos encerra a tentativa
- O Xvfb privado sobe com `-nolisten tcp` e grava a própria saída no dispositivo nulo
- O display privado exige um `MIT-MAGIC-COOKIE-1` guardado num arquivo com modo 0600 no diretório de runtime do usuário, com o diretório temporário como reserva
- O servidor recebe esse arquivo por `-auth`, e o Chrome o recebe por `XAUTHORITY`
- O nome do arquivo carrega o pid de quem o criou, então o próximo launch remove um cookie deixado por uma CLI morta com `SIGKILL`
- O arquivo é removido no encerramento depois que o servidor que o lê já saiu
- A mensagem `virtual_display` do `doctor` nomeia o valor para o qual `auto` resolve neste host desde a 0.2.0
- macOS e Windows nunca sobem Xvfb

### Pipe do DevTools por plataforma
- Um Chrome lançado pelo próprio produto recebe DevTools por `--remote-debugging-pipe` e não abre porta TCP
- A CLI fala com o Chrome por uma ponte WebSocket em loopback, num caminho com os 122 bits aleatórios de um UUID v4
- A ponte repassa exatamente um cliente válido, responde `403` a qualquer outro caminho e não serve `/json/version`
- A ponte dá 2 segundos a um cliente para concluir o handshake WebSocket
- A ponte limita cada mensagem de DevTools a 256 MiB nas duas direções
- A ponte espera no máximo 2 segundos pelas threads do pipe no encerramento
- Os drenadores de stdout e stderr do Chrome e do Lightpanda recebem o mesmo limite de 2 segundos no encerramento
- No Linux e no macOS o Chrome lê o pipe nos descritores 3 e 4
- No Linux os dois pipes nascem com close-on-exec de forma atômica por `pipe2`
- No macOS os pipes nascem por `pipe` e o flag close-on-exec é aplicado logo depois, o que deixa uma janela estreita em que um fork em outra thread herda as pontas
- No Windows o Chrome recebe dois handles herdáveis em `--remote-debugging-io-pipes`, e os handles que a CLI guarda são marcados como não herdáveis
- Um host sem descritores POSIX nem handles Windows recusa o pipe com erro
- No Unix um lançamento do Chrome com falha mata o grupo de processos inteiro do Chrome, e no Windows esse kill de grupo não faz nada
- Em todo sistema o encerramento de um lançamento com falha roda numa thread bloqueante por `kill_off_the_runtime` em `src/native/cdp/chrome/spawn.rs`, então `--timeout` ainda encerra o comando com exit 124 em vez de 69
- Linux é o único host onde o pipe foi medido
- macOS NÃO foi validado ao vivo para o pipe
- Windows NÃO foi compilado nem testado para o pipe
- Sessões KDE e Sway NÃO foram validadas ao vivo para o pipe nem para o pino X11
- O motor Lightpanda e o launch legado atrás de `chrome_legacy_oxide_launch` mantêm o transporte anterior


## Notas macOS
- Instale Google Chrome pelo canal oficial
- Prefira path completo do binário via XDG `chrome_path` só quando a descoberta por PATH falhar
- Apple Silicon e Intel usam descoberta de Chrome do sistema
- Conceda permissões de acessibilidade ou tela só se usar debug headed fora de agentes
- Universal binary / notarização são ops de release (não exigidos para build a partir do source)


## Notas Windows
- Use PowerShell ou cmd com quoting explícito em torno de URLs
- Prefira `--json` para evitar parsing de prosa dependente de locale
- Mantenha argv UTF-8 limpo
- Evite mojibake ao pipear por code pages legadas
- Quote paths com espaços: `"C:\Users\me\out.png"`
- Prefira `grab --path` com path completo em vez de depender do cwd
- Helpers de processo Windows ficam atrás de `cfg(windows)` e não mudam o contrato JSON
- Basenames de path reservados no Windows (`CON`, `NUL`, `COM1`, …) são rejeitados em todos os hosts para scripts portáveis
- Higiene residual de processo usa Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) para árvores Chrome morrerem com o processo da CLI
- Campos de relatório residual de disco (`residual` / `residual_disk`) permanecem disponíveis via doctor para diagnósticos de marker e temp


## Residual de Permissão de Arquivo no Windows (declarado, não corrigido)
- Cinco arquivos nascem com modo POSIX restritivo no Unix e herdam a ACL do diretório pai no Windows
- `mitm_local/ca.rs` grava a CHAVE PRIVADA da autoridade certificadora do MITM com `0o600`
- `xdg/config_write.rs` grava o `config.toml` com `0o600`, e ele guarda `encryption_key`, `openrouter_api_key` e `proxy_password`
- `mitm_local/util.rs` grava os corpos capturados de requisição e resposta com `0o600`
- `xdg/paths.rs` cria o diretório de estado XDG com `0o700`
- `native/stealth/seed_cache.rs` grava o seed de identidade do stealth com `0o600`
- Nada QUEBRA no Windows: as escritas funcionam e nenhum caminho entra em pânico
- O que difere é a POSTURA de segurança, e só nesses cinco caminhos
- No Windows a proteção efetiva é a que o diretório pai conceder
- Guarde o produto num diretório cuja ACL você controla quando o host for compartilhado
- Isto é residual declarado, não descuido: uma implementação de ACL não testada seria resposta pior que a medição honesta da lacuna
- Fechar exige um host Windows para verificar, que a medição acima não teve


## Anti-Detecção Entre Plataformas
- `stealth` vem ligado por padrão e se comporta igual nos três sistemas
- `stealth_profile auto` resolve pelo host: Windows dá chrome-win, macOS dá chrome-mac
- Todo outro host resolve para chrome-linux, incluindo container e WSL
- Perfil estrangeiro é reportado, nunca bloqueado, em `profile_contradicts_host`
- O override headless troca só o token de produto `HeadlessChrome` pelo token `Chrome`
- Essa troca preserva a plataforma real do host e nunca inventa outra
- `browser_mode auto` resolve para headed dentro de um display virtual privado no Linux com Xvfb no PATH e sem `--no-xvfb`
- Em qualquer outro caso, macOS e Windows inclusive, `browser_mode auto` resolve para headless
- macOS sempre tem Quartz e Windows sempre tem DWM, então nenhum usa Xvfb
- `doctor` reporta o check `xvfb` como info em todo host fora do Linux
- Sem Xvfb no PATH o launch headed cai no display atual
- A dica de install vem do `/etc/os-release` e a CLI nunca instala nada
- `DISPLAY` e `WAYLAND_DISPLAY` são fatos do host lidos somente no Linux
- Ler os dois não é configuração de produto, que segue flags mais XDG
- O check `virtual_display` expõe `host_has_display`, `browser_mode_auto_resolves` e `private_display_supported`
- Flags Vulkan e ANGLE sob `--enable-unsafe-webgpu` são emitidas somente no Linux
- O comportamento de proxy não varia por plataforma em nenhum ponto do caminho
- `proxy_url` alimenta o `--proxy-server` do Chrome e o cliente HTTP compartilhado
- `HTTP_PROXY`, `HTTPS_PROXY` e `ALL_PROXY` nunca são herdados em host nenhum
- Sem `proxy_url` o cliente chama `no_proxy` e desliga a descoberta de proxy do sistema
- `cdp_proxy_bypass_loopback` acrescenta loopback à lista de bypass dos dois lados
- Credenciais vêm somente de `proxy_username` e `proxy_password` sob XDG
- Com stealth ligado, o Chrome recebe `--disable-quic` em toda plataforma
- Os valores de janela e frame do HTTP/2 são idênticos em toda plataforma
- `http2_enabled false` derruba o cliente para HTTP/1.1 e reporta `http2_profile: disabled`
- `input_profile human` sintetiza eventos somente por chamadas do domínio CDP `Input`
- Nenhuma API de input do sistema é usada, então a cinemática é idêntica em toda parte
- macOS não pede permissão de acessibilidade porque nenhuma API nativa de input é tocada
- Keycodes viajam em `windows_virtual_key_code` e `native_virtual_key_code` em todo host
- O modificador Cmd do macOS é escolha de bitmask do chamador, não padrão do produto
- `stealth_seed` fixa a identidade entre processos e seu cache é 0600 no Unix
- Windows não recebe esse aperto de permissão de arquivo no cache de seed
- O Chrome recebe `--password-store=basic` e `--use-mock-keychain` por padrão em todo host
- Ambas são suprimidas juntas quando o launch opta pelo keychain real, o que nenhum caminho de produto faz hoje
- Nomes de chave e defaults dessa família vivem em [CONFIGURATION.pt-BR.md](CONFIGURATION.pt-BR.md)


## Containers
- Instale Chrome ou Chromium na imagem antes de testes de runtime
- Forneça shared memory suficiente para o Chrome (`/dev/shm` ou equivalente)
- Mantenha expectativas de cleanup one-shot sob reinícios de orquestração
- Não assuma arquivo de settings de produto montado do host fora do XDG
- Use flags e mounts XDG para levar settings ao container
- Forma de exemplo: empacote `browser-automation-cli` mais Chromium, depois chame `doctor --json`
- Opcional: servidor Redis ao testar `cache_backend redis`
- Opcional: binário Lighthouse ou mock para auditorias
- Probe de host: `doctor --json` → `host_environment.container` / `.wsl` / `.termux`


## Probe de ambiente do host
- O módulo `platform::HostEnvironment` detecta WSL, container, Termux, Flatpak e Snap
- Usado por diagnósticos do doctor e flags de launch do Chrome (container → flags de sandbox/dev-shm)
- Marcadores do host são só observabilidade, e nunca settings de produto


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
- Material de CA do MITM fica sob XDG data (`mitm/ca`)
- Capturas do MITM ficam sob XDG state (`mitm/`)
- Journals de workflow ficam sob XDG state (`workflows`)
- Chave de cifragem é definida com `config set encryption_key <value>`
- Descubra chaves vivas de config com `config list-keys --json`, e não fixe contagem como “16 chaves”
- Settings de produto são só flags e `config` XDG — nunca variáveis de ambiente de produto
- Settings de produto usam só flags e a CLI XDG (`config path|init|show|set|unset|get|list-keys`)
- Desfaça qualquer chave gravada com `config unset <key>`
- Idioma das sugestões humanas: só `--lang` ou XDG `lang`
- Inventário completo de comandos (71 nomes de agente) e padrões de agente: [docs/HOW_TO_USE.pt-BR.md](HOW_TO_USE.pt-BR.md)
- Cache Redis: `cache_backend redis` + `cache_redis_url redis://…` apenas (`rediss://` fail-closed)
- Logging de produto: `--verbose` / `--debug` / `-q` ou XDG `log_level`
- Cor: `config set color`
- Path do Chrome: `config set chrome_path`


## Superfície de agente v0.2.0 (compacta)
- Família anti-detecção está viva: `stealth`, `stealth_profile`, `stealth_seed`, `browser_mode`, `input_profile`
- A mesma família soma chaves de proxy, de `SETTINGS` HTTP/2 e as dez chaves `input_*`
- Flags globais anti-detecção: `--no-stealth`, `--stealth-profile`, `--stealth-seed`, `--input-profile`, `--input-seed`
- Mais da mesma família: `--proxy`, `--proxy-bypass`, `--headed`, `--no-xvfb`, `--warmup`, `--warmup-url`
- Cada uma dessas flags é parseada de forma idêntica em Linux, macOS e Windows
- Envelopes de scrape revelam `stealth`, `profile_contradicts_host`, `http2_profile` e `tls_impersonation`
- O comportamento por plataforma da família vive em [Anti-Detecção Entre Plataformas](#anti-detecção-entre-plataformas)
- Um Chrome lançado pelo próprio produto usa o pipe do DevTools em toda plataforma, como diz [Pipe do DevTools por plataforma](#pipe-do-devtools-por-plataforma)
- Booleano `dialog_settled` após accept/dismiss real de diálogo (GAP-054)
- O isolamento multi-aba de diálogo passa por `Page::session_id` / `dialog_map_key`
- `dialog_settle_ms` só via XDG `config set` (flags + XDG, nunca env de produto)
- Chave pública `wait_timeout_ms` nos passos wait de run (GAP-053)
- Scrape `format`/`formats` em run sem monstro HTML (GAP-057)
- Select nativo `pick`/`select-option` despacha `input` e depois `change`, `via: native_select` (GAP-055)
- Flags universais de envelope: `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes` nos 71 comandos, idênticas em toda plataforma
- `agent_ops` aparece no envelope de sucesso somente quando uma dessas flags roda
- `unresolved_paths` nomeia caminho que nenhuma linha carregava
- `agent_ops` é omitido quando não há o que reportar: flag que rodou e resolveu limpo deixa a forma do envelope intacta, em toda plataforma
- `--select`/`--filter`/`--limit`/`--sort` NÃO são globais: são flags por comando em scrape, crawl, map, search, batch-scrape e verbos `info` de mídia
- Chaves XDG: 217 documentadas em [CONFIGURATION.pt-BR.md](CONFIGURATION.pt-BR.md)
- Descubra ao vivo com `config list-keys --json`
- Encode do `grab`: só png|jpeg|webp
- AVIF removido (breaking)
- Inventário 71 inclui `submit` + `storage` + `image`+`video`+`audio`+`record`
- A lei residual-zero de disco da 0.1.5 continua corrente
- GAP-021 parcial (fixtures unit LHR, e2e lighthouse mock SKIP)
- GAP-022 residual ~53 dups aceitos
- GAP-023/024 divergências intencionais


## Inventário completo de agente (71)
- Descubra ao vivo: `browser-automation-cli commands --json`
```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract feed fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write sitemap storage submit text type
upload version view wait webmcp workflow write
```
- `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`
- A contagem de subcomandos clap de produto é 69 (71 nomes de agente − 2 só-run)


## Performance por Target
- Desktop e servidores Linux são o alvo primário de otimização
- Cold start permanece limitado pelo Chrome em todo SO quando usa a engine browser
- Prefira `--engine http` em comandos estilo scrape quando um browser completo for desnecessário
- Validação local do mantenedor usa `cargo build --release`, Chrome do host e scripts e2e


## Agentes Validados por Plataforma
- Modo de integração em toda parte: subprocesso one-shot com `--json`
- Linux: Claude Code, Codex, Gemini CLI, Cursor, shell local, agentes de editor
- macOS: agentes shell locais e integrações de editor
- Windows: integrações shell e editor com quoting explícito
- Listas expandidas de agentes em [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md) são compatíveis via subprocesso com validação local por cargo e scripts e2e
