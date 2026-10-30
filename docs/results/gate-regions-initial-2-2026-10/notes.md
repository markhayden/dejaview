Run on 2026-10-30 from the matrix beside this document: the second initial admission gate of the
`regions` tool for source fidelity (PLAN task 6.0, ADR 0017), after the first was rejected on
blind-v1 because the control detected every planted failure. Arms C and D_regions on the sealed
blind set `source-fidelity/blind-v2` (25 items in 5 new families, 15 expected fail and 10 expected
pass, opaque item ids) under gemini-4-argon, the first gate's judging configuration unchanged, one
repetition, decision cap $10, matrix cap $25, bootstrap seed 20261030, 10,000 resamples, **two
hypotheses** (the earlier attempt counts, so the interval is read at 0.0125 and 0.9875). The labels
were held by the owner until the gate scored; receipt `be41ae402334a394`.

- **Outcome: rejected, the same way.** Both arms detected all 15 planted failures, so the
  detected-failure difference is zero, its interval zero to zero, and no family differs. The set
  was built to be hard for the control: five small objects removed (a lamp post, a post box, a
  signpost, an anchor, a coin telescope), four removals of one among several alike (a market stall
  of six, a chimney of five, a picnic table of three, a rowing boat of four), two parts altered (a
  clock face, a bandstand roof) and four removals under a strong restyle (a car, a flagpole, a
  crane, a bench). gemini-4-argon with `view` and `align` saw every one.
- **What this says.** For source fidelity under this model the control is at ceiling on every
  removal-shaped failure the generators can plant cleanly, so a detection gate cannot separate any
  tool from it here. A third detection attempt on a harder set of the same kind is unlikely to
  differ; the next step is an owner decision (findings, "after the second gate").
- **What the tool changed.** Nothing in the verdicts: 0 of 25 items flipped. Localisation was the
  same in hits (8 of 15 labelled regions under both arms) and worse in noise (45 spurious of 53
  predictions against 25 of 33): the model cited the tool's pointers without landing more of them.
  Cost was 1.39 times C ($0.97 against $0.70) and the median decision 34% slower.
- **The one false reject.** Both arms rejected `boatyard-5`, labelled pass (a dusk restyle), with
  the same reason: the workshop's roller door is raised far higher than in the source. If the
  generator moved the door during the restyle the label is wrong and both arms were right; the
  owner checks the item, and a label correction is recorded with the set.
- **How the model used the tools.** Under C, `align` on all 25 items (21 registered, 4 not
  applicable) and `view` on 17 (23 calls). Under D_regions it added `regions` on 16 items (15
  answered with regions, one measured nothing) and never called `classes`. Turns rose from 57 to
  80.
- **Spend.** $1.67 of $25: $0.70 under C, $0.97 under D_regions, all Google. No errors, no
  unsure answers, no resume. The set's build (30 images across fal, OpenAI and Gemini) is
  recorded in owner storage.
