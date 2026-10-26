# 0006: Checklist plus findings; the verdict is computed in code

Date: 2026-10-25. Status: accepted.

## Context

Two external reviews of the prototype required one authority, a disposition for every item the
model was shown, and that the model's own opinion be recorded and never used. Silence on a
reference subject or region must not be mistaken for "held".

## Decision

Before the run, code builds a checklist: one item per rule that matters, one per reference image,
and one per region a tool pointed at. The submission must disposition every item
(`violation | held | allowed_change | hidden | cant_tell`) with confidence, reason, a location
for violations, and the tool call ids used as evidence. A findings list holds anything beyond the
checklist. An opinion field is recorded only. Aggregation: confident violation on a matters rule
is fail; cant-tell or tentative violation on a matters rule is unsure; else pass. Two runs: any
fail is fail, else any error is error, else any unsure or disagreement is unsure, else pass.
Schema errors get one repair, then error.

## Alternatives

- **Findings only.** Misses hide in silence.
- **Per-use-case schemas.** Five aggregators and five fixture sets, against the brief.

## Consequences

- The checklist can grow during a run as tools return regions; the model sees the current list
  before the final round.
- A provider error, timeout or budget overrun is never a verdict.

## Refined by spec reviews 1 to 4 (2026-10-25)

- Checklist items are built per **obligation instance** (`SF1@candidate`) with code-owned
  dependencies; the model never supplies rule ids for items. Every verdict-bearing item has at
  least one mandatory dependency.
- Aggregation is one normative table evaluated per instance with that instance's own `matters` and
  `hidden_ok`, strictest row kept. Confident violations of mandatory instances always fail;
  `hidden`, `cant_tell`, tentative `held` and tentative `allowed_change` on something that matters
  yield unsure. An `observed` finding disposition exists for observational rules.
- The canonical schema is a closed tagged record with nullable fields so it survives every
  provider's strict mode; adapters own tested wire transformations.
- Caching happens only at the decision boundary; two decision runs never share a cached result.
- Operational errors are irreversible for the run; repairs come in two forms (replayable
  rejection with paired error results; non-replayable exclusion and reissue).
