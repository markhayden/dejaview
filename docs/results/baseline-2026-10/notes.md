Run on 2026-10-28 from the matrix beside this document, six models under arm A (no tools), two
repetitions, decision cap $10, matrix cap $75 (owner decision 6).

- **Spend.** The ledger's $46.54 is not what the vendors charged. The first pass ran out of
  Anthropic credit during its last 17 items: their 68 Anthropic decisions (17 items × 2 models ×
  2 repetitions) came back `HTTP 400`, "credit balance too low", and were recorded as
  `error: provider`. A refused call returns no usage, and the ledger charges a hold with unknown
  usage at its bound, so those 68 decisions were booked at $18.48 while the vendor charged nothing
  for them. After the credit was topped up, the 68 were set aside and the matrix resumed; the
  resume skipped the 1,252 complete decisions and redid the 68 for $2.83. The cost column of the
  cells table below sums the decisions' own reports and is the vendor-side figure: $28.05 in all,
  $17.91 of it Anthropic. Every decision in this document completed with a verdict; the error
  column is zero.
- **Binary.** The first pass ran a build of the `m3-evalsets` branch at `f731caa`; the resume ran
  a build of the working tree that became `737268a` (the M3 review fixes; its version string still
  names `f731caa`, the last commit when it was built), which writes `attempt` on ledger lines and
  the arm selections into the descriptor. The descriptor written by the first build could not be
  read by the second (its format had changed), so it was set aside and the resume wrote a fresh
  one over the same inputs; `descriptor.json` here is that one. The matrix identity is unchanged.
- **Localisation was re-scored.** The first scoring reported almost no localisation hits. The
  models put their violation locations on checklist items, as the prompt asks, while the scorer
  read only free-form findings. Checklist items dispositioned `violation` are now predictions too
  (commit after `d3bde65`); `metrics.json` and the numbers below are from the re-scoring of the
  saved reports, not from a new run.
- **What the localisation numbers mean.** Hits are low and spurious predictions high for two
  reasons visible in the data, neither of them a model missing the defect. Most labels in the
  brief-compliance and reality-checks sets are `whole_image` (a missing lantern, a wrong shadow),
  while the models answer with a box around where the thing is or should be; the rule treats
  `whole_image` against a box as no match. And a model that marks three obligations violated by
  one defect gives the same box three times; one-to-one matching counts two of them as spurious.
  Both are open questions about the metric's definition, recorded in `tasks/todo.md`, not
  settled here.
- **Development sets.** All ten sets are public and were labelled by the owner; the prompt was
  not tuned against them before this run, but later runs may be. The adversarial sets hold two
  items each, so their intervals are wide.
