# M3 checkpoint: review and coverage findings

Branch `m3-evalsets` (tasks 3.1 to 3.8) was reviewed by the code-review and test-engineer passes
on 2026-10-28, while the arm A baseline ran, before the owner's review. Every finding is either
fixed on the branch or recorded here with where it is tracked.

## Code review

The pass found the set checks, the seal, blind-label handling, the gate stages, the metrics and
localisation in line with the spec, the viewer's path handling tight, and no account id or key in
any tracked file (accounts appear only as `${SQUINT_*_ACCOUNT}` references). Its findings:

| Severity | Finding | Resolution |
|---|---|---|
| Critical | The evaluation ledger was keyed by decision id, so a decision re-run after a crash settled nothing (its id was already charged) and replay collapsed its holds into one: real spend went uncounted and the cap could be exceeded. | Fixed: every attempt has its own hold (`<id>#<attempt>`); a re-run is a new attempt charged on top of the one the crash left; replay sums per attempt; `settle` applies only to an attempt reserved in this process. Test `a_decision_run_again_after_a_crash_is_charged_for_both_attempts`. Older ledger lines without an attempt replay as attempt 0. |
| Important | The binary's factories never supplied pack lock digests, so the descriptor (and the gate's judging digest) could not see a changed Python pack. | Fixed: `tools_for_config` returns the lock digest of the installed bundle each enabled pack tool came from; the factories keep the last registry's digests for the descriptor. |
| Important | The admission record was not a descriptor input, so arm D could change between runs of one evaluation. | Fixed: the descriptor records every arm's tool selection per use case; a change refuses resume naming "arm selections". Test `a_changed_admission_record_changes_arm_d_and_refuses_resume`. |
| Important | Arm D projected `align` to `visual_only` even when `align` itself was the admitted tool. | Fixed: the projection is skipped when the admitted set holds `align`. |
| Important | Once the matrix cap was exhausted every remaining decision was recorded as `error: budget`, skipped on resume forever, and counted against the arm. | Fixed: the runner checks the cap against the next envelope and stops with `budget_exhausted` in the summary, writing no row; a resume tries again; the gate refuses such a summary. Test `the_cap_stops_the_matrix_before_a_decision_it_cannot_cover_and_records_nothing_for_it`. |
| Important | The stale-lock takeover raced (two readers of a dead pid), and `kill -0` reports "dead" on EPERM. | Fixed: an OS advisory lock on the `LOCK` file, released however the holder ends; the file is never removed. |
| Important | `result.json` and `descriptor.json` were written in place; a crash mid-write left truncated JSON that every resume then refused. | Fixed: `write_atomic` (temporary file, sync, rename) for the descriptor, results, summary, receipts, admissions and canary state. |
| Important | The canary counted an operational error as a deviation, so one transient provider failure made a configuration `suspect`. | Fixed: an error is never a verdict; errored items are listed apart and a run with errors and no deviation is `inconclusive`; a requalification with an errored item records nothing. SPEC "Drift" amended. |
| Important | The canary's configuration identity left out the prompt bytes, tool manifests, catalog and harness version. | Fixed: all four are folded in. |
| Important | Canary and requalify ran with no aggregate spend cap. | Fixed: `--spend-cap-usd` (default $25) through a cap ledger, refusing an item whose envelope no longer fits before sending anything. |
| Important | The viewer ignored `Host`, so a page elsewhere could rebind a name to loopback and read the report. | Fixed: a request whose `Host` is not `127.0.0.1:<port>` or `localhost:<port>` gets `421`; the page carries `Content-Security-Policy: default-src 'self'`. Tested in `viewer_serve`. |
| Important | `seal --labels-out` accepted a directory inside a repository. | Fixed: refused when any ancestor holds a `.git`. |
| Important | The gate checked family disjointness only against sets with receipts and never applied the one-split-per-family rule across the root. | Fixed: `sets::check_families` (manifests only, so unfetched images do not matter) runs at the start of every gate. |
| Suggestion | The receipt id hashed the whole `summary.json`, so a no-op resume changed it and a resumed gate read as a repeat. | Fixed: the hash is over the decisions in id order. |
| Suggestion | Operator precedence in `pin_labels_hash` would rewrite a second `labels=` line. | Fixed: parenthesised. |
| Suggestion | A non-finite control would hash to an empty string and collide identities. | Fixed: the matrix refuses non-finite controls. |
| Suggestion | Control keys were spliced into the TOML override unquoted. | Fixed: quoted. |
| Suggestion | `frames` and `findings` joined `report_version` 1 under `deny_unknown_fields`. | Recorded in docs/viewer.md: added fields default when absent, so a current binary reads every version 1 report; an older binary refuses a newer report rather than misread it. |
| Suggestion | `DecisionResult`, `Summary`, `Estimate` and `Matrix` lack `JsonSchema`. | Recorded in tasks/todo.md for the checkpoint follow-ups. |
| Suggestion | One thread per viewer connection, unbounded. | Recorded (local single-user tool) in tasks/todo.md. |
| Suggestion | `results-doc.py`: the shebang failed on Linux, the published `summary.json` carried raw error text, the adversarial sets were told by suffix. | Fixed: `env -S`; error text withheld from the published copy; the suffix stays as the naming convention the sets follow and the test checks. |
| Suggestion | `squint-eval` declared `squint-tools` without using it. | Fixed: dropped, and the crate graph with it. |
| Suggestion | No test that an environment-expanded matrix account reaches the report as `[redacted]`. | Recorded in tasks/todo.md (the test would have to set a process variable). |

## Test coverage

The pass found the gate and canary tests strong and most acceptance criteria covered. Its gaps
and what was done:

| Gap | Resolution |
|---|---|
| No ledger test re-used a decision id after recovery (which is why the critical defect passed). | Added with the fix. |
| Completed errors kept on resume untested. | Added `a_completed_error_is_kept_on_resume`. |
| The cap stop asserted loosely. | Rewritten for the new behaviour: exactly one row, no directory for the refused decision, the flag, and the same stop on resume. |
| The frozen policy tested under `review` only. | Added `the_frozen_policy_decides_what_counts_as_missed` (`on_unsure = pass`, `on_error = fail`). |
| Wilson intervals only bracketed. | Golden `missed_low`, `missed_high`, `unsure_low`, `unsure_high` in `fixtures/stats/metrics-1.toml`, matched to 1e-9. |
| The IoU boundary and an empty cell untested. | Added `an_iou_at_the_threshold_is_not_a_hit_and_an_empty_cell_does_not_panic`. |
| The viewer assets missing from the packaging self-test. | Added to `scripts/test-checks.sh`. |
| The unsupported `report_version` branch and the no-external-URL rule untested. | Static checks added to `viewer_serve`. |
| The shipped baseline matrix never parsed by a test. | Added `the_shipped_baseline_matrix_parses_and_names_sets_that_exist`. |
| `results-doc.py` only compile-checked. | Run on the fake evaluation in `eval_run_cli`, sections and cells asserted. |
| Nothing ran offline against the committed sets. | Added `the_published_sets_are_consistent_at_the_manifest_level`: every set labelled in full, sized by split, failure types, families declared, no family in two splits, a README row each. |
| The canary's uncached run and errored items untested. | Added: `cache.status == bypass` in every canary decision; errors are inconclusive; the spend cap refuses. |
| Arm D with a changed admission record untested. | Added with the descriptor fix. |
| A real SIGINT through the binary; prompt equivalence on the journaled prompt; the multi-cell matrix; the per-cell envelope mix; a gate interrupted between the receipt and the publication; the seal's image and rules components isolated; one fixture per `sets check` failure mode; an automated devtools check; instruction-like text never altering the prompt. | Recorded in tasks/todo.md as checkpoint follow-ups; the gate's resume path already redoes the publication and admission writes idempotently from the receipt. |

## The baseline run (3.8, 2026-10-28)

The first live run of the matrix tested the harness as much as the models. What it found and
what was done:

| Finding | Resolution |
|---|---|
| The localisation scorer read only free-form findings, while the prompt asks for a location on every checklist item marked `violation` and that is where every model put them; the first scoring reported almost no hits. | `ItemRow` carries `root` like findings do; checklist-item violations are predictions; tests in `report.rs` (core) and `runner.rs` (eval, including the raw-frame fallback for reports without `root`); the baseline re-scored from its saved reports; docs and SPEC updated. |
| The Anthropic credit balance ran out during the last 17 items; 68 decisions were recorded as `error: provider` and charged at their bound ($18.48 the vendor never billed). | By design (unknown usage charges the bound); the resume redid exactly the 68 after the top-up. Whether a refusal before processing should cost nothing is an owner decision, recorded in tasks/todo.md. |
| The descriptor written by the first build was unreadable by the review-fixed build (`selections` added, sha over the whole document). | Expected under the no-compatibility rule; the old descriptor was set aside and the resume wrote a fresh one over identical inputs, noted in the results document. |
| Most `whole_image` labels never match a boxed prediction; one defect under several obligations repeats a region and the extras count as spurious. | Questions about the metric's definition for the owner, recorded in tasks/todo.md; the numbers are published as the rules stand, with the explanation. |
