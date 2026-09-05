[English](SECURITY.md) | [Português Brasileiro](SECURITY.pt-BR.md)

# Política de Segurança

## Versões Suportadas
- `0.1.x` é a linha suportada atual

| Versão | Suportado |
|--------|-----------|
| 0.1.x  | sim       |

## Reportar uma Vulnerabilidade
- Não abra issue pública no GitHub para problemas sensíveis de segurança
- Envie email ao maintainer em daniloaguiarbr@proton.me
- Inclua descrição clara e cenário de ataque
- Inclua passos de reprodução e a versão afetada
- Inclua comportamento esperado versus real
- Inclua mitigações conhecidas quando existirem

## SLA de Resposta
- Critical (CVSS 9.0-10.0): reconhecimento em até 24 horas
- High (CVSS 7.0-8.9): reconhecimento em até 48 horas
- Medium (CVSS 4.0-6.9): reconhecimento em até 72 horas
- Low (CVSS 0.1-3.9): reconhecimento em até 5 dias úteis

## SLA de Correção
- Critical: alvo de fix ou mitigação em 7 dias após confirmação
- High: alvo de fix em 14 dias após confirmação
- Medium: alvo de fix em 30 dias após confirmação
- Low: alvo de fix na próxima janela de release

## Política de Disclosure
- Coordene o timing do disclosure com o reporter
- Prefira fixes privados antes do texto público de advisory
- Credite reporters que desejarem reconhecimento após o fix

## Política de Update de Segurança
- Fixes de segurança saem em patch releases quando possível
- Entradas do CHANGELOG marcam fixes de segurança em Fixed
- Usuários devem atualizar para o patch suportado mais recente

## Hall of Fame
- Ainda não há reports públicos creditados
- Reporters legítimos podem ser listados aqui após disclosure coordenado

## Boas Práticas para Usuários
- Mantenha Chrome ou Chromium atualizados no host
- Nunca passe secrets em argv quando houver alternativa via stdin
- Trate `--ignore-robots` como escolha explícita de alto risco
- Armazene material de encryption com `browser-automation-cli config set encryption_key <secret>` (somente config XDG)
- Mantenha encryption keys e valores de cookie fora de history e logs duráveis
- Prefira pipelines `--json` que descartem secrets de stderr em logs duráveis
- Não aponte a CLI a páginas não confiáveis sem expectativas de isolamento
- Nunca use `rediss://` para cache (somente TCP plain; `rediss://` é fail-closed)
- Armazene URL Redis só com `config set cache_redis_url` sob XDG (nunca env de produto)
- Armazene chaves LLM só com `config set openrouter_api_key` sob XDG
- Armazene credenciais do proxy de saída só com `config set proxy_username` e `config set proxy_password` sob XDG, e nunca passe essas credenciais em argv, porque a tabela de processos expõe argv a qualquer usuário da máquina
- `proxy_password` está nas varreduras de zeroize de `src/xdg/secrets.rs`, portanto é limpo no drop junto de `encryption_key`, `openrouter_api_key` e `cache_redis_url`
- Não confunda o proxy de saída (`--proxy`, chave XDG `proxy_url`) com o proxy MITM local de interceptação tratado abaixo: o primeiro roteia seu tráfego para fora por um terceiro, o segundo decifra o seu próprio tráfego em `127.0.0.1`
- Higiene residual: doctor `residual_disk` reporta dirs temporários órfãos locais; BORN/FINALIZE fazem scavenge só de markers owned da CLI e dirs Chromium Singleton-only stale (nunca mata Chrome Flatpak do host)

## Boas Práticas MITM
- Faça bind e use MITM apenas em `127.0.0.1` (proxy local one-shot; não exponha em LAN ou interfaces públicas)
- Mantenha a CA local sob XDG data (`mitm/ca`) e proteja instalações no trust-store do host
- Prefira `mitm init-ca` e deixe o material da CA sob o caminho XDG data reportado por `config path`
- Prefira one-shots curtos: `mitm start --seconds N` ou `mitm capture-url <url> --seconds N [--har caminho]`
- Exporte HAR com `mitm har --out <caminho>` (obrigatório) ou com a global `--mitm-har` no FINALIZE quando `--mitm` estiver ativo
- Redija segredos na exportação: `mitm redact --secrets` e/ou a global `--mitm-redact-secrets` (redação de Authorization/Cookie)
- Não exponha o proxy MITM além da máquina do operador
- Trate capturas, exports HAR e material privado da CA como sensíveis
- Prefira orçamentos curtos de `--seconds` em `mitm start` e limpe artefatos de captura após a análise
- Prefira `mitm capture-url` one-shot em vez de deixar proxy aberto além do necessário

