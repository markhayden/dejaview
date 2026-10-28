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
| Core domain: job, images, frames, config, obligations, checklist, submission, aggregation, journal, report | done (1.1 to 1.6) | M1 |
| Run state machine: tool turns, final turn (`refuse`), terminal error classes, repairs, retries, cost ledger, artifacts, cancellation, report rebuild and prune, decision cache, isolation and the environment allowlist | done (1.7a to 1.12) | M1 |
| Provider adapters, catalog, profiles | catalog with 7 models and 3 API-key profiles, resolve-time capability validation, `catalog show|list` (2.1); wire schema transformations per adapter with resolve-time representability and limit checks (2.2); deterministic image preparation (`prep_version` 1) and request preflight against catalog limits (2.3); consent binding, credential resolution (`error: auth`), the HTTPS client refusing cross-host redirects, request hygiene scan over recorded fixtures (2.4); the transport seam with the replay player and recorder, and the Anthropic Messages adapter with thinking echoed, strict tools and the three final-turn mechanisms (2.5a); the Gemini generateContent adapter with function-call pairing and verbatim part echo (2.6a) and the OpenAI Responses adapter with strict tools, `store = false` and reasoning items echoed (2.7a); fifteen Anthropic, thirteen Gemini and thirteen OpenAI conversations recorded and replaying (2.5b, 2.6b, 2.7b; tag `m2.1`); provider-data rows for every shipped profile, checked by a catalog test (2.9); embedded assets packaged with their crates, publish dry run, install from the packaged crates and the offline smoke in the `package` workflow (2.10a); `dist` release archives for three architectures with the archive smoke gating the GitHub release, the release dry run in the `package` workflow, one example config per use case, and the live smoke through the three direct profiles from an unpacked archive (2.10b) | M2 |
| Evaluation sets, runner, metrics, gates, viewer | statistics contract frozen (1.15: bootstrap, Wilson, gate rules, ADR 0016); set manifests, families and labels parsed strictly, `sets check` over a tree, `sets fetch` by hash from a directory or URL, `sets seal` and `verify-seal` by component (3.1); the set-building toolkit with scene and subject families, draft labels from construction, rebuild and promote (3.2a); five dev sets built, reviewed and sealed, one per use case (3.2b, 3.3a to 3.3d); adversarial sets, runner, viewer pending | M3 |
| Built-in tools, MCP client | MCP client lifecycle done (1.13); built-ins, pagination, concurrency pending | M4 |
| Python tool pack | provisioning bundle, setup and offline launch proven with `echo_region` (1.14); real tools pending | M5 |
