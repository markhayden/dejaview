# gate-regions-initial-2-2026-10: the second initial gate of regions, on blind-v2

**These are development sets.** Every item here was built, reviewed and labelled by the
owner, and the sets are public; prompts and tools are tuned against them, so these numbers
describe the harness on material it may have been shaped by, not on a blind set. The
adversarial sets are reported separately below. Repetition 1 is the headline; there is one repetition.

- Matrix identity `5a85cc8081e93f1fbc1e5ea927d95fffb8717a5cb5ee0c685fd60e8821b3153e`, descriptor `9088222b88ceb3a0a9616057586f585b0d70c8eea128389281ba3ce1f3141eb8`
- Harness 0.1.0 (0e2b9599c), prompt v1, catalog version 1
- 50 decisions; $1.6746 of $25.00 spent; completed 50, resumed 0, partial redone 0
- Files beside this document: `matrix.toml`, `summary.json`, `metrics.json`, `descriptor.json`, `notes.md`

## Notes

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
  generator moved the door during the restyle the label is wrong and both arms were right. The
  owner checked it the same day: the door moved, so the label is wrong and
  `evalsets/source-fidelity/blind-v2/corrections.toml` records it as fail (RQ3). Read with the
  correction, both arms detected 16 of 16 with no false reject of 9; the verdict is unchanged,
  and the numbers below stay as scored against the sealed labels.
- **How the model used the tools.** Under C, `align` on all 25 items (21 registered, 4 not
  applicable) and `view` on 17 (23 calls). Under D_regions it added `regions` on 16 items (15
  answered with regions, one measured nothing) and never called `classes`. Turns rose from 57 to
  80.
- **Spend.** $1.67 of $25: $0.70 under C, $0.97 under D_regions, all Google. No errors, no
  unsure answers, no resume. The set's build (30 images across fal, OpenAI and Gemini) is
  recorded in owner storage.

## Cells: resolved model, host, API version, cost

| Cell | Profile | Host | Resolved model | API version | Decisions | Errors | Cost |
|---|---|---|---|---|---|---|---|
| google-ai-studio:gemini-4-argon/C | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 25 | 0 | $0.7006 |
| google-ai-studio:gemini-4-argon/D_regions | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 25 | 0 | $0.9740 |

## Development sets

Sets: source-fidelity/blind-v2

Matrix identity `5a85cc8081e93f1fbc1e5ea927d95fffb8717a5cb5ee0c685fd60e8821b3153e`, descriptor `9088222b88ceb3a0a9616057586f585b0d70c8eea128389281ba3ce1f3141eb8`. 50 decisions; 50 labelled items scored across 2 cells; $1.6746 of $25.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 15 / 15 | 1.000 | 0.796 to 1.000 |
| Missed failures | 0 / 15 | 0.000 | 0.000 to 0.204 |
| False rejects | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Unsure | 0 / 25 | 0.000 | 0.000 to 0.133 |
| Errors | 0 / 25 | 0.000 | 0.000 to 0.133 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 15 | 0 | 0 |
| pass | 9 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 8 hits of 15 labelled regions, 25 spurious of 33 predictions.
Cost $0.7006 total, $0.0267 median per item. Wall 943010 ms total, 36067 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/D_regions

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 15 / 15 | 1.000 | 0.796 to 1.000 |
| Missed failures | 0 / 15 | 0.000 | 0.000 to 0.204 |
| False rejects | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Unsure | 0 / 25 | 0.000 | 0.000 to 0.133 |
| Errors | 0 / 25 | 0.000 | 0.000 to 0.133 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 15 | 0 | 0 |
| pass | 9 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 8 hits of 15 labelled regions, 45 spurious of 53 predictions.
Cost $0.9740 total, $0.0363 median per item. Wall 1304638 ms total, 48178 ms median.
Repetitions 1.

## Paired: google-ai-studio:gemini-4-argon under D_regions against C

25 items scored under both arms (repetition 1); 0 flipped. Detected failures: 0 gained, 0 lost. False rejects: 0 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (D_regions minus C), seed 20261030, 10000 resamples over 5 families, 2 hypotheses: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0125 and 0.9875.

## Adversarial sets

Sets: 

(none in this matrix)

## Every set together

Matrix identity `5a85cc8081e93f1fbc1e5ea927d95fffb8717a5cb5ee0c685fd60e8821b3153e`, descriptor `9088222b88ceb3a0a9616057586f585b0d70c8eea128389281ba3ce1f3141eb8`. 50 decisions; 50 labelled items scored across 2 cells; $1.6746 of $25.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 15 / 15 | 1.000 | 0.796 to 1.000 |
| Missed failures | 0 / 15 | 0.000 | 0.000 to 0.204 |
| False rejects | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Unsure | 0 / 25 | 0.000 | 0.000 to 0.133 |
| Errors | 0 / 25 | 0.000 | 0.000 to 0.133 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 15 | 0 | 0 |
| pass | 9 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 8 hits of 15 labelled regions, 25 spurious of 33 predictions.
Cost $0.7006 total, $0.0267 median per item. Wall 943010 ms total, 36067 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/D_regions

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 15 / 15 | 1.000 | 0.796 to 1.000 |
| Missed failures | 0 / 15 | 0.000 | 0.000 to 0.204 |
| False rejects | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Unsure | 0 / 25 | 0.000 | 0.000 to 0.133 |
| Errors | 0 / 25 | 0.000 | 0.000 to 0.133 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 15 | 0 | 0 |
| pass | 9 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 8 hits of 15 labelled regions, 45 spurious of 53 predictions.
Cost $0.9740 total, $0.0363 median per item. Wall 1304638 ms total, 48178 ms median.
Repetitions 1.

## Paired: google-ai-studio:gemini-4-argon under D_regions against C

25 items scored under both arms (repetition 1); 0 flipped. Detected failures: 0 gained, 0 lost. False rejects: 0 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (D_regions minus C), seed 20261030, 10000 resamples over 5 families, 2 hypotheses: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0125 and 0.9875.
