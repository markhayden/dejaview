# 0002: Tools are in-process built-ins or MCP servers over stdio

Date: 2026-10-25. Status: accepted.

## Context

Two tool populations: pixel tools that can be Rust (view, resize, edge similarity, change map,
alignment) and ML tools that stay Python (segmenter-based region finder, class lookup, later
OCR, subject matching, hand checks). The segmenter loads 1.26 GB of weights, so an external tool
process must live for a whole run or evaluation. The model never calls a tool directly: the
harness validates arguments against the tool's schema, runs it, and returns text plus images.
squint ships complete with its own tools; no consumer's tooling shapes this.

## Decision

One registry, two transports. Built-ins implement a `Tool` trait in `squint-tools`. External
packs are MCP servers over stdio, started by a configured command, one long-lived process per
pack per run or evaluation, with `initialize`, `tools/list` and `tools/call`. Images pass by local
path when the pack declares it accepts paths, by base64 otherwise; results may carry image
content blocks. The Python pack is published to PyPI as `squint-tools` and launched with `uvx`
at a version pinned in config and hashed into the cache key.

## Alternatives

- **MCP only.** Uniform, but the binary cannot work alone and cheap calls pay process overhead.
- **Bespoke JSON-lines protocol.** Fewer moving parts, but both ends are ours forever and nothing
  else can plug in.
- **In-process only.** Forces the ML tools into Rust via ort, which the prototype only did for
  inference, not post-processing.

## Consequences

- Third-party MCP servers become usable as a side effect, not a goal.
- The harness may later expose its built-ins as an MCP server for other harnesses.
- A browser or WASM transport could be added without touching the core.

## Refined by spec reviews 2 to 4 (2026-10-25)

- MCP revision pinned to `2025-11-25`; later revisions are separate implementations.
- Two argument schemas per tool: the model schema the provider sees and the input schema with a
  required `_squint` object the harness injects (images, named frames with transform chains and
  lossless pixels, hints). Results are a validated envelope as `structuredContent` plus image
  blocks in exact correspondence.
- Tools receive canonical full-resolution lossless images, never provider-prepared ones.
- The Python pack is distributed as a provisioning bundle (provisioning project, lock, wheel,
  digests), installed once by `squint tools setup`, launched offline; `uvx` at runtime was
  dropped because it can download.
- Shared processes are leased per run with a bounded acquisition deadline and a reset contract; a
  process with a timed-out or cancelled call is never reused.
