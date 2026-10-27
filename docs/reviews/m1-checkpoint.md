# M1 checkpoint: review and coverage findings

Branch `m1-skeleton` (tasks 1.1 to 1.15) was reviewed by the code-review and test-engineer
passes on 2026-10-26 before the owner's review. Every finding is either fixed on the branch or
recorded here with the task that owns it.

## Code review

| Severity | Finding | Resolution |
|---|---|---|
| Critical | Bundle installer joined `bundle.toml` file keys, `package`, `version` and `executable` onto paths unchecked (zip-slip). | Fixed: plain-name and relative-path checks in `provision.rs`, negative test `provisioning_bundle_refuses_paths_that_escape`. |
| Important | Tool-returned frame ids were not validated; a tool could replace a raw frame or register an image without bytes. | Fixed: a returned image must be `crop:<call id>:<n>`, not an existing frame, and carry bytes; tests in `tool_turns.rs`. |
| Important | `FrameHandle.pixels` reported zero dimensions and always `image/png`. | Fixed: dimensions from the frame, media type from the stored file. |
| Important | The final failed attempt (retries exhausted, a 4xx) was never charged. | Fixed: charged at its bound and journaled as `attempt_failed`; the report sums it. On cancellation the charge is kept in the ledger only, so the terminal event still names the state reached. |
| Important | A cancelled first run did not stop a two-run decision. | Fixed: the loop stops on cancellation; the report treats the missing run as an error decision. |
| Important | A journal I/O failure left the envelope reserved in a shared ledger. | Fixed: the account is closed and settled whatever the runs did. |
| Important | A cache hit returned the source decision's policy, config and exit code. | Fixed: the hit is judged under this decision's policy; test `cache_hit_applies_this_decisions_policy_to_the_exit_code`. |
| Important | Only `artifacts/` had mode 0600. | Fixed: journal, frames, `decision.json` and `report.json` too; test `journal_every_run_directory_file_is_private`. |
| Important | A synchronous tool call blocked the signal task for up to `tool_timeout_s`. | Fixed: tool calls run on their own thread and race the cancellation; test `a_cancel_during_a_slow_tool_call_lands_without_waiting_for_the_tool`. |
| Important | Calls refused for the call budget or on the final turn were absent from the report. | Fixed: `ToolCallEntry` carries `executed` and `reason`; refused calls appear with the provider's id. |
| Suggestion | MCP faults (EOF, unknown id, dead server) did not taint the process. | Fixed. |
| Suggestion | The pack kept the full parent environment for restarts. | Fixed: only the allowlisted variables are kept. |
| Suggestion | Redaction missed a secret in its JSON-escaped form. | Fixed. |
| Suggestion | Two decisions in one millisecond collided on the directory name. | Fixed: a suffix on `AlreadyExists`. |
| Suggestion | The cache key included how a pack is launched (`command`, `args`, timeouts). | Fixed: the key carries the enabled tools, their order and the frozen manifests. |
| Suggestion | The report lacked the tool version and the findings' rows. | Fixed. |
| Suggestion | The final-turn instruction is a separate user message, not a trailing text block. | Done in 2.5a: the instruction is a `System` message in the typed history; an adapter sends it as a mid-conversation system message where the model supports one, otherwise as the trailing text block of the preceding user message. |
| Suggestion | Calls are validated one at a time rather than the whole batch first. | Recorded for 4.4a; the results are the same either way today (invalid calls get error results, valid ones execute). |
| Suggestion | Server stderr is captured but never reaches the journal; the buffer is unbounded. | Recorded for 4.4b. |
| Suggestion | `--dry-run` connects packs. | Kept: the Tools section of the prompt needs the frozen manifests. |
| Suggestion | `first_look_bytes` is an estimate; images are encoded per run and `images_sent` holds the canonical PNG, not what the adapter sends. | Done in 2.3 and 2.5a: the provider prepares every image (`prepare_image`) and `images_sent` holds the prepared bytes the adapter sends; `first_look_bytes` remains an estimate by design (the bound is conservative). |
| Suggestion | No wall time in the report. | Done in 2.5a: `wall_ms` per run, from the first journal line to the last. |
| Suggestion | Very short secrets redact unrelated text. | Recorded; config review in 2.4. |

## Coverage

The test-engineer pass found the state machine, repairs, retries, ledger, cache and isolation
covered, and listed thirteen gaps. All are closed on the branch except the ones below.

Closed: the final turn's non-replayable path (and its terminal class, now `model_output` as the
repair protocol says), findings aggregation through the run, the aggregation table made
exhaustive (48 cells against a transcribed oracle), strictest-row with `matters = false` and
`hidden_ok = true` including the single-run `unsure` exit code, the repaired turn not consuming a
tool turn, envelope bound breaches and bad returned images through the run loop,
`measured_nothing` and `not_applicable` as citable evidence (`not_applicable` was wrongly
excluded), the final-turn invalid submission repair, the call budget exactly exhausted, a two-run
decision with an errored run, a provider that cannot price, a cancel before the first turn,
`cache.write = false`, 408 and 409 as retryable, retries exhausted through the run loop, refusal
twice, and the rebuild from every truncation point of a journal with tools and repairs.

Recorded: concurrent dispatch for `concurrent = true` packs is 4.4a (noted in PLAN.md 1.7a);
`request_timeout_s` belongs to the adapters (M2).

## Checks

- `just check`: fmt, clippy and tests in both feature configurations, deny, crate graph, hook and
  check self-tests; about one minute, 278 tests with the testkit feature plus the no-testkit run.
- `just python-check`: ruff and pytest on the tool pack.
- `just ci-local`: the `rust` job passed under act. The `python` job passed every step up to the
  namespaced launch; `unshare` is not permitted inside act's container, so that step runs only on
  GitHub and act runs the same install and call without a namespace. The rerun of the python job
  did not complete on this machine because Docker Desktop's data store went read-only when the
  disk filled (see below).

## Machine note

The data volume reached 100% during the checkpoint (239 MiB free). This session removed only
what it had created: the repository's `target/` directory (11.4 GiB, rebuilt since). Docker holds
about 90 GB of images, volumes and build cache, which is the owner's to prune.
