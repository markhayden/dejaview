# 0008: Native adapters, endpoint presets as data, a pinned model catalog

Date: 2026-10-25. Status: accepted.

## Context

Model access must work with direct keys, gateways (Vercel AI Gateway, Cloudflare AI Gateway),
cloud providers (Bedrock, Vertex) and whatever comes next, and adding one must be cheap. Research
on 2026-10-25: Anthropic's legal page restricts subscription OAuth to Anthropic's own apps
and forbids third parties routing through Free, Pro or Max credentials. OpenAI's Sign in with
ChatGPT lets Plus and Pro subscribers run requests on their plan in approved apps (client id by
application). Vercel exposes an Anthropic Messages endpoint with `anthropic/` model ids; Cloudflare
has provider-native endpoints behind a gateway header. fal has no vision-language endpoint that
returns text. Rust unified LLM crates are young and hide exactly the pairing, thought-signature
and image details the brief says we must control.

## Decision

Three layers. Wire adapters behind a `Provider` trait, written as direct HTTPS with hand-written
serde types and recorded conformance fixtures: `anthropic-messages`, `gemini-generate-content`,
`openai`, all in the first adapter milestone. Endpoint presets as TOML data: base URL, auth scheme
(`api-key-header`, `bearer`, `aws-sigv4`, `gcp-adc`, `chatgpt-oauth`), extra headers, model id
mapping; shipped presets `anthropic`, `google-ai-studio`, `openai`, `vercel-ai-gateway`,
`cloudflare-ai-gateway`, `bedrock`, `vertex`, user presets allowed. A pinned model catalog as
data: prices, context, image limits, controls, verified date. Credentials from the environment
only. Claude subscription OAuth unsupported and documented as such. Every report records adapter,
endpoint and resolved model.

## Alternatives

- **Vendor SDKs.** None official for Rust; opaque retries and image handling.
- **A unified Rust crate.** Immature; hides what we must prove.
- **OpenAI-compatible everywhere.** Loses thought signatures and native image controls.

## Consequences

- Adding Vercel, Cloudflare or Bedrock is a preset file, not code.
- Three fixture sets before the baseline.
- `docs/provider-data.md` must cover every shipped preset.

## Refined by spec reviews 1 to 4 (2026-10-25)

- "Endpoint presets" became **endpoint profiles**: full transport descriptions with path templates,
  body transformations, redirect policy and an **upstream contract** (allowlist plus provenance);
  gateway profiles ship only when routing and provenance fixtures pass.
- First release commits to API-key access to `anthropic`, `google-ai-studio` and `openai` only;
  Vercel and Cloudflare profiles are candidates; Bedrock, Vertex and the ChatGPT-plan OAuth flow
  are later milestones with their own ADRs.
- Capabilities (forced tool choice, strict keywords, function call ids, sampling, final-turn
  mechanism) are per-model catalog data, not API-wide assumptions.
- Image limits are preflighted over the whole provider history every turn; tool images are
  bounded by manifests.
