# gate-regions-initial-2026-10: the initial regions gate, rejected

**These are development sets.** Every item here was built, reviewed and labelled by the
owner, and the sets are public; prompts and tools are tuned against them, so these numbers
describe the harness on material it may have been shaped by, not on a blind set. The
adversarial sets are reported separately below. Repetition 1 is the headline; there is one repetition.

- Matrix identity `7f1852fd00c6a30c0f3e39687ee8b7cd09d04fc1dc3758662fd61bccd9b7dc0c`, descriptor `66b077dd28048f3f125c59470984504a90dc5e4d0a08062def4b5155017bb11e`
- Harness 0.1.0 (a6dde9e2e), prompt v1, catalog version 1
- 40 decisions; $1.3172 of $25.00 spent; completed 40, resumed 0, partial redone 0
- Files beside this document: `matrix.toml`, `summary.json`, `metrics.json`, `descriptor.json`, `notes.md`

## Notes

Run on 2026-10-29 from the matrix beside this document: the initial admission gate of the
`regions` tool for source fidelity (PLAN task 5.6). Arms C and D_regions on the sealed blind set
`source-fidelity/blind-v1` (20 items in 5 families, 10 expected fail and 10 expected pass) under
gemini-4-argon, one repetition, decision cap $10, matrix cap $25, bootstrap seed 20261030, 10,000
resamples, one hypothesis. The labels were held by the owner until the gate scored; receipt
`8e0c8307a72475ac`.

- **Outcome: rejected.** Both arms detected every one of the ten planted failures, so the
  detected-failure difference is zero, its paired interval is zero to zero, and no family's
  detection differs between the arms. The gate holds a tool to "detected failures higher than C"
  and that cannot be met on a set the control already gets entirely right. The result says the
  set is too easy for this model, not that the tool is useless: the admission rule is doing what
  it was written to do.
- **What the tool did change.** The one flip was on the pass side: under C the model rejected
  `greenhouse-interior-grade`, a colour grade, as a failure; under D_regions, with `regions`
  reporting what differed, it passed it. False rejects went from one to none. Localisation of the
  ten detected failures went from 9 hits of 10 labelled regions to 10 of 10, with more spurious
  predictions (30 of 40 against 20 of 29): the tool's evidence items are pointers the model
  then cites, and it cited more of them.
- **How the model used the tools.** Under C the model called `align` on every item (15 registered;
  the 5 structural transforms came back not applicable, which it read correctly as the SF2
  violation) and `view` on 11. Under D_regions it added `regions` on 14 of 20 items (13 answered
  with regions, one measured nothing), which added 82 evidence items to the checklists, and never
  called `classes`. Turns rose from 44 to 62.
- **Spend.** The ledger's $1.32 is the vendor-side figure: $0.52 under C, $0.79 under D_regions
  (1.5 times), all Google. No errors, no unsure answers, no resume.
- **Binary.** The run used a build of `3957e6b` (the `m5-ml-tool` branch after the blind set was
  published). The pack was the installed bundle 0.0.1 at lock digest `ca3e746…`, with the
  segmenter fetched from the export's directory, as one process for the whole evaluation.
- **Two things learned about the harness.** The summary of this run recorded decision
  directories as absolute paths, because the run now makes them absolute for the packs it
  restarts there while the evaluation directory stays as given; the runner strips both forms
  from now on, and the published `summary.json` here is relativised by the results-doc script
  (the receipt's results hash covers the summary as written, so the run's own file is left
  alone). And the blind set's item ids name their constructions, as the dev sets' do; the model
  never sees an id, so the gate stands, but the next blind plan gives items opaque ids.

## Cells: resolved model, host, API version, cost

| Cell | Profile | Host | Resolved model | API version | Decisions | Errors | Cost |
|---|---|---|---|---|---|---|---|
| google-ai-studio:gemini-4-argon/C | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 20 | 0 | $0.5248 |
| google-ai-studio:gemini-4-argon/D_regions | google-ai-studio | generativelanguage.googleapis.com | gemini-4-argon | v1beta | 20 | 0 | $0.7924 |

## Development sets

Sets: source-fidelity/blind-v1

Matrix identity `7f1852fd00c6a30c0f3e39687ee8b7cd09d04fc1dc3758662fd61bccd9b7dc0c`, descriptor `66b077dd28048f3f125c59470984504a90dc5e4d0a08062def4b5155017bb11e`. 40 decisions; 40 labelled items scored across 2 cells; $1.3172 of $25.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 10 / 10 | 1.000 | 0.722 to 1.000 |
| Missed failures | 0 / 10 | 0.000 | 0.000 to 0.278 |
| False rejects | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Unsure | 0 / 20 | 0.000 | 0.000 to 0.161 |
| Errors | 0 / 20 | 0.000 | 0.000 to 0.161 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 10 | 0 | 0 |
| pass | 9 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 9 hits of 10 labelled regions, 20 spurious of 29 predictions.
Cost $0.5248 total, $0.0259 median per item. Wall 782178 ms total, 35520 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/D_regions

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 10 / 10 | 1.000 | 0.722 to 1.000 |
| Missed failures | 0 / 10 | 0.000 | 0.000 to 0.278 |
| False rejects | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Unsure | 0 / 20 | 0.000 | 0.000 to 0.161 |
| Errors | 0 / 20 | 0.000 | 0.000 to 0.161 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 10 | 0 | 0 |
| pass | 10 | 0 | 0 | 0 |

Localisation over detected failures with labelled regions: 10 hits of 10 labelled regions, 30 spurious of 40 predictions.
Cost $0.7924 total, $0.0368 median per item. Wall 1170828 ms total, 54342 ms median.
Repetitions 1.

## Paired: google-ai-studio:gemini-4-argon under D_regions against C

20 items scored under both arms (repetition 1); 1 flipped. Detected failures: 0 gained, 0 lost. False rejects: 0 gained, 1 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (D_regions minus C), seed 20261030, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | C | D_regions |
|---|---|---|---|
| source-fidelity/blind-v1/greenhouse-interior-grade | Pass | Fail | Pass |

## Adversarial sets

Sets: 

(none in this matrix)

## Every set together

Matrix identity `7f1852fd00c6a30c0f3e39687ee8b7cd09d04fc1dc3758662fd61bccd9b7dc0c`, descriptor `66b077dd28048f3f125c59470984504a90dc5e4d0a08062def4b5155017bb11e`. 40 decisions; 40 labelled items scored across 2 cells; $1.3172 of $25.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## google-ai-studio:gemini-4-argon/C

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 10 / 10 | 1.000 | 0.722 to 1.000 |
| Missed failures | 0 / 10 | 0.000 | 0.000 to 0.278 |
| False rejects | 1 / 10 | 0.100 | 0.018 to 0.404 |
| Unsure | 0 / 20 | 0.000 | 0.000 to 0.161 |
| Errors | 0 / 20 | 0.000 | 0.000 to 0.161 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 10 | 0 | 0 |
| pass | 9 | 1 | 0 | 0 |

Localisation over detected failures with labelled regions: 9 hits of 10 labelled regions, 20 spurious of 29 predictions.
Cost $0.5248 total, $0.0259 median per item. Wall 782178 ms total, 35520 ms median.
Repetitions 1.

## google-ai-studio:gemini-4-argon/D_regions

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 10 / 10 | 1.000 | 0.722 to 1.000 |
| Missed failures | 0 / 10 | 0.000 | 0.000 to 0.278 |
| False rejects | 0 / 10 | 0.000 | 0.000 to 0.278 |
| Unsure | 0 / 20 | 0.000 | 0.000 to 0.161 |
| Errors | 0 / 20 | 0.000 | 0.000 to 0.161 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 0 | 10 | 0 | 0 |
| pass | 10 | 0 | 0 | 0 |

Localisation over detected failures with labelled regions: 10 hits of 10 labelled regions, 30 spurious of 40 predictions.
Cost $0.7924 total, $0.0368 median per item. Wall 1170828 ms total, 54342 ms median.
Repetitions 1.

## Paired: google-ai-studio:gemini-4-argon under D_regions against C

20 items scored under both arms (repetition 1); 1 flipped. Detected failures: 0 gained, 0 lost. False rejects: 0 gained, 1 lost. Families with an expected-fail item whose detection differs: 0.
Paired bootstrap of the detected-failure difference (D_regions minus C), seed 20261030, 10000 resamples over 5 families, 1 hypothesis: point 0.0000, interval 0.0000 to 0.0000 at percentiles 0.0250 and 0.9750.

| Item | Expected | C | D_regions |
|---|---|---|---|
| source-fidelity/blind-v1/greenhouse-interior-grade | Pass | Fail | Pass |
