# External reviews

Verbatim output of Codex CLI (`codex exec --sandbox read-only`), one file per round. Each round's
prompt asked for a resolution table over the previous findings plus new findings, most severe
first.

## Spec

| Round | Reviewed | Verdict | Answered by |
|---|---|---|---|
| 1 | SPEC.md draft 1 | not ready: 25 findings, 5 blockers | draft 2 |
| 2 | draft 2 | not ready: 6 of 25 resolved, 16 new, 3 blockers | draft 3 |
| 3 | draft 3 | not ready: 13 new, 2 blockers | draft 4 |
| 4 | draft 4 | ready with changes: 8 new, 2 blockers; split into before-planning and M1 | draft 5 (approved by the owner 2026-10-25) |

## Plan

| Round | Reviewed | Verdict | Answered by |
|---|---|---|---|
| 1 | PLAN.md draft 1 | with changes: 14 findings, 1 blocker, 11 coverage gaps | draft 2 |
| 2 | draft 2 | with changes: 13 new; 7 before M0, 6 in the owning task | draft 3 |
| 3 | draft 3 | with changes: 8 new; 5 before M0, 3 in the owning task | draft 4 |

Draft 4 of the plan made every before-M0 change from round 3 and records the owning-task items in
the tasks that own them.

## Checkpoints

| Checkpoint | Record |
|---|---|
| M1 | [m1-checkpoint.md](m1-checkpoint.md): code review and coverage findings, each fixed or recorded with its owning task |
| M2 | [m2-checkpoint.md](m2-checkpoint.md): code review and coverage findings, each fixed or recorded with its owning task; live smoke in [m2-live-smoke.md](m2-live-smoke.md) |
| M3 | [m3-checkpoint.md](m3-checkpoint.md): code review and coverage findings, each fixed or recorded with its owning task |
| M4 | [m4-checkpoint.md](m4-checkpoint.md): code review and coverage findings, each fixed or recorded in tasks/todo.md |

## Release records

| Record | What it holds |
|---|---|
| [m2-live-smoke.md](m2-live-smoke.md) | The task 2.10b live smoke: one `check` per direct profile from the unpacked release archive, with verdicts, exit codes and costs (owner decision 4) |
