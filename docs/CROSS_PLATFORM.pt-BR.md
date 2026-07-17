[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Cross Platform — browser-automation-cli

> Pare de reescrever automação de browser para cada host OS. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## A Dor Que Você Já Conhece
- Tooling de browser costuma assumir um layout de path de um único OS
- Agentes locais falham quando a descoberta do Chrome é host-específica e não documentada
- Quoting de shell e separadores de path quebram wrappers frágeis
- Env vars de produto se multiplicam entre shells e imagens de CI sem fonte única de verdade


## Matriz de Suporte

| Plataforma | Arquitetura | Status | Notas |
|------------|-------------|--------|-------|
| Linux | x86_64 | primary | paths comuns de Chromium e Google Chrome |
| Linux | aarch64 | supported | exige Chrome ou Chromium local |
| macOS | x86_64 | supported | descoberta do Chrome do sistema |
| macOS | aarch64 | supported | descoberta do Chrome do sistema |
| Windows | x86_64 | supported | helpers de processo específicos de Windows |
| Windows | aarch64 | compile-time | build a partir do source quando o target Rust estiver disponível |

- docs.rs documenta `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` e `aarch64-unknown-linux-musl`
- musl e Alpine são possibilidades de target em tempo de compilação (`aarch64-unknown-linux-musl` e similares)
- Este repositório não distribui artefatos pré-compilados musl ou multi-arch por default
- Valide o binário no seu host com `doctor --json` após o install


## Notas Linux
- Binários comuns incluem `chromium-browser`, `chromium` e `google-chrome`
- Rode `doctor` após install de pacote para confirmar descoberta
- Headless é o default para execuções locais de agentes
- Em Alpine ou outros hosts musl, faça cross-compile ou build nativo para o target musl
- Forneça um binário real de Chrome ou Chromium; a CLI não embute browser


## Notas macOS
- Instale Google Chrome pelo canal oficial
- Prefira path completo do binário só quando a descoberta por PATH falhar
- Apple Silicon e Intel usam descoberta do Chrome do sistema
- Conceda permissões de acessibilidade ou tela só se usar headed debugging fora de agentes


## Notas Windows
- Use PowerShell ou cmd com quoting explícito em URLs
- Prefira `--json` para evitar parsing de prosa dependente de locale
- Mantenha argv UTF-8 limpo; evite mojibake ao pipar por code pages legadas
- Quote paths com espaços: `"C:\Users\me\out.png"`
- Prefira `grab --path` com path completo em vez de depender do cwd
- Helpers de processo Windows vivem sob `cfg(windows)` e não mudam o contrato JSON


## Containers
- Instale Chrome ou Chromium na imagem antes dos testes de runtime
- Forneça shared memory suficiente para o Chrome (`/dev/shm` ou equivalente)
- Mantenha expectativas de cleanup one-shot sob restarts de orquestração
- Não assuma arquivo de env de produto montado no host; use flags e mounts XDG se necessário
- Forma de exemplo: empacote `browser-automation-cli` mais Chromium e chame `doctor --json`


## Suporte de Shell
- bash, zsh, fish e PowerShell podem spawnar o binário
- Completions são geradas por `completions <shell>`
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
- Chave de cifragem é definida com `config set encryption_key <value>`, não via env de produto
- Não existem settings de produto `BROWSER_AUTOMATION_CLI_*`
- Convenções de SO apenas: `RUST_LOG`, `NO_COLOR` (e `PATH` do host para descoberta do Chrome)


## Performance por Target
- Desktop e servidores Linux são o alvo primário de otimização
- Cold start permanece limitado pelo Chrome em todo OS quando usa a engine browser
- Prefira `--engine http` em comandos estilo scrape quando um browser completo for desnecessário
- Validação local do mantenedor usa `cargo build --release` e Chrome do host; não se afirma matriz CI hospedada multi-OS


## Agentes Validados por Plataforma
- Modo de integração em toda parte: subprocesso one-shot com `--json`
- Linux: Claude Code, Codex, Gemini CLI, Cursor, shell local, agentes de editor
- macOS: agentes shell locais e integrações de editor
- Windows: integrações shell e editor com quoting explícito
- Listas expandidas de agentes em [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md) são compatíveis via subprocesso; não são resultados de matriz CI por plataforma
