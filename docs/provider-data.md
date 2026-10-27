# Data leaving the machine, per profile

For every shipped endpoint profile: what the harness enforces at request level, what the operator
must configure on the account, and what the provider does not expose for verification. Rows are
filled when a profile's fixtures are recorded (task 2.9).

- **Enforced by the harness.** Request-level settings the adapter always sends or refuses, which
  the recorded fixtures prove: for example OpenAI `store = false`, a gateway's upstream allowlist,
  no cross-host redirects, no filenames or paths, no image metadata, credentials only in the one
  header the scheme names.
- **Attested by the operator.** Account settings the harness cannot see and the `[privacy]
  account` entry attests to: data retention, no training on inputs, gateway logging and caching
  switched off, the region the account is pinned to.
- **Not verifiable.** What the provider does not expose at all: internal retention of abuse-review
  samples, the resize or re-encoding applied to images, which hardware region served a request.

Consent (`[privacy] cloud_calls = true` and `account`) is bound to the profile: the account must
carry the profile's prefix (`anthropic:…` for the `anthropic` profile, `google:…` for
`google-ai-studio`, `openai:…` for `openai`), so a config cannot be pointed at another provider
without the attestation changing with it. The decision header records the recipient chain
observed: profile, host, upstream provenance where a gateway reports one, and the resolved model.

| Profile | Enforced by the harness | Attested by the operator | Not verifiable |
|---|---|---|---|
| `anthropic` | Credential only in `x-api-key`; `anthropic-version` pinned; HTTPS to `api.anthropic.com` with same-origin redirects only; no `temperature`, `top_p`, `top_k`, `metadata`, `stream` or server-side fallback parameters; `thinking: adaptive` with the prefix check when configured; tools with `strict: true` and the wire schema; images as base64 at the catalog's native size without metadata; the final turn as a mid-conversation system message, `tool_removal` blocks or a forced `tool_choice` only where the catalog allows | (2.5b) | (2.5b) |
| `google-ai-studio` | Credential only in `x-goog-api-key`, never in the URL; HTTPS to `generativelanguage.googleapis.com` with same-origin redirects only; no `temperature`, `topP`, `topK` or cached-content parameters; tools as `functionDeclarations` with `parametersJsonSchema` (references inlined); images as `inlineData` at the catalog's native size without metadata; `functionResponse` paired by id with the call, or without an id only when the response carried none; function-calling mode `ANY` restricted to `submit_verdict` only where the mechanism is `forced` | (2.6b) | (2.6b) |
| `openai` | Credential only in `Authorization: Bearer`; HTTPS to `api.openai.com` with same-origin redirects only; `store = false` and `include = ["reasoning.encrypted_content"]` on every request; `truncation = "disabled"`; no `temperature`, `top_p`, `metadata` or `previous_response_id`; function tools with `strict: true` and the wire schema; images as data URLs at the catalog's native size without metadata; `function_call_output` paired by `call_id`; reasoning items echoed as received | (2.7b) | (2.7b) |

## Model catalog provenance

`providers/catalog.toml` (task 2.1) carries, per model, the `verified` date on which every value
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
| Anthropic | A prepared image at the native long edge may still exceed the 4,784-token tier and be downscaled by the provider | The token formula (⌈w/28⌉ × ⌈h/28⌉) and the long-edge figure are both documented; which wins for a square image is not. The ledger bounds every image at the maximum. |
| OpenAI | The request payload limit (512 MB) stands in for the per-image byte limit | No per-image limit is stated. |
| Gemini | No maximum pixel dimension is enforced before the 20 MB inline request limit | None is documented. |

Wire assumptions of the Gemini and OpenAI adapters (2.6a, 2.7a), re-verified by the recordings:

| Adapter | Assumption | Why |
|---|---|---|
| Gemini | `generateContent` serves `gemini-4-argon` and `gemini-3.8-flash` | Google's guides now lead with the Interactions API; the model pages list neither surface, and the `generateContent` reference still documents thought signatures and function calls for current models. |
| Gemini | `generationConfig.thinkingConfig.thinkingLevel` takes the lowercase level (`low`, `medium`, `high`) | The guides show the Interactions API field; the `generateContent` enum casing is not shown. |
| Gemini | Media resolution is a per-part `mediaResolution.level` of `MEDIA_RESOLUTION_*` | The reference lists the enum on `Part`; the guide documents only the Interactions API field. |
| Gemini | Images returned by tools travel as `functionResponse.parts[].inlineData` | The reference documents the field; no guide shows it. |
| Gemini | Every model part is echoed verbatim, so thought signatures stay where they were | The thinking guide says to resend every thought block exactly as received. |
| OpenAI | Images returned by tools travel as `input_image` parts in `function_call_output.output` | The reference says the output may be an array of text, image or file parts. |
| OpenAI | The final-turn instruction is a `developer` message mid-conversation | Developer and system messages are accepted input items; acceptance mid-history is confirmed by recording. |
| OpenAI | The request id is the `x-request-id` header | Not stated on the pages read. |

Two prices in the catalog are introductory and dated: `gemini-4-argon` to 2026-11-30 and
`gemini-3.8-flash` to 2026-12-31. The entries carry the later prices in a comment; re-pin them when
the dates pass.

Settings the catalog rejects at resolve time (`Catalog::check_config`): an unknown profile or
model, a profile whose adapter differs from the model's, a control the model does not declare or a
value outside its allowed list, `images.max_long_edge` above the model's native size, and output
allowances above the model's maximum. The test-only `fake` profile with `fake-model` is admitted
only by a binary built with the `testkit` feature.
