# browser-automation-cli

Notas de release do produto crates-only one-shot
Binário: `browser-automation-cli`
Sem daemon npm e sem BrowserFetcher embutido no MVP

## 0.1.0

### One-shot Chrome
- Lançamento via `chromiumoxide::Browser::launch` com Chrome/Chromium de sistema
- Flags de launch (`proxy`, `webgpu`, `extensions`, sandbox) aplicadas no path oxide
- FINALIZE: close + wait + kill fallback
- Sem attach CDP externo no path default

### CLI
- Comandos PRD: doctor, open/goto, extract, scrape, run, grab, view, click, fill, robots
- Captura opcional de console e network
- Política robots com dual-flag

### Limpeza residual P1/P2/P3
- `src/install.rs` slim (descoberta local apenas)
- Removido monólito de spawn dual `launch_chrome` / `ChromeProcess`
- Stack CDP Chrome 100% chromiumoxide
- Zero telemetria

### Paridade DevTools §5C (wave v7)
- Flags tool-ref: include-snapshot em hover/drag/keys/upload/fill-form
- net/console list com page-idx, page-size, filtros e include-preserved
- eval com --args, --dialog-action, --file-path
- perf start --auto-stop; perf insight --insight-set-id; stop emite available_insight_sets
- screencast stop --path (.webm/.mp4 via ffmpeg)
- heap details/class-nodes/dup-strings/edges/retainers com paginação; paths max-nodes/max-siblings
- page new --background/--isolated-context; page select --bring-to-front
- wait --text repetível (OR); type --focus-only
- Gate tests/parity_toolref_schema.rs

### Fora deste release (explícito)
- PRD §5D Firecrawl (crawl/map/search), §5E MITM, §5H workflow SQLite
- Superfície DevTools §5C inventário 52 tools está no binário one-shot
