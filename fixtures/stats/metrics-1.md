# Evaluation report: metrics-1

Matrix identity `0000000000000000000000000000000000000000000000000000000000000000`, descriptor `1111111111111111111111111111111111111111111111111111111111111111`. 0 decisions; 6 labelled items scored across 1 cell; $1.5000 of $5.00 spent. Headline numbers come from repetition 1; the frozen policy sends unsure to `Review` and error to `Review`.

## fake:fake-model/A

| Metric | Count | Rate | 95% Wilson |
|---|---|---|---|
| Detected failures | 1 / 3 | 0.333 | 0.061 to 0.792 |
| Missed failures | 1 / 3 | 0.333 | 0.061 to 0.792 |
| False rejects | 1 / 3 | 0.333 | 0.061 to 0.792 |
| Unsure | 1 / 6 | 0.167 | 0.030 to 0.564 |
| Errors | 1 / 6 | 0.167 | 0.030 to 0.564 |

Confusion, expected by row and computed by column:

| Expected | pass | fail | unsure | error |
|---|---|---|---|---|
| fail | 1 | 1 | 1 | 0 |
| pass | 1 | 1 | 0 | 1 |

Localisation over detected failures with labelled regions: 2 hits of 2 labelled regions, 1 spurious of 3 predictions.
Cost $1.5000 total, $0.2500 median per item. Wall 1550 ms total, 250 ms median.
Repetitions 2: later repetitions agree with repetition 1 on 0.500 of items.
