# Política de Privacidade — browser-automation-cli

[English](PRIVACY.md)

## Resumo

Esta CLI é **local-first** e **agent-first**. Ela **não** implementa telemetria remota, analytics, envio automático de relatórios de falha nem endpoints de rastreamento de terceiros.

## Dados processados

| Tipo | Onde permanece |
|------|----------------|
| Sessões de navegador | Processo Chrome local + perfil temporário sob o diretório temporário do sistema; recolhido no FINALIZE |
| Diretórios temporários residuais | Apenas locais: diretórios com marcador da CLI (`browser-automation-cli-chrome-`) e diretórios Chromium órfãos com apenas Singleton sob `/tmp`, varridos por BORN/FINALIZE; o `residual_disk` do `doctor` os inspeciona localmente — **nunca enviados** |
| Configuração | Apenas o diretório XDG de configuração, via `config` (`path`, `init`, `show`, `set`, `get`, `list-keys`) |
| Preferência de idioma da interface | Definida com `--lang pt-BR` ou `config set lang pt-BR` (XDG). Resolvida uma única vez no boot, apenas para as mensagens humanas de `suggestion`; o JSON de máquina permanece em inglês. **Nunca enviada.** |
| Logs opcionais | Arquivo local sob o diretório XDG de estado quando `log_to_file` está habilitado |
| Cache | SQLite local ou URL Redis opcional que você configura via `config set` |
| Chaves de LLM | Apenas XDG (`openrouter_api_key`, `llm_base_url`, `llm_model`) — nunca em hardcode |

## O que nunca fazemos

- Nenhum envio automático de dados de navegação, capturas de tela, HAR ou heap snapshots
- Nenhuma chamada de verificação de versão para servidor externo
- Nenhum identificador de publicidade
- Nenhuma mistura de segredos nos envelopes JSON de stdout além do que você passa como argumento

## Configuração é XDG, nunca variável de ambiente

- O produto não lê variável de ambiente de produto para configuração durável
- Toda configuração vive no arquivo `config.toml` sob o diretório XDG de configuração
- Segredos como `openrouter_api_key` e `encryption_key` são gravados com permissão 0600
- Consulte [docs/CONFIGURATION.pt-BR.md](docs/CONFIGURATION.pt-BR.md) para a referência completa das chaves

## Responsabilidade do operador

- Você controla quais URLs e páginas a ferramenta abre
- Você controla se a captura MITM e de rede está habilitada
- Você é responsável pela conformidade ao automatizar sites de terceiros (robots, termos de uso, dados pessoais)

## Contato

Consulte `SECURITY.pt-BR.md` para reporte de vulnerabilidades.
