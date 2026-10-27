# M2 checkpoint: review and coverage findings

Branch `m2-adapters` (tasks 2.1 to 2.10b, without the optional 2.8) was reviewed by the
code-review and test-engineer passes on 2026-10-27 before the owner's review. Every finding is
either fixed on the branch or recorded here with the task that owns it.

## Code review

The code-review pass found no critical defect, the HTTPS client, credential handling, consent
binding, request hygiene, adapter echo and pairing, retry classification, catalog checks and the
release gating clean, and clippy and the crate graph unchanged. Its findings:

| Severity | Finding | Resolution |
|---|---|---|
| Important | The recorder scrubbed only the credential from response headers, so the recorded fixtures carried the organisation, workspace and project identifiers the vendors return (`anthropic-organization-id`, `anthropic-workspace-id`, `openai-organization`, `openai-project`). | Fixed: the recorder keeps an allowlist of response headers (`content-type`, `request-id`, `x-request-id`, `retry-after`); every committed fixture was re-scrubbed to that list and the Anthropic and OpenAI conversations re-recorded; the recorder test asserts an organisation header never reaches the file. The identifiers remain in this branch's earlier commits; see the note to the owner below. |
| Important | `Pricing::cost` charged uncached input and output only: prompt-cache reads were free, cache writes unbilled, and the long-prompt tier was chosen from the uncached count. | Fixed: reads at the cached rate, writes at the write rate (the input rate when the catalog states none), the tier from the whole prompt; test `cost_prices_cache_reads_and_writes_and_tiers_on_the_whole_prompt`. |
| Important | The CLI's HTTP client used a hard-coded 600 s timeout and ignored `budget.request_timeout_s`. | Fixed: the client takes the configured timeout. |
| Important | The Anthropic adapter echoed `tool_use` blocks rebuilt from the normalised arguments rather than as received; Gemini and OpenAI echo the raw item. | Fixed: the raw block is echoed as an opaque block, like every other block; the harness still works from the normalised call. The recordings changed (the `parallel-batch` conversation had sent a normalised block) and were re-recorded. |
| Important | Preflight can fail with `error: image_limit` mid-run once tool images carry the history past `many_image_threshold`, after money is spent. | Recorded for 4.1 (tools return images from M4): prepare at the many-image size when the worst case can pass the threshold, or refuse at resolve. |
| Important | The default budget plans an envelope above the default cap on two of three vendors (every request at `1 + retries` full attempts). | Owner decision (tasks/todo.md): the reviewer recommends planning each request once plus a run-wide retry reserve, which amends SPEC "Cost ledger" and keeps the defaults; the examples carry `retries = 1` and a $5 cap until then. |
| Suggestion | `short` and `redact_account` sliced strings at byte offsets and would panic inside a multi-byte character. | Fixed: character-based. |
| Suggestion | The CLI used the Anthropic adapter's volatile set for every adapter's recordings. | Fixed: the volatile set follows the adapter. A `--replay` run still does not check that every exchange was served; the replay suite does, and the CLI check is recorded for 3.4b with the eval runner's replay mode. |
| Suggestion | `tool_removal` on a model without mid-conversation system messages would silently drop the removal blocks. | Fixed: the catalog self-test rejects a model listing `tool_removal` without `system_messages_in_history`. |
| Suggestion | OpenAI has no error flag on a function result, so a failed tool call looked like output. | Fixed: the output text is prefixed `error: `; re-recorded (`tool-error`). |
| Suggestion | Transport-error excerpts reach the journal and stderr without the redaction pass. | Recorded (tasks/todo.md): no credential can be in a provider's response body; routed through redaction when the journal gains that step. |
| Suggestion | `registry-install-test.sh` removed whatever `REGISTRY_STAGE_DIR` named and never cleaned a scratch stage. | Fixed: a named stage must be new or empty; the stage is removed on exit. |
| Suggestion | `live-smoke.sh` aborted the whole smoke when the report reader printed nothing. | Fixed: an unreadable report fails that vendor only. |
| Suggestion | `check-packaging.sh` split `CRATE_DIRS` on spaces and hid cargo's error. | Fixed: one directory per line; the error is printed. |
| Suggestion | The archive smoke skipped the checksum when no file was present. | Fixed: `REQUIRE_CHECKSUM=1` in both workflows. |
| Suggestion | The archive-smoke workflow inherited write permissions from the generated caller. | Fixed: `permissions: contents: read`. |
| Suggestion | The `dist` installer is fetched with `curl \| sh` unverified. | Recorded (tasks/todo.md); the generated release workflow does the same. |
| Suggestion | `build.rs` watched only `HEAD`, so a new commit on the same branch could leave a stale sha in `--version`. | Fixed: the branch ref and `packed-refs` are watched too. |
| Suggestion | The envelope planned exactly one repair while `budget.repairs` has no bound. | Fixed: the plan holds `repairs` repair requests; SPEC "Cost ledger" says so; test `ledger_envelope_plans_every_allowed_repair`. |

**Note to the owner.** The branch's earlier commits (`8f18001`, `aed5852`, `bcb0d20`) still hold
the fixtures with the organisation, workspace and project identifiers in their response headers.
Nothing has been pushed. Before `main` is pushed at the `m2` tag, decide whether to rewrite those
commits (the agent has not rewritten history) or accept the identifiers in the public history;
they are account identifiers, not credentials.

## Coverage

The test-engineer pass ran both feature configurations offline (295 and 334 tests passing, 5
ignored: the three live recorders and two provisioning tests; under two minutes each including the
build) and found nothing weakened in M2. It listed eleven gaps.

| Gap | Resolution |
|---|---|
| Replay skipped a case whose recording was missing, and the profile-conformance check was behind an existence test, so a deleted fixture passed green. | Fixed: `replay()` fails on a missing recording or profile fixture, and asserts every exchange of a recording was served (`Player::remaining() == 0`). |
| The recipient chain was never asserted from an output file. | Fixed: every replayed case checks `decision.json` names the profile, model and a host. |
| No test at the binary for `check` without a credential. | Fixed: `catalog_cli.rs::check_without_a_credential_is_an_auth_error_with_exit_64`. |
| The catalog self-test covered two of its rules. | Fixed: `catalog_self_test_names_each_broken_profile_rule` mutates the shipped catalog once per profile rule (version, adapter, auth scheme, account prefix, https). Model-level rules remain covered by the shipped catalog parsing; mutations for them are recorded for the next catalog change. |
| Preflight's per-request image count had no test. | Fixed: `preflight_refuses_one_image_over_the_per_request_count`. |
| `offline-smoke.sh` had no negative case. | Fixed: `test-checks.sh` runs it against a stub binary that exits 3 instead of 64, and one whose `rules show` prints no `SF1`. |
| `check-packaging.sh` listed assets without an expected set. | Fixed: `test-checks.sh` asserts the prompt, the source-fidelity rules and the catalog appear as embedded by their crates. |
| `examples_cli.rs` repeated the live smoke's vendors by hand. | Fixed: the test reads the `smoke` lines of `scripts/live-smoke.sh`. |
| Wire refusals for a non-object root and an unknown keyword were untested. | Fixed: two rows in `wire_refuses_unrepresentable_constructs_with_the_reason`. |
| The Player's "holds no exchanges" path was untested (the overrun path already was). | Fixed: `replay_refuses_a_directory_with_no_exchanges`. |
| Four recordings did not reach the scenario they were steered into: Gemini and OpenAI did not refuse the reasoning-extraction prompt, Gemini answered the call-less prompt with a call, and Gemini made no stray call under `refuse`. | Recorded, not fixable by the harness: these are vendor behaviours, and the recordings prove the paths do not trigger on those models. The repair paths themselves (truncation, call-less text, refusal, stray call) are exercised end to end by the scripted provider in `repair_retry.rs` and `final_turn_cli.rs`, and the Anthropic recordings reached every one of them. Re-check when the catalog adds a model (3.8). |

## Checkpoint M2 list

- Live smoke on all three direct profiles: `m2-live-smoke.md`, about $0.02 in total.
- Fixture replay covers every capability combination and one profile-conformance fixture per
  shipped profile: proven by `replay_*_recordings`, which now fails on a missing fixture.
- SPEC open question 1 (image limits, prices, capabilities and strict-schema keyword lists per
  model) is closed by the catalog's `verified` dates and the recordings; the fields the
  documentation did not state are listed in docs/provider-data.md with the recording that
  confirmed or corrected each.
- Owner review, merge and tag `m2`: the owner's.
