# 0003: One tool manifest, rendered twice

Date: 2026-10-25. Status: accepted.

## Context

The brief wants adding a tool to be all it takes for the model to use it. Providers receive
name, description and JSON Schema as native tool definitions; cost, ordering and pairing guidance
has no native home and would otherwise be hand-written into prompts per use case.

## Decision

Every tool, Rust or Python, exposes the same manifest: name, version, purpose, when to use, when
not to use, cost tier with a rough duration, pairs-with, which arguments take image ids, which
frames its output can be in, and JSON Schema for inputs and outputs. MCP tools carry the extras
under a `dejaview` key in their annotations; the harness refuses tools without them. The schema
goes to the provider as the native definition; the guidance is rendered by a versioned template
into a Tools section of the system prompt in the order the config gives (default cheap first).
The rendered text is deterministic and part of the cache key.

## Alternatives

- **Description only.** Guidance ends up hand-written per use case.
- **Everything in the schema.** One source, but bloated definitions and no control over
  ordering or grouping.

## Consequences

- Tool authors write one manifest and nothing else to be usable.
- Changing a manifest changes the cache key, which is correct: it can change the answer.
