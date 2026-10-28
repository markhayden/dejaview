# Evaluation: matrices, the descriptor, the durable ledger, admissions

The contract is `SPEC.md` under "Evaluation" ("Runner and arms", "Gates"); this page is the
shape of the files task 3.4a introduced. The runner (3.4b), the estimate (3.4c), metrics (3.5a)
and the gate (3.5b) build on them.

## `matrix.toml`

```toml
[matrix]
name = "baseline-a"
config = "examples/source-fidelity.toml"   # the base config every cell starts from
sets = ["reference-consistency/pets-v1"]   # set ids under the evalsets root
arms = ["A"]                               # A | C | D_<tool> | D
decision_runs = 1                          # 1 or 2
repetitions = 1                            # per cell; repetition 1 is the decision repetition
spend_cap_usd = 25.0                       # the evaluation's cap, above the per-job cap

[[matrix.models]]
profile = "anthropic"
model = "claude-haiku-5-5"
controls = { effort = "low" }              # optional, validated against the catalog by the runner
```

A **cell** is one model under one arm, named `<profile>:<model>/<arm>`; the matrix makes
`items × cells × repetitions` decisions. The **identity** is a SHA-256 over the matrix content in
canonical form, so formatting does not change it and any setting does. `fixtures/matrix/` holds
the matrices the tests use.

## The execution descriptor

Before the first decision, `eval run` computes `descriptor.json` in the evaluation directory
over every answer-affecting input: harness version; matrix identity and text; the base config
text; prompt version and template bytes; catalog version and content (which carries the profiles);
every set's seal components (items, families, jobs, images, rules, labels); every enabled tool's
frozen manifest and pack lock digest. A resume recomputes it and refuses on any difference,
naming what changed, so an evaluation directory only ever holds results of one configuration. A
`LOCK` file naming the writer's pid admits one writer at a time; a lock whose process is gone is
stale and taken over.

## The durable ledger

`ledger.jsonl` in the evaluation directory is append-only and fsynced per line. A decision's
reservation (its envelope) is written before anything is sent; its settlement is written once.
On open the file is replayed: settled decisions count their settled cost, and a decision
reserved but never settled, which a crash may have sent, is charged at its reservation. Admission
of the next decision is against the cap less everything spent or reserved so far, across every
restart, so cumulative spend never exceeds the cap. A late settlement of a decision already
settled or charged changes nothing. The recovery tests crash the ledger at each of the four
points (before and after the reservation is persisted, before and after the settlement) and
reopen it.

## `evalsets/admissions.toml`

```toml
admissions_version = 1

[[admission]]
tool = "regions"
use_case = "source-fidelity"
stage = "provisional"            # provisional | confirmed
receipt = "rcpt-…"               # the gate receipt that produced the record
```

Written by `dejaview eval gate`; read by the runner, where arm D includes only the tools whose
stage is `confirmed`. One record per tool and use case. `fixtures/evalsets/admissions.toml`
holds the synthetic records the tests use.

## Running a matrix

```
dejaview eval run --matrix matrix.toml --out eval-out/<name> [--evalsets-root evalsets] [--admissions evalsets/admissions.toml]
```

The runner takes the writer lock, loads and checks every set, forms each arm's tool selection
(docs: SPEC "Runner and arms"; `D` reads the admission record), computes the descriptor (writing
it on a first run, refusing on a resume if anything differs), opens the ledger, then runs every
decision in a fixed order: set, item, cell, repetition. Each decision gets its own directory,
`results/<set>/<item>/<profile>_<model>/<arm>/rep-<n>/`, holding the harness's decision directory
and, once complete, `result.json`. On resume a decision with `result.json` is skipped, and one
whose directory holds a started attempt but no result is listed as partial and run again in a
fresh attempt directory; its earlier reservation was charged at its bound by the ledger. The cache
is never read (`cache.status` is `bypass` in every report). When the ledger refuses a decision's
envelope, that decision ends with `error: budget` and the runner carries on, so every decision has
a row. SIGINT or SIGTERM cancels between decisions and inside one; the same command resumes.

`summary.json` lists every decision of the matrix as it stands (set, item, cell, repetition,
whether it is the decision repetition, verdict, exit code, error class, cost, wall time,
directory) with what the run did (completed, skipped, partial, cancelled) and the ledger's
cumulative spend against the cap. Repetition 1 is the decision repetition every gate and headline
number uses; the others are reported descriptively. Projections (`align` under `visual_only` in
arms C, D_X and D) are part of the selection the runner hands to the tool factory and are applied
once the tools arrive in M4.

## The report

Every run ends by writing `metrics.json` and `report.md` next to `summary.json`, and

```
dejaview eval report --out eval-out/<name> [--evalsets-root evalsets]
```

regenerates both from the saved results alone: `summary.json` for the decisions, each decision's
`report.json` for its findings and the policy, and the sets' `labels.toml` for the answers. The
rendering is a pure function of those inputs, so regenerating reproduces the stored Markdown byte
for byte. Only labelled items are scored; an unlabelled item contributes nothing.

Per cell, over repetition 1 (SPEC "Metrics"):

- the confusion of expected `pass | fail` against computed `pass | fail | unsure | error`;
- detected failures (expected fail, computed fail) and missed failures (expected fail, let through
  as a pass by the frozen policy, which is the config's `[policy]` as every report records it),
  both over expected fails; false rejects over expected passes; unsure and error rates over all
  items; each with its 95% Wilson interval;
- localisation over detected failures that have labelled regions: a prediction (a finding with
  disposition `violation`, mapped to the root image's raw frame) and a label are eligible only when
  they name the same root image and their `obligation_ids` intersect; a hit is an eligible pair
  with IoU above 0.3 (polygons by their bounding box), or `whole_image` against `whole_image`, or
  `absent` against `absent`; pairs are matched one to one greedily by descending IoU; unmatched
  predictions are spurious; reported as hits over labelled regions and spurious over predictions.
  A finding whose location did not resolve to a root image matches nothing and counts as spurious;
  a `view` location has no root geometry and likewise matches nothing;
- cost total and median per item (errors included) and wall time total and median.

Later repetitions never move a headline number; they are reported as the share of (item,
repetition) pairs whose computed verdict agrees with repetition 1. `fixtures/stats/metrics-1.toml`
is the hand-computed fixture the tests hold the numbers to, and `fixtures/stats/metrics-1.md` its
stored rendering.

## Estimating first

```
dejaview eval estimate --matrix matrix.toml [--evalsets-root evalsets] [--json]
```

Prints every decision's envelope exactly as the ledger would reserve it (the same setup and
planning the run uses), their sum, a cost range from "every decision submits at once" (the first
request's bound) to "every envelope spent", and the cap. When the sum exceeds the cap it warns
that completion is not guaranteed and still exits 0: admission is per decision against what is
left, and settled costs sit far below envelopes, so such a matrix usually completes. The owner
approves a baseline against this estimate (owner decision 6).
