# Data leaving the machine, per profile

For every shipped endpoint profile: what the harness enforces at request level, what the operator
must configure on the account (retention, no training, gateway logging and caching), and what the
provider does not expose for verification. Columns defined in task 2.4; rows filled in task 2.9.

| Profile | Enforced by the harness | Attested by the operator | Not verifiable |
|---|---|---|---|
| (none shipped yet) | | | |

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

Two prices in the catalog are introductory and dated: `gemini-4-argon` to 2026-11-30 and
`gemini-3.8-flash` to 2026-12-31. The entries carry the later prices in a comment; re-pin them when
the dates pass.

Settings the catalog rejects at resolve time (`Catalog::check_config`): an unknown profile or
model, a profile whose adapter differs from the model's, a control the model does not declare or a
value outside its allowed list, `images.max_long_edge` above the model's native size, and output
allowances above the model's maximum. The test-only `fake` profile with `fake-model` is admitted
only by a binary built with the `testkit` feature.
