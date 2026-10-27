# Data leaving the machine, per profile

For every shipped endpoint profile: what the harness enforces at request level, what the operator
must configure on the account, and what the provider does not expose for verification. A row is
filled when a profile's fixtures are recorded; a catalog test fails when a shipped profile or
model is missing from this page.

- **Enforced by the harness.** Request-level settings the adapter always sends or refuses, which
  the recorded fixtures prove: for example OpenAI `store = false`, a gateway's upstream allowlist,
  no cross-host redirects, no filenames or paths, no image metadata, credentials only in the one
  header the scheme names.
- **Attested by the operator.** Account settings the harness cannot see and the `[privacy]
  account` entry attests to: data retention, no training on inputs, gateway logging and caching
  switched off, the region the account is pinned to.
- **Not verifiable.** What the provider does not expose at all: internal retention of abuse-review
  samples, the resize or re-encoding applied to images, which hardware region served a request.

What a recording keeps (`fixtures/adapters/`, `fixtures/profiles/`): the request URL, headers and
body with every credential replaced by `<redacted>` and the volatile headers blanked; the response
status, body, and only the response headers the adapters read (`content-type`, `request-id`,
`x-request-id`, `retry-after`). Rate-limit state, organisation, workspace and project identifiers,
cookies and server details are dropped at recording time, so a committed fixture carries no
account identifier.

Consent (`[privacy] cloud_calls = true` and `account`) is bound to the profile: the account must
carry the profile's prefix (`anthropic:…` for the `anthropic` profile, `google:…` for
`google-ai-studio`, `openai:…` for `openai`), so a config cannot be pointed at another provider
without the attestation changing with it. The decision header records the recipient chain
observed: profile, host, upstream provenance where a gateway reports one, and the resolved model.

| Profile | Enforced by the harness | Attested by the operator | Not verifiable |
|---|---|---|---|
| `anthropic` | Credential only in `x-api-key`; `anthropic-version` pinned; HTTPS to `api.anthropic.com` with same-origin redirects only; no `temperature`, `top_p`, `top_k`, `metadata`, `stream` or server-side fallback parameters; `thinking: adaptive` with the prefix check when configured; tools with `strict: true` and the wire schema; images as base64 at the catalog's native size without metadata; the final turn as a mid-conversation system message, `tool_removal` blocks or a forced `tool_choice` only where the catalog allows. Recorded 2026-10-27 under `fixtures/adapters/anthropic-messages/`. | Retention and training settings of the organisation in the Claude Console; which workspace the key belongs to | Internal retention of abuse-review samples; the resize applied to images past the token tier; the region that served a request (`usage.inference_geo` reports `global`) |
| `google-ai-studio` | Credential only in `x-goog-api-key`, never in the URL; HTTPS to `generativelanguage.googleapis.com` with same-origin redirects only; no `temperature`, `topP`, `topK` or cached-content parameters; tools as `functionDeclarations` with `parametersJsonSchema` (references inlined); images as `inlineData` at the catalog's native size without metadata; `functionResponse` paired by id with the call, or without an id only when the response carried none; function-calling mode `ANY` restricted to `submit_verdict` only where the mechanism is `forced`. Recorded 2026-10-27 under `fixtures/adapters/gemini-generate-content/`. | Data-use settings of the Google Cloud project behind the key (paid tier, no training on prompts); the project the key belongs to | Internal retention of abuse-review samples; the resize applied to images; the region that served a request; implicit caching (`cachedContentTokenCount` appears without any request setting) |
| `openai` | Credential only in `Authorization: Bearer`; HTTPS to `api.openai.com` with same-origin redirects only; `store = false` and `include = ["reasoning.encrypted_content"]` on every request; `truncation = "disabled"`; no `temperature`, `top_p`, `metadata` or `previous_response_id`; function tools with `strict: true` and the wire schema; images as data URLs at the catalog's native size without metadata; `function_call_output` paired by `call_id`; reasoning items echoed as received. Recorded 2026-10-27 under `fixtures/adapters/openai-responses/`. | Data-retention and training settings of the organisation; the project the key belongs to | Internal retention of abuse-review samples; the resize applied to images; the region that served a request; prompt caching (`cache_write_tokens` appears without any request setting) |

## Model catalog provenance

`crates/dejaview-providers/catalog.toml` (task 2.1) carries, per model, the `verified` date on which every value
was read and the documentation pages it was read from (`sources`). `dejaview catalog show <model>`
prints both. Fields the documentation did not state on 2026-10-26 are listed in each entry's
`unverified` array and carried as the best estimate until the recordings (2.5b, 2.6b, 2.7b)
confirm or correct them:

| Model | Unverified field | Estimate carried | Why |
|---|---|---|---|
| `gemini-4-argon`, `gemini-3.8-flash` | `images.max_dimension_px` | none (no limit enforced beyond bytes) | The image-understanding guide states formats, tiles and counts but no maximum pixel dimensions. |
| `gemini-4-argon`, `gemini-3.8-flash` | `capabilities.function_call_ids` | `false` | `FunctionCall.id` and `FunctionResponse.id` are optional in the REST reference; whether responses carry them is confirmed by recording. |
| `gemini-4-argon`, `gemini-3.8-flash` | `capabilities.strict_unsupported_keywords` | the structured-output guide's unsupported list | The function-declaration schema subset is documented for `responseSchema`; the same subset is assumed for tool parameters. |
| `gpt-6.1-sol`, `gpt-6.1-terra` | `images.max_bytes` | 512 MB (the request payload limit) | No per-image byte limit is stated; only the request payload limit. |
| `gpt-6.1-sol`, `gpt-6.1-terra` | `capabilities.sampling` | `false` | The reasoning guide does not list which sampling parameters reasoning models accept; the adapter sends none. |
| `gpt-6.1-sol`, `gpt-6.1-terra` | `capabilities.system_messages_in_history` | `true` | `developer`/`system` items are accepted as input items; acceptance mid-history is confirmed by recording. |
| `gpt-6.1-sol`, `gpt-6.1-terra` | `images.native_long_edge_px`, `images.tokens` | the gpt-5.6 family values (2048 px, 2,500 patches at `high`) | The vision guide's sizing table does not name the gpt-6.1 family. |

Wire schema assumptions (task 2.2) carried the same way, re-verified by the recordings:

| Adapter | Assumption | Why |
|---|---|---|
| Gemini | `$ref`/`$defs` are inlined before sending | The `parametersJsonSchema` reference shows raw JSON Schema but does not state that definitions are accepted; the structured-output guide shows `$ref` only in an example. |
| Gemini | string `format` kept only for `date-time`, `date`, `time` | The structured-output guide names those three; the Schema reference says any value is allowed but ignored. |
| Gemini | `const` is sent as a one-value `enum` | `const` is not named in either guide. |
| OpenAI | `const` is sent as a one-value `enum`; `default` is removed | Neither keyword appears in the supported-schemas list. |
| Anthropic | `minItems` above 1 is removed; `maxItems` removed | "Array constraints beyond `minItems` of 0 or 1" are unsupported in strict mode. |

Image handling the documentation does not describe (task 2.3), carried as assumptions until the
recordings:

| Provider | Assumption | Why |
|---|---|---|
| all | The provider does no processing beyond its documented downscale; the harness fits the long edge to the catalog's native size and sends JPEG at quality 90, or PNG when alpha is present | Only the resize behaviour at the native size is documented. |
| Anthropic | A prepared image at the native long edge may still exceed the 4,784-token tier and be downscaled by the provider | The token formula (⌈w/28⌉ × ⌈h/28⌉) and the long-edge figure are both documented; which wins for a square image is not. The ledger bounds every image at the maximum. The `many-images` recording (21 images at 2000×1500) was accepted and billed about 4,100 tokens per image. |
| OpenAI | The request payload limit (512 MB) stands in for the per-image byte limit | No per-image limit is stated. |
| Gemini | No maximum pixel dimension is enforced before the 20 MB inline request limit | None is documented. |

Wire assumptions of the Gemini and OpenAI adapters (2.6a, 2.7a), re-verified by the recordings:

| Adapter | Assumption | Why |
|---|---|---|
| Gemini | `generateContent` serves `gemini-4-argon` and `gemini-3.8-flash` | Confirmed by the recordings of 2026-10-27: both models answered `generateContent` with function calls, thought signatures and implicit caching. |
| Gemini | `generationConfig.thinkingConfig.thinkingLevel` takes the lowercase level (`low`, `medium`, `high`) | Confirmed by the recordings: `low` was accepted. |
| Gemini | Media resolution is a per-part `mediaResolution.level` of `MEDIA_RESOLUTION_*` | The reference lists the enum on `Part`; the guide documents only the Interactions API field. |
| Gemini | Images returned by tools travel as `functionResponse.parts[].inlineData` | Confirmed by the `multimodal-tool-result` recording. |
| Gemini | Every model part is echoed verbatim, so thought signatures stay where they were | The thinking guide says to resend every thought block exactly as received. |
| OpenAI | Images returned by tools travel as `input_image` parts in `function_call_output.output` | Confirmed by the `multimodal-tool-result` recording. |
| OpenAI | The final-turn instruction is a `developer` message mid-conversation | Confirmed by the `final-turn-refuse` recording. |
| OpenAI | The request id is the `x-request-id` header | Confirmed by the recordings. Gemini sends no request-id header; its `responseId` in the body stands in. |

Settled cost (`Pricing::cost`) prices uncached input at the input rate, prompt-cache reads at the
cached rate, prompt-cache writes at the write rate where the catalog states one (the input rate
otherwise), and output at the output rate; a long-prompt tier is chosen from the whole prompt,
cached tokens included. Bounds (`Pricing::bound`) assume nothing is cached.

Two prices in the catalog are introductory and dated: `gemini-4-argon` to 2026-11-30 and
`gemini-3.8-flash` to 2026-12-31. The entries carry the later prices in a comment; re-pin them when
the dates pass.

Settings the catalog rejects at resolve time (`Catalog::check_config`): an unknown profile or
model, a profile whose adapter differs from the model's, a control the model does not declare or a
value outside its allowed list, `images.max_long_edge` above the model's native size, and output
allowances above the model's maximum. The test-only `fake` profile with `fake-model` is admitted
only by a binary built with the `testkit` feature.

## Confirmed by the Anthropic recordings (2026-10-27)

Fifteen conversations under `fixtures/adapters/anthropic-messages/` (about $0.80 per recording;
recorded three times on 2026-10-27, last after the M2 checkpoint fixes) confirmed, for `claude-opus-5-5`, `claude-sonnet-5-5` and `claude-haiku-5-5`: the `thinking-binding-controls-2026-08-01` beta
with `prefix_mismatch_behavior = "error"` accepts an append-only history with thinking blocks
echoed verbatim; `role: "system"` mid-conversation is accepted after a user turn; `tool_removal`
blocks under `inline-tools-2026-09-15` are accepted with the tools array unchanged; Haiku 5.5
accepts `tool_choice: {type: "tool"}` and answers without a thinking block; a request over the
many-image threshold with every image at 2000 pixels is accepted; the `reasoning_extraction`
refusal arrives as `stop_reason: "refusal"` with `stop_details` and no content. The final turn is
not deterministic on Opus 5.5: across three recordings of the same two cases it once made a stray
tool call (caught by the `stray_call` repair), once submitted directly, and once submitted a
verdict the validator rejected (an `invalid_submission` repair, then a valid resubmission). Every
path ended in a verdict. The `tool_use` block is echoed exactly as received since the checkpoint;
the API accepted the raw echo under the prefix check on every multi-turn case.

## Confirmed by the Gemini and OpenAI recordings (2026-10-27)

Thirteen conversations per vendor under `fixtures/adapters/gemini-generate-content/` and
`fixtures/adapters/openai-responses/` (about $0.22 and $0.30 per recording, the OpenAI figure with
prompt-cache writes priced) confirmed: Gemini 4 Argon and
Gemini 3.8 Flash answer `generateContent` with `functionCall` parts that carry ids (`call_…`) and
thought signatures, accept `functionResponse` parts paired by those ids with inline images, and
omit null-valued fields from function arguments, which the harness now fills back in before
validation; mode `ANY` with `allowedFunctionNames` forces `submit_verdict` on the final turn.
GPT-6.1 Sol and Terra accept the strict wire schema once every `$ref` stands alone and nullable
arrays are written as `anyOf` with `null` (both learned from live 400s and pinned in the wire
transformation), echo reasoning items with `store = false`, take images inside
`function_call_output`, accept a `developer` message mid-conversation, and report
`cache_write_tokens` and `cached_tokens` in usage. Sol made a stray call during the final turn in
both of its recordings, and the repair caught it both times; in the second recording its
resubmission then placed an observation on an image the rule does not cover and the run ended in
`error: submission`, which is the harness refusing to turn a bad submission into a verdict. A
failed tool result reaches Sol with its text prefixed `error: ` (the Responses API has no error
flag) and the `tool-error` recording shows it carrying on to a verdict. Neither Gemini
nor OpenAI refused the reasoning-extraction prompt. The first recordings of every vendor had at
least one model name the `SF3` exemption in `allowed_by`; after the rule text, the rendered hint
and the rejection were reworded (see docs/rules.md), the second recordings landed every first
submission that was not meant to fail.

