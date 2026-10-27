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
| Important | The recorder scrubbed only the credential from response headers, so the recorded fixtures carried the organisation, workspace and project identifiers the vendors return (`anthropic-organization-id`, `anthropic-workspace-id`, `openai-organization`, `openai-project`). | Fixed: the recorder keeps an allowlist of response headers (`content-type`, `request-id`, `x-request-id`, `retry-after`); every committed fixture was re-scrubbed to that list and the Anthropic and OpenAI conversations re-recorded; the recorder test asserts an organisation header never reaches the file. The branch's earlier commits were rewritten to the scrubbed fixtures before anything was pushed (below). |
| Important | `Pricing::cost` charged uncached input and output only: prompt-cache reads were free, cache writes unbilled, and the long-prompt tier was chosen from the uncached count. | Fixed: reads at the cached rate, writes at the write rate (the input rate when the catalog states none), the tier from the whole prompt; test `cost_prices_cache_reads_and_writes_and_tiers_on_the_whole_prompt`. |
| Important | The CLI's HTTP client used a hard-coded 600 s timeout and ignored `budget.request_timeout_s`. | Fixed: the client takes the configured timeout. |
| Important | The Anthropic adapter echoed `tool_use` blocks rebuilt from the normalised arguments rather than as received; Gemini and OpenAI echo the raw item. | Fixed: the raw block is echoed as an opaque block, like every other block; the harness still works from the normalised call. The recordings changed (the `parallel-batch` conversation had sent a normalised block) and were re-recorded. |
| Important | Preflight can fail with `error: image_limit` mid-run once tool images carry the history past `many_image_threshold`, after money is spent. | Recorded for 4.1 (tools return images from M4): prepare at the many-image size when the worst case can pass the threshold, or refuse at resolve. |
| Important | The default budget plans an envelope above the default cap on two of three vendors (every request at `1 + retries` full attempts). | Fixed as the reviewer recommended, with the owner's go-ahead: each request is planned once plus a per-run retry reserve of `retries` times the largest bound; a retry the balance cannot cover is refused with `error: budget` instead of overrunning. SPEC "Cost ledger" amended; the examples are back on the defaults; the live smoke was re-run under them (`m2-live-smoke.md`). |
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

**History.** The three commits that first added the recordings carried the identifiers in their
response headers. On the owner's decision (2026-10-27) the branch was rewritten with
`git filter-repo`, restricted to this branch and its `m2.1` tag, so that every version of every
recording in history holds only the allowlisted headers; the working tree was unchanged by the
rewrite and nothing had been pushed. Commit ids quoted elsewhere in these records therefore
predate the rewrite.
