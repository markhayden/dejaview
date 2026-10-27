# Owner decisions

Decisions only the owner can make, recorded before the task that needs them (PLAN.md task 0.4).
A blank entry blocks the task named in its row. Dates are when the owner decided.

| # | Decision | Needed before | Status |
|---|---|---|---|
| 1 | Initial judge-model subset for live fixtures and matrices | 2.1 | 2026-10-26: two per vendor: `claude-opus-5-5`, `claude-sonnet-5-5`, `gemini-4-argon`, `gemini-3.8-flash`, and two OpenAI vision models (frontier and mid-tier) named at 2.1. Confirmed 2026-10-26 from the live model list: `gpt-6.1-sol` (mid-tier, $2/$10) and `gpt-6.1-terra` (low-tier, $1/$5); the frontier `gpt-6-astra` was declined as cost prohibitive at $10/$50 |
| 2 | Anthropic account for recordings: enforced-thinking (created on or after 2026-08-31) or not | 2.5b | 2026-10-26: the existing account, with the prefix check switched on per request (`thinking-binding-controls` beta, `prefix_mismatch_behavior = "error"`) during recording |
| 3 | Spend cap: fixture recordings (all three vendors) | 2.5b | decide before 2.5b |
| 4 | Spend cap: release and gateway smoke tests | 2.8, 2.10b | decide before 2.8 |
| 5 | Spend cap: image generation for public sets | 3.2a | decide before 3.2a |
| 6 | Spend cap: arm A baseline | 3.8 | decide before 3.8, against `eval estimate` |
| 7 | Spend cap: arm C | 4.5 | decide before 4.5, against `eval estimate` |
| 8 | Spend cap: initial gate | 5.6 | decide before 5.6, against `eval estimate` |
| 9 | Spend cap: confirmation gate | 6.2 | decide before 6.2, against `eval estimate` |
| 10 | Labelling plan: worksheet format, batch size, adjudicator | 3.2b | 2026-10-26: draft answer key from construction plus a contact sheet per batch; batches of 20 to 25; the owner confirms or corrects |
| 11 | Reuse of prototype images, outputs and the segmenter export | 3.2a, 4.0, 4.6 | 2026-10-26: nothing from the prototype. Public upstream models (for example the EoMT-L ADE20K weights on Hugging Face under their own licences) are not prototype assets and may be exported afresh; golden panoramas are newly generated. Task 4.6 produces its own export and provenance record |
| 12 | Whether M6 (confirmation) is funded in this release | 6.1 | 2026-10-26: yes, funded |
| 13 | Set generators and publication licence | 3.2a | deferred to 3.2a by design (ADR 0013) |

## Notes on the decisions

- **1.** Two models per vendor from the start; the rest of the catalog may join after the baseline.
  For OpenAI the pair is the mid-tier and low-tier model rather than frontier and mid-tier: the
  frontier model's output price would dominate every matrix row.
- **2.** The harness sends the thinking-binding controls header with `error` behaviour during fixture
  recording so any history edit fails loudly; the same header is available to consumers.
- **11.** "Nothing from the prototype" covers its images, outputs, stored views and ONNX export.
  Public upstream weights and code under their own licences are not prototype assets; task 4.6
  exports afresh and records provenance. Golden panoramas for the equirectangular path are
  generated or taken from CC-licensed sources.
- **12.** M6 is funded; `regions` can reach confirmed admission in this release.
- **3 to 9.** Spend caps are set when each paid task is about to run, against `dejaview eval
  estimate` where it exists; the task refuses to start without its cap entry.

## Open questions for 3.2a

- Generators and publication licence (ADR 0013).
