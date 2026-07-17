[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Cross Platform — browser-automation-cli

> Pare de reescrever automação de browser para cada host OS.

## A Dor Que Você Já Conhece
- Tooling de browser costuma assumir um layout de path de um único OS
- Agentes locais falham quando a descoberta do Chrome é host-específica e não documentada
- Quoting de shell e separadores de path quebram wrappers frágeis

## Matriz de Suporte

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x86_64 | primary | paths comuns de Chromium e Google Chrome |
| Linux aarch64 | supported | exige Chrome ou Chromium local |
| macOS x86_64 | supported | descoberta do Chrome do sistema |
| macOS aarch64 | supported | descoberta do Chrome do sistema |
| Windows x86_64 | supported | helpers de processo específicos de Windows |

## Notas Linux
- Binários comuns incluem `chromium-browser`, `chromium` e `google-chrome`
- Rode `doctor` após install de pacote para confirmar descoberta
- Headless é o default para execuções locais de agentes

## Notas macOS
- Instale Google Chrome pelo canal oficial
- Prefira path completo do binário só quando a descoberta por PATH falhar

## Notas Windows
- Use PowerShell ou cmd com quoting explícito em URLs
- Prefira `--json` para evitar parsing de prosa dependente de locale

## Containers
- Instale Chrome ou Chromium na imagem antes dos testes de runtime
- Forneça shared memory suficiente para o Chrome quando o runtime exigir
- Mantenha expectativas de cleanup one-shot sob restarts de orquestração

## Suporte de Shell
- bash, zsh, fish e PowerShell podem spawnar o binário
- Completions são geradas por `completions <shell>`

## Paths de Arquivo e XDG
- Artefatos seguem `--artifacts-dir` quando fornecido
- Cache e state ficam em diretórios locais do usuário
- State cifrado exige `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY`

## Performance por Target
- Desktop e servidores Linux são o alvo primário de otimização
- Cold start permanece limitado pelo Chrome em todo OS

## Agentes Validados por Plataforma
- Linux: Claude Code, Codex, shell local, agentes de editor
- macOS: agentes shell locais e integrações de editor
- Windows: integrações shell e editor com quoting explícito
