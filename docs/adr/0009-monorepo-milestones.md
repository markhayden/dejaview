# 0009: Monorepo; skeleton, adapters, public sets, baseline, zoom, first ML tool

Date: 2026-10-25. Status: accepted.

## Context

The method says baseline first: each model alone on labelled sets, then tools one at a time,
each kept only if answers improve. The harness, the Python pack, the rules, the set manifests and
the docs change together.

## Decision

One repository holds the Cargo workspace, `tools/python/`, `rules/`, `providers/`, `prompts/`,
`fixtures/`, `evalsets/` manifests, `viewer/` and `docs/`. Images live elsewhere by hash.
Milestones, each a checkpoint with tests green: M1 skeleton running end to end offline against a
fake provider; M2 the three adapters with fixtures, image sizing, cost reservation, cache; M3
public dev sets built and labelled, the eval runner, the baseline (arm A); M4 MCP client and the
built-in view and align tools for flat and equirectangular images, arm C; M5 the Python pack with
the change-region finder, arm D, admit or reject; then publish.

## Alternatives

- **Adapters first.** Delays anything running end to end.
- **Tools first.** Violates the method.
- **Separate repos.** Cross-repo pinning slows every iteration.

## Consequences

- Crate boundaries inside the monorepo let server mode and the tool library split later.
- The plan's commit strategy follows the milestones with a tag per checkpoint.
