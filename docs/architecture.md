# Architecture

The authoritative description is `SPEC.md`; this page is the short version kept current as code
lands. Filled progressively from M1 onward; the crate graph is live from task 0.1.

## Shape

```
            config.toml   rules/*.toml   job (images, requirements, brief)
                 │             │              │
                 ▼             ▼              ▼
┌──────────────────────────────────────────────────────────────┐
│ dejaview-core                                                 │
│  config → obligations → instances per target → checklist      │
│  prompt → decision (1..n runs of the state machine)           │
│  validation → aggregation → verdict → journal + report        │
│  cache (decision boundary) · ledger · redaction · frames      │
└───────────┬───────────────────────────────┬──────────────────┘
            │ Provider trait                 │ Tool trait
            ▼                                ▼
┌───────────────────────┐        ┌──────────────────────────────┐
│ dejaview-providers     │        │ dejaview-tools                │
│  anthropic · gemini    │        │  built-ins (Rust): view,      │
│  openai adapters       │        │   resize, edge, change_map,   │
│  endpoint profiles     │        │   align                       │
│  model catalog         │        │  mcp client (2025-11-25) ─────┼──▶ tools/python (dejaview-tools)
│  wire schemas          │        │  manifest registry, envelope  │        regions, classes, later ocr…
│  injected HTTP client  │        └──────────────────────────────┘
└───────────────────────┘
            ▲
            │ recorded fixtures replace HTTP in tests
┌───────────────────────┐        ┌──────────────────────────────┐
│ dejaview-eval          │        │ dejaview (crates/dejaview-cli)│
│  sets, seal, matrix    │        │  check · eval · tools · sets  │
│  descriptor, ledger    │        │  report · catalog · setup     │
│  metrics, gates        │        └──────────────────────────────┘
└───────────────────────┘
```

## Principles

- **One authority.** The verdict is computed in code from the model's structured submission and
  the code-owned checklist. Tool outputs are evidence. The model's opinion is recorded, never used.
- **Tools point, the model decides.** No tool output is a hard gate.
- **Operational failure is error.** A provider or tool failure, a budget overrun or an unrepaired
  model-output failure invalidates the run; it never becomes a verdict.
- **Nothing silent.** No silent resize, model fallback, cache hit in an evaluation, or change of
  auth or upstream path.
- **Trust boundary.** Config, rules, catalog, profiles and approved manifests are trusted. Image
  content, filenames, labels, the brief and tool observations are data.
- **The core is a library.** It takes an HTTP client and a tool transport as inputs.

## Crate graph

The allowed edges are in `scripts/crate-graph.txt` and enforced by `just check`. `dejaview-core`
depends on no workspace crate; `dejaview-testkit` is a production dependency of the binary only
behind the `testkit` feature, and a dev-dependency anywhere.

## Status

| Area | State | Filled by |
|---|---|---|
| Workspace, checks, CI | done | 0.1, 0.2 |
| Core domain: job, images, frames, config, obligations, checklist, submission, aggregation, journal, report | in progress (1.1 to 1.4b done) | M1 |
| Provider adapters, catalog, profiles | pending | M2 |
| Evaluation sets, runner, metrics, gates, viewer | pending | M3 |
| Built-in tools, MCP client | pending | M4 |
| Python tool pack | pending | M5 |
