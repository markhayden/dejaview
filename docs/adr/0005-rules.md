# 0005: Rules are numbered plain-language statements in TOML, layered

Date: 2026-10-25. Status: accepted.

## Context

The prototype's rules were measurement-centric class buckets and thresholds, and every rule added
to make measurements decide encoded a judgement they could not make. The new design needs rules
the model reads and cites, so code can aggregate by severity. Tools still want hints.

## Decision

A rules file per use case. Each rule has an id, plain-language text, a kind (`must_hold`,
`must_not`, `allowed`, `observe`), a `matters` flag (a can't-tell on it yields unsure) and an
optional scope. Tool hints live in `[hints.<tool>]` and are never shown as rules. Base sets ship
inside the binary; a consumer names a base and any number of files; same id overrides,
`disabled = true` drops, new id appends. The merged list with provenance goes in every report.
Rendered as a numbered list; the model cites ids in every disposition.

## Alternatives

- **Free text.** Findings cannot be tied to severities in code.
- **Fully structured predicates.** Precise, but the prototype showed they multiply.

## Consequences

- Five starter rule sets are part of the first milestone.
- A rules change changes the cache key and must be re-evaluated.

## Refined by spec reviews 2 to 4 (2026-10-25)

Rules and job requirements became one **obligation** model: `kind` (`must_hold`, `must_not`,
`allowed`, `observe`) decides aggregation; `matters` controls uncertainty only; `hidden_ok` lets a
mandatory obligation record `hidden` instead of unsure (the architectural profile sets it for any
occlusion, the owner's confirmed call); `scope` names the images judged and `evidence` the images
consulted; exemptions declare `waives` (possibly empty) and a `condition`. The generic
source-fidelity base protects what the job names; the prototype's architectural rules are a named
profile whose furniture exemption waives nothing. See ADR 0011 for instances and evidence binding.
