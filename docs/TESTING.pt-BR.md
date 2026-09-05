[English](TESTING.md) | [Português Brasileiro](TESTING.pt-BR.md)

# Testes — browser-automation-cli

> Rode a suite certa para o risco, não todo path de browser por default.


## Por que Testes Categorizados
- Testes de runtime de browser são mais lentos e dependentes do host
- Testes de schema e inventário pegam drift de contrato sem Chrome
- Manter categorias explícitas protege a velocidade de iteração local
- Prefira validação local com cargo e scripts e2e


## Categorias de Teste
- Testes unitários e de library em `src/` (`cargo test --lib`)
- Smokes de CLI como `tests/doctor_cli.rs` e `tests/goto_smoke.rs`
- Gates de envelope e schema como `tests/envelope_schema.rs` e `tests/parity_toolref_schema.rs`
- Testes de inventário e matriz de paridade (`tests/parity_inventory.rs`, `tests/parity_matrix.rs`)
- Gate de inventário run: `tests/parity_run_inventory.rs` enforce `RUN_DISPATCHED_CMDS` ∪ exclude intencional (inclui `print-pdf`, `select-option`, `pick`)
- Gate de superfície clap: `tests/clap_command_debug_assert.rs` roda `Cli::command().debug_assert()`
- Testes de robots e comportamento de pipe (`tests/robots_http.rs`, `tests/pipe_broken.rs`)
- Helpers de golden i18n e cold-start (`tests/golden_i18n.rs`, `tests/cold_start.rs`)
- Cobertura e2e opcional de eventos CDP quando Chrome está disponível (`tests/e2e_cdp_events.rs`)
- Script e2e completo das **53 tools** DevTools (nome legado do arquivo): `scripts/e2e_all_52_tools.sh`
- Inventário vivo da CLI é **71 nomes de agente** (`commands --json`) — mais amplo que o conjunto e2e de 53 tool-ref; inclui inventário de agente `select-option` e `pick` (run/exec/schema, não clap), meta `locale` e `man`, mais clap `submit` e `storage`
- Gates de produto introduzidos na v0.1.7 e ainda vigentes na 0.1.9 (locais; Chrome serial quando necessário):
  - `tests/dialog_multitab_gate.rs` — isolamento multi-aba de diálogo + `dialog_settled` (GAP-054)
  - `tests/option_pick_gate.rs` — select nativo `input`+`change` (GAP-055)
  - `tests/wait_conditions_gate.rs` — honesty de prazo `wait_timeout_ms` (GAP-053)
  - `tests/scrape_step_gate.rs` — scrape `format`/`formats` em run sem monstro HTML (GAP-057)
  - Fixtures unit de lighthouse: `scripts/fixtures/lighthouse/minimal_lhr.json` + `chrome_captured_lhr.json` (parse LHR real de scores_from_lhr; GAP-021 parcial)
- Suite de integração residual: `tests/residual_one_shot.rs` (marker zero, não-crescimento de Singleton, wipe de fixture no BORN, campos residual no doctor)
- Gates locais residual: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (só scripts locais do mantenedor)
- Fixture vendored de tool-ref: `tests/fixtures/tool-reference.md`
- A suíte EXIGE `--test-threads=1`; nada no código impõe isso
- Medido: lançamentos concorrentes de Chrome produzem `SingletonLock: No such file or directory`, `No chromiumoxide Page for session_id` e `Page.navigate: Request timed out`
- Serial também é mais rápido aqui: 101s serial contra 148s paralelo
- Não existe dependência `serial_test` nem `#[serial]`, então `cargo test` puro, runner de IDE ou `cargo nextest` reproduzem essas falhas
- A suíte também exige que NADA MAIS toque cargo sobre o mesmo `target/` enquanto ela roda
- Medido em 2026-08-25: um `cargo run` lançado junto com a suíte reconstruiu `target/debug/browser-automation-cli` com features DEFAULT e trocou o binário em voo
- Quatro gates de `tests/image_media_cli_e2e.rs` então falharam com `rebuild with the image-svg Cargo feature` — afirmação verdadeira sobre o artefato errado, que se lê como regressão de produto
- `CARGO_BIN_EXE_*` NÃO protege contra isso: ele resolve em tempo de COMPILAÇÃO e garante qual build PRODUZIU o artefato, nunca quem sobrescreve esse caminho compartilhado depois
- Vermelho que some na reexecução isolada diagnostica a medição, nunca o produto; reexecute o teste falho sozinho antes de concluir regressão
- Este projeto usa ZERO `#[ignore]` INCONDICIONAL, deliberadamente: libtest reporta teste ignorado como `ok`, o mesmo falso verde que um skip produz
- A árvore carrega NOVE atributos `#[cfg_attr(…, ignore = …)]` em três arquivos de teste, e exatamente dois deles ignoram SOB `--all-features`: os `#[cfg_attr(feature = "…", ignore = "feature is on in this build")]` de `tests/image_wave6_codecs.rs` e `tests/video_manifest_hls_dash.rs`
- Os outros sete têm a forma `not(feature = "…")`, que sob `--all-features` não ignora nada; contagem afirmada sem o predicado que a mede conta o que o BUILD ignora e se lê como o que a ÁRVORE contém, e fica falsa na primeira feature nova
- Os dois asserem o comportamento fail-closed de um build SEM a feature, então sob `--all-features` a asserção é inalcançável por construção e o teste ainda roda de verdade no build que ele descreve
- É a mesma distinção que `strict-gates` faz: inalcançável por construção é desculpado, ferramenta ausente não é
- Teste que não pode rodar declina por `skip_with_reason` ou `skip_with_remedy` (`tests/common/mod.rs`), nunca por `eprintln!` cru
- Sob `--features strict-gates` esses helpers dão `panic!` em vez de retornar, então gate que declina FALHA em vez de reportar pass
- `scripts/ci-check.sh` roda com `--all-features`, o que liga `strict-gates`
- Declínio no lado unit usa `skip_unit_test` (`src/test_utils.rs`)
- Como declinar ali é falhar, `strict-gates` transforma toda ferramenta de host que um teste precisa em PRÉ-REQUISITO DURO
- Exigido no PATH para um run verde com `--all-features`: um Chrome ou Chromium que `find_chrome` resolva, `ffmpeg`, `ffprobe`, `Xvfb` (Linux), `/bin/sh` e `redis-server`
- `redis-server` guarda apenas `redis_real_server_if_present` (`src/cache/tests.rs`); o eixo do protocolo RESP em si é sempre coberto in-process por `redis_roundtrip_via_resp_mock`, que não precisa de binário externo
- Medido no Fedora 44: `dnf install redis` resolve para `valkey` mais `valkey-compat-redis`, que fornece `/usr/bin/redis-server` e satisfaz o gate sem alteração
- Ferramenta ausente NÃO é desculpada de propósito: instalá-la torna a asserção alcançável, e essa é a distinção que `strict-gates` existe para impor
- A forma oposta — asserção inalcançável por construção, como o host já ocupar o display que o teste precisa livre — declina com `eprintln!` cru mais justificativa escrita (`src/native/cdp/xvfb/spawn.rs`)


## Como Rodar
```bash
timeout 300 cargo test --locked
timeout 300 cargo test --lib --locked
timeout 120 cargo test --lib residual:: --locked
timeout 120 cargo test --test residual_one_shot --locked
timeout 120 cargo test --test parity_run_inventory --locked
timeout 120 cargo test --test clap_command_debug_assert --locked
timeout 180 cargo test --test dialog_multitab_gate --locked
timeout 180 cargo test --test option_pick_gate --locked
timeout 180 cargo test --test wait_conditions_gate --locked
timeout 120 cargo test --test scrape_step_gate --locked
timeout 120 cargo test --lib scores_from --locked
timeout 120 cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```
- Rode um arquivo com `cargo test --test doctor_cli --locked`
- Se `cargo test` abortar com stack overflow ao montar a árvore clap / schema, eleve a stack da thread de teste: `RUST_MIN_STACK=8388608 cargo test --locked` (8 MiB; threads de teste Rust costumam ser 2 MiB). Prefira isso a pular a suite
- Use `-- --nocapture` só durante debug
- Prefira library e gates de schema primeiro ao iterar contratos


## E2E 53 Tools
```bash
cargo build --release --locked
bash scripts/e2e_all_52_tools.sh
```
- Exige binário release em `target/release/browser-automation-cli` (faça `cargo build --release --locked` antes)
- Exercita tools de paridade DevTools na página fixture em `scripts/fixtures/e2e_page/`
- Escreve relatório em workdir temp e imprime contagens PASS/FAIL/SKIP
- Evidência do mantenedor para v0.1.4: 53 PASS / 0 FAIL em host local com Chrome (residual A001 fechado; GAP-001…025 hard-close)
- Evidência do mantenedor para v0.1.5: residual-zero em disco fechado (RES-01…12); `cargo test --lib residual::` + `cargo test --test residual_one_shot` + residual-check local PASS
- **Evidência do mantenedor para v0.1.6 (honesta):** `TOTAL=53 PASS=52 FAIL=0 SKIP=1` — caminho mock de lighthouse é **SKIP** (CONTRACT-ONLY). Nunca alegue PASS completo do parser lighthouse em e2e
- Confiança do parser lighthouse é em unit: `scores_from_lhr` contra `minimal_lhr.json` e `chrome_captured_lhr.json` real sanitizado (forma Lighthouse 13.4.1)
- A suite de 52-tools não substitui smokes residuais de comandos fora do conjunto tool-ref


## Gates Residual-Zero de Disco (lei da v0.1.5 — ainda corrente na 0.1.9)
```bash
cargo build --release --locked
cargo test --lib residual:: --locked
cargo test --test residual_one_shot --locked
bash scripts/residual-check.sh
# stress opcional de N one-shots:
# bash scripts/residual-stress.sh
```
- `residual_one_shot` cobre: marker CLI zero após goto, não-crescimento de Singleton Chromium após print-pdf, wipe BORN de fixture Singleton stale, campos residual no doctor
- `residual-check.sh` roda doctor (BORN GC path-light) + print-pdf one-shot + assert zero markers CLI e JSON `residual` do doctor
- `residual-stress.sh` repete trabalho one-shot para estressar higiene residual localmente
- Check id do doctor sob teste: `residual_disk` (higiene residual de disco path-light)
- Campo JSON de topo do doctor sob teste: `residual` (`ResidualDiskReport`)
- Campos residual do doctor sob teste: `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `ghost_marker_processes`, `foreign_root_orphans`, `scanned_roots`
- Contrato residual-zero de agente: `residual_disk` não pode ser `fail` (zeros em `orphan_marker_dirs` + `ghost_marker_processes`); após DIE sozinho também zero `cli_marker_dirs` + `chromium_tmp_singleton_orphans`; `sibling_live_processes>0` é concorrência saudável; **não** exija zero `live_cli_marker_processes`
- Age floor do GC stale de produção é 60s; testes podem usar helpers de library com age zero para fixtures


## Gates de Produto Introduzidos na v0.1.7 (dialog / select / wait / scrape / units lighthouse)
```bash
cargo test --test dialog_multitab_gate --locked
cargo test --test option_pick_gate --locked
cargo test --test wait_conditions_gate --locked
cargo test --test scrape_step_gate --locked
# Parse puro de lighthouse (sem alegar PASS e2e):
cargo test --lib --locked scores_from
# Residual ainda obrigatório:
bash scripts/residual-check.sh
```
- `dialog_multitab_gate`: isolamento tab1 + accept owner via gate multi-aba `Page::session_id`; asserta `dialog_settled` sem wait inventado (GAP-054)
- `option_pick_gate`: eventos nativos de select + `via: native_select` (GAP-055)
- `wait_conditions_gate`: prazo honra `wait_timeout_ms` (~2s, não default silencioso) (GAP-053)
- `scrape_step_gate`: scrape em run com `format=text` sem dump de HTML (GAP-057)
- Lighthouse e2e mock permanece SKIP; fixtures unit são o gate honesto do parser (GAP-021 parcial)
- **Encode do `grab`:** só png|jpeg|webp; AVIF removido (breaking) — smokes residuais não devem passar `--format avif`
- **Residual intencional GAP-024:** divergências wishlist de PRD seguem intencionais (não alegue paridade PRD completa)
- **Não** trate dashboards remotos de orquestração como superfície de produto; use só cargo local e `scripts/*-check.sh`


## Famílias de Gate Sob tests/
- `tests/` tem 67 arquivos de gate de integração, cada um rodado com `cargo test --test <name> --locked`
- Cada família abaixo fecha uma classe de defeito, nunca um comando
- Todo gate é local e não precisa de runner além do cargo
- Um gate sem sua pré-condição faz SKIP em voz alta em vez de aprovar em silêncio

### Fidelidade de Anti-Detecção e Stealth
- `tests/block_detection_gate.rs` — um muro de CAPTCHA precisa chegar ao envelope em vez de passar como conteúdo
- Ele exige exit `6`, um `error.kind` igual a `blocked` e `data.block_detection` juntos
- Uma página comum é o controle negativo e precisa voltar sem marcação
- O engine browser precisa reportar o mesmo bloqueio que o engine HTTP reporta
- `tests/input_trace_gate.rs` — asserta o que a página RECEBEU, não o efeito resultante
- O perfil `human` precisa entregar eventos `wheel` e de tecla reais com espaçamento não uniforme
- O perfil `direct` é o controle negativo e não emite wheel nem tecla sintéticos
- `tests/xvfb_gate.rs` — a alegação do doctor sobre Xvfb precisa casar com o que o host consegue
- Uma execução headed no Linux não pode deixar lock de display para trás
- Um host sem Xvfb é skip impresso, nunca execução vermelha
- `tests/compression_gate.rs` — todo content-coding anunciado precisa chegar decodificado
- Nenhum `content-encoding` pode sobreviver no envelope, o que prova que o corpo foi descomprimido

### Superfície de CLI e Higiene do Clap
- `tests/clap_arg_coverage.rs` — todo subcomando de topo renderiza help e argumento faltante vira erro
- Ele fixa os payload flags renomeados `fields-json`, `cookies-json` e `detailed` nos seus campos
- Ele também fixa que `image` não expõe ação de reconhecimento de texto
- `tests/clap_global_flag_collision.rs` — nenhum flag local de subcomando pode sombrear global long ou short
- `tests/help_description_gate.rs` — todo subcomando em toda profundidade carrega descrição não vazia
- Seu detector é função compartilhada também dirigida contra árvore sintética, provando a própria sensibilidade
- `tests/manpage_cli.rs` — `man` emite roff, grava atomicamente e rejeita path traversal

### Forma de Envelope e Taxonomia de Erro
- `tests/envelope_shape_gate.rs` — nenhum passo de `run` pode carregar `data` e `result` com conteúdo igual
- Um script fixo de dez passos precisa ficar dentro de um orçamento declarado de bytes
- `tests/view_precondition_gate.rs` — página em branco responde kind de pré-condição, nunca `usage`
- `--allow-empty` precisa ter sucesso e argv malformado precisa continuar respondendo `usage`
- `tests/config_key_diagnosis_gate.rs` — `config set` diagnostica a CHAVE independentemente do VALOR
- Uma chave desconhecida lê igual seja qual for a cara do valor
- `tests/devtools_envelope_behavior.rs` — campos de envelope dos comandos de paridade DevTools, offline e com Chrome
- `tests/grab_envelope_gate.rs` — `grab` reporta a largura e a altura que acabou de gravar
- O tamanho reportado precisa casar com o arquivo em viewport, full-page e elemento
- `tests/preflight_no_browser.rs` — um script inválido falha antes de o Chrome ser lançado
- A prova é residual: nenhum perfil marker aparece e a falha é muito mais rápida

### Gestos de Interação com Chrome Vivo
- `tests/drag_route_gate.rs` — o drag precisa alcançar o handler da página e reportar `route` como `intercepted`
- Uma página sem handler `dragstart` precisa reportar a rota sintética degradada e avisar
- `tests/submit_form_gate.rs` — `submit` dispara o evento do form e espera a navegação
- Pressionar o botão sósia tem sucesso sem submeter nada, que é o defeito sendo separado
- `tests/upload_cdp_e2e_gate.rs` — o arquivo realmente chega a um input de um Chrome real
- Ele fixa `--script -` lendo stdin e `--script` recebendo caminho, nunca JSON inline
- `tests/extract_step_gate.rs` — dois seletores em um run precisam resolver para dois nós diferentes
- `tests/cookie_jar_gate.rs` — um cookie gravado é achado por um `list` posterior, e `clear` esvazia mesmo
- `tests/dialog_if_present_gate.rs` — a ausência é tolerada com a flag e fatal sem ela
- `tests/eval_typed_gate.rs` — retorno de objeto chega como estrutura, e `typed` reporta `value_type`
- `tests/ref_invalidation_gate.rs` — o marcador de staleness `@eN` aparece só quando a árvore realmente mudou
- `tests/assert_step_gate.rs` — todo kind de `assert` pode REPROVAR o run e alcançar o exit code
- `tests/record_gate.rs` — o NDJSON que `record` grava replaya por `run --script` sem tradução

### Contenção de Filesystem e Evidência de Falha
- `tests/allowed_roots_gate.rs` — caminho local fora das raízes permitidas é recusado como política
- A recusa não pode ser classificada como `usage`, e `--allow-outside-roots` restaura o acesso
- `tests/failure_dump_gate.rs` — um run que falha grava em disco os anéis de console e rede capturados
- Um run bem-sucedido, ou um run que falha sem a flag, não pode deixar artefato

### Ciclo de Vida de Processo e Contabilidade Residual
- `tests/lifecycle_group_kill.rs` — um sinal ceifa o grupo de processos inteiro, com fallback por árvore de pid
- Ele recusa o nosso próprio grupo, o `init` e o grupo zero
- `tests/lifecycle_hard_kill_gate.rs` — `SIGKILL` na CLI não deixa processo de browser do seu grupo
- `tests/signal_shutdown.rs` — SIGTERM e SIGINT contra um filho vivo da CLI não podem travar
- `tests/residual_report_contract.rs` — pids são contados uma vez e processos impostores não são contados
- O relatório emite as raízes que varreu e reporta órfãos de raiz estrangeira em separado
- Um pid de dono vivo protege uma invocação concorrente da coleta

### Comportamento de Scrape, Crawl e Feed
- `tests/scrape_agent_native_gate.rs` — projeção offline, flag de truncagem, filtros de path e promoção de formato
- `tests/scrape_wave6_gate.rs` — `--format feed`, seguimento de `rel=next` e colapso de quase duplicatas
- Cada comportamento vem com seu controle DESLIGADO, então implementação incondicional reprova
- `tests/scrape_wave7_e2e_gate.rs` — `rel=next` em âncora, JSON Feed e `batch-scrape --dedup-similar`
- A descoberta de paginação nunca pode passar por cima de um `Disallow` do robots
- `tests/crawl_plan_llms_txt_gate.rs` — `crawl --dry-run` resolve o plano e não busca nada
- Ele é apontado para host inalcançável, então um envelope de sucesso É a evidência

### Pipelines Locais de Mídia
- `tests/audio_local_gate.rs` — inventário, ações de schema, bloqueio SSRF e conversão quando há ffmpeg
- `tests/image_wave6_codecs.rs` — encode AVIF, HEIC fail-closed, raster SVG, quadros GIF, backend de resize
- `tests/image_metadata_iptc_xmp.rs` — leitores IPTC IIM e XMP mais o gate de ameaça do sanitizador SVG
- `tests/image_media_cli_e2e.rs` — as mesmas superfícies wave-6 dirigidas pelo binário real
- Uma feature implementada porém inalcançável a partir do `main` é o que ele pega
- `tests/video_manifest_parse.rs` — o parser HLS e DASH em nível de library, sem buscar nada
- `tests/video_manifest_hls_dash.rs` — escadas de variante, resolução de URI relativa e o teto de bytes
- `tests/video_manifest_cli_gate.rs` — `video manifest` é ação anunciada e alcançável pelo argv
- `tests/video_site_extraction_rejected.rs` — página de player e manifesto recebem erros distintos e acionáveis

### Paridade, Propriedades e Logging Local
- `tests/parity_semantics.rs` — a terceira camada de paridade: pré-condição e efeito, não só nome
- Ele faz SKIP em voz alta quando a árvore de referência e `docs_prd/` faltam no checkout
- `tests/proptest_parsers.rs` — testes de propriedade para parsers offline, corpo de robots e round trip de envelope
- `tests/tracing_local_log_schema.rs` — nomes de campo das linhas JSON rotacionadas sob `config set log_to_file`
- As linhas são só arquivos locais; este produto não tem telemetria remota


## Inventário completo de agente (71)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract feed fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write sitemap storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`; a contagem de subcomandos clap de produto é **69** (71 nomes de agente − 2 só-run).

Gate local de honesty do inventário (sem GHA): após editar inventário ou listas planas, rode `bash scripts/inventory-flat-check.sh` (espera `commands --json` com **71** nomes incluindo `image`+`video`+`audio`+`record`).

O gate agora se chama `scripts/inventory-flat-check.sh`. O nome antigo `scripts/verify-inventory-flat.sh` permanece como shim fino que delega para ele. Motivo: `scripts/ci-check.sh` descobre verificadores pelo glob `scripts/*-check.sh`, e o nome antigo nunca casava com esse glob, então o gate jamais rodou no bundle e os docs derivaram para a contagem obsoleta 67 enquanto o runner reportava verde.

## Smokes Residuais de PRD (além das 53 tools)
Rode após o e2e ao validar o inventário completo de **71** nomes:

```bash
# print-pdf artifact (one-shot + run)
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf

# monitor baseline check
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline

# QR encode/decode (sem Chrome)
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png

# Pipeline local de imagem (sem Chrome; agent-native — sem base64 de pixels)
browser-automation-cli --json image download 'https://www.w3.org/People/mimasa/test/imgformat/img/w3c_home.png' -o /tmp/w3c.png
browser-automation-cli --json image info --path /tmp/w3c.png --select format,width,height,sha256
browser-automation-cli --json image convert --path /tmp/w3c.png --format webp -o /tmp/w3c.webp
browser-automation-cli --json image exif --path /tmp/w3c.webp --select tags,path  # alias tags→exif; só EXIF
# AVIF/HEIC: magic reject (sem encode pure-Rust). SVG: use --allow-non-image para bytes crus (sem resvg).
# image download = URL de uma imagem (SSRF+magic) — NÃO é download de árvore de site inteiro.
# Upload precisa de Chrome + input de arquivo já navegado (dry: schema upload):
browser-automation-cli schema upload >/dev/null
# browser-automation-cli --json run --script '[{"cmd":"goto","url":"…"},{"cmd":"upload","target":"input[type=file]","path":"/tmp/w3c.webp"}]'
# O fuzzing dos parsers de magic é um gate normal — sem nightly, sem libFuzzer, sem binário à parte:
cargo test --test fuzz_magic_parsers_gate
#   Corpus xorshift determinístico: prefixos reais de container (PNG/JPEG APP1+APP13/GIF/RIFF/
#   ISOBMFF ftyp/Matroska/OGG/FLAC/ID3/ADTS/WAV/AIFF), truncados e com bits invertidos, alimentando
#   image_local::detect_format, video_local::detect_container e audio_local::detect_container.
#   A propriedade é que eles classificam ou devolvem erro tipado — nunca entram em pânico nem travam.
#   A receita antiga de `cargo fuzz` nunca foi executável aqui: exige nightly, exige libFuzzer do
#   LLVM (dependência C++ num crate rust-native), e nenhum gate a invocava.

# Pipeline local de vídeo (sem Chrome; precisa ffmpeg/ffprobe no host para convert/to-mp3/trim/thumbnail)
# Gate de integração (pula convert/trim/thumbnail se ffmpeg ausente):
#   cargo test --test video_local_gate --locked
# ffmpeg -y -f lavfi -i testsrc=duration=0.5:size=160x120:rate=10 -c:v libx264 -pix_fmt yuv420p /tmp/in.mp4
browser-automation-cli --json video info --path /tmp/in.mp4 --select container,duration_secs,streams
browser-automation-cli --json video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm  # auto re-encode se copy incompatível
browser-automation-cli --json video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3
browser-automation-cli --json video trim --path /tmp/in.mp4 --start 0 --duration 0.2 -o /tmp/clip.mp4
browser-automation-cli --json video thumbnail --path /tmp/in.mp4 --at 0.1 -o /tmp/thumb.png
# Resumo de manifesto não precisa de ffmpeg: estrutura HLS .m3u8 / DASH .mpd, zero mídia baixada
browser-automation-cli --json video manifest --path /tmp/master.m3u8
browser-automation-cli schema video >/dev/null

# find-paths (sem Chrome)
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .

# sheet-write / sg-scan / sg-rewrite (no Chrome)
printf 'a,b\n1,2\n' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
browser-automation-cli --json sg-scan . --limit 20
browser-automation-cli --json sg-rewrite .

# run JSON array + json-steps stream (GAP-020)
cat > /tmp/demo.array.json <<'JSON'
[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.array.json

# wait multi-selector / url_contains (GAP-019/024)
cat > /tmp/wait.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1, body","ms":3000},
  {"cmd":"wait","url_contains":"example.com","ms":3000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait.json

# pick / select-option (run-only inventory; GAP-023)
# browser-automation-cli --timeout 60 --json run --script '[{"cmd":"goto","url":"…"},{"cmd":"pick","target":"…","option":"…"}]'

# assert console kinds (GAP-025)
# browser-automation-cli --capture-console --timeout 60 --json run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"assert","kind":"console_empty"}]'

# schema positional (GAP-022)
browser-automation-cli --json schema run
browser-automation-cli --json schema --cmd wait

# view --allow-empty (GAP-012)
browser-automation-cli --json view --allow-empty

# multi-format scrape + batch/crawl browser engine (GAP-009/010)
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http
printf '%s\n' 'https://example.com' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --engine http --concurrency 1
# browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --engine browser --concurrency 1

# MITM capture-url + har --out (GAP-011)
browser-automation-cli --json mitm init-ca
# browser-automation-cli --json mitm capture-url https://example.com --seconds 15 --har /tmp/cap.har
# browser-automation-cli --json mitm har --out /tmp/capture.har
# browser-automation-cli --json mitm redact

# config list-keys + redis honesty (no rediss)
browser-automation-cli --json config list-keys
# browser-automation-cli --json config set cache_backend redis
# browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379

# lighthouse binary_source (mock)
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh | jaq '.data.binary_source // .'

# parse PDF / DOCX with optional PII redact
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii

# extract --llm fail-closed without XDG key
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
# expect usage envelope requiring: config set openrouter_api_key

# clap JSON usage error (GAP-002)
browser-automation-cli --json not-a-real-command 2>/dev/null | jaq -e '.ok == false' || true

# caminho suave de dialog
browser-automation-cli --json dialog accept --if-present
# console dump sempre []
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
# superfície de help da flag beforeunload
browser-automation-cli goto --help | rg handle-before-unload
# contexto isolado de página
browser-automation-cli page new --help | rg isolated-context
# print-pdf dentro de run
# cat > /tmp/pdf.run.json <<'JSON'
# [{"cmd":"goto","url":"https://example.com"},{"cmd":"print-pdf","path":"/tmp/page-from-run.pdf"}]
# JSON
# browser-automation-cli --timeout 60 --json run --script /tmp/pdf.run.json
# schema já coberto

# locale / man meta + submit/storage/image/video/audio/record (inventário 71)
browser-automation-cli --json locale
browser-automation-cli --json man >/tmp/browser-automation-cli.1
browser-automation-cli --json schema submit
browser-automation-cli --json schema storage
browser-automation-cli --json config list-keys
browser-automation-cli --json config set dialog_settle_ms 2000

# campos residual do doctor (lei da v0.1.5 ainda corrente)
browser-automation-cli doctor --offline --quick --json | jaq '.residual'
```
- Também úteis: scrape browser com format, `config path`, `mitm start`, doctor XDG, i18n `--lang pt-BR`
- Testes de contrato a citar em evidência: `parity_run_inventory`, `clap_command_debug_assert`, `residual_one_shot`, testes lib residual, `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`


## Mock de Lighthouse (honesty SKIP em e2e)
```bash
browser-automation-cli --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Use `--lighthouse-path` ou XDG `lighthouse_path` apontando para `scripts/mock-lighthouse.sh` quando não houver Lighthouse real
- Ordem de resolve: flag → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- O mock grava reports HTML/JSON mínimos para paths de smoke
- Doctor reporta presença/origem de lighthouse como informativo quando o binário está ausente
- **Honesty v0.1.6:** a suite e2e faz **SKIP** do caminho mock de lighthouse — nunca reporte isso como PASS completo do parser
- Confiança do parser: testes unit em `scripts/fixtures/lighthouse/minimal_lhr.json` e `chrome_captured_lhr.json`


## Perfis de Validação Local
- Rode fmt, clippy e testes de contrato sem browser primeiro na sua máquina
- Testes com browser exigem Chrome ou Chromium instalado localmente
- A validação roda localmente com cargo e scripts e2e na máquina do mantenedor
- Mantenha publish no crates.io bloqueado sem aprovação explícita do mantenedor
- Smokes opcionais de pilares após e2e: `run` + `--json-steps`, comandos residuais de PRD acima, residual-check, `config path`, `mitm capture-url`, doctor XDG + residual


## Auditoria de Schemas e Documentação Bilíngue
```bash
cargo build --release --locked
bash scripts/generate_command_schemas.sh
bash scripts/generate_command_schemas.sh --check
bash scripts/audit_bilingual_docs.sh
```
- `generate_command_schemas.sh` grava um `docs/schemas/<cmd>.schema.json` por comando do inventário a partir de `schema --cmd` ao vivo (superfície de meta.rs)
- `--check` falha quando schemas estáticos de comando divergem do binário instalado
- Envelopes e `run-script-step.schema.json` são preservados e não sobrescritos pelo gerador
- `audit_bilingual_docs.sh` compara invocações de `browser-automation-cli` dentro de fences de código entre pares EN e `.pt-BR`
- Exit `0` significa multisets de fences iguais; exit `1` significa drift; exit `2` significa par de arquivo ausente


## Gate de Contrato de Binário Agent-Ops
```bash
cargo build --release --locked
bash scripts/agent-ops-check.sh
cargo test --test agent_ops_cli --locked
```
- `agent-ops-check.sh` roda o binário compilado, nunca as funções internas
- Ele asserta que teto de saída impossível reporta exit `2` com envelope
- Ele asserta que teto plausível emite payload ou erro, nunca silêncio
- Ele asserta que `--fields`, `--sort-rows` e `--dedupe-by` nomeiam o caminho não resolvido
- Ele asserta que um `--fields` que resolve mantém o envelope quieto
- Ele asserta que mensagens de sugestão citam só flags globais, em EN e pt-BR
- `tests/agent_ops_cli.rs` acrescenta 10 testes de integração dirigidos por argv
- A cobertura das oito flags agent-ops sob `tests/` era zero absoluto antes
- `scripts/ci-check.sh` descobre este gate pelo glob `scripts/*-check.sh`
- `verifier-controls-check.sh` carrega 1 controle positivo deste gate
- O script resolve o binário com fallback pelo PATH de propósito
- O harness de controles copia a árvore sem `target/`, então a busca não pode abortar
- Um gate que aborta antes da primeira asserção é indistinguível de um gate que aprova
- Lei do projeto: um controle que não falha é um verificador que não verifica
- Estado medido do bundle: `ci-check OK (all steps passed)` com 249 PASS e 0 FAIL


## Gate de Cobertura de Documentação
```bash
cargo build --release --locked
bash scripts/doc-coverage-check.sh
```
- `doc-coverage-check.sh` lê a superfície viva do binário, nunca uma lista transcrita
- Asserção 1: ambos os documentos `CONFIGURATION` cobrem toda chave XDG viva
- Asserção 2: nenhum documento ainda ensina chave de configuração aposentada
- Asserção 3: documentos de entrada nomeiam todos os comandos vivos
- Asserção 4: todo documento público tem espelho `.pt-BR`
- Asserção 5: nenhum documento apresenta flag de comando como global
- Asserção 6: nenhum documento ensina variável de ambiente de produto
- Asserção 7: todo link `llms` resolve para arquivo real
- Um contador local protege a linha de flags de envelope contra falha anterior
- `scripts/ci-check.sh` descobre este gate pelo glob `scripts/*-check.sh`
- `verifier-controls-check.sh` carrega 3 controles positivos deste gate
- O script resolve o binário com o mesmo fallback de PATH pelo mesmo motivo
- LIMITAÇÃO CONHECIDA de todo este conjunto de gates: nenhum gate de documentação EXECUTA bloco cercado
- O `audit_bilingual_docs.sh` compara as invocações de CLI entre os dois idiomas e nunca as roda
- Uma receita publicada que sai diferente de zero passa, portanto, por todos os gates de documentação
- Medido em 2026-08-28: quatro receitas `run --script` do `docs/COOKBOOK.md` saíram diferente de zero com os cinco gates verdes
- As causas foram elemento ausente da página alvo, contradição de perfil stealth que o produto recusa de propósito, e dois blocos supondo arquivos de entrada que eles nunca criam
- NENHUMA era erro de chave de passo, então uma checagem estática de todo passo documentado contra `schema <comando>` não teria achado nenhuma delas
- Essa checagem estática foi escrita e rodada sobre 327 passos documentados: ela produziu seis achados e os seis eram falsos positivos
- Receita se verifica EXECUTANDO, e essa execução é manual hoje


## Harness de Controles de Verificadores
- `scripts/verifier-controls-check.sh` é o meta-gate: ele prova que os demais verificadores conseguem FALHAR
- Lei do projeto: um controle que nunca falha é um verificador que não verifica
- Cada controle copia a árvore de trabalho para um sandbox descartável e a muta
- A mutação é escolhida para que o verificador sob teste TENHA de reportar falha
- Um verificador que continua verde sob a própria mutação está cego, e o harness diz isso
- `bash scripts/verifier-controls-check.sh` executa; o `ci-check.sh` também o descobre pelo glob `scripts/*-check.sh`
- A contagem de controles é derivada em tempo de execução contando `^run_control `, nunca congelada em prosa
- Este é o passo mais caro do pacote: uma cópia inteira da árvore por controle
- A cópia usa `fd -H -I`, então caminhos escondidos pelas regras de ignore do git seguem presentes no sandbox
- Sem o `-I` o sandbox ficava sem `docs_prd/`, `gaps.md` e `CLAUDE.md`, que é a cópia parcial contra a qual o próprio cabeçalho adverte
- Matá-lo no meio deixa um diretório `bac-verifier-control-*` para trás, porque a limpeza só roda nos caminhos de retorno


## Escala da Suíte Completa de Verificadores
- `scripts/` tem **44** arquivos `.sh` de topo; `tests/` tem **92** arquivos `.rs` de gate
- Medido em 2026-09-01 com `fd -d 1 -e sh . scripts/` e `fd -d 1 -e rs . tests/`
- `tests/doc_measured_claims_gate.rs` remede os dois, porque o par de 2026-08-28 dizia 42 e 73 e envelheceu sem ninguém notar
- O `-d 1` faz parte da medição e não é enfeite: sem ele a contagem de `scripts/` alcança subdiretórios e devolve 45
- `bash scripts/ci-check.sh` é o runner local do bundle
- Ele descobre sozinho todo `scripts/*-check.sh` executável por esse glob
- Um script cujo nome não termina em `-check.sh` nunca entra no bundle e precisa ser invocado pelo nome
- Ferramentas de medição e geradores têm nome assim de propósito: eles reportam ou escrevem, não assertam
- Prefira rodar o bundle inteiro antes de release, e um verificador isolado durante a iteração


## Verificadores de Conformidade Rust
- `scripts/interior-mutability-check.sh` — rejeita `static mut` e formas `Arc<RefCell>` / `Rc<RefCell>` sob `src/`
- `scripts/memory-check.sh` — higiene de RAII, ownership e alocação; rejeita `std::process::exit` sob `src/`
- `scripts/ownership-check.sh` — regras de ownership, borrowing e lifetime, ancoradas nos módulos de session interact e de launch e tabs nativos
- `scripts/macros-check.sh` — higiene de macros declarativas e embutidas; rejeita `todo!(`, `unimplemented!(` e `dbg!(` deixados em produção
- `scripts/json-ndjson-check.sh` — regras de JSON e NDJSON; asserta `serde_json` como único parser de produção e rejeita `simd-json` / `sonic-rs`
- `scripts/network-check.sh` — regras de rede para CLI de agente one-shot: só CLI mais XDG, `no_proxy`, política SSRF e tetos de corpo, ancorado nos módulos de robots e de descoberta CDP
- `scripts/process-check.sh` — regras de execução de processo externo: helper de captura com timeout, defesa BatBadBut, nenhum spawn de shell em produção
- `scripts/parallelism-check.sh` — paralelismo limitado; exige `src/concurrency/` e rejeita `Box::leak` / `mem::forget` dentro dele
- `scripts/shutdown-check.sh` — encerramento gracioso em torno de `run_from_args`, incluindo o handler de SIGPIPE e o flush duplo
- `scripts/tracing-check.sh` — regras de logging e rotação; `--inventory-only` roda só a metade estática
- `scripts/multiplatform-check.sh` — rejeita shell-out para `which` / `where`, que deve passar por `platform::which_bin`
- `scripts/natives-check.sh` — allowlist de crates nativas; proíbe CLIs humanas sob `src/` fora dos binários de domínio (chrome, lightpanda, lighthouse, ffmpeg, redis-server em testes), e proíbe `openssl` (TLS permanece só rustls) e `nasm-rs` no `Cargo.lock`
- `scripts/filesize-check.sh` — teto de 300 linhas de **código** por arquivo de produção; prosa de rustdoc não é código, então contar linhas físicas premiaria apagar documentação; exceções declaradas carregam versão de expiração e reprovam como qualquer outro arquivo depois dela
- `scripts/orphan-module-check.sh` — todo `src/**/*.rs` deve ser alcançável a partir de uma raiz de crate; um arquivo que nenhum pai declara com `mod` fica ausente do binário enquanto build, clippy e a suíte inteira seguem verdes
- `scripts/reachability-check.sh` — itens `pub use` sem call site sob `src/`; o lint `dead_code` para na fronteira do crate, então um item reexportado que ninguém chama permanece silencioso
- `scripts/split-conservation-audit.sh` — recebe pares `<original.rs> <new_dir>` e asserta que toda linha significativa de um arquivo pré-split ainda existe sob o diretório que o substituiu; dividir `commands/ops/lighthouse.rs` apagou em silêncio um doc comment `pub(crate)` enquanto build, clippy, `cargo doc -D warnings` e a suíte inteira seguiam verdes, porque `missing_docs` dispara em documentação AUSENTE de item público e é cego a documentação APAGADA de item que ele não cobre


## Verificadores de Superfície e Paridade de Esquema
- `scripts/clap-schema-parity-check.sh` — compara o **parser** clap contra o esquema publicado, o eixo que o `schema-drift-check.sh` estruturalmente não enxerga porque os dois lados dele derivam do mesmo módulo de esquema; medidos 29 flags aceitos pelo clap e ausentes do `schema`, incluindo o obrigatório `storage export --path`
- `scripts/schema-drift-check.sh` — adaptador fino sobre o `--check` do gerador; o runtime é a fonte da verdade e `docs/schemas/*.json` é artefato derivado; a capacidade existia muito antes da ligação, e sete esquemas tinham derivado enquanto toda auditoria reportava verde
- `scripts/config-roundtrip-check.sh` — toda chave de `CONFIG_KEYS` precisa existir no gravador e no leitor (veja v0.1.8 abaixo)
- `scripts/phantom-flag-gate.sh` — adaptador bash sobre `tests/phantom_flag_gate.rs` (veja v0.1.8 abaixo)


## Verificadores de Documentação e Localização
- `scripts/docs-check.sh` — pipeline local de validação docs.rs: `cargo check`, `cargo doc --no-deps --features docs-mermaid`, fases opcionais de doc nightly e de rustdoc JSON, depois auditoria de cobertura da feature e dos metadados; progresso NDJSON no stdout e logs humanos no stderr; exit `0` todas as fases passaram, `65` auditoria de documentação ou metadados falhou, `70` falha de build
- `scripts/i18n-check.sh` — paridade de ids de mensagem entre `locales/en.ftl` e `locales/pt-BR.ftl`, mais a superfície unit de i18n
- `scripts/doc-coverage-check.sh` — prosa contra a superfície viva do binário; veja a seção Gate de Cobertura de Documentação acima


## Verificadores de Recursos e Desempenho
- `scripts/perf-check.sh` — higiene de performance: inventário de perfil mais smoke de build release; `--inventory-only` pula o rebuild, `--rss` encadeia a baseline de RSS, `--bench` roda o bench lento `cli_parse`
- `scripts/latency-check.sh` — gates de higiene de latência; `--baseline` acrescenta números de relógio, `--inventory-only` mantém tudo estático
- `scripts/latency-baseline.sh` — ferramenta de medição, não gate; reporta P50, P99, P999, P9999 e máximo sobre caminhos meta de agente, para que o boot do Chrome nunca mascare uma regressão de Rust; preserva outliers e sempre sai com `0` depois de imprimir
- `scripts/rss-baseline.sh` — ferramenta de medição; lê `Maximum resident set size` de `/usr/bin/time -v` para o binário release
- `scripts/profile-cdp.sh` — profiling local de um `goto about:blank` one-shot; tempo de relógio por padrão, com flamegraph ou samply opcionais; nenhum profile commitado


## Geradores (não são verificadores)
- `scripts/gen-completions.sh` — congela completions de shell em `target/completions/` para empacotamento de distro; em runtime as completions continuam vindo de `browser-automation-cli completions <shell>`
- `scripts/gen-llms-txt.sh` — regenera o bloco de inventário legível por máquina do `llms.txt` a partir de `commands --json` ao vivo, substituindo o bloco gerado anterior e deixando a prosa intacta
- `scripts/gen-flag-reconciliation.sh` — reconcilia os flags que o PRD declara globais contra onde a capacidade de fato vive, classificando cada um como `global`, `local`, `xdg` ou `absent` (GAP-023); `xdg` deliberadamente NÃO é sinônimo de `global`, porque chave XDG é por host e não varia por invocação, então fundir os dois reportaria um gap como fechado enquanto o controle por invocação segue ausente
- Nenhum dos dois asserta nada, então nenhum pertence ao glob `scripts/*-check.sh`


## Verificadores da v0.1.8 e os Defeitos que Fecham
- `scripts/config-roundtrip-check.sh` — asserta que toda chave literal de `CONFIG_KEYS` está presente no gravador `src/xdg/config_write.rs` e no leitor `src/xdg/config_io.rs`; chaves de política promovidas ficam isentas porque uma tabela de macro as gera
- A classe que ele fecha: `config set` reconstrói o `config.toml` inteiro a partir de um template escrito à mão, então uma chave ausente dele é descartada em vez de preservada, e o `match` escrito à mão do leitor a joga fora na carga
- A falha era silenciosa da pior forma: `config set` respondia `ok: true` com exit `0` e o processo seguinte lia `null` de volta
- Medido em 2026-08-09: dezessete chaves ausentes das duas metades. Medido em 2026-08-10: **mais seis** ainda ausentes — `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `stealth_seed`, `robots_user_agent`
- Duas dessas seis são credenciais de proxy que a documentação manda o operador guardar na configuração justamente porque o argv é visível na tabela de processos; o canal seguro descartava o valor e deixava o canal vazado como o único que funcionava
- Por que não um teste Rust: um teste de round-trip precisa de um valor que passe validação por chave e degenera em amostra hardcoded, que é exatamente como `tests/v018_parity_gate.rs` deixou passar as seis; a invariante é comparação estática de conjuntos e pertence a uma checagem estática
- `scripts/phantom-flag-gate.sh` — adaptador que expõe uma propriedade de `tests/phantom_flag_gate.rs` ao runner de controles, que dirige todo controle com `bash $script`; ele não carrega asserção própria e aceita um filtro do libtest
- Ele deliberadamente **não** se chama `*-check.sh`: esse glob rodaria as mesmas propriedades uma segunda vez, e um segundo verde não acrescenta informação e custa o link inteiro de um binário de teste
- O scan original era um script Python de 411 linhas; foi portado para Rust e deletado, porque o produto é Rust de ponta a ponta
- `scripts/natives-check.sh` — fixa a allowlist de crates nativas e proíbe `openssl` e `nasm-rs`; o primeiro quebraria a lei de TLS só rustls, o segundo tornaria um NASM de sistema requisito de build
- Uma nova dependência nativa precisa entrar em `SYS_ALLOWLIST` com justificativa ou ser removida
- `scripts/doc-coverage-check.sh` — lê o binário vivo e reprova quando a prosa deriva da superfície viva de chaves ou de comandos; veja a seção própria acima para as sete asserções


## Gates de Afirmação Contra o Binário
- Estes quatro gates vivem em `tests/` e nenhum documento de `docs/` os descrevia até agora
- Cada um compara uma afirmação escrita em prosa contra a superfície que o binário vivo publica
### tests/doc_binary_numeral_gate.rs
- Rode com `cargo test --test doc_binary_numeral_gate --locked`
- Uma contagem de chaves de configuração escrita em prosa é uma afirmação que o binário resolve, e nada estava perguntando a ele
- Medido em 2026-09-04: dezessete pontos da documentação publicada afirmavam uma contagem de 206 enquanto `config list-keys` devolvia 215, com todos os gates verdes
- `scripts/doc-coverage-check.sh` valida COBERTURA, isto é, que cada chave viva apareça em `docs/CONFIGURATION.md`, e nunca lê o NUMERAL na prosa ao lado
- Cobertura dos itens e afirmação sobre quantos itens existem são duas asserções diferentes, e só uma tinha dono
### tests/root_doc_xdg_coverage_gate.rs
- Rode com `cargo test --test root_doc_xdg_coverage_gate --locked`
- Os documentos `llms-full` da raiz publicam a superfície XDG duas vezes, uma como NUMERAL e outra como ENUMERAÇÃO, e nada checava se as duas metades da mesma frase concordam entre si e com o binário
- Medido em 2026-09-04: `llms-full.txt` anunciava uma superfície agrupada por família de 217 e enumerava apenas 205 abaixo dela, e `llms-full.pt-BR.txt` carregava o defeito idêntico em português
- O numeral fora atualizado quando a superfície cresceu, e a lista embaixo dele não
### tests/bilingual_flag_parity_gate.rs
- Rode com `cargo test --test bilingual_flag_parity_gate --locked`
- Um par traduzido que enumera flags DEVE enumerar as MESMAS flags
- A flag global `--mitm-ws` estava listada em `README.md` e ausente de `README.pt-BR.md`
- `scripts/audit_bilingual_docs.sh` imprimiu `Summary: ok=18 fail=0` antes e depois da correção, porque compara INVOCAÇÕES da CLI entre as duas metades e nunca olha uma flag enumerada num bullet de prosa
- A forma geral é que um gate que verifica a PRESENÇA de um item nunca enxerga a ausência desse item do outro lado
### tests/schema_input_drift_gate.rs
- Rode com `cargo test --test schema_input_drift_gate --locked`
- Todo `docs/schemas/<cmd>.schema.json` DEVE descrever a mesma superfície de ENTRADA que o binário vivo publica em `schema --cmd <cmd>`
- A capacidade já existia como `scripts/generate_command_schemas.sh --check`, alcançada por `scripts/schema-drift-check.sh`
- Um script de shell não é executado por `cargo test`, então nada no laço comum compara os dois lados e os arquivos envelheceram em silêncio
- `docs/schemas/config.schema.json` não listava `user_data_dir` nem `input_typo_permille`, as duas chaves acrescentadas pela versão mais recente
### O Fio que Une os Quatro
- Os quatro nasceram do mesmo padrão: um gate que verifica a presença de um item nunca vê a ausência de outro, e um gate que lê um número nunca lê a lista ao lado dele


## Logging e Paths Durante Testes
- Logging de produto na CLI sob teste: `--verbose` / `--debug` / `-q` ou XDG `config set log_level`
- Defaults de cor via `config set color`
- Overrides de path do Chrome via `config set chrome_path` quando a descoberta precisar
- Resolva o layout XDG com `config path --json`


## Troubleshooting
- Doctor falha em chrome: instale Chromium ou Google Chrome primeiro, ou defina `config set chrome_path`
- Timeouts no goto smoke: eleve o timeout do processo ou inspecione política de rede
- Falhas de schema gate: atualize código e `docs/schemas/` na mesma mudança
- Drift de schema de comando: reexecute `bash scripts/generate_command_schemas.sh` após mudar `meta.rs`
- Drift bilíngue de fences: reexecute `bash scripts/audit_bilingual_docs.sh` e alinhe blocos de comando EN e `.pt-BR`
- Drift de inventário: reconcilie com `commands --json` (71) e `tests/fixtures/tool-reference.md` (53 tools)
- Leaks residual de disco: reexecute `cargo test --test residual_one_shot` e `bash scripts/residual-check.sh`; inspecione `residual` do doctor
- Drift de inventário run: atualize `RUN_DISPATCHED_CMDS` e reexecute `cargo test --test parity_run_inventory`
- Falhas de clap assert: corrija `GlobalOpts` / definições de subcomando e reexecute `cargo test --test clap_command_debug_assert`
- Script e2e sem binário: rode `cargo build --release --locked` primeiro para existir `target/release/browser-automation-cli`
- Path de lighthouse ausente: passe `--lighthouse-path ./scripts/mock-lighthouse.sh` ou defina XDG `lighthouse_path`
- Extract LLM fail-closed: esperado sem `config set openrouter_api_key`
- Problemas de bind MITM: garanta loopback livre e revise `mitm status --json`
- Confusão de journal de workflow: inspecione `workflow status` e o XDG `workflow_dir` de `config path --json`
