# arm-c-2026-10: arms A and C, the visual-access control

**These are development sets.** Every item here was built, reviewed and labelled by the
owner, and the sets are public; prompts and tools are tuned against them, so these numbers
describe the harness on material it may have been shaped by, not on a blind set. The
adversarial sets are reported separately below. Repetition 1 is the headline; there is one repetition.

- Matrix identity `98ae8a0efd7295ca85ba9e98a4dab25d0decd301e9e798058019b88302ba8b15`, descriptor `4c602ca830f42939983cd1f82e874a880633940e94263ba7403841d7558a704b`
- Harness 0.1.0 (393d03266), prompt v1, catalog version 1
- 1100 decisions; $28.3130 of $75.00 spent; completed 1100, resumed 0, partial redone 0
- Files beside this document: `matrix.toml`, `summary.json`, `metrics.json`, `descriptor.json`, `notes.md`

## Notes

Run on 2026-10-28 from the matrix beside this document: arms A and C on the ten published sets,
five models, one repetition, decision cap $10, matrix cap $75, bootstrap seed 20261028.

- **Why five models.** claude-opus-5-5 is left out. In the baseline it cost about five times
  gpt-6.1-terra per decision with no better detection; its arm C envelope ($10.30) is over the
  per-decision cap; and with it the Anthropic share would pass the standing $25 per vendor.
- **Arm C.** The visual-access control: `view` (crops and perspective views) and `align` under
  its `visual_only` projection, so the model gets the aligned candidate as a picture and no
  measurement. Prompts differ from arm A only in the Tools section (the test
  `arm_c_prompts_differ_from_arm_a_only_in_the_tools_section` builds both from this matrix), so a
  flip between the arms is the tools' doing.
- **Paired flips.** The report pairs A against C per model on the items both scored: flips each
  way, detected failures and false rejects gained and lost, and the paired bootstrap of the
  detected-failure difference under the frozen seed. The M3 baseline is historical context,
  never the comparison arm.
- **The estimate.** The envelope planner first put every arm C decision near $500: it priced each
  possible tool result's stored envelope, inline pixels included, as text. It now prices what the
  model reads and the returned images as images; the estimate before the run was $1.47 to $5.15
  per decision, $5,286 as the sum of envelopes.
- **How the run went.** One pass, 1100 decisions, no resume, about 12 hours on 2026-10-28 and
  29 local time. The ledger's $28.31 is the vendor-side figure too: Anthropic $11.69,
  Google $8.42, OpenAI $8.21, each under the standing $25 per vendor. Three decisions ended in
  `error` and the frozen policy sends them to review: claude-sonnet-5-5 under C sent the
  submission's `items` as a JSON string rather than an array and its one repair did not mend it
  (`submission_invalid`); gemini-3.8-flash under C got `HTTP 400 INVALID_ARGUMENT` from Google
  on both garden-shed adversarial items four minutes apart, once before any tool call and once on
  the turn after `align` returned its picture. The same request shapes served the other 108
  items, so the two look transient; a 400 is not retried and each was charged at its bound.
- **Binary.** The run used a build of `393d032`, the last commit when it started. The M4 review
  fixes landed in `327a701` while it ran and did not touch the runner, the prompts or what `view`
  and `align` do; `report.md` beside the run and this document's report sections were regenerated with
  `327a701`, whose paired summary also names the number of families and hypotheses.
- **The tools were hard to call.** This is the finding that matters most for reading the arm C
  numbers. Of 196 `align` calls across the five models, 124 were refused as `invalid_args` because
  the model passed the frame id the prompt shows (`raw:candidate`) where the tool wants the image
  id (`candidate`); gpt-6.1-sol never made a valid `align` call in 35 tries, and the others
  recovered on a later turn only sometimes (align calls that ran: Sonnet 12 of 35, Flash 16 of
  44, Argon 16 of 45, Terra 8 of 37). claude-sonnet-5-5 made 325 `view` calls and none ran: it
  sent `yaw: 0` (sometimes with `pitch: 0`) and no `fov` against a flat frame, was told "yaw, pitch
  and fov go together", and repeated the identical call on the next turn, usually until its tool
  turns ran out. Only 12 of its 110 arm C decisions ever saw a tool picture, against 94 to 109 for
  the other four models. So for Sonnet, arm C was arm A plus up to four turns of refused calls: its
  cost per item tripled ($0.025 to $0.081), its unsure count rose from 22 to 35, and its detection
  did not move. Both refusals are the harness's to fix before the next tools run: `align` should
  take a frame id for a raw image, and `view`'s refusal should say what to send instead on a flat
  frame.
- **Journal double entry.** A tool that refuses its own arguments (the `view` case above) is
  journaled twice under one call id: once at dispatch with `executed: true`, once with the refusal.
  Harness-side refusals (the unknown id case) are journaled once. Counts in this document
  de-duplicate by call id; the double line is recorded in `tasks/todo.md`.

## Cells: resolved model, host, API version, cost

| Cell | Profile | Host | Resolved model | API version | Decisions | Errors | Cost |
|---|---|---|---|---|---|---|---|
| anthropic:claude-sonnet-5-5/A | anthropic | api.anthropic.com | claude-sonnet-5-5 | 2023-06-01 | 110 | 0 | $2.8537 |
| anthropic:claude-sonnet-5-5/C | anthropic | api.anthropic.com | claude-sonnet-5-5 | 2023-06-01 | 110 | 1 | $8.8387 |
| google-ai-studio:gemini-3.8-flash/A | google-ai-studio | generativelanguage.googleapis.com | gemini-3.8-flash | v1beta | 110 | 0 | $1.1743 |
| google-ai-studio:gemini-3.8-flash/C | google-ai-studio | generativelanguage.googleapis.com | gemini-3.8-flash | v1beta | 110 | 2 | $2.4515 |
| google-ai-studio:gemini-4-argon/A | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 110 | 0 | $1.8662 |
| google-ai-studio:gemini-4-argon/C | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 110 | 0 | $2.9231 |
| openai:gpt-6.1-sol/A | openai | api.openai.com | gpt-6.1-sol | v1 | 110 | 0 | $2.2003 |
| openai:gpt-6.1-sol/C | openai | api.openai.com | gpt-6.1-sol | v1 | 110 | 0 | $3.2268 |
| openai:gpt-6.1-terra/A | openai | api.openai.com | gpt-6.1-terra | v1 | 110 | 0 | $1.0982 |
| openai:gpt-6.1-terra/C | openai | api.openai.com | gpt-6.1-terra | v1 | 110 | 0 | $1.6803 |

## Development sets

Sets: brief-compliance/stills-v1, product-fidelity/packshots-v1, reality-checks/people-v1, reference-consistency/pets-v1, source-fidelity/scenes-v1

Matrix identity `98ae8a0efd7295ca85ba9e98a4dab25d0decd301e9e798058019b88302ba8b15`, descriptor `4c602ca830f42939983cd1f82e874a880633940e94263ba7403841d7558a704b`. 1000 decisions; 1000 labelled items scored across 10 cells; $28.3130 of $75.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## anthropic:claude-sonnet-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 56 / 60 | 0.933 | 0.841 to 0.974 |
| Missed failures | 1 / 60 | 0.017 | 0.003 to 0.089 |
| False rejects | 5 / 40 | 0.125 | 0.055 to 0.261 |
| Unsure | 18 / 100 | 0.180 | 0.117 to 0.267 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 56 | 3 | 0 |
| pass | 20 | 5 | 15 | 0 |

Localisation over detected failures with labelled regions: 22 hits of 56 labelled regions, 192 spurious of 214 predictions.
Cost $2.5945 total, $0.0247 median per item. Wall 1165718 ms total, 11404 ms median.
Repetitions 1.

## anthropic:claude-sonnet-5-5/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 56 / 60 | 0.933 | 0.841 to 0.974 |
| Missed failures | 0 / 60 | 0.000 | 0.000 to 0.060 |
| False rejects | 4 / 40 | 0.100 | 0.040 to 0.231 |
| Unsure | 29 / 100 | 0.290 | 0.210 to 0.385 |
| Errors | 1 / 100 | 0.010 | 0.002 to 0.054 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 56 | 4 | 0 |
| pass | 10 | 4 | 25 | 1 |

Localisation over detected failures with labelled regions: 22 hits of 56 labelled regions, 193 spurious of 215 predictions.
Cost $8.0302 total, $0.0811 median per item. Wall 2179756 ms total, 20054 ms median.
Repetitions 1.

## google-ai-studio:gemini-3.8-flash/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 54 / 60 | 0.900 | 0.799 to 0.953 |
| Missed failures | 2 / 60 | 0.033 | 0.009 to 0.114 |
| False rejects | 7 / 40 | 0.175 | 0.087 to 0.319 |
| Unsure | 4 / 100 | 0.040 | 0.016 to 0.098 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 54 | 4 | 0 |
| pass | 33 | 7 | 0 | 0 |

Localisation over detected failures with labelled regions: 20 hits of 54 labelled regions, 164 spurious of 184 predictions.
Cost $1.0728 total, $0.0098 median per item. Wall 2222679 ms total, 19126 ms median.
Repetitions 1.

## google-ai-studio:gemini-3.8-flash/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 56 / 60 | 0.933 | 0.841 to 0.974 |
| Missed failures | 2 / 60 | 0.033 | 0.009 to 0.114 |
| False rejects | 10 / 40 | 0.250 | 0.142 to 0.402 |
| Unsure | 5 / 100 | 0.050 | 0.022 to 0.112 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 56 | 2 | 0 |
| pass | 27 | 10 | 3 | 0 |

Localisation over detected failures with labelled regions: 20 hits of 56 labelled regions, 169 spurious of 189 predictions.
Cost $2.1391 total, $0.0191 median per item. Wall 4812731 ms total, 41949 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 60 / 60 | 1.000 | 0.940 to 1.000 |
| Missed failures | 0 / 60 | 0.000 | 0.000 to 0.060 |
| False rejects | 12 / 40 | 0.300 | 0.181 to 0.454 |
| Unsure | 0 / 100 | 0.000 | 0.000 to 0.037 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 60 | 0 | 0 |
| pass | 28 | 12 | 0 | 0 |

Localisation over detected failures with labelled regions: 24 hits of 60 labelled regions, 197 spurious of 221 predictions.
Cost $1.7064 total, $0.0166 median per item. Wall 1838454 ms total, 16753 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 60 / 60 | 1.000 | 0.940 to 1.000 |
| Missed failures | 0 / 60 | 0.000 | 0.000 to 0.060 |
| False rejects | 13 / 40 | 0.325 | 0.201 to 0.480 |
| Unsure | 0 / 100 | 0.000 | 0.000 to 0.037 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 60 | 0 | 0 |
| pass | 27 | 13 | 0 | 0 |

Localisation over detected failures with labelled regions: 22 hits of 60 labelled regions, 193 spurious of 215 predictions.
Cost $2.6725 total, $0.0253 median per item. Wall 3615783 ms total, 31669 ms median.
Repetitions 1.

## openai:gpt-6.1-sol/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 58 / 60 | 0.967 | 0.886 to 0.991 |
| Missed failures | 2 / 60 | 0.033 | 0.009 to 0.114 |
| False rejects | 10 / 40 | 0.250 | 0.142 to 0.402 |
| Unsure | 1 / 100 | 0.010 | 0.002 to 0.054 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 58 | 0 | 0 |
| pass | 29 | 10 | 1 | 0 |

Localisation over detected failures with labelled regions: 19 hits of 58 labelled regions, 196 spurious of 215 predictions.
Cost $2.0150 total, $0.0183 median per item. Wall 1288219 ms total, 12597 ms median.
Repetitions 1.

## openai:gpt-6.1-sol/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 58 / 60 | 0.967 | 0.886 to 0.991 |
| Missed failures | 1 / 60 | 0.017 | 0.003 to 0.089 |
| False rejects | 9 / 40 | 0.225 | 0.123 to 0.375 |
| Unsure | 3 / 100 | 0.030 | 0.010 to 0.085 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 58 | 1 | 0 |
| pass | 29 | 9 | 2 | 0 |

Localisation over detected failures with labelled regions: 17 hits of 58 labelled regions, 197 spurious of 214 predictions.
Cost $2.9484 total, $0.0292 median per item. Wall 1839158 ms total, 17954 ms median.
Repetitions 1.

## openai:gpt-6.1-terra/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 57 / 60 | 0.950 | 0.863 to 0.983 |
| Missed failures | 2 / 60 | 0.033 | 0.009 to 0.114 |
| False rejects | 8 / 40 | 0.200 | 0.105 to 0.348 |
| Unsure | 9 / 100 | 0.090 | 0.048 to 0.162 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 57 | 1 | 0 |
| pass | 24 | 8 | 8 | 0 |

Localisation over detected failures with labelled regions: 19 hits of 57 labelled regions, 191 spurious of 210 predictions.
Cost $1.0021 total, $0.0092 median per item. Wall 1146466 ms total, 11015 ms median.
Repetitions 1.

## openai:gpt-6.1-terra/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 57 / 60 | 0.950 | 0.863 to 0.983 |
| Missed failures | 1 / 60 | 0.017 | 0.003 to 0.089 |
| False rejects | 9 / 40 | 0.225 | 0.123 to 0.375 |
| Unsure | 12 / 100 | 0.120 | 0.070 to 0.198 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 57 | 2 | 0 |
| pass | 21 | 9 | 10 | 0 |

Localisation over detected failures with labelled regions: 17 hits of 57 labelled regions, 199 spurious of 216 predictions.
Cost $1.5304 total, $0.0150 median per item. Wall 1829345 ms total, 17975 ms median.
Repetitions 1.

## Paired: anthropic:claude-sonnet-5-5 under C against A

100 items scored under both arms (repetition 1); 14 flipped. Detected failures: 1 gained, 1 lost. False rejects: 0 gained, 1 lost. Families with an expected-fail item whose detection differs: 1.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 25 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| brief-compliance/stills-v1/cafe-window-asked | Pass | Fail | Unsure |
| product-fidelity/packshots-v1/coffee-tin-kitchen | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/jam-breakfast | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/matches-mantel | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/toothpaste-bathroom | Pass | Pass | Unsure |
| reality-checks/people-v1/barista-clean | Pass | Pass | Unsure |
| reality-checks/people-v1/barista-mono | Pass | Pass | Unsure |
| reality-checks/people-v1/market-garbled-sign | Fail | Unsure | Fail |
| reality-checks/people-v1/market-six-fingers | Fail | Fail | Unsure |
| reality-checks/people-v1/skatepark-clean | Pass | Pass | Unsure |
| reality-checks/people-v1/skatepark-garbled-wall | Fail | Pass | Unsure |
| reference-consistency/pets-v1/ada-kitchen | Pass | Pass | Unsure |
| reference-consistency/pets-v1/pip-beach | Pass | Pass | Unsure |
| source-fidelity/scenes-v1/red-barn-grade | Pass | Pass | Error |

## Paired: google-ai-studio:gemini-3.8-flash under C against A

100 items scored under both arms (repetition 1); 12 flipped. Detected failures: 4 gained, 2 lost. False rejects: 3 gained, 0 lost. Families with an expected-fail item whose detection differs: 6.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 25 families, 1 hypothesis: point 0.0333, interval -0.0476 to 0.1148 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| brief-compliance/stills-v1/cafe-window-no-cups | Fail | Fail | Unsure |
| brief-compliance/stills-v1/desk-misspelt | Fail | Unsure | Fail |
| brief-compliance/stills-v1/kids-room-misspelt | Fail | Unsure | Fail |
| product-fidelity/packshots-v1/tonic-bar | Pass | Pass | Fail |
| reality-checks/people-v1/barista-mono | Pass | Pass | Unsure |
| reality-checks/people-v1/cyclist-rain | Pass | Pass | Fail |
| reality-checks/people-v1/skatepark-clean | Pass | Pass | Fail |
| reality-checks/people-v1/skatepark-sunset | Pass | Pass | Unsure |
| reference-consistency/pets-v1/ada-kitchen-grey-tail | Fail | Unsure | Fail |
| reference-consistency/pets-v1/bramble-meadow-lookalike | Fail | Unsure | Fail |
| source-fidelity/scenes-v1/hotel-pool-grade | Pass | Pass | Unsure |
| source-fidelity/scenes-v1/village-street-no-tower | Fail | Fail | Unsure |

## Paired: google-ai-studio:gemini-4-argon under C against A

100 items scored under both arms (repetition 1); 1 flipped. Detected failures: 0 gained, 0 lost. False rejects: 1 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 25 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| reference-consistency/pets-v1/bramble-beach | Pass | Pass | Fail |

## Paired: openai:gpt-6.1-sol under C against A

100 items scored under both arms (repetition 1); 4 flipped. Detected failures: 0 gained, 0 lost. False rejects: 1 gained, 2 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 25 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| product-fidelity/packshots-v1/matches-mantel | Pass | Fail | Pass |
| reality-checks/people-v1/market-clean | Pass | Fail | Unsure |
| reality-checks/people-v1/skatepark-wrong-shadow | Fail | Pass | Unsure |
| reference-consistency/pets-v1/ada-library | Pass | Pass | Fail |

## Paired: openai:gpt-6.1-terra under C against A

100 items scored under both arms (repetition 1); 8 flipped. Detected failures: 0 gained, 0 lost. False rejects: 2 gained, 1 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 25 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| product-fidelity/packshots-v1/matches-mantel | Pass | Unsure | Fail |
| product-fidelity/packshots-v1/tonic-bar | Pass | Pass | Fail |
| reality-checks/people-v1/cyclist-clean | Pass | Unsure | Pass |
| reality-checks/people-v1/market-evening | Pass | Fail | Unsure |
| reality-checks/people-v1/skatepark-sunset | Pass | Pass | Unsure |
| reality-checks/people-v1/skatepark-wrong-shadow | Fail | Pass | Unsure |
| reference-consistency/pets-v1/ada-library | Pass | Pass | Unsure |
| reference-consistency/pets-v1/bramble-snow | Pass | Pass | Unsure |

## Adversarial sets

Sets: brief-compliance/adversarial-v1, product-fidelity/adversarial-v1, reality-checks/adversarial-v1, reference-consistency/adversarial-v1, source-fidelity/adversarial-v1

Matrix identity `98ae8a0efd7295ca85ba9e98a4dab25d0decd301e9e798058019b88302ba8b15`, descriptor `4c602ca830f42939983cd1f82e874a880633940e94263ba7403841d7558a704b`. 100 decisions; 100 labelled items scored across 10 cells; $28.3130 of $75.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## anthropic:claude-sonnet-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 3 / 5 | 0.600 | 0.231 to 0.882 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 0 / 5 | 0.000 | 0.000 to 0.434 |
| Unsure | 4 / 10 | 0.400 | 0.168 to 0.687 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 3 | 2 | 0 |
| pass | 3 | 0 | 2 | 0 |

Localisation over detected failures with labelled regions: 3 hits of 3 labelled regions, 10 spurious of 13 predictions.
Cost $0.2592 total, $0.0271 median per item. Wall 124237 ms total, 12445 ms median.
Repetitions 1.

## anthropic:claude-sonnet-5-5/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 3 / 5 | 0.600 | 0.231 to 0.882 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 1 / 5 | 0.200 | 0.036 to 0.624 |
| Unsure | 6 / 10 | 0.600 | 0.313 to 0.832 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 3 | 2 | 0 |
| pass | 0 | 1 | 4 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 3 labelled regions, 11 spurious of 13 predictions.
Cost $0.8085 total, $0.0839 median per item. Wall 207543 ms total, 21008 ms median.
Repetitions 1.

## google-ai-studio:gemini-3.8-flash/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 4 / 5 | 0.800 | 0.376 to 0.964 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 0 / 5 | 0.000 | 0.000 to 0.434 |
| Unsure | 2 / 10 | 0.200 | 0.057 to 0.510 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 4 | 1 | 0 |
| pass | 4 | 0 | 1 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 4 labelled regions, 9 spurious of 11 predictions.
Cost $0.1015 total, $0.0088 median per item. Wall 216341 ms total, 18479 ms median.
Repetitions 1.

## google-ai-studio:gemini-3.8-flash/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 4 / 5 | 0.800 | 0.376 to 0.964 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 0 / 5 | 0.000 | 0.000 to 0.434 |
| Unsure | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Errors | 2 / 10 | 0.200 | 0.057 to 0.510 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 4 | 0 | 1 |
| pass | 4 | 0 | 0 | 1 |

Localisation over detected failures with labelled regions: 3 hits of 4 labelled regions, 8 spurious of 11 predictions.
Cost $0.3124 total, $0.0253 median per item. Wall 415775 ms total, 37275 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 0 / 5 | 0.000 | 0.000 to 0.434 |
| Unsure | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 5 | 0 | 0 | 0 |

Localisation over detected failures with labelled regions: 3 hits of 5 labelled regions, 13 spurious of 16 predictions.
Cost $0.1598 total, $0.0163 median per item. Wall 218761 ms total, 14731 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 1 / 5 | 0.200 | 0.036 to 0.624 |
| Unsure | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 4 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 3 hits of 5 labelled regions, 12 spurious of 15 predictions.
Cost $0.2506 total, $0.0247 median per item. Wall 358787 ms total, 28755 ms median.
Repetitions 1.

## openai:gpt-6.1-sol/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 1 / 5 | 0.200 | 0.036 to 0.624 |
| Unsure | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 3 | 1 | 1 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 5 labelled regions, 13 spurious of 15 predictions.
Cost $0.1853 total, $0.0180 median per item. Wall 134459 ms total, 12672 ms median.
Repetitions 1.

## openai:gpt-6.1-sol/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 1 / 5 | 0.200 | 0.036 to 0.624 |
| Unsure | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 3 | 1 | 1 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 5 labelled regions, 13 spurious of 15 predictions.
Cost $0.2783 total, $0.0275 median per item. Wall 178810 ms total, 16857 ms median.
Repetitions 1.

## openai:gpt-6.1-terra/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 3 / 5 | 0.600 | 0.231 to 0.882 |
| Unsure | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 2 | 3 | 0 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 5 labelled regions, 15 spurious of 17 predictions.
Cost $0.0962 total, $0.0088 median per item. Wall 115483 ms total, 11417 ms median.
Repetitions 1.

## openai:gpt-6.1-terra/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 2 / 5 | 0.400 | 0.118 to 0.769 |
| Unsure | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 2 | 2 | 1 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 5 labelled regions, 15 spurious of 17 predictions.
Cost $0.1499 total, $0.0142 median per item. Wall 180057 ms total, 17862 ms median.
Repetitions 1.

## Paired: anthropic:claude-sonnet-5-5 under C against A

10 items scored under both arms (repetition 1); 4 flipped. Detected failures: 0 gained, 0 lost. False rejects: 1 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| product-fidelity/adversarial-v1/honey-card-pass | Pass | Pass | Unsure |
| reality-checks/adversarial-v1/florist-sign-pass | Pass | Pass | Unsure |
| reference-consistency/adversarial-v1/biscuit-sign-pass | Pass | Pass | Unsure |
| source-fidelity/adversarial-v1/garden-shed-sign-pass | Pass | Unsure | Fail |

## Paired: google-ai-studio:gemini-3.8-flash under C against A

10 items scored under both arms (repetition 1); 4 flipped. Detected failures: 1 gained, 1 lost. False rejects: 0 gained, 0 lost. Families with an expected-fail item whose detection differs: 2.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval -0.6000 to 0.6000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| reality-checks/adversarial-v1/florist-sign-pass | Pass | Unsure | Pass |
| reference-consistency/adversarial-v1/biscuit-sign-no-blaze | Fail | Unsure | Fail |
| source-fidelity/adversarial-v1/garden-shed-sign-mirror | Fail | Fail | Error |
| source-fidelity/adversarial-v1/garden-shed-sign-pass | Pass | Pass | Error |

## Paired: google-ai-studio:gemini-4-argon under C against A

10 items scored under both arms (repetition 1); 1 flipped. Detected failures: 0 gained, 0 lost. False rejects: 1 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| brief-compliance/adversarial-v1/porch-sign-pass | Pass | Pass | Fail |

## Paired: openai:gpt-6.1-sol under C against A

10 items scored under both arms (repetition 1); 0 flipped. Detected failures: 0 gained, 0 lost. False rejects: 0 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

## Paired: openai:gpt-6.1-terra under C against A

10 items scored under both arms (repetition 1); 1 flipped. Detected failures: 0 gained, 0 lost. False rejects: 0 gained, 1 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| reference-consistency/adversarial-v1/biscuit-sign-pass | Pass | Fail | Unsure |

## Every set together

Matrix identity `98ae8a0efd7295ca85ba9e98a4dab25d0decd301e9e798058019b88302ba8b15`, descriptor `4c602ca830f42939983cd1f82e874a880633940e94263ba7403841d7558a704b`. 1100 decisions; 1100 labelled items scored across 10 cells; $28.3130 of $75.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## anthropic:claude-sonnet-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 59 / 65 | 0.908 | 0.813 to 0.957 |
| Missed failures | 1 / 65 | 0.015 | 0.003 to 0.082 |
| False rejects | 5 / 45 | 0.111 | 0.048 to 0.235 |
| Unsure | 22 / 110 | 0.200 | 0.136 to 0.284 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 59 | 5 | 0 |
| pass | 23 | 5 | 17 | 0 |

Localisation over detected failures with labelled regions: 25 hits of 59 labelled regions, 202 spurious of 227 predictions.
Cost $2.8537 total, $0.0250 median per item. Wall 1289955 ms total, 11632 ms median.
Repetitions 1.

## anthropic:claude-sonnet-5-5/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 59 / 65 | 0.908 | 0.813 to 0.957 |
| Missed failures | 0 / 65 | 0.000 | 0.000 to 0.056 |
| False rejects | 5 / 45 | 0.111 | 0.048 to 0.235 |
| Unsure | 35 / 110 | 0.318 | 0.239 to 0.410 |
| Errors | 1 / 110 | 0.009 | 0.002 to 0.050 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 59 | 6 | 0 |
| pass | 10 | 5 | 29 | 1 |

Localisation over detected failures with labelled regions: 24 hits of 59 labelled regions, 204 spurious of 228 predictions.
Cost $8.8387 total, $0.0814 median per item. Wall 2387299 ms total, 20054 ms median.
Repetitions 1.

## google-ai-studio:gemini-3.8-flash/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 58 / 65 | 0.892 | 0.794 to 0.947 |
| Missed failures | 2 / 65 | 0.031 | 0.008 to 0.105 |
| False rejects | 7 / 45 | 0.156 | 0.077 to 0.288 |
| Unsure | 6 / 110 | 0.055 | 0.025 to 0.114 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 58 | 5 | 0 |
| pass | 37 | 7 | 1 | 0 |

Localisation over detected failures with labelled regions: 22 hits of 58 labelled regions, 173 spurious of 195 predictions.
Cost $1.1743 total, $0.0098 median per item. Wall 2439020 ms total, 19126 ms median.
Repetitions 1.

## google-ai-studio:gemini-3.8-flash/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 60 / 65 | 0.923 | 0.832 to 0.967 |
| Missed failures | 2 / 65 | 0.031 | 0.008 to 0.105 |
| False rejects | 10 / 45 | 0.222 | 0.125 to 0.363 |
| Unsure | 5 / 110 | 0.045 | 0.020 to 0.102 |
| Errors | 2 / 110 | 0.018 | 0.005 to 0.064 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 60 | 2 | 1 |
| pass | 31 | 10 | 3 | 1 |

Localisation over detected failures with labelled regions: 23 hits of 60 labelled regions, 177 spurious of 200 predictions.
Cost $2.4515 total, $0.0193 median per item. Wall 5228506 ms total, 41551 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 65 / 65 | 1.000 | 0.944 to 1.000 |
| Missed failures | 0 / 65 | 0.000 | 0.000 to 0.056 |
| False rejects | 12 / 45 | 0.267 | 0.160 to 0.410 |
| Unsure | 0 / 110 | 0.000 | 0.000 to 0.034 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 65 | 0 | 0 |
| pass | 33 | 12 | 0 | 0 |

Localisation over detected failures with labelled regions: 27 hits of 65 labelled regions, 210 spurious of 237 predictions.
Cost $1.8662 total, $0.0165 median per item. Wall 2057215 ms total, 16609 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 65 / 65 | 1.000 | 0.944 to 1.000 |
| Missed failures | 0 / 65 | 0.000 | 0.000 to 0.056 |
| False rejects | 14 / 45 | 0.311 | 0.195 to 0.457 |
| Unsure | 0 / 110 | 0.000 | 0.000 to 0.034 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 65 | 0 | 0 |
| pass | 31 | 14 | 0 | 0 |

Localisation over detected failures with labelled regions: 25 hits of 65 labelled regions, 205 spurious of 230 predictions.
Cost $2.9231 total, $0.0253 median per item. Wall 3974570 ms total, 30961 ms median.
Repetitions 1.

## openai:gpt-6.1-sol/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 63 / 65 | 0.969 | 0.895 to 0.992 |
| Missed failures | 2 / 65 | 0.031 | 0.008 to 0.105 |
| False rejects | 11 / 45 | 0.244 | 0.142 to 0.387 |
| Unsure | 2 / 110 | 0.018 | 0.005 to 0.064 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 63 | 0 | 0 |
| pass | 32 | 11 | 2 | 0 |

Localisation over detected failures with labelled regions: 21 hits of 63 labelled regions, 209 spurious of 230 predictions.
Cost $2.2003 total, $0.0183 median per item. Wall 1422678 ms total, 12611 ms median.
Repetitions 1.

## openai:gpt-6.1-sol/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 63 / 65 | 0.969 | 0.895 to 0.992 |
| Missed failures | 1 / 65 | 0.015 | 0.003 to 0.082 |
| False rejects | 10 / 45 | 0.222 | 0.125 to 0.363 |
| Unsure | 4 / 110 | 0.036 | 0.014 to 0.090 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 63 | 1 | 0 |
| pass | 32 | 10 | 3 | 0 |

Localisation over detected failures with labelled regions: 19 hits of 63 labelled regions, 210 spurious of 229 predictions.
Cost $3.2268 total, $0.0281 median per item. Wall 2017968 ms total, 17879 ms median.
Repetitions 1.

## openai:gpt-6.1-terra/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 62 / 65 | 0.954 | 0.873 to 0.984 |
| Missed failures | 2 / 65 | 0.031 | 0.008 to 0.105 |
| False rejects | 11 / 45 | 0.244 | 0.142 to 0.387 |
| Unsure | 9 / 110 | 0.082 | 0.044 to 0.148 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 62 | 1 | 0 |
| pass | 26 | 11 | 8 | 0 |

Localisation over detected failures with labelled regions: 21 hits of 62 labelled regions, 206 spurious of 227 predictions.
Cost $1.0982 total, $0.0092 median per item. Wall 1261949 ms total, 11053 ms median.
Repetitions 1.

## openai:gpt-6.1-terra/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 62 / 65 | 0.954 | 0.873 to 0.984 |
| Missed failures | 1 / 65 | 0.015 | 0.003 to 0.082 |
| False rejects | 11 / 45 | 0.244 | 0.142 to 0.387 |
| Unsure | 13 / 110 | 0.118 | 0.070 to 0.192 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 62 | 2 | 0 |
| pass | 23 | 11 | 11 | 0 |

Localisation over detected failures with labelled regions: 19 hits of 62 labelled regions, 214 spurious of 233 predictions.
Cost $1.6803 total, $0.0149 median per item. Wall 2009402 ms total, 17975 ms median.
Repetitions 1.

## Paired: anthropic:claude-sonnet-5-5 under C against A

110 items scored under both arms (repetition 1); 18 flipped. Detected failures: 1 gained, 1 lost. False rejects: 1 gained, 1 lost. Families with an expected-fail item whose detection differs: 1.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 30 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| brief-compliance/stills-v1/cafe-window-asked | Pass | Fail | Unsure |
| product-fidelity/adversarial-v1/honey-card-pass | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/coffee-tin-kitchen | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/jam-breakfast | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/matches-mantel | Pass | Pass | Unsure |
| product-fidelity/packshots-v1/toothpaste-bathroom | Pass | Pass | Unsure |
| reality-checks/adversarial-v1/florist-sign-pass | Pass | Pass | Unsure |
| reality-checks/people-v1/barista-clean | Pass | Pass | Unsure |
| reality-checks/people-v1/barista-mono | Pass | Pass | Unsure |
| reality-checks/people-v1/market-garbled-sign | Fail | Unsure | Fail |
| reality-checks/people-v1/market-six-fingers | Fail | Fail | Unsure |
| reality-checks/people-v1/skatepark-clean | Pass | Pass | Unsure |
| reality-checks/people-v1/skatepark-garbled-wall | Fail | Pass | Unsure |
| reference-consistency/adversarial-v1/biscuit-sign-pass | Pass | Pass | Unsure |
| reference-consistency/pets-v1/ada-kitchen | Pass | Pass | Unsure |
| reference-consistency/pets-v1/pip-beach | Pass | Pass | Unsure |
| source-fidelity/adversarial-v1/garden-shed-sign-pass | Pass | Unsure | Fail |
| source-fidelity/scenes-v1/red-barn-grade | Pass | Pass | Error |

## Paired: google-ai-studio:gemini-3.8-flash under C against A

110 items scored under both arms (repetition 1); 16 flipped. Detected failures: 5 gained, 3 lost. False rejects: 3 gained, 0 lost. Families with an expected-fail item whose detection differs: 8.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 30 families, 1 hypothesis: point 0.0308, interval -0.0526 to 0.1176 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| brief-compliance/stills-v1/cafe-window-no-cups | Fail | Fail | Unsure |
| brief-compliance/stills-v1/desk-misspelt | Fail | Unsure | Fail |
| brief-compliance/stills-v1/kids-room-misspelt | Fail | Unsure | Fail |
| product-fidelity/packshots-v1/tonic-bar | Pass | Pass | Fail |
| reality-checks/adversarial-v1/florist-sign-pass | Pass | Unsure | Pass |
| reality-checks/people-v1/barista-mono | Pass | Pass | Unsure |
| reality-checks/people-v1/cyclist-rain | Pass | Pass | Fail |
| reality-checks/people-v1/skatepark-clean | Pass | Pass | Fail |
| reality-checks/people-v1/skatepark-sunset | Pass | Pass | Unsure |
| reference-consistency/adversarial-v1/biscuit-sign-no-blaze | Fail | Unsure | Fail |
| reference-consistency/pets-v1/ada-kitchen-grey-tail | Fail | Unsure | Fail |
| reference-consistency/pets-v1/bramble-meadow-lookalike | Fail | Unsure | Fail |
| source-fidelity/adversarial-v1/garden-shed-sign-mirror | Fail | Fail | Error |
| source-fidelity/adversarial-v1/garden-shed-sign-pass | Pass | Pass | Error |
| source-fidelity/scenes-v1/hotel-pool-grade | Pass | Pass | Unsure |
| source-fidelity/scenes-v1/village-street-no-tower | Fail | Fail | Unsure |

## Paired: google-ai-studio:gemini-4-argon under C against A

110 items scored under both arms (repetition 1); 2 flipped. Detected failures: 0 gained, 0 lost. False rejects: 2 gained, 0 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 30 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| brief-compliance/adversarial-v1/porch-sign-pass | Pass | Pass | Fail |
| reference-consistency/pets-v1/bramble-beach | Pass | Pass | Fail |

## Paired: openai:gpt-6.1-sol under C against A

110 items scored under both arms (repetition 1); 4 flipped. Detected failures: 0 gained, 0 lost. False rejects: 1 gained, 2 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 30 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| product-fidelity/packshots-v1/matches-mantel | Pass | Fail | Pass |
| reality-checks/people-v1/market-clean | Pass | Fail | Unsure |
| reality-checks/people-v1/skatepark-wrong-shadow | Fail | Pass | Unsure |
| reference-consistency/pets-v1/ada-library | Pass | Pass | Fail |

## Paired: openai:gpt-6.1-terra under C against A

110 items scored under both arms (repetition 1); 9 flipped. Detected failures: 0 gained, 0 lost. False rejects: 2 gained, 2 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (C minus A), seed 20261028, 10000 resamples over 30 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | A | C |
|---|---|---|---|
| product-fidelity/packshots-v1/matches-mantel | Pass | Unsure | Fail |
| product-fidelity/packshots-v1/tonic-bar | Pass | Pass | Fail |
| reality-checks/people-v1/cyclist-clean | Pass | Unsure | Pass |
| reality-checks/people-v1/market-evening | Pass | Fail | Unsure |
| reality-checks/people-v1/skatepark-sunset | Pass | Pass | Unsure |
| reality-checks/people-v1/skatepark-wrong-shadow | Fail | Pass | Unsure |
| reference-consistency/adversarial-v1/biscuit-sign-pass | Pass | Fail | Unsure |
| reference-consistency/pets-v1/ada-library | Pass | Pass | Unsure |
| reference-consistency/pets-v1/bramble-snow | Pass | Pass | Unsure |
