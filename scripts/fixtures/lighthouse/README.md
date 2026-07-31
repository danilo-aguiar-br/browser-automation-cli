# Lighthouse LHR fixtures (GAP-021)

Offline fixtures for unit-testing `scores_from_lhr` without dumping LHR bodies
into product stdout (agent-native: scores + paths only).

| File | Origin |
|---|---|
| `minimal_lhr.json` | Synthetic LHR-shaped document (edges: null audit score). |
| `chrome_captured_lhr.json` | Sanitized subset from real `npx lighthouse` **13.4.1** against `https://example.com` with Chrome headless (`--headless=new`). Full artifacts / screenshots / traces stripped. |

## Re-capture (optional)

```bash
npx --yes lighthouse https://example.com --quiet \
  --chrome-flags="--headless=new --no-sandbox" \
  --output=json --output-path=/tmp/lh-report \
  --only-categories=performance,accessibility,seo,best-practices
# Then sanitize to categories + small audits sample (see residual plan).
```

e2e against the **mock** binary remains SKIP (never counted as parser PASS).
