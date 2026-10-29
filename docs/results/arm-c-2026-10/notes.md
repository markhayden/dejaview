Run on 2026-10-28 from the matrix beside this document: arms A and C on the ten published sets,
five models, one repetition, decision cap $10, matrix cap $75, bootstrap seed 20261028.

- **Why five models.** claude-opus-5-5 is left out. In the baseline it cost about five times
  gpt-6.1-terra per decision with no better detection; its arm C envelope ($10.30) is over the
  per-decision cap; and with it the Anthropic share would pass the standing $25 per vendor.
- **Arm C.** The visual-access control: `view` (crops and perspective views) and `align` under
  its `visual_only` projection, so the model gets the aligned candidate as a picture and no
  measurement. Prompts differ from arm A only in the Tools section (`prompt_equivalence` in the
  runner tests), so a flip between the arms is the tools' doing.
- **Paired flips.** The report pairs A against C per model on the items both scored: flips each
  way, detected failures and false rejects gained and lost, and the paired bootstrap of the
  detected-failure difference under the frozen seed. The M3 baseline is historical context,
  never the comparison arm.
- **The estimate.** The envelope planner first put every arm C decision near $500: it priced each
  possible tool result's stored envelope, inline pixels included, as text. It now prices what the
  model reads and the returned images as images; the estimate before the run was $1.47 to $5.15
  per decision, $5,286 as the sum of envelopes.
