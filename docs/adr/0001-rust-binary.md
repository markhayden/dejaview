# 0001: The harness is a Rust binary in a Cargo workspace

Date: 2026-10-25. Status: accepted.

## Context

The brief asks for a small CLI or binary, configured by a file, usable from any project. Targets
agreed in the interview: CLI on macOS and Linux, CI, a sidecar or HTTP server inside the owner's
own image-generation pipelines, and later a hosted service on existing infrastructure.
Nothing runs in a browser except a static report viewer. The ML tools stay Python regardless.

## Decision

Rust, stable, edition 2024. A Cargo workspace: `dejaview-core` (library), `dejaview-providers`,
`dejaview-tools`, `dejaview-eval`, `dejaview-cli` (thin binary). Server and Lambda crates are
added later as further thin consumers of the core. Release binaries for macOS arm64 and Linux
x86_64 and arm64, plus `cargo install`. Pure-Rust dependencies (`rustls`, `image`) so cross
compilation stays painless.

## Alternatives

- **Go.** Fastest path to a baseline and effortless cross compilation, but thin standard imaging
  with weak resampling, and a clumsy WASM story. Runner-up on the brief as written.
- **TypeScript on Bun.** Reaches Workers, Vercel and the browser natively and the ecosystem
  (MCP reference SDK, AI SDK, pi-ai) is TS-first, but no small static binary, native imaging
  bindings that do not run on Workers, and heavier as a sidecar. Would be chosen if a hosted
  service were the first goal; revisited and declined on 2026-10-25.
- **Python.** Reuses prototype code directly, but consumers need Python and torch, and there is
  no binary.
- **Zig.** Small WASM, but no ecosystem for HTTP, JSON Schema or MCP.

## Consequences

- Slower early iteration on orchestration code; mitigated by plain structs and enums, boxed trait
  objects, and a deliberately short dependency list.
- The core takes an injected HTTP client and tool transport, so it runs under any thin shell and
  tests never touch the network. WASM remains possible but is not planned.
- Built-in pixel tools use the `image` crate; the equirectangular perspective projection is
  implemented here (nothing from the private prototype is reused, owner decision 2026-10-26).
