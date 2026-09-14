[English](README.md) | [Português Brasileiro](README.pt-BR.md)

# browser-automation-cli

> Automação one-shot do Chrome CDP para agentes de IA. BORN, EXECUTE, FINALIZE, DIE.

[![docs.rs](https://img.shields.io/docsrs/browser-automation-cli)](https://docs.rs/browser-automation-cli)
[![crates.io](https://img.shields.io/crates/v/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![License](https://img.shields.io/crates/l/browser-automation-cli)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88.0-orange)](Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-blue)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/github-browser--automation--cli-black.svg)](https://github.com/danilo-aguiar-br/browser-automation-cli)

```bash
cargo install browser-automation-cli
```

Mapa de descoberta para agentes: [llms.pt-BR.txt](llms.pt-BR.txt) (curto) e [llms-full.pt-BR.txt](llms-full.pt-BR.txt) (expandido).

## O que é
- CLI de automação de browser em um único processo para agentes de IA
- Fala com Chrome ou Chromium do sistema via chromiumoxide CDP
- Sem daemon, sem empacotamento npm e sem telemetria remota
- O ciclo de vida é sempre BORN, EXECUTE, FINALIZE, DIE
- Envelopes JSON no stdout para agentes programáticos
- Config e caminhos XDG só via comandos `config`

## A Dor
- Fluxos de agente precisam de browser multi-passo sem daemon sticky
- Stacks Node e npm adicionam peso de runtime e superfície de supply-chain
- Ferramentas baseadas em sessão deixam Chrome órfão e ownership obscuro
- Contratos JSON costumam divergir de flags e exit codes reais
- Settings de produto fora do `config` XDG tornam prompts de agente frágeis

## Por que browser-automation-cli
- Um processo é dono de um ciclo completo de Chrome do launch ao kill fallback
- Trabalho multi-passo usa `run --script` NDJSON ou um array JSON de passos no mesmo processo
- Refs de acessibilidade `@eN` só valem dentro daquele processo
- Envelopes `--json` estáveis para agentes programáticos; erros de usage do clap também emitem JSON quando `--json` está no argv
- Caminho de install é Rust puro via cargo
- v0.2.0 é a release atual: um Chrome lançado pela CLI controla o DevTools por pipe em vez de porta TCP, e um lançamento headed no Linux fica dentro de um Xvfb privado protegido por cookie; inventário de **71** nomes de agente via `commands --json`; **217** chaves XDG

## Superpoderes
- Navegação e ciclo de página: `goto`, `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `select-option`, `pick` (select nativo + HIG badge/popover / `role=option` com eventos de pick), `submit`, `upload`
- Observação: `view` (recusa about:blank vazio a menos que `--allow-empty`), `grab` (formatos `png|jpeg|webp` apenas; AVIF removido), `extract`, `text`, `attr`, `scroll`, `assert`
- Wait: múltiplos `--text` resolvem como OR; multi-seletor CSS OR (`#a, #b` / `selectors`); `url` / `url_contains` / `navigation` / `wait_timeout_ms` no `run`
- Assert: `url` / `text` / `console` mais `console_empty` / `console_no_match` (na CLI `console-empty` / `console-no-match`)
- Scrape: `scrape` com `--format` / `--formats` multi/CSV e `--engine http|browser`; 15 formatos vivos `text|markdown|html|rawHtml|links|metadata|screenshot|summary|product|branding|images|jsonld|json|feed|attributes` (`raw-html` continua alias aceito de `rawHtml`); engine browser aplica formatos via outerHTML; `format`/`formats` também no passo scrape do `run`
- Superfície local scrape/crawl/map/search/parse: `batch-scrape` (`--engine http|browser`), `crawl` (`--engine http|browser`), `map`, `search` (limpa redirects SERP `uddg=`), `parse` (PDF/DOCX/xlsx/ods + `--redact-pii`)
- Extract LLM: `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
- Captura: `console` e `net` com flags globais opcionais
- Diálogos: `dialog accept|dismiss` devolve `.data.dialog_settled`; XDG `dialog_settle_ms`; isolamento multi-aba por `session_id` com gate e2e
- Storage: `storage export|import` para cookies + estado por origem no mesmo processo
- Profundidade DevTools: `eval`, `emulate`, `resize`, `perf`, `lighthouse` (resolve flag → XDG `lighthouse_path` → PATH; envelope `binary_source` real|mock; fixtures unitárias incluem LHR 13.4.1 capturado do Chrome; e2e mock permanece SKIP), `heap`
- Impressão PDF: `print-pdf` one-shot CDP `Page.printToPDF` (também no multi-passo `run`)
- Monitor: `monitor check --url --baseline [--write-baseline]`
- Utilitários (sem Chrome): `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `audio info|download|convert|trim`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Aliases de assert: `url_contains` / `text_contains`; kinds `console_empty` / `console_no_match`; `attr` faz fallback para properties DOM
- Aliases de scroll em `run`: `dy`/`dx` para `delta_y`/`delta_x`
- Categorias opcionais: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast com export via ffmpeg
- Anti-detecção: stealth LIGADO por padrão, com `--no-stealth`, `--stealth-profile`, `--stealth-seed`, `--proxy`, `--proxy-bypass`, `--input-profile human|direct`, `--warmup`, `--no-xvfb`
- MITM one-shot: `mitm start` / `mitm capture-url` escuta só em `127.0.0.1` (hudsucker); flags globais `--mitm*`
- Workflow DAG: `workflow run|resume|status` com journal SQLite (resume pula ok)
- Config XDG: `config path|init|show|set|get|unset|list-keys` para config.toml (descubra todas as chaves com `config list-keys --json`)
- Descoberta: `doctor` (browsers_dir, origem lighthouse, `cache_redis`, `residual_disk`), `commands` (**71** nomes), `schema <cmd>` ou `schema --cmd` (goto/eval/type/scroll/assert expandidos), `version`, `locale`, `man`, `completions`
- Flags globais: o help global declara **57** flags longas, sendo **55** delas flags de produto mais `--help` e `--version`; `browser-automation-cli --help` é a fonte da verdade
- Observabilidade multi-passo: o envelope final de `run --json` inclui `ok` mais os `steps[].data` completos; a global `--json-steps` streama uma linha NDJSON por passo
- Fail-fast multi-passo: `run` devolve `data.steps` parciais em envelopes de erro
- Residual-zero de disco (ainda verdadeiro desde 0.1.5 RES-01…12): BORN faz auto-GC de dirs Chromium Singleton-only em `/tmp` com mais de 60s; FINALIZE dual scavenge + re-scan; nunca mata Chrome Flatpak do host; prefixo de marker `browser-automation-cli-chrome-`
- Ciclo de vida: BORN + FINALIZE fazem scavenge de órfãos Chromium em `/tmp` owned; product law é residual-zero de processo + disco
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) e `cache_redis_url`; `rediss://` fail-closed
- Residual intencional: GAP-022 ~53 dups multi-versão transitivos; GAP-023/024 divergências de PRD registradas

## Novidades da 0.2.0
- Histórico completo em [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md)
- A ponte de pipe e o pino X11 abaixo foram validados ao vivo somente no Linux
### Segurança do DevTools e do Xvfb
- Um Chrome lançado pela CLI não abre mais porta TCP de DevTools, então um processo local não consegue mais ler `/json/version` e controlar o navegador
- O Chrome agora roda com `--remote-debugging-pipe`, e uma ponte WebSocket em loopback repassa exatamente um cliente válido num caminho com os 122 bits aleatórios de um UUID v4
- A ponte limita cada mensagem a 256 MiB e espera no máximo dois segundos pelas threads do pipe no encerramento
- O Xvfb privado exige um `MIT-MAGIC-COOKIE-1`, guardado num arquivo com modo 0600 que o encerramento remove
- Um arquivo de cookie deixado por uma CLI morta com `SIGKILL` é removido pelo próximo lançamento
- O motor Lightpanda e o caminho legado de lançamento escolhido por `chrome_legacy_oxide_launch` mantêm o comportamento anterior
### Headed no Linux Sob Wayland
- Um Chrome headed dentro do Xvfb privado não abre mais janela na área de trabalho Wayland
- O lançamento fixa `--ozone-platform=x11` sempre que o display privado subiu, e nenhuma flag nem chave XDG passa switch de plataforma ao Chrome, então `--no-xvfb` é o jeito de manter o seu display
- O pino segue o Xvfb que realmente subiu, então um Xvfb ausente não força mais X11 num lançamento sem servidor X
- `display_backend` informa o display realmente usado: `headless`, `xvfb` ou `host`
- O caminho de lançamento com extensão também sobe o display privado em execuções headed
- A mensagem `virtual_display` do `doctor` nomeia o valor para o qual `auto` resolve neste host
### Displays Concorrentes
- Dois lançamentos headed simultâneos não dividem mais o mesmo display privado
- A prontidão exige que o arquivo de lock nomeie o pid do Xvfb que este lançamento iniciou, e um servidor que sai é tentado de novo no próximo número livre
- Um número de display cujo lock nomeia um processo morto é reaproveitado
- O Xvfb privado para com `SIGTERM` e um prazo antes do `SIGKILL`, então ele remove o próprio lock e socket
- O Xvfb privado escreve no dispositivo nulo em vez de um pipe que ninguém lia
### Encerramento e Exit 124
- Um lançamento cancelado entre o fork e a primeira espera de prontidão não deixa mais um Chrome sem dono
- Um lançamento que falha mata o grupo de processos inteiro do Chrome, e não só o pid
- O `--timeout` durante esse encerramento termina o comando com exit 124 em vez de 69
- Os drenadores de saída do Chrome e do Lightpanda esperam no máximo dois segundos no encerramento, então um descendente segurando a saída não segura mais o comando

## Início Rápido
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
browser-automation-cli doctor --offline --quick --json | jaq '.residual // .data.residual // .'
browser-automation-cli locale --json
browser-automation-cli goto https://example.com --json
browser-automation-cli view --json
```

## Instalação
- Install de desenvolvimento local:
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cargo install --path browser-automation-cli --locked
```
- Do crates.io após o primeiro publish:
```bash
cargo install browser-automation-cli --locked
```
- Runtime exige Chrome ou Chromium no path do shell (ou `config set chrome_path`)
- Opcional: `ffmpeg` para export de screencast
- Opcional: binário `lighthouse` para auditorias lighthouse (ou `config set lighthouse_path`)

## Uso
- Passe sempre `--json` em pipelines de agente
- Mantenha diagnósticos humanos no stderr com `-q` ao pipar
- Use `--timeout` para orçamento wall-clock do processo em segundos
- Use `run --script` (linhas NDJSON ou um array JSON de passos) para sessões multi-passo que compartilham refs `@eN`
- Streame o progresso por passo com a global `--json-steps` (linhas NDJSON: `step`, `cmd`, `ok`, `result`)
- Prefira flags de CLI em chamadas one-off; use `config` para defaults XDG duráveis
- Detalhe de logging: `--verbose` / `--debug` / `-q`, ou `config set log_level`
- Localize sugestões humanas com `--lang pt-BR` ou `config set lang pt-BR`
- Opcional: scrape `--webhook-url` faz POST único do resultado para URL do operador (não é telemetria de produto)
- MITM opcional: global `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`

```bash
browser-automation-cli config set openrouter_api_key sk-or-...
browser-automation-cli --json goto https://example.com
browser-automation-cli --json wait --text Hello --text Welcome --ms 5000
browser-automation-cli --json schema run
browser-automation-cli --json schema --cmd wait
browser-automation-cli --json mitm capture-url https://example.com --seconds 30
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/browser-automation-cli-artifacts/cap.har
browser-automation-cli --json mitm har --out /tmp/browser-automation-cli-artifacts/capture.har
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine browser
browser-automation-cli --json scrape https://example.com --format markdown --engine http --webhook-url https://example.com/hook
browser-automation-cli --json sitemap https://example.com --limit 200
browser-automation-cli --json feed https://example.com/feed.xml
browser-automation-cli --json extract --llm --question "What is the title?" https://example.com
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json workflow resume --manifest workflow.toml
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/browser-automation-cli-artifacts/page.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/browser-automation-cli-artifacts/base.txt --write-baseline
browser-automation-cli --json parse ./doc.pdf --redact-pii
browser-automation-cli --json parse ./doc.ods
browser-automation-cli --json qr encode --text "hello" --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json qr decode --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json find-paths --glob '**/*.rs' '' src
browser-automation-cli --json sheet-write rows.csv --out /tmp/browser-automation-cli-artifacts/out.xlsx
browser-automation-cli --json sg-scan src
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --engine browser --concurrency 2
browser-automation-cli --capture-console --json assert console-empty
browser-automation-cli --json record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200
```

- `--script` recebe um caminho de arquivo, nunca JSON inline; grave o arquivo de passos primeiro (NDJSON, um passo por linha):
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
```
- Depois rode esse arquivo em um processo só:
```bash
browser-automation-cli --json run --script /tmp/steps.jsonl
browser-automation-cli --json --json-steps run --script /tmp/steps.jsonl
```
- Ler payload de API exige captura e navegação no mesmo processo, então coloque o passo `net` dentro do script (`/tmp/net.jsonl`):
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"net","action":"get","id":"0","response_path":"/tmp/browser-automation-cli-artifacts/res.json"}
```
```bash
browser-automation-cli --capture-network --json run --script /tmp/net.jsonl
```

### Referência de Flags Globais
- As flags globais aparecem em `<cmd> --help`, por exemplo `version --help`, e o `--help` de raiz não lista nenhuma delas
- Grave um padrão durável com `browser-automation-cli config set <key> <value>` no XDG em vez de repetir uma flag
- Saída e redução de payload
  - `--json` emite envelopes JSON de sucesso e de erro legíveis por máquina no stdout
  - `--json-steps` transmite um objeto NDJSON por passo de `run` no stdout com `step`, `cmd`, `ok` e `result`
  - `--correlation-id <ID>` define um id ecoado nos envelopes JSON e nos passos NDJSON, e ele não é segredo
  - `--fields <PATHS>` projeta `data` para UM CSV de caminhos pontuados relativos a `data`, e repetir a flag sai com exit 2
  - `--filter-rows <EXPR>` mantém as linhas que casam `key=value`, `key!=value` ou `key~substring`, repetível e combinada por AND
  - Um campo ausente nunca casa uma expressão de `--filter-rows`, nem mesmo com `!=`
  - `--dedupe-by <PATH>` descarta as linhas cujo valor no caminho pontuado se repete e mantém a primeira
  - `--sort-rows <PATH>` ordena as linhas por um caminho pontuado e compara números numericamente
  - `--limit-rows <N>` emite no máximo N linhas depois de filtro, dedupe e ordenação
  - `--max-items <N>` é apelido aceito de `--limit-rows` e limita o que é EMITIDO, enquanto o `--limit` de um comando limita o que é BUSCADO
  - `--count-only` emite só `{"count": N}` em vez das linhas
  - `--truncate-content <CHARS>` corta cada string do payload em N caracteres e marca `truncated`
  - `--max-output-bytes <BYTES>` é um teto rígido de bytes emitidos que descarta linhas do fim e marca `truncated`
  - `--expect <EXPR>` afirma o payload emitido com a gramática de `--filter-rows`, repetível e combinada por AND
  - `--expect` é avaliada DEPOIS de `--fields` e `--filter-rows`, sobre o payload que o chamador de fato recebe
  - Uma `--expect` não cumprida aparece em `agent_ops.expectation_unmet` e o exit code continua 0
  - `--expect-exit-code` sai com 65 quando alguma `--expect` não é cumprida
  - `--help` imprime a ajuda e `--version` imprime a versão
- Tempo e concorrência
  - `--timeout <SECS>` define o timeout global de relógio em segundos, em que 0 significa sem override e o máximo é 86400
  - `--step-timeout <SECS>` define o timeout por passo dos scripts de `run`, em que 0 herda o timeout global e o máximo é 86400
  - `--max-concurrency <N>` limita as tarefas de I/O concorrentes de batch, crawl e fan-out CDP, em que 0 significa automático
  - `--min-delay-ms <MS>` define um piso por invocação entre requisições à mesma origem em milissegundos
  - A espera efetiva de `--min-delay-ms` é o máximo entre a flag, o XDG `scrape_min_delay_ms` e o `Crawl-delay` do site
- Janela e display
  - `--browser-mode <MODE>` escolhe `auto`, `headless` ou `headed` nesta execução e vence o XDG `browser_mode`
  - `--headed` mostra a janela do navegador no seu próprio display para depuração
  - `--headless` exige navegador headless nesta execução e sobrepõe qualquer modo persistido
  - `--no-xvfb` pula o display virtual privado no Linux e usa o display atual
- Captura e artefatos
  - `--artifacts-dir <DIR>` define o diretório de screenshots, PDFs e outros artefatos one-shot
  - `--capture-console` captura mensagens de console durante comandos de navegador
  - `--capture-network` captura requisições de rede durante comandos de navegador
  - `--dump-on-failure` grava a evidência capturada de console e rede no diretório de artefatos quando o comando falha
- Robots e raízes
  - `--ignore-robots` pula as checagens de política do robots.txt e exige aceite de risco para hosts bloqueados
  - `--i-accept-robots-risk` aceita de forma explícita o risco de sobrepor o robots.txt junto com `--ignore-robots`
  - `--allow-outside-roots` permite leituras locais e gravação de artefatos fora das raízes permitidas
- Stealth e entrada
  - `--no-stealth` desliga os patches anti-detecção nesta execução
  - `--stealth-profile <PROFILE>` escolhe `auto`, `chrome-linux`, `chrome-win` ou `chrome-mac` como identidade personificada
  - `--stealth-seed <SEED>` fixa a identidade stealth entre processos
  - `--warmup` visita a raiz da origem antes da URL alvo para a sessão carregar cookies
  - `--warmup-url <URL>` aquece essa URL em vez da raiz da origem e implica `--warmup`
  - `--input-profile <PROFILE>` escolhe a modelagem de input `human` (padrão) ou `direct`
  - `--input-seed <SEED>` semeia o jitter de input para uma execução `human` reproduzir exatamente
- Proxy
  - `--proxy <URL>` define o proxy de saída do Chrome e do motor HTTP com `http`, `https` ou `socks5`
  - `--proxy-bypass <HOSTS>` lista os hosts que ignoram o proxy na sintaxe de bypass-list do Chrome
  - Credenciais de proxy ficam no XDG por `config set proxy_url`, nunca em argv
- MITM
  - `--mitm` liga o proxy MITM local one-shot e roteia o Chrome por ele
  - `--mitm-ca-dir <DIR>` define o diretório da chave e do certificado PEM da CA MITM, com padrão no XDG data
  - `--mitm-har <FILE>` grava HAR 1.2 nesse caminho no FINALIZE quando `--mitm` está ativo
  - `--mitm-hosts <HOSTS>` lista os hosts separados por vírgula a decifrar, e vazio significa todos
  - `--mitm-ws` repete o padrão, porque frames WebSocket são sempre capturados sob `--mitm`
  - `--mitm-max-body-bytes <BYTES>` limita os bytes de corpo retidos por troca
  - `--mitm-no-media-bodies` descarta corpos de imagem, vídeo e áudio da captura
  - `--mitm-redact-secrets` repete a redação padrão dos segredos de Authorization e Cookie
  - `--mitm-no-redact-secrets` mantém legíveis os valores de Authorization e Cookie na captura
- Gates de categoria
  - `--category-memory` liga as ferramentas de análise profunda de heap
  - `--category-extensions` liga as ferramentas de gestão de extensões
  - `--category-third-party` liga a superfície de ferramentas de desenvolvedor de terceiros
  - `--category-webmcp` liga a superfície de ferramentas `webmcp`
  - `--experimental-screencast` liga o screencast experimental, que pode exigir ffmpeg para exportar arquivo
  - `--experimental-vision` liga as ferramentas de visão `click-at` por coordenada
- Idioma e log
  - `--lang <LANG>` força o idioma da interface `en` ou `pt-BR`, e o JSON de máquina continua em inglês
  - `-q, --quiet` suprime os logs humanos que não são erro no stderr
  - `-v, --verbose` eleva a verbosidade do stderr para info
  - `--debug` liga o detalhe máximo de tracing no stderr
  - `--plain` força stderr simples sem cores ANSI

## Comandos
Inventário completo de agente (**71** nomes via `commands --json`, ordenados):
- `assert` — Afirmações sobre url, texto ou console
- `attr` — Lê um atributo de um alvo
- `audio` — Pipeline local de áudio one-shot sem Chrome: info, download, convert, trim
- `back` — Volta no histórico
- `batch-scrape` — Faz scrape de muitas URLs a partir de um arquivo, com motor HTTP ou browser, one-shot
- `click-at` — Clica em coordenadas CSS da página, exige `--experimental-vision`
- `commands` — Lista os comandos disponíveis
- `completions` — Gera completions de shell, em nível de caminho, sem Chrome
- `config` — Gestão de config e caminhos XDG, sem `.env` em runtime
- `console` — Mensagens de console capturadas, exige `--capture-console`
- `cookie` — Utilitários do cookie jar da página ativa pelo domínio Network
- `crawl` — Rastreia a partir de uma URL semente, por BFS HTTP ou browser, one-shot
- `devtools3p` — Superfície de ferramentas de desenvolvedor de terceiros, exige `--category-third-party`
- `dialog` — Aceita ou dispensa diálogos
- `doctor` — Diagnostica a instalação do Chrome e a prontidão one-shot
- `drag` — Arrasta de um alvo para outro com drag-and-drop HTML5
- `emulate` — Emula dispositivo, rede, UA, geolocalização ou CPU
- `eval` — Avalia JavaScript como expressão ou declaração de função
- `exec` — Comando inline de passo único com a mesma superfície dos passos de `run`
- `extension` — Ferramentas de extensão do Chrome, exige `--category-extensions`
- `extract` — Extrai texto ou atributo de um alvo, ou roda extração por LLM com `--llm`
- `feed` — Lê um documento RSS, Atom ou JSON Feed por HTTP
- `fill-form` — Preenche vários campos de formulário a partir de um array JSON de pares alvo e valor
- `find-paths` — Descobre caminhos do sistema de arquivos com UX parecida com a do fd
- `forward` — Avança no histórico
- `goto` — Navega para uma URL, one-shot
- `grab` — Captura um screenshot
- `heap` — Ferramentas de heap snapshot, a análise profunda exige `--category-memory`
- `hover` — Passa o ponteiro sobre um elemento
- `image` — Pipeline local de imagem one-shot sem Chrome: info, convert, resize, download, exif
- `keys` — Pressiona uma tecla
- `lighthouse` — Roda uma auditoria Lighthouse pelo binário externo
- `locale` — Mostra o locale de interface resolvido e o diagnóstico da detecção
- `man` — Gera uma man page roff, em nível de caminho, sem Chrome
- `map` — Mapeia as URLs de um site a partir de uma semente por HTTP
- `mitm` — Captura MITM, CA e HAR, local e one-shot
- `monitor` — Checagem one-shot de mudança contra um arquivo de baseline por hash ou texto
- `net` — Requisições de rede capturadas, exige `--capture-network`
- `page` — Informação da página ou gestão de múltiplas abas
- `parse` — Extrai texto de um arquivo local html, md, txt, pdf, docx ou xlsx
- `perf` — Trace e métricas de performance
- `pick` — Escolhe uma opção de select customizado, popover de badge ou `role=option`
- `press` — Clica em um elemento por seletor ou ref `@eN`
- `print-pdf` — Imprime a página atual em PDF pelo CDP `Page.printToPDF`, one-shot
- `qr` — Codifica e decodifica QR one-shot sem Chrome
- `record` — Grava as interações da página como arquivo NDJSON reproduzível por `run --script`
- `reload` — Recarrega a página atual
- `resize` — Redimensiona o viewport da página
- `run` — Roda um script multi-passo em um processo, NDJSON ou array JSON de passos
- `schema` — Fragmento JSON Schema de um comando, aceitando `schema run` ou `schema --cmd run`
- `scrape` — Navega e devolve o texto do corpo ou formatos, por HTTP local ou CDP
- `screencast` — Início e fim de screencast, experimental
- `scroll` — Rola a página ou um elemento por pixels de delta
- `search` — Busca local sobre links de SERP HTTP ou um mapa de URLs
- `select-option` — Escolhe uma opção de select customizado, popover de badge ou `role=option`
- `sg-rewrite` — Reescrita estrutural para correções seguras conhecidas, dry-run por padrão e `--apply` grava
- `sg-scan` — Varredura estrutural de lint por padrões proibidos do produto, one-shot
- `sheet-write` — Grava uma planilha XLSX simples a partir de CSV ou JSON, one-shot
- `sitemap` — Lista as URLs declaradas no sitemap.xml de um site por HTTP
- `storage` — Exporta ou importa estado de autenticação portátil: cookies, localStorage e sessionStorage
- `submit` — Submete um formulário, ou o formulário dono de um campo, e espera o resultado
- `text` — Extrai o texto visível de um alvo
- `type` — Digita texto em `--target` ou no elemento focado com `--focus-only`
- `upload` — Envia um arquivo para um input de arquivo
- `version` — Imprime a versão da CLI
- `video` — Pipeline local de vídeo one-shot sem Chrome: info, download, convert, to-mp3, trim, thumbnail, manifest
- `view` — Snapshot de acessibilidade com refs `@eN`
- `wait` — Espera milissegundos, texto, seletor ou estado de carga
- `webmcp` — Ferramentas de superfície web, exige `--category-webmcp`
- `workflow` — Journal DAG de workflow persistido em SQLite
- `write` — Preenche o valor de um input com preenchimento inteligente para select, checkbox, radio e texto

Agrupados para humanos:
- Descoberta: `doctor`, `commands`, `schema`, `version`, `locale`, `man`, `completions`
- Navegação: `goto`, `back`, `forward`, `reload`
- Interação: `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `select-option`, `pick`, `submit`, `upload`, `click-at`
- Observação: `view`, `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Scrape: `scrape`, `batch-scrape`, `crawl`, `map`, `sitemap`, `feed`, `search`, `parse`
- Captura: `console`, `net`, `print-pdf`, `monitor`, `screencast`
- Abas/Diálogos: `page`, `dialog`, `cookie`, `storage`
- Utilitários: `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
- Avançado: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `heap`, `extension`, `devtools3p`, `webmcp`, `mitm`, `workflow`
- Config: `config path|init|show|set|get|unset|list-keys`
- Multi-passo: `run`, `exec`, `record`
- Ensino de record: `browser-automation-cli --json record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200` grava as interações da página como NDJSON reproduzível, e `browser-automation-cli --json run --script /tmp/steps.jsonl` reproduz tudo em um processo só
- Ensino de audio: `browser-automation-cli --json audio info|download|convert|trim` roda o pipeline local de áudio sem Chrome
- Ensino de sitemap: `browser-automation-cli --json sitemap https://example.com --limit 200` lê o sitemap DECLARADO — as dicas `Sitemap:` do `robots.txt`, o próprio documento e a descida em `sitemapindex` aninhado — e nunca percorre o grafo de links, então não existe `--depth` para passar
- Ensino de feed: `browser-automation-cli --json feed https://example.com/feed.xml` parseia RSS, Atom e JSON Feed a partir do corpo BRUTO pelo motor HTTP; as flags que moldam HTML estão ausentes porque um seletor destruiria o documento, e o Chrome não é oferecido porque renderizaria o visualizador XML do navegador em vez do feed
- Nota de inventário: **71** nomes de agente via `commands --json` (inclui `select-option`, `pick`, `submit`, `storage`, `image`+`video`+`audio`+`record`); e2e DevTools cobre 53 tools (lighthouse mock SKIP)

## Anti-detecção, Proxy e Modelagem de Input
- Stealth é LIGADO por padrão e mascara os marcadores de automação que um Chrome real nunca expõe
- `--no-stealth` desliga os patches anti-detecção nesta execução
- `--stealth-profile <PROFILE>` escolhe a identidade personificada: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac` (`list` imprime os tokens)
- `--stealth-seed <SEED>` fixa `hardwareConcurrency`, `deviceMemory`, vendor/renderer da GPU e `history.length` — não UA, platform, idiomas, fuso, tela, plugins nem o build do Chrome, que vem da crate de identidade com override de User-Agent e do Chrome instalado sem override
- `doctor --fingerprint` compara webdriver, platform vs UA e screen vs viewport; sem `--quick` pontua a página ao vivo e falha se ela contradisser o plano
- O launch aplica métricas 1920×1080 para `screen` não ficar no padrão headless 800×600; `config set screen WxH` e o passo `screen` do `run` são os knobs explícitos
- `auto` segue o host e quase sempre está certo
- `--stealth-seed <SEED>` fixa essa identidade entre processos
- Com semente o major do Chrome do último lançamento também fica guardado em `state_dir/stealth/host-major-<hash>.txt`, e o motor HTTP anuncia esse major; sem semente, ou sob `--no-stealth`, nada dele toca o disco
- `scrape` informa de onde veio o major do Chrome anunciado em `user_agent_major_source`: `projected`, `host_binary`, `host_unprobed`, ou `null` sob `--no-stealth`
- Sem a semente cada execução sorteia identidade nova, então um crawl de 50 URLs em 50 processos one-shot se apresenta como 50 máquinas distintas
- `--proxy <URL>` define o proxy de saída para o Chrome **e** para o motor HTTP, aceitando `http`, `https` e `socks5`
- `--proxy-bypass <HOSTS>` lista os hosts que ignoram o proxy, na sintaxe de bypass-list do Chrome
- `--input-profile <PROFILE>` é `human` (padrão) ou `direct`
- `human` interpola trajetórias do ponteiro, aplica dwell entre press e release e ritma a digitação
- Medido em 2026-09-04 nesta árvore: o custo do ritmo `human` cresce de forma superlinear com o tamanho digitado, em 2281 ms para 1 caractere, 14236 ms para 2 e 95781 ms para 4, então um `type` longo pode esgotar o `--timeout` e devolver exit 124
- Passe `--input-profile direct` quando o campo for longo e o ritmo não importar; este é um defeito ABERTO, registrado no `gaps.md`, e a contramedida está declarada aqui em vez de ficar para o operador descobrir por um timeout
- `--input-seed <SEED>` semeia o jitter de input para que uma execução `human` reproduza exatamente
- `--warmup` visita a raiz da origem antes da URL alvo, então a sessão já carrega cookies e cadeia de referrer
- `--warmup-url <URL>` aquece essa URL em vez da raiz da origem alvo
- `--no-xvfb` pula o display virtual privado no Linux e usa o display atual (só faz sentido em modo headed no Linux)
- `--expect <EXPR>` afirma que o payload emitido casa com `key=value`, `key!=value` ou `key~substring`; ela é repetível e cada expressão é conjugada por AND
- `--expect-exit-code` sai com `65` quando alguma expectativa falha, em vez de apenas reportar
- Ela fica desligada por padrão porque mudar exit code por conteúdo de dado quebraria em silêncio os chamadores que já ramificam nele
- Chaves XDG duráveis: `stealth` (`true`), `stealth_profile` (`auto`), `stealth_seed`, `browser_mode` (`auto`), `input_profile` (`human`)
- `browser_mode` é `auto|headed|headless`; `auto` resolve para headed dentro de um display virtual privado no Linux com Xvfb no PATH e sem `--no-xvfb`, e para headless em qualquer outro caso; o `doctor` reporta o modo efetivo
- Chaves XDG de proxy: `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `cdp_proxy_bypass_loopback` (`true`)
- Guarde credenciais de proxy somente no XDG, porque argv aparece na tabela de processos
- `cdp_proxy_bypass_loopback` sempre ignora loopback para o canal de controle CDP sobreviver ao proxy
- `robots_user_agent` define o token de user-agent contra o qual as regras do robots.txt são casadas
- Chaves de fingerprint HTTP/2: `http2_enabled` (`true`), `http2_initial_stream_window_size` (`6291456`), `http2_initial_connection_window_size` (`15663105`), `http2_max_header_list_size` (`262144`), `http2_max_frame_size` (`16384`), `http2_adaptive_window` (`false`)
- `http2_adaptive_window` fica desligado para manter o fingerprint constante
- Chaves de cinemática de input: `input_move_steps` (`24`), `input_move_gap_ms` (`12`), `input_click_dwell_ms` (`65`), `input_key_dwell_ms` (`45`), `input_type_delay_ms` (`95`), `input_scroll_tick_px` (`100`), `input_scroll_max_ticks` (`40`), `input_target_jitter_px` (`3`), `input_scroll_settle_rounds` (`3`)
- Chaves de dispersão e ritmo de input: `input_timing_distribution` (`lognormal`), `input_move_steps_stddev` (`6`), `input_move_gap_stddev_ms` (`5`), `input_click_dwell_stddev_ms` (`26`), `input_key_dwell_stddev_ms` (`18`), `input_type_delay_stddev_ms` (`40`), `input_scroll_tick_stddev_px` (`25`), `input_word_pause_ms` (`320`), `input_word_pause_permille` (`120`), `input_typo_permille` (`0`)
- `input_timing_distribution` é `lognormal|normal|uniform` e governa só o ritmo rápido, porque a cauda de pausa longa é `input_word_pause_permille`
- `input_word_pause_permille` aceita `0` como valor legítimo, que remove por completo a cauda de pausa longa entre palavras
- `input_typo_permille` fica em `0` porque um erro de digitação corrigido com Backspace muda o que a página vê no meio da palavra
- `user_data_dir` não tem padrão e fica ausente, e essa ausência é o que sustenta a garantia de residual-zero em disco
- Ativar `user_data_dir` é opt-in e abre mão dessa garantia, porque o perfil do Chrome passa a persistir entre execuções
- `capture_preserved_rings` (`3`) é o número de fronteiras de navegação preservadas para `console` e `net --include-preserved`

```bash
browser-automation-cli --json --stealth-seed fleet-01 goto https://example.com
browser-automation-cli --json --proxy socks5://127.0.0.1:1080 scrape https://example.com --format text --engine http
browser-automation-cli --json --input-profile human --input-seed 42 goto https://example.com
browser-automation-cli --json --warmup goto https://example.com/deep/page
browser-automation-cli --json --warmup-url https://example.com/login goto https://example.com/app
browser-automation-cli --json --no-stealth goto http://127.0.0.1:8080
browser-automation-cli --json config set stealth_profile chrome-linux
browser-automation-cli --json config set proxy_url http://user:pass@127.0.0.1:8888
browser-automation-cli --json config unset stealth_seed
```

## Configuração
- Prefira flags de CLI para chamadas one-off de agente
- Settings de produto só via flags e XDG `config path|init|show|set|get|unset|list-keys`
- Descubra a lista completa de chaves (a contagem não é mais fixa em 16) com `config list-keys --json`
- Toda chave com seu padrão e sua descrição: [Referência de Chaves XDG](#referência-de-chaves-xdg)
- Chaves importantes: `dialog_settle_ms`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`, `lang`, `log_level`
- Logging: `--verbose` / `--debug` / `-q`, ou XDG `config set log_level` / `log_to_file`
- Color: `config set color true|false`
- Binário Chrome: path do shell ou XDG `config set chrome_path`
- Binário Lighthouse: flag `--lighthouse-path`, XDG `config set lighthouse_path`, ou PATH (envelope reporta `binary_source`)
- Orçamento de settle de diálogo: XDG `config set dialog_settle_ms <ms>` (`dialog_settled` visível ao agente em dialog accept|dismiss)
- Cache: `config set cache_backend sqlite|memory|redis` e opcional `cache_redis_url` (somente `redis://`; `rediss://` fail-closed)
- `config init` cria o layout XDG e o config.toml padrão
- `config unset <CHAVE>` restaura uma chave ao default embutido e é o inverso real de `set`
- `config set <chave> ""` não é inverso: em chave string grava um valor vazio que o caminho normal nunca produz, e em chave numérica é erro de parse
- Desfazer chave já ausente tem sucesso, então um script nunca precisa saber o estado anterior
- `config path` imprime paths resolvidos de config, data, cache, state e browsers_dir
- CLI flags sobrescrevem valores do config.toml
- Doctor reporta browsers_dir, origem lighthouse, `cache_redis` e `residual_disk` entre as checagens de readiness
- Campo JSON de topo `residual` do doctor reporta: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`

## Referência de Chaves XDG
- Esta seção lista todas as 217 chaves XDG agrupadas por família, cada uma com o padrão embutido e a descrição
- A fonte da verdade é `browser-automation-cli --json config list-keys`, que responde com a mesma chave, o mesmo padrão e a mesma descrição
- Grave uma chave com `config set <chave> <valor>`, leia com `config get <chave>` e restaure o padrão com `config unset <chave>`
- Padrão nenhum significa que a chave fica ausente até você gravá-la
- As descrições espelham [docs/CONFIGURATION.pt-BR.md](docs/CONFIGURATION.pt-BR.md), e os padrões vêm do binário
### `audio_*` (4)
- `audio_default_bitrate` — taxa de bits padrão na codificação de áudio com perda, por exemplo `192k`. Padrão: `192k`
- `audio_default_format` — formato padrão da conversão de áudio, aceitando `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` ou `aac`. Padrão: `mp3`
- `audio_download_max_bytes` — número máximo de bytes do corpo HTTP no download de áudio. Padrão: `256000000`
- `audio_max_input_bytes` — número máximo de bytes na materialização de áudio vindo do stdin ou na verificação prévia do caminho. Padrão: `256000000`

### `browser_*` (3)
- `browser_close_wait_secs` — orçamento de espera por `Browser.close` e pelo término do processo durante a fase FINALIZE, em segundos. Padrão: `5`
- `browser_mode` — modo de janela: `auto` resolve para `headed` dentro de um display virtual privado no Linux com Xvfb no PATH e sem `--no-xvfb`, e para `headless` em qualquer outro caso; `headed` coloca uma janela real no seu display, e no Linux essa janela é renderizada em um display virtual privado quando há Xvfb; `headless` é o mais barato e o mais detectável. `--headed` continua vencendo. Inverter o padrão de `auto` tem custo de latência e é decisão separada; o `doctor` reporta para que `auto` resolve neste host no check `virtual_display`, então a resposta nunca diverge do binário. Padrão: `auto`
- `browser_scrape_max_body_bytes` — número máximo de bytes do corpo nos auxiliares de scrape do motor de navegador. Padrão: `2000000`

### `cache_*` (4)
- `cache_backend` — backend de cache, aceitando `sqlite`, `memory` ou `redis`. Padrão: `sqlite`
- `cache_max_resp_bulk_bytes` — teto de tamanho da bulk string RESP do Redis em bytes. Padrão: `16777216`
- `cache_max_resp_line_bytes` — teto de tamanho da linha RESP do Redis em bytes. Padrão: `16777216`
- `cache_redis_url` — URL do Redis quando o backend é `redis`. Padrão: nenhum

### `capture_*` (1)
- `capture_preserved_rings` — fronteiras de navegação mantidas para `--include-preserved` de console e rede. Padrão: `3`

### `cdp_*` (8)
- `cdp_connection_probe_timeout_secs` — timeout da sondagem de vitalidade `Browser.getVersion` do CDP em segundos. Padrão: `3`
- `cdp_discovery_max_body_bytes` — número máximo de bytes do corpo HTTP de descoberta CDP em `/json/version` e `/json/list`. Só a sondagem de prontidão do Lightpanda lê essa chave, porque o Chrome lançado pelo próprio produto usa o pipe de DevTools. Padrão: `1048576`
- `cdp_discovery_timeout_secs` — timeout da descoberta HTTP do CDP nas sondagens de `/json/version`, em segundos. Nenhum caminho de lançamento lê essa chave: o Chrome lançado pelo próprio produto usa o pipe de DevTools e não expõe `/json/version`, e a prontidão do Lightpanda usa `lightpanda_discovery_timeout_ms`. Padrão: `2`
- `cdp_event_broadcast_capacity` — capacidade do canal local de difusão de eventos CDP dentro do processo. Padrão: `4096`
- `cdp_event_drain_poll_ms` — fatia de sondagem no esvaziamento de eventos CDP durante a espera por navegação, em milissegundos. Padrão: `100`
- `cdp_network_idle_settle_ms` — janela de estabilização de rede ociosa do CDP em milissegundos. Padrão: `500`
- `cdp_proxy_bypass_loopback` — sempre ignorar o loopback quando o Chrome roda sob `--proxy`. O canal de controle CDP é loopback, então um proxy que o captura produz um browser que nunca responde — reportado como timeout de inicialização do Chrome, o que culpa o componente errado. Padrão: `true`
- `cdp_target_event_wait_ms` — espera curta por evento de target do CDP em milissegundos. Padrão: `600`

### `chrome_*` (5)
- `chrome_default_timeout_ms` — timeout padrão por operação do motor Chrome em milissegundos. Padrão: `25000`
- `chrome_legacy_oxide_launch` — inicia o Chrome via `chromiumoxide` em vez do caminho de auto-spawn, servindo como recuo de estabilização, perdendo o alvo de encerramento residual, reabrindo uma porta DevTools em loopback sem autenticação e nunca subindo o Xvfb privado. Padrão: `false`
- `chrome_path` — caminho absoluto do binário Chrome ou Chromium. Padrão: nenhum
- `chrome_search_paths` — caminhos ordenados de descoberta do Chrome ou Chromium, separados pelo separador da plataforma, com o valor vazio usando o layout embutido de cada sistema. Padrão: nenhum
- `chrome_startup_timeout_secs` — espera pela prontidão do CDP no auto-spawn do Chrome em segundos, medida na ponte do pipe de DevTools; a ponte dá ao cliente CDP o dobro desse orçamento para conectar. Padrão: `20`

### `default_*` (3)
- `default_jpeg_quality` — qualidade JPEG de 1 até 100 quando `grab` omite `--quality`. Padrão: `80`
- `default_viewport_height` — altura padrão da janela do Chrome headless (`--window-size`) quando as opções de inicialização omitem o viewport. Padrão: `1080`
- `default_viewport_width` — largura padrão da janela do Chrome headless (`--window-size`) quando as opções de inicialização omitem o viewport. Padrão: `1920`

### `heap_*` (10)
- `heap_dominator_max_states` — teto de estados visitados no cálculo de dominadores, como proteção contra grafos patológicos. Padrão: `50000`
- `heap_final_iters` — iterações finais de esvaziamento do snapshot de heap. Padrão: `20`
- `heap_inner_iters` — iterações internas de esvaziamento do snapshot de heap após a conclusão. Padrão: `10`
- `heap_max_class_nodes` — teto da lista `class_nodes` do heap. Padrão: `500`
- `heap_max_edges` — número máximo de arestas devolvidas por operação de nó do heap. Padrão: `200`
- `heap_max_path_depth` — profundidade máxima dos caminhos do heap. Padrão: `8`
- `heap_max_paths` — número máximo de caminhos na enumeração de caminhos do heap. Padrão: `32`
- `heap_max_retainers` — número máximo de retentores devolvidos por operação de nó do heap. Padrão: `200`
- `heap_outer_iters` — número máximo de iterações da sondagem externa do snapshot de heap. Padrão: `200`
- `heap_snapshot_max_bytes` — teto de tamanho do arquivo de snapshot de heap analisado offline, em bytes. Padrão: `536870912`

### `http_*` (5)
- `http_connect_timeout_secs` — timeout da fase de conexão HTTP em segundos. Padrão: `10`
- `http_pool_max_idle_per_host` — número máximo de conexões ociosas do pool `reqwest` por host. Padrão: `4`
- `http_redirect_max` — número máximo de redirecionamentos HTTP seguidos pelos clientes do produto. Padrão: `10`
- `http_ssrf_mode` — política HTTP contra SSRF, aceitando `strict`, `allow_loopback` ou `off`. Padrão: `strict`
- `http_timeout_secs` — timeout total do cliente HTTP compartilhado em segundos. Padrão: `30`

### `http2_*` (6)
- `http2_adaptive_window` — deixar a pilha HTTP/2 redimensionar janelas dinamicamente. Desligado mantém os valores anunciados fixos, que é o que torna o fingerprint reproduzível. Padrão: `false`
- `http2_enabled` — oferecer `h2` no ALPN. O ALPN é visível em claro durante o handshake TLS e o Chrome sempre lista `h2`, então um cliente que só oferece `http/1.1` já respondeu "não sou browser" antes de enviar um byte. Padrão: `true`
- `http2_initial_connection_window_size` — janela de controle de fluxo no nível da conexão anunciada ao par. Padrão: `15663105`
- `http2_initial_stream_window_size` — `SETTINGS_INITIAL_WINDOW_SIZE` anunciado ao par. Os defaults de biblioteca ficam três ordens de magnitude longe do Chrome. Padrão: `6291456`
- `http2_max_frame_size` — `SETTINGS_MAX_FRAME_SIZE` anunciado ao par. Padrão: `16384`
- `http2_max_header_list_size` — `SETTINGS_MAX_HEADER_LIST_SIZE` anunciado ao par. Padrão: `262144`

### `image_*` (6)
- `image_avif_speed` — velocidade do codificador AVIF, de 1 até 10, sendo 1 a mais lenta e de melhor resultado, exigindo a feature `image-avif`. Padrão: `6`
- `image_default_format` — formato padrão da conversão de imagem, aceitando `png`, `jpeg`, `webp` ou `gif`. Padrão: `png`
- `image_default_quality` — qualidade padrão com perda, de 1 até 100, para conversão e redimensionamento de imagem. Padrão: `85`
- `image_download_max_bytes` — número máximo de bytes do corpo HTTP no download de imagem. Padrão: `32000000`
- `image_max_input_bytes` — número máximo de bytes de entrada para decodificar, converter ou redimensionar imagem local. Padrão: `32000000`
- `image_max_pixels` — produto máximo de largura por altura na decodificação de imagem, como proteção contra bomba de descompressão. Padrão: `64000000`

### `input_*` (20)
- `input_click_dwell_ms` — tempo de retenção entre `mousePressed` e `mouseReleased`, em milissegundos. Padrão: `65`
- `input_click_dwell_stddev_ms` — desvio padrão da retenção entre pressionar e soltar o botão, em milissegundos. Padrão: `26`
- `input_key_dwell_ms` — tempo de retenção entre `keyDown` e `keyUp`, em milissegundos. Padrão: `45`
- `input_key_dwell_stddev_ms` — desvio padrão da retenção entre `keyDown` e `keyUp`, em milissegundos. Padrão: `18`
- `input_move_gap_ms` — atraso entre as posições sintetizadas do ponteiro, em milissegundos. Padrão: `12`
- `input_move_gap_stddev_ms` — desvio padrão do atraso entre as posições sintetizadas do ponteiro, em milissegundos. Padrão: `5`
- `input_move_steps` — posições intermediárias do ponteiro sintetizadas em um movimento (perfil `human`). Padrão: `24`
- `input_move_steps_stddev` — desvio padrão do orçamento de amostras de ponteiro por gesto, para que dois movimentos sobre a mesma distância não carreguem o mesmo número de posições intermediárias. Um drag reescala esse valor para o próprio orçamento menor em vez de herdar o número absoluto. Padrão: `6`
- `input_profile` — modelagem de input padrão quando `--input-profile` está ausente: `human` sintetiza trajetória, tiques de roda e eventos de tecla; `direct` mantém o despacho anterior a 0.1.8. A flag continua vencendo. Padrão: `human`
- `input_scroll_max_ticks` — teto do número de tiques de roda que um gesto de rolagem sintetiza. Cada tique é um round-trip CDP, então sem teto o custo da rolagem cresce linearmente com a distância pedida e um `--delta-y` grande esgota o timeout do comando. Acima do teto cada tique carrega mais pixels; o percurso total não muda e só a granularidade degrada. Padrão: `40`
- `input_scroll_settle_rounds` — rodadas extras permitidas para entregar um delta de roda que o renderizador descartou. Padrão: `3`
- `input_scroll_tick_px` — distância de rolagem carregada por um tique de roda sintetizado, em pixels CSS. Padrão: `100`
- `input_scroll_tick_stddev_px` — desvio padrão da distância que um tique de roda sintetizado carrega, em pixels CSS. Padrão: `25`
- `input_target_jitter_px` — raio do deslocamento aleatório aplicado ao alvo do clique, em pixels CSS. Padrão: `3`
- `input_timing_distribution` — forma da dispersão sorteada em torno de cada atraso de input: `lognormal`, `normal` ou `uniform`. `lognormal` é o padrão porque o intervalo humano entre teclas é assimétrico à direita, e um sorteio simétrico reproduz a largura da distribuição humana sem a assimetria dela. Ela governa apenas o ritmo rápido; a cauda de pausas longas é `input_word_pause_permille`. Toda média recebe um piso de 5% de dispersão e é truncada entre um quarto e quatro vezes ela mesma, então zerar um desvio padrão não compra a variância zero que um detector lê como máquina. Padrão: `lognormal`
- `input_type_delay_ms` — atraso entre caracteres durante a digitação, em milissegundos. Padrão: `95`
- `input_type_delay_stddev_ms` — desvio padrão do atraso entre caracteres, em milissegundos. Um chamador que pede o próprio ritmo de digitação recebe essa dispersão reescalada na mesma proporção, então metade da média vira metade da largura, em vez de um valor absoluto que já não cabe. Padrão: `40`
- `input_typo_permille` — chance em mil de um caractere ser digitado errado, apagado com `Backspace` e redigitado. O campo termina sempre com exatamente o texto pedido. `0` por padrão, e é a única chave de humanização que é: todas as outras dispersam TEMPO, que a página não lê como valor diferente, enquanto esta muda o FLUXO DE CARACTERES, então um ouvinte de `input` vê o prefixo errado e pode autocompletar ou navegar com ele. A tecla errada é sempre uma vizinha física na fileira QWERTY. Padrão: `0`
- `input_word_pause_ms` — média da pausa extra tomada em um limite de palavra ou de frase, em milissegundos, ela mesma dispersa por metade do próprio valor. Essa pausa é o que produz a cauda longa à direita de um traço de digitação, que nenhum jitter em torno da média por caractere cria. Padrão: `320`
- `input_word_pause_permille` — chance em mil de um limite de palavra ou de frase ganhar essa pausa longa. `0` remove a cauda e deixa só o ritmo rápido. Padrão: `120`

### `lightpanda_*` (8)
- `lightpanda_cdp_connect_timeout_secs` — timeout da tentativa de conexão CDP com o Lightpanda em segundos. Padrão: `5`
- `lightpanda_discovery_timeout_ms` — timeout por sondagem de descoberta CDP durante a espera pelo Lightpanda, em milissegundos. Padrão: `500`
- `lightpanda_max_log_lines` — anel limitado de log da inicialização do Lightpanda, contado em linhas por fluxo. Padrão: `40`
- `lightpanda_poll_interval_ms` — intervalo de sondagem da prontidão do CDP do Lightpanda em milissegundos. Padrão: `100`
- `lightpanda_ready_slice_ms` — fatia de esvaziamento após a saída do processo filho Lightpanda, antes de capturar os logs, em milissegundos. Padrão: `25`
- `lightpanda_session_timeout_secs` — duração máxima da sessão informada em `--timeout` do Lightpanda, em segundos, na faixa de 1 até 604800. Padrão: `604800`
- `lightpanda_startup_timeout_secs` — espera pela inicialização do processo Lightpanda em segundos. Padrão: `10`
- `lightpanda_target_init_timeout_secs` — espera pela inicialização do target do Lightpanda após a conexão, em segundos. Padrão: `10`

### `llm_*` (3)
- `llm_base_url` — URL base compatível com a API OpenAI. Padrão: nenhum
- `llm_http_timeout_secs` — timeout HTTP bloqueante para chamadas de LLM e webhook em segundos. Padrão: `60`
- `llm_model` — identificador do modelo de LLM usado por padrão. Padrão: nenhum

### `log_*` (3)
- `log_level` — filtro de tracing aplicado quando as flags de argv estão silenciosas, sem qualquer leitura de `RUST_LOG`. Padrão: `error`
- `log_rotation` — política de rotação, aceitando `daily`, `hourly` ou `never`. Padrão: `daily`
- `log_to_file` — grava logs JSON locais rotacionados sob o diretório de estado XDG, nunca remotos. Padrão: `false`

### `max_*` (6)
- `max_cli_json_payload_bytes` — número máximo de bytes das cargas JSON passadas por flag da CLI. Padrão: `4194304`
- `max_json_file_bytes` — número máximo de bytes de arquivos JSON ou NDJSON usados como script ou manifesto. Padrão: `33554432`
- `max_log_files` — número de arquivos de log rotacionados retidos, na faixa de 1 até 90. Padrão: `14`
- `max_ndjson_line_bytes` — número máximo de bytes de uma única linha NDJSON em scripts de `run` e em traces. Padrão: `1048576`
- `max_sg_file_bytes` — número máximo de bytes de um arquivo-fonte lido por `sg-scan` e `sg-rewrite`. Padrão: `16777216`
- `max_urls_file_bytes` — número máximo de bytes da lista informada em `batch-scrape --urls-file`. Padrão: `8388608`

### `mitm_*` (9)
- `mitm_ca_cache_size` — tamanho do cache de certificados dinâmicos do MITM, contado em hosts. Padrão: `1000`
- `mitm_capture_wait_max_ms` — teto da espera de captura MITM após navegar, em milissegundos. Padrão: `8000`
- `mitm_capture_wait_min_ms` — piso da espera de captura MITM após navegar, em milissegundos. Padrão: `800`
- `mitm_chrome_settle_ms` — estabilização após a inicialização do Chrome no MITM, antes de navegar, em milissegundos. Padrão: `150`
- `mitm_list_limit_max` — teto de itens nas operações de listagem e consulta do MITM. Padrão: `10000`
- `mitm_proxy_seconds_max` — janela máxima do proxy MITM em execução única, em segundos. Padrão: `600`
- `mitm_rebind_attempts` — número de novas tentativas de bind do proxy MITM quando a porta está temporariamente em uso. Padrão: `3`
- `mitm_ws_frames_cap` — teto de quadros WebSocket mantidos em memória por processo de captura. Padrão: `500`
- `mitm_ws_preview_chars` — truncamento da prévia de texto WebSocket, contado em caracteres Unicode. Padrão: `256`

### `perf_*` (5)
- `perf_autostop_settle_ms` — estabilização da parada automática de perf após carga ou recarga, em milissegundos. Padrão: `500`
- `perf_trace_inner_iters` — iterações internas de esvaziamento do trace de perf após a conclusão. Padrão: `5`
- `perf_trace_inner_slice_ms` — fatia interna de sondagem do trace de perf em milissegundos. Padrão: `20`
- `perf_trace_outer_iters` — número máximo de iterações da sondagem externa do trace de perf. Padrão: `100`
- `perf_trace_outer_slice_ms` — intervalo externo de sondagem do trace de perf em milissegundos. Padrão: `50`

### `proxy_*` (4)
- `proxy_bypass` — hosts que ignoram o proxy, na sintaxe de bypass-list do Chrome. Padrão: nenhum
- `proxy_password` — senha do proxy, enviada como basic auth. Nunca ecoada por `config get` nem `config show`. Padrão: nenhum
- `proxy_url` — proxy de saída para o Chrome e para o motor HTTP (`http`, `https`, `socks5`, `socks5h`). Guarde credenciais aqui em vez de `--proxy`, onde a tabela de processos as expõe. Padrão: nenhum
- `proxy_username` — nome de conta do proxy, enviado como basic auth. Fica aqui e não no argv, onde a tabela de processos o exporia. Padrão: nenhum

### `redis_*` (3)
- `redis_allow_remote` — permite hosts Redis fora do loopback, desligado por padrão. Padrão: `false`
- `redis_connect_timeout_secs` — timeout de conexão TCP com o Redis em segundos. Padrão: `2`
- `redis_io_timeout_secs` — timeout de entrada e saída do stream RESP do Redis em segundos. Padrão: `3`

### `retry_*` (16)
- `retry_base_delay_ms` — atraso base padrão de retentativa em milissegundos. Padrão: `50`
- `retry_budget_secs` — orçamento padrão de tempo real das retentativas em segundos. Padrão: `10`
- `retry_cdp_base_delay_ms` — atraso base das retentativas do CDP em milissegundos. Padrão: `100`
- `retry_cdp_budget_secs` — orçamento de tempo real das retentativas do CDP em segundos. Padrão: `15`
- `retry_cdp_max_attempts` — número máximo de tentativas nas retentativas do CDP. Padrão: `4`
- `retry_cdp_max_delay_secs` — atraso máximo das retentativas do CDP em segundos. Padrão: `3`
- `retry_default_max_attempts` — número máximo padrão de tentativas, incluindo a primeira. Padrão: `3`
- `retry_http_base_delay_ms` — atraso base das retentativas de scrape HTTP em milissegundos. Padrão: `75`
- `retry_http_budget_secs` — orçamento de tempo real das retentativas de scrape HTTP em segundos. Padrão: `12`
- `retry_http_max_attempts` — número máximo de tentativas nas retentativas de scrape HTTP. Padrão: `3`
- `retry_http_max_delay_secs` — atraso máximo das retentativas de scrape HTTP em segundos. Padrão: `2`
- `retry_llm_base_delay_ms` — atraso base das retentativas HTTP de LLM em milissegundos. Padrão: `200`
- `retry_llm_budget_secs` — orçamento de tempo real das retentativas HTTP de LLM em segundos. Padrão: `20`
- `retry_llm_max_attempts` — número máximo de tentativas nas retentativas HTTP de LLM. Padrão: `2`
- `retry_llm_max_delay_secs` — atraso máximo das retentativas HTTP de LLM em segundos. Padrão: `4`
- `retry_max_delay_secs` — atraso máximo padrão de retentativa em segundos. Padrão: `2`

### `robots_*` (4)
- `robots_loopback_exempt` — hosts de loopback pulam o `robots.txt`, e o valor `false` passa a exigir conformidade contra `localhost`. Padrão: `true`
- `robots_max_body_bytes` — limite de bytes do corpo do `robots.txt`, como proteção contra estouro de memória. Padrão: `524288`
- `robots_probe_timeout_secs` — timeout da requisição de `robots.txt` em segundos. Padrão: `5`
- `robots_user_agent` — token de user-agent contra o qual as regras do `robots.txt` são avaliadas. Defina quando o stealth enviar um User-Agent de navegador, para que as regras avaliadas sejam as que valem para a requisição realmente enviada. Padrão: nenhum

### `scrape_*` (21)
- `scrape_charset_peek_bytes` — janela de inspeção usada para detectar o charset, em bytes. Padrão: `4096`
- `scrape_crawl_limit_max` — orçamento máximo de páginas do crawl, atuando como teto contra abuso para `--limit`. Padrão: `500`
- `scrape_crawl_max_depth` — profundidade máxima da busca em largura (BFS) para `crawl` e `map`. Padrão: `10`
- `scrape_dedup_similar` — colapsa páginas quase duplicadas por similaridade de conteúdo em crawl e batch-scrape. Padrão: `false`
- `scrape_dedup_similar_distance` — distância de Hamming do SimHash, de 0 até 64, abaixo da qual as páginas são quase duplicadas. Padrão: `3`
- `scrape_default_engine` — motor de scrape usado quando a CLI omite `--engine`, aceitando `http` ou `browser`. Padrão: `http`
- `scrape_delay_jitter_ratio` — razão de variação aleatória do atraso de polidez, de 0.0 até 1.0, com `0` desligando. Padrão: `0.2`
- `scrape_feed_max_entries` — número máximo de entradas mantidas pelo formato `feed` de scrape, cobrindo RSS, Atom e JSON Feed. Padrão: `50`
- `scrape_follow_rel_next` — segue links de paginação com `rel=next` durante o crawl. Padrão: `false`
- `scrape_honor_meta_robots` — respeita as diretivas `meta robots` e `X-Robots-Tag` do tipo `noindex`. Padrão: `true`
- `scrape_honor_nofollow` — pula links com `rel=nofollow` durante a descoberta do crawl. Padrão: `true`
- `scrape_http_cache_ttl_secs` — período de validade do cache L2 de respostas HTTP de scrape em segundos. Padrão: `3600`
- `scrape_max_body_bytes` — número máximo de bytes do corpo em scrape HTTP. Padrão: `5000000`
- `scrape_max_parse_bytes` — tamanho máximo de arquivo local aceito para parse antes da rejeição, em bytes. Padrão: `50000000`
- `scrape_max_text_chars` — número máximo de caracteres de texto ou markdown nos envelopes de scrape, com `0` removendo o teto. Padrão: `32768`
- `scrape_min_delay_ms` — atraso mínimo entre requisições GET de mesma origem em milissegundos. Padrão: `0`
- `scrape_no_cache` — ignora o cache de resposta na LEITURA e sempre busca na origem. A resposta nova continua sendo gravada, então uma chamada que faz bypass atualiza a entrada para quem vier depois em vez de deixar uma entrada velha. `--no-cache` no `scrape` sobrescreve por invocação. Não há como dizer o mesmo com `scrape_http_cache_ttl_secs`: o TTL `0` já significa "nunca expira", que é o oposto, e a chave o rejeita. O `monitor check` faz bypass incondicional e ignora esta chave, porque um corpo vindo do cache o fazia comparar uma página armazenada consigo mesma e reportar `changed: false`. Padrão: `false`
- `scrape_search_limit_max` — orçamento máximo de resultados de busca, atuando como teto contra abuso. Padrão: `50`
- `scrape_sitemap_max_bytes` — número máximo de bytes do corpo do sitemap. Padrão: `2000000`
- `scrape_summary_chars` — número máximo de caracteres do formato `summary` de scrape. Padrão: `400`
- `scrape_use_sitemap` — prefere o `sitemap.xml` ao mapear um site. Padrão: `true`

### `screencast_*` (4)
- `screencast_ffmpeg_framerate` — taxa de quadros de entrada do `ffmpeg` no screencast, em quadros por segundo. Padrão: `10`
- `screencast_jpeg_quality` — qualidade JPEG do screencast via CDP, de 1 até 100. Padrão: `60`
- `screencast_start_pump_iters` — iterações imediatas de bombeamento logo após `Page.startScreencast`. Padrão: `15`
- `screencast_stop_pump_iters` — iterações de esvaziamento antes de `Page.stopScreencast`. Padrão: `40`

### `state_*` (3)
- `state_collect_deadline_secs` — prazo externo da coleta de storage via CDP em segundos. Padrão: `5`
- `state_event_recv_secs` — fatia de recebimento de eventos de storage do CDP em segundos. Padrão: `2`
- `state_load_settle_ms` — atraso de estabilização após a navegação de `load_state`, em milissegundos. Padrão: `500`

### `stealth_*` (3)
- `stealth` — patches de anti-detecção aplicados antes da primeira navegação. `--no-stealth` desliga por uma execução. Padrão: `true`
- `stealth_profile` — identidade personificada: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`. `auto` segue o host, e é o único valor que não contradiz os hashes de Canvas e WebGL que a GPU real produz. Padrão: `auto`
- `stealth_seed` — fixa a identidade de stealth para reproduzir o mesmo fingerprint entre processos. A ausência significa identidade nova a cada execução, que é o padrão justamente porque cachear identidade a grava em disco. Padrão: nenhum

### `svg_*` (3)
- `svg_max_bytes` — número máximo de bytes do código-fonte SVG aceito antes da rasterização. Padrão: `4000000`
- `svg_max_depth` — profundidade máxima de aninhamento XML aceita em um código-fonte SVG. Padrão: `128`
- `svg_max_entities` — número máximo de declarações `<!ENTITY>` toleradas na DTD do SVG, com `0` rejeitando qualquer uma. Padrão: `0`

### `video_*` (5)
- `video_default_audio_bitrate` — taxa de bits padrão na conversão de vídeo para MP3, por exemplo `192k`. Padrão: `192k`
- `video_default_container` — contêiner padrão da conversão de vídeo, aceitando `mp4`, `webm`, `mkv`, `mov`, `avi` ou `m4v`. Padrão: `mp4`
- `video_default_crf` — valor CRF padrão, de 1 até 51, para recodificação de vídeo com perda. Padrão: `23`
- `video_download_max_bytes` — número máximo de bytes do corpo HTTP no download de vídeo. Padrão: `512000000`
- `video_max_input_bytes` — número máximo de bytes na materialização de vídeo vindo do stdin ou na verificação prévia do caminho. Padrão: `512000000`

### `webhook_*` (3)
- `webhook_max_attempts` — número máximo de tentativas de webhook, incluindo a primeira. Padrão: `3`
- `webhook_post_timeout_secs` — timeout do POST de webhook operacional em segundos. Padrão: `15`
- `webhook_retry_base_delay_ms` — atraso base de retentativa de webhook em milissegundos, dobrando a cada tentativa. Padrão: `50`

### Chaves avulsas (39)
- `allowed_roots` — raízes adicionais permitidas para leituras locais e escrita de artefatos, separadas pelo separador da plataforma, sendo que os padrões já cobrem o diretório atual, os diretórios XDG e o temporário. Padrão: nenhum
- `artifacts_dir` — diretório de saída dos artefatos gerados. Padrão: nenhum
- `color` — habilita cores ANSI na saída humana enviada ao stderr. Padrão: nenhum
- `dialog_settle_ms` — espera máxima após responder um diálogo JavaScript até o evento `javascriptDialogClosed`, em milissegundos. Padrão: `2000`
- `dom_stable_window_ms` — janela de silêncio usada por `wait --dom-stable-ms` em milissegundos. Padrão: `500`
- `drag_move_gap_ms` — atraso entre as posições sintetizadas de arrasto em milissegundos. Padrão: `16`
- `drag_move_steps` — número de posições intermediárias do mouse sintetizadas em um arrasto HTML5. Padrão: `6`
- `encryption_key` — material de chave para cifrar o estado de sessão. Padrão: nenhum
- `eval_drain_slice_ms` — fatia de esvaziamento durante a espera pelos resultados de `Runtime.evaluate`, em milissegundos. Padrão: `40`
- `event_pump_slice_ms` — fatia da bomba de eventos usada em `wait` e `eval`, em milissegundos. Padrão: `50`
- `event_tracker_max_entries` — tamanho do anel em memória do rastreador de console e rede por sessão de página. Padrão: `1000`
- `extension_attach_poll_iters` — iterações de sondagem ao anexar extensão; fatia vezes iterações é a espera total. Padrão: `20`
- `extension_attach_poll_ms` — fatia de sondagem ao anexar uma extensão, em milissegundos. Padrão: `150`
- `ffmpeg_path` — caminho absoluto do `ffmpeg`, opcional para codificar screencast e converter vídeo ou extrair MP3. Padrão: nenhum
- `ffmpeg_timeout_secs` — teto de tempo real da codificação `ffmpeg` em segundos, na faixa de 1 até 3600. Padrão: `120`
- `file_parse_cache_ttl_secs` — período de validade do cache L2 de parse de arquivo local em segundos. Padrão: `86400`
- `gif_max_frames` — número máximo de quadros de animação decodificados de um GIF. Padrão: `2000`
- `ignore_robots` — ignora `robots.txt` por padrão, sendo que as flags de risco continuam obrigatórias. Padrão: `false`
- `interact_settle_ms` — atraso de estabilização da interface após clique, digitação ou ação de extensão, em milissegundos. Padrão: `200`
- `lang` — sobrescreve o idioma das mensagens humanas, aceitando `en` ou `pt-BR`, com `pt` puro rejeitado. Padrão: nenhum
- `lighthouse_path` — caminho absoluto da CLI `lighthouse`. Padrão: nenhum
- `lighthouse_timeout_secs` — teto de tempo real da CLI `lighthouse` em segundos, na faixa de 1 até 3600. Padrão: `300`
- `manifest_max_bytes` — número máximo de bytes aceitos no corpo de um manifesto HLS ou DASH. Padrão: `8000000`
- `manifest_max_variants` — número máximo de entradas de variante ou representação emitidas por envelope de manifesto. Padrão: `500`
- `monitor_diff_max_bytes` — teto em bytes do payload de `monitor check --diff-mode`. Uma página reescrita por inteiro produz um diff com a página duas vezes, e quem chamou perguntou o que mudou, não pediu tudo. `diff_truncated` avisa quando o teto valeu, e `added_count` / `removed_count` seguem reportando o tamanho real. Padrão: `65536`
- `namespace` — namespace isolado de estado do produto. Padrão: nenhum
- `nav_micro_settle_ms` — microestabilização de navegação após transições de página, em milissegundos. Padrão: `100`
- `network_idle_window_ms` — janela de silêncio usada por `wait --network-idle` em milissegundos. Padrão: `500`
- `openrouter_api_key` — chave de API do provedor de LLM, armazenada com permissão `0600`. Padrão: nenhum
- `platform_child_poll_ms` — intervalo de sondagem de saída do processo filho durante o FINALIZE em milissegundos. Padrão: `50`
- `platform_child_wait_secs` — prazo de espera pelo processo filho da plataforma em segundos. Padrão: `5`
- `residual_orphan_min_age_secs` — idade mínima antes que um perfil marcador de dono morto se torne coletável, em segundos. Padrão: `60`
- `run_max_include_depth` — profundidade máxima de aninhamento nas cadeias de inclusão de `run --script`. Padrão: `16`
- `screen` — Tela da página `WxH` para `Emulation.setDeviceMetricsOverride` (`screen.width`/`screen.height`). Ausente = espelha o viewport. Argv `--screen` e o campo `screen` de emulate/resize no `run` ainda vencem. Nunca menor que o viewport. Padrão: nenhum
- `search_base_url` — URL base do endpoint HTML de busca, ao qual `?q=` é anexado. Padrão: `https://html.duckduckgo.com/html/`
- `shutdown_deadline_secs` — prazo rígido de desligamento aguardando a saída do navegador, em segundos. Padrão: `30`
- `support_settle_ms` — estabilização da thread de suporte para os auxiliares síncronos, em milissegundos. Padrão: `80`
- `timeout` — timeout global de execução em segundos. Padrão: `0`
- `user_data_dir` — diretório de perfil persistente do Chrome, opt-in. Ausente por padrão, e a ausência é o que preserva o residual-zero: o launch recebe um perfil descartável e a execução não deixa nada em disco. Defina apenas quando um detector atestar sessão entre invocações, porque perfil persistente é diretório que esta CLI nunca apaga por você. Criado com mode 0700 em Unix. `--profile` no argv vence esta chave. Padrão: nenhum

## Recursos
- Este crate não tem feature flags de Cargo
- Categorias opcionais são flags de processo, não features de compile-time
- `--category-memory` habilita ferramentas profundas de heap
- `--category-extensions` habilita ferramentas de extension
- `--category-third-party` habilita helpers DevTools de terceiros
- `--category-webmcp` habilita ferramentas webmcp
- `--experimental-vision` habilita `click-at`
- `--experimental-screencast` habilita export de screencast com ffmpeg

## Alvos
- Documentado para `x86_64-unknown-linux-gnu`
- Documentado para `x86_64-apple-darwin`
- Documentado para `aarch64-apple-darwin`
- Documentado para `x86_64-pc-windows-msvc`
- Documentado para `aarch64-unknown-linux-musl`
- Sem suporte em `wasm32-unknown-unknown` (CDP exige browser desktop)
- Metadados docs.rs declaram esses targets após a mudança multi-target de 2026-05-01

## MSRV (Rust mínimo)
- Minimum Supported Rust Version é 1.88.0
- Política: subir MSRV só em release minor ou major com nota no CHANGELOG
- Docs locais: `timeout 180 cargo doc --no-deps`

## Padrões de Integração
- Claude Code, Codex, Cursor e agentes de shell disparam um processo por ação
- Planos multi-passo de agentes devem usar `run --script` (NDJSON ou array JSON) em vez de encadear processos separados
- Parseie stdout com `jaq` e ignore stderr salvo em diagnóstico
- Streame o progresso dos passos com `--json-steps` quando o agente precisar de feedback progressivo
- Persista defaults duráveis com `config set` sob XDG
- Veja [INTEGRATIONS.pt-BR.md](INTEGRATIONS.pt-BR.md) e [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md)

## Performance
- Cold start é dominado pelo launch do Chrome, não pelo tamanho do binário Rust
- Prefira `doctor --offline --quick` para checagens de install sem rede
- Reutilize scripts multi-passo para evitar launches repetidos do Chrome
- Prefira `scrape --engine http` quando CDP não for necessário
- Use concorrência de `batch-scrape` para fetches paralelos (`--engine http` default; `--engine browser` por URL)

## Requisitos de Memória
- Espere memória do processo Chrome muito acima do binário da CLI
- Tools de heap exigem `--category-memory` e snapshots maiores elevam RAM
- Screencast pode invocar ffmpeg como helper externo
- Journals de workflow e capturas MITM ficam sob paths XDG de state/data

## FAQ de Troubleshooting
- Chrome não encontrado: instale Chromium ou Google Chrome, garanta o path do shell, ou `config set chrome_path`, e rode `doctor`
- Config / XDG: rode `config init` e depois `config path` para inspecionar o layout; use `config set|get` para valores
- Settings de produto só via flags e `config set` (XDG)
- Exit 69 unavailable: binário do browser ausente, bloqueado ou não lançável
- Exit 124 timeout: eleve `--timeout` ou encurte o script
- Exit 2 usage: confira flags com `browser-automation-cli help <cmd>`; com `--json` no argv, erros de usage do clap emitem envelopes JSON
- Refs `@eN` inválidas entre comandos: mantenha passos dentro de um `run`; refs não atravessam processos
- Network vazio: passe `--capture-network` no mesmo processo que navega
- Leitura de payload de API: `net get <IDX>` grava corpos com `--response-path` e `--request-path`, mas `net list` e `net get` só enxergam tráfego capturado no mesmo processo, então um `net get 0` avulso depois de um `goto` separado recusa com exit 2; ponha o passo `net` ao lado do passo `goto` em um script só e rode `browser-automation-cli --capture-network --json run --script /tmp/net.jsonl`
- Wait multi-text: repita `--text` para semântica OR (qualquer texto listado desbloqueia)
- Wait multi-seletor / URL: OR de CSS `#a, #b`; no `run` use `url` / `url_contains` / `navigation`
- View blank vazio: um about:blank vazio recusa sucesso silencioso a menos que você passe `--allow-empty` / `allow_empty:true`
- Bind MITM: `mitm start` / `mitm capture-url` escuta só em `127.0.0.1` com porta efêmera; flags globais `--mitm*`
- HAR do MITM: `mitm har --out <path>` (obrigatório); ou a global `--mitm-har` no FINALIZE; ou `capture-url --har`
- Redação MITM: `mitm redact` MOSTRA a política efetiva, `mitm redact --secrets true|false` persiste um default, e a global `--mitm-redact-secrets` vence as duas; a CA fica sob o diretório de dados XDG
- Workflow resume: `workflow resume` pula passos já `ok` no journal
- Scrape multi-formato: `--format markdown,html,links` (CSV ou repetível) devolve campos por formato; os 15 formatos vivos são `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed`, `attributes` (`raw-html` continua alias aceito de `rawHtml`)
- Formatos scrape browser: `--engine browser` aplica `--format` via outerHTML
- Engine browser em batch/crawl: `batch-scrape --engine browser` e `crawl --engine browser` (GAP-010)
- Aliases de scroll: em scripts `run` use `dy`/`dx` como aliases de `delta_y`/`delta_x`
- Descoberta de schema: `schema <cmd>` ou `schema --cmd goto|eval|type|scroll|assert` expõe flags tool-ref expandidas
- Lang: `--lang pt-BR` ou `config set lang pt-BR` localiza sugestões humanas
- Fail-fast com steps parciais: envelopes de erro de `run` podem incluir `data.steps` parciais
- Stream de JSON steps: `--json-steps` emite um objeto NDJSON por passo; o envelope final `--json` continua incluindo os `steps[]` completos
- Path do Lighthouse: flag, `config set lighthouse_path`, ou PATH; envelope `binary_source` é `real` ou `mock` (mock é honestidade de e2e, não produção)
- Redirects de search: `search` limpa wrappers `uddg=` para URLs de destino
- Parse de documentos: `parse` suporta PDF/DOCX/xlsx/ods e `--redact-pii`
- Extract LLM: exige XDG `openrouter_api_key` (opcionais `llm_base_url`, `llm_model`)
- Print PDF: `print-pdf --url <url> --path <file>` one-shot CDP
- Baseline de monitor: `monitor check --url <url> --baseline <file> [--write-baseline]`
- Assert de console: `assert console-empty` / `assert console-no-match --pattern …` (exige `--capture-console`)
- Aliases de assert: `url_contains` / `text_contains`; kinds `console_empty` / `console_no_match`; `attr` usa fallback de property DOM quando o atributo HTML é null
- Pick / select-option: nomes no inventário de agente; select nativo dispara input+change; HIG badge/popover / `role=option` via `pick`
- Submit / storage: `submit` para submit de formulário; `storage export|import` para cookies + estado por origem
- Tamanho do inventário: `commands --json` lista **71** nomes de agente (inclui `select-option`, `pick`, `submit`, `storage`, `image`+`video`+`audio`+`record`)
- Locale: `locale --json` diagnostica o idioma resolvido; defina com `--lang pt-BR` ou `config set lang pt-BR`
- `file://` + `scrape --engine http`: erro Usage — use engine browser ou `parse` para arquivos locais
- `reload --ignore-cache`: CDP `Page.reload` com `ignoreCache` (não é no-op em JS)
- Formatos de script `run`: `--script` é sempre um caminho de arquivo (JSON inline é recusado com exit 66); o arquivo é NDJSON um objeto por linha, ou um único array JSON de passos; suporta `wait_timeout_ms` e scrape `format`/`formats`
- Formatos do grab: `png|jpeg|webp` apenas (AVIF removido na 0.1.6)
- Cache Redis: defina `cache_backend redis` e `cache_redis_url`; nunca use `rediss://`
- Residual /tmp higiene de disco (0.1.5 RES-01…12 ainda verdadeiro na 0.1.7):
  - BORN auto-GC: `scavenge_stale_singleton_orphans` remove dirs `/tmp` `org.chromium.Chromium.*` Singleton-only com mais de 60s
  - FINALIZE dual scavenge + re-scan de dirs marker owned (prefixo `browser-automation-cli-chrome-`)
  - Nunca mata Chrome Flatpak do host nem processos de browser fora da CLI
  - Checagem doctor `residual_disk` + campo JSON de topo `residual` (`scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`)
  - Gates locais: `scripts/residual-check.sh`, `scripts/residual-stress.sh`
- Settle de diálogo: `dialog accept|dismiss` → leia `.data.dialog_settled`; orçamento via XDG `dialog_settle_ms`; isolamento multi-aba por `session_id` (com gate e2e)
- Lighthouse: fixtures unitárias incluem LHR 13.4.1 capturado do Chrome; caminho e2e mock permanece SKIP (somente contrato)
- Residual intencional: GAP-022 ~53 dups multi-versão transitivos; GAP-023/024 divergências de PRD registradas
- Utils de planilha/lint: `sheet-write <input> --out <arquivo>`, `sg-scan <paths>`, `sg-rewrite <paths>` recebem entradas posicionais; `find-paths --glob` para globs shell
- A ordem posicional de `find-paths` é `[PATTERN] [PATHS]...`, então um posicional solitário é lido como PATTERN regex e as raízes caem em silêncio no diretório atual; passe um pattern vazio para mirar uma raiz: `find-paths --glob '**/*.rs' '' src`
- Caminho soft de diálogo: `dialog accept --if-present` / `if_present:true` no run devolvem soft-ok quando nenhum diálogo está aberto

## Códigos de Saída
- `0` sucesso
- `2` usage ou falha de parse do clap
- `6` bloqueado — a origem serviu uma checagem de bot no lugar do conteúdo
- `64` capacidade desligada — falta uma flag de categoria ou de recurso experimental
- `65` erro de dados
- `66` sem entrada
- `69` indisponível
- `70` falha de software, browser ou protocolo
- `74` falha de I/O
- `75` pré-condição — a página ou a sessão não atende o comando
- `78` erro de config
- `124` timeout
- `130` cancelado por SIGINT
- `141` broken pipe
- `255` caminho fatal inesperado — rota de panic plausível, mas não mapeada por nenhum `error.kind` e não observável pela superfície de descoberta

## Mapa de Documentação
- [docs/HOW_TO_USE.pt-BR.md](docs/HOW_TO_USE.pt-BR.md) primeiro comando em 60 segundos
- [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md) contrato de integração para agentes
- [docs/COOKBOOK.pt-BR.md](docs/COOKBOOK.pt-BR.md) receitas práticas
- [docs/CONFIGURATION.pt-BR.md](docs/CONFIGURATION.pt-BR.md) cada chave XDG, seu padrão e seu propósito
- [docs/CROSS_PLATFORM.pt-BR.md](docs/CROSS_PLATFORM.pt-BR.md) matriz de plataformas
- [docs/STEALTH_PARITY.pt-BR.md](docs/STEALTH_PARITY.pt-BR.md) paridade anti-detecção contra as implementações de referência
- [docs/MIGRATION.pt-BR.md](docs/MIGRATION.pt-BR.md) notas de migração
- [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md) categorias de teste
- [docs/schemas/README.md](docs/schemas/README.md) índice de JSON schemas
- [docs/ARCHITECTURE.pt-BR.md](docs/ARCHITECTURE.pt-BR.md) organização dos módulos e internals do ciclo de vida
- [docs/ROADMAP.pt-BR.md](docs/ROADMAP.pt-BR.md) o que está planejado e o que está fechado por limite físico
- [PRIVACY.pt-BR.md](PRIVACY.pt-BR.md) o que permanece local e o que nunca é enviado
- [skills/browser-automation-cli-pt/SKILL.md](skills/browser-automation-cli-pt/SKILL.md) skill imperativa
- [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) histórico Keep a Changelog
- [SECURITY.pt-BR.md](SECURITY.pt-BR.md) reporte de vulnerabilidades
- [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) fluxo do contribuidor
- [CODE_OF_CONDUCT.pt-BR.md](CODE_OF_CONDUCT.pt-BR.md) Contributor Covenant 2.1
- [llms.pt-BR.txt](llms.pt-BR.txt) mapa curto de descoberta para LLMs

## Contribuindo
- Leia [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) antes de abrir um PR
- Siga o Código de Conduta em [CODE_OF_CONDUCT.pt-BR.md](CODE_OF_CONDUCT.pt-BR.md)

## Segurança
- Reporte vulnerabilidades em privado via [SECURITY.pt-BR.md](SECURITY.pt-BR.md)
- Contato do maintainer: daniloaguiarbr@proton.me

## Changelog
- O histórico de versões vive somente em [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md)

## Agradecimentos
- A equipe do Chrome DevTools Protocol, cujo contrato publicado é o que torna possível um cliente CDP one-shot sem daemon
- `chromiumoxide`, `hudsucker`, `clap`, `tokio`, `reqwest` e `feed-rs`, os crates sobre os quais esta CLI é construída
- O projeto Rust, por uma toolchain em que `clippy -D warnings` e `cargo deny` são baratos o bastante para rodar em todo gate
- Quem reporta falhas de segurança é creditado em [SECURITY.pt-BR.md](SECURITY.pt-BR.md) após divulgação coordenada; ainda ninguém
- Ainda não há contribuidores externos a creditar; o [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) descreve como isso muda

## Licença
- Dual license sob MIT OR Apache-2.0
- Veja [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE)
