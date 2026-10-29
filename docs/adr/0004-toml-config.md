# 0004: TOML config, one file per job type

Date: 2026-10-25. Status: accepted.

## Context

The config carries provider and model, controls, image limits, tools enabled, a pointer to the
rules, budgets, cache settings and the explicit data-leaves-the-machine opt-in. Keys must never
appear in it.

## Decision

`squint.toml`, one per use case or job type, with sections `[provider]`, `[images]`, `[rules]`,
`[tools]`, `[budget]`, `[cache]`, `[privacy]`, `[output]`. CLI flags override single values.
Secrets come only from the environment, expanded with `${VAR}`. The resolved config, redacted, is
written into every report.

## Alternatives

- **YAML.** Anchors, implicit typing and indentation errors buy nothing here.
- **JSON.** No comments, and humans write these by hand.
- **TOML plus a user-level file.** More convenient on one machine, but adds a merge step and
  hides what a run used.

## Consequences

- One file explains a run. Reproduction is copying it.
