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
