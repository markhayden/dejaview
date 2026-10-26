# Tools

Tools supply evidence; the model decides what it means; code computes the verdict. This page
describes the tool contract as built so far. The full design is SPEC.md "Tools".

## Manifest

Every tool describes itself once, in a manifest (`schemas/manifest-v1.json`): name, version,
purpose and guidance text, cost tier, dependencies, which arguments take image ids and frame ids,
the evidence kind its regions carry, the bounds its results stay within (`returns`), and three
schemas: the model schema (what the provider receives), the input schema (the model schema plus
the required `_dejaview` object the harness injects), and the output schema. `manifest_version`
defaults to the current version and is validated. `fixtures/tools/regions.toml` is the example.

## Envelope

Every result is an envelope (`schemas/envelope-v1.json`): status, a one-paragraph summary,
measurements, regions, images, transforms and provenance. Regions become checklist evidence items
by evidence kind; images become crop frames; `invalid_args` is answered to the model as a tool
error; anything else out of bounds is `error: tool`.

## Context injection

After validating the model's arguments against the model schema (no `_dejaview`, no unknown
keys, required keys present), the harness resolves image and frame arguments through its frame
registry and injects `_dejaview`: run and call ids, the named images with their lossless pixels by
path under the run directory, the named frames with their transform chains, and the rules' hints
for the tool. An id that is not in the registry, such as a path, is an unknown id and never
touches the filesystem.

## Transports

**Built-in (in-process Rust).** A `Tool` implementation; arrives with M4.

**MCP over stdio, revision 2025-11-25** (`crates/dejaview-tools/src/mcp/`). The client is
synchronous like the `Tool` trait. On connect it spawns the server with the environment allowlist
(`PATH`, `HOME`, the locale, the pack's declared variables minus every credential name), sends
`initialize` and requires exactly `2025-11-25` and the `tools` capability (otherwise
`error: resolve`), sends `notifications/initialized`, lists tools following `nextCursor`, and
freezes the manifests: each tool's `name`, `inputSchema` and `outputSchema` from the native
fields, the rest from a `dejaview` object in its annotations, with `description` equal to
`purpose`.

A pack serves one run at a time under a lease. A run acquires the lease before its first call,
within `startup_timeout_s`, one deadline for everything: `dejaview/reset` and its acknowledgement
(after which no prior-run state survives), or, when the server answers method-not-found, hangs, or
is tainted, shutdown and a restart whose tool list must equal the frozen one. A call runs under
`budget.tool_timeout_s`; past it the client sends `notifications/cancelled`, tombstones the id so
a late answer is ignored, and marks the process unreusable. Shutdown closes stdin, waits, sends
SIGTERM, waits, then SIGKILL. `structuredContent` is the envelope; image content blocks must match
`images` in count and media type; `isError` with an `invalid_args` envelope is the one recoverable
form.

Configured per pack as `[tools.packs.<name>] command = "..."` (plus `args`, `env`,
`startup_timeout_s`); provisioned bundles by `package` and `version` arrive with task 1.14.
Pagination beyond a cursor loop, concurrent calls, shared leases across an evaluation and the
`rmcp` decision are M4.

## Testing

`dejaview-testkit` ships `dejaview-testkit-mcp`, a fake server with flags for every misbehaviour
the contract guards against (`--protocol-version`, `--no-tools-capability`, `--no-reset`,
`--hang-reset`, `--slow-call-ms`, `--ignore-stdin-close`, `--envelopes <file>`), and
`mcp_server::path()` builds and locates it. `cargo test -p dejaview-tools -- mcp::minimal` runs
the lifecycle tests.
