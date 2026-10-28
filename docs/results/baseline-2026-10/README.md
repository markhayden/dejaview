# baseline-a-2026-10: arm A baseline

**These are development sets.** Every item here was built, reviewed and labelled by the
owner, and the sets are public; prompts and tools are tuned against them, so these numbers
describe the harness on material it may have been shaped by, not on a blind set. The
adversarial sets are reported separately below. Repetition 1 is the headline; repetition 2
is reported as agreement.

- Matrix identity `6a7e0995261d6541310923636e60312e4028017887789c817b43cf7f0bdc9054`, descriptor `ee0930d5efc9593edb3ffcdab40c62ad003e94950bd9cf2d65a3bde3351d1426`
- Harness 0.1.0 (f731caa65), prompt v1, catalog version 1
- 1320 decisions; $46.5399 of $75.00 spent; completed 68, resumed 1252, partial redone 0
- Files beside this document: `matrix.toml`, `summary.json`, `metrics.json`, `descriptor.json`, `notes.md`

## Notes

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

## Cells: resolved model, host, API version, cost

| Cell | Profile | Host | Resolved model | API version | Decisions | Errors | Cost |
|---|---|---|---|---|---|---|---|
| anthropic:claude-opus-5-5/A | anthropic | api.anthropic.com | claude-opus-5-5 | 2023-06-01 | 220 | 0 | $12.0326 |
| anthropic:claude-sonnet-5-5/A | anthropic | api.anthropic.com | claude-sonnet-5-5 | 2023-06-01 | 220 | 0 | $5.8749 |
| google-ai-studio:gemini-3.8-flash/A | google-ai-studio | generativelanguage.googleapis.com | gemini-3.8-flash | v1beta | 220 | 0 | $2.2635 |
| google-ai-studio:gemini-4-argon/A | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 220 | 0 | $3.3681 |
| openai:gpt-6.1-sol/A | openai | api.openai.com | gpt-6.1-sol | v1 | 220 | 0 | $3.0116 |
| openai:gpt-6.1-terra/A | openai | api.openai.com | gpt-6.1-terra | v1 | 220 | 0 | $1.5137 |

## Development sets

Sets: brief-compliance/stills-v1, product-fidelity/packshots-v1, reality-checks/people-v1, reference-consistency/pets-v1, source-fidelity/scenes-v1

Matrix identity `6a7e0995261d6541310923636e60312e4028017887789c817b43cf7f0bdc9054`, descriptor `ee0930d5efc9593edb3ffcdab40c62ad003e94950bd9cf2d65a3bde3351d1426`. 1200 decisions; 600 labelled items scored across 6 cells; $46.5399 of $75.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## anthropic:claude-opus-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 57 / 60 | 0.950 | 0.863 to 0.983 |
| Missed failures | 0 / 60 | 0.000 | 0.000 to 0.060 |
| False rejects | 8 / 40 | 0.200 | 0.105 to 0.348 |
| Unsure | 22 / 100 | 0.220 | 0.150 to 0.311 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 57 | 3 | 0 |
| pass | 13 | 8 | 19 | 0 |

Localisation over detected failures with labelled regions: 24 hits of 57 labelled regions, 192 spurious of 216 predictions.
Cost $5.3409 total, $0.0501 median per item. Wall 1540960 ms total, 14815 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.940 of items.

## anthropic:claude-sonnet-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 56 / 60 | 0.933 | 0.841 to 0.974 |
| Missed failures | 0 / 60 | 0.000 | 0.000 to 0.060 |
| False rejects | 5 / 40 | 0.125 | 0.055 to 0.261 |
| Unsure | 21 / 100 | 0.210 | 0.142 to 0.300 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 56 | 4 | 0 |
| pass | 18 | 5 | 17 | 0 |

Localisation over detected failures with labelled regions: 23 hits of 56 labelled regions, 189 spurious of 212 predictions.
Cost $2.7049 total, $0.0252 median per item. Wall 1207007 ms total, 11166 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.940 of items.

## google-ai-studio:gemini-3.8-flash/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 52 / 60 | 0.867 | 0.758 to 0.931 |
| Missed failures | 3 / 60 | 0.050 | 0.017 to 0.137 |
| False rejects | 7 / 40 | 0.175 | 0.087 to 0.319 |
| Unsure | 9 / 100 | 0.090 | 0.048 to 0.162 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 3 | 52 | 5 | 0 |
| pass | 29 | 7 | 4 | 0 |

Localisation over detected failures with labelled regions: 19 hits of 52 labelled regions, 154 spurious of 173 predictions.
Cost $1.1258 total, $0.0095 median per item. Wall 2413930 ms total, 20176 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.850 of items.

## google-ai-studio:gemini-4-argon/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 60 / 60 | 1.000 | 0.940 to 1.000 |
| Missed failures | 0 / 60 | 0.000 | 0.000 to 0.060 |
| False rejects | 11 / 40 | 0.275 | 0.161 to 0.428 |
| Unsure | 2 / 100 | 0.020 | 0.006 to 0.070 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 60 | 0 | 0 |
| pass | 27 | 11 | 2 | 0 |

Localisation over detected failures with labelled regions: 25 hits of 60 labelled regions, 189 spurious of 214 predictions.
Cost $1.7288 total, $0.0169 median per item. Wall 1871258 ms total, 16225 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.970 of items.

## openai:gpt-6.1-sol/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 58 / 60 | 0.967 | 0.886 to 0.991 |
| Missed failures | 1 / 60 | 0.017 | 0.003 to 0.089 |
| False rejects | 11 / 40 | 0.275 | 0.161 to 0.428 |
| Unsure | 3 / 100 | 0.030 | 0.010 to 0.085 |
| Errors | 0 / 100 | 0.000 | 0.000 to 0.037 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 58 | 1 | 0 |
| pass | 27 | 11 | 2 | 0 |

Localisation over detected failures with labelled regions: 17 hits of 58 labelled regions, 196 spurious of 213 predictions.
Cost $1.9885 total, $0.0183 median per item. Wall 1284721 ms total, 12420 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.960 of items.

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

Localisation over detected failures with labelled regions: 18 hits of 57 labelled regions, 196 spurious of 214 predictions.
Cost $1.0090 total, $0.0093 median per item. Wall 1148701 ms total, 10945 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.970 of items.

## Adversarial sets

Sets: brief-compliance/adversarial-v1, product-fidelity/adversarial-v1, reality-checks/adversarial-v1, reference-consistency/adversarial-v1, source-fidelity/adversarial-v1

Matrix identity `6a7e0995261d6541310923636e60312e4028017887789c817b43cf7f0bdc9054`, descriptor `ee0930d5efc9593edb3ffcdab40c62ad003e94950bd9cf2d65a3bde3351d1426`. 120 decisions; 60 labelled items scored across 6 cells; $46.5399 of $75.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## anthropic:claude-opus-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 4 / 5 | 0.800 | 0.376 to 0.964 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 0 / 5 | 0.000 | 0.000 to 0.434 |
| Unsure | 6 / 10 | 0.600 | 0.313 to 0.832 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 4 | 1 | 0 |
| pass | 0 | 0 | 5 | 0 |

Localisation over detected failures with labelled regions: 3 hits of 4 labelled regions, 13 spurious of 16 predictions.
Cost $0.5943 total, $0.0541 median per item. Wall 173560 ms total, 17332 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.900 of items.

## anthropic:claude-sonnet-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 0 / 5 | 0.000 | 0.000 to 0.434 |
| Unsure | 2 / 10 | 0.200 | 0.057 to 0.510 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 3 | 0 | 2 | 0 |

Localisation over detected failures with labelled regions: 3 hits of 5 labelled regions, 13 spurious of 16 predictions.
Cost $0.2878 total, $0.0270 median per item. Wall 131013 ms total, 12374 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.800 of items.

## google-ai-studio:gemini-3.8-flash/A

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
Cost $0.0917 total, $0.0090 median per item. Wall 192738 ms total, 18228 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.900 of items.

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
Cost $0.1801 total, $0.0162 median per item. Wall 202595 ms total, 14973 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.900 of items.

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

Localisation over detected failures with labelled regions: 2 hits of 5 labelled regions, 12 spurious of 14 predictions.
Cost $0.1912 total, $0.0190 median per item. Wall 129139 ms total, 11753 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.900 of items.

## openai:gpt-6.1-terra/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 5 / 5 | 1.000 | 0.566 to 1.000 |
| Missed failures | 0 / 5 | 0.000 | 0.000 to 0.434 |
| False rejects | 2 / 5 | 0.400 | 0.118 to 0.769 |
| Unsure | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Errors | 0 / 10 | 0.000 | 0.000 to 0.278 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 5 | 0 | 0 |
| pass | 3 | 2 | 0 | 0 |

Localisation over detected failures with labelled regions: 2 hits of 5 labelled regions, 14 spurious of 16 predictions.
Cost $0.0968 total, $0.0090 median per item. Wall 110968 ms total, 11277 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.900 of items.

## Every set together

Matrix identity `6a7e0995261d6541310923636e60312e4028017887789c817b43cf7f0bdc9054`, descriptor `ee0930d5efc9593edb3ffcdab40c62ad003e94950bd9cf2d65a3bde3351d1426`. 1320 decisions; 660 labelled items scored across 6 cells; $46.5399 of $75.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## anthropic:claude-opus-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 61 / 65 | 0.938 | 0.852 to 0.976 |
| Missed failures | 0 / 65 | 0.000 | 0.000 to 0.056 |
| False rejects | 8 / 45 | 0.178 | 0.093 to 0.313 |
| Unsure | 28 / 110 | 0.255 | 0.182 to 0.343 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 61 | 4 | 0 |
| pass | 13 | 8 | 24 | 0 |

Localisation over detected failures with labelled regions: 27 hits of 61 labelled regions, 205 spurious of 232 predictions.
Cost $5.9352 total, $0.0506 median per item. Wall 1714520 ms total, 14974 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.936 of items.

## anthropic:claude-sonnet-5-5/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 61 / 65 | 0.938 | 0.852 to 0.976 |
| Missed failures | 0 / 65 | 0.000 | 0.000 to 0.056 |
| False rejects | 5 / 45 | 0.111 | 0.048 to 0.235 |
| Unsure | 23 / 110 | 0.209 | 0.144 to 0.294 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 61 | 4 | 0 |
| pass | 21 | 5 | 19 | 0 |

Localisation over detected failures with labelled regions: 26 hits of 61 labelled regions, 202 spurious of 228 predictions.
Cost $2.9928 total, $0.0252 median per item. Wall 1338020 ms total, 11351 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.927 of items.

## google-ai-studio:gemini-3.8-flash/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 57 / 65 | 0.877 | 0.775 to 0.936 |
| Missed failures | 3 / 65 | 0.046 | 0.016 to 0.127 |
| False rejects | 7 / 45 | 0.156 | 0.077 to 0.288 |
| Unsure | 9 / 110 | 0.082 | 0.044 to 0.148 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 3 | 57 | 5 | 0 |
| pass | 34 | 7 | 4 | 0 |

Localisation over detected failures with labelled regions: 22 hits of 57 labelled regions, 167 spurious of 189 predictions.
Cost $1.2175 total, $0.0095 median per item. Wall 2606668 ms total, 19966 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.855 of items.

## google-ai-studio:gemini-4-argon/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 65 / 65 | 1.000 | 0.944 to 1.000 |
| Missed failures | 0 / 65 | 0.000 | 0.000 to 0.056 |
| False rejects | 11 / 45 | 0.244 | 0.142 to 0.387 |
| Unsure | 2 / 110 | 0.018 | 0.005 to 0.064 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 65 | 0 | 0 |
| pass | 32 | 11 | 2 | 0 |

Localisation over detected failures with labelled regions: 28 hits of 65 labelled regions, 202 spurious of 230 predictions.
Cost $1.9089 total, $0.0168 median per item. Wall 2073853 ms total, 16179 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.964 of items.

## openai:gpt-6.1-sol/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 63 / 65 | 0.969 | 0.895 to 0.992 |
| Missed failures | 1 / 65 | 0.015 | 0.003 to 0.082 |
| False rejects | 12 / 45 | 0.267 | 0.160 to 0.410 |
| Unsure | 4 / 110 | 0.036 | 0.014 to 0.090 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 63 | 1 | 0 |
| pass | 30 | 12 | 3 | 0 |

Localisation over detected failures with labelled regions: 19 hits of 63 labelled regions, 208 spurious of 227 predictions.
Cost $2.1797 total, $0.0183 median per item. Wall 1413860 ms total, 12211 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.955 of items.

## openai:gpt-6.1-terra/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 62 / 65 | 0.954 | 0.873 to 0.984 |
| Missed failures | 2 / 65 | 0.031 | 0.008 to 0.105 |
| False rejects | 10 / 45 | 0.222 | 0.125 to 0.363 |
| Unsure | 9 / 110 | 0.082 | 0.044 to 0.148 |
| Errors | 0 / 110 | 0.000 | 0.000 to 0.034 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 2 | 62 | 1 | 0 |
| pass | 27 | 10 | 8 | 0 |

Localisation over detected failures with labelled regions: 20 hits of 62 labelled regions, 210 spurious of 230 predictions.
Cost $1.1058 total, $0.0092 median per item. Wall 1259669 ms total, 10981 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.964 of items.
