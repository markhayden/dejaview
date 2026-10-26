# 0010: The final turn never edits provider history

Date: 2026-10-25. Status: accepted.

## Context

The brief asks for a final round in which only the submission is allowed. Current Anthropic
models bind replayed thinking blocks to the system prompt, the tool definitions and every earlier
message; an edited prefix is rejected for enforced accounts. Offering a reduced tool list on the
final turn is such an edit. Opus 5.5 and Sonnet 5.5 also reject forced `tool_choice`.

## Decision

The `tools` array is declared on the first request and frozen for the conversation. The final
turn is enforced by the harness: an appended instruction says only `submit_verdict` will be
executed, and any other call is answered with a provider-native "not available" error result.
The catalog names the mechanism per model: `refuse` (default, no beta), `forced` (adds forced tool
choice where supported), `tool_removal` (Anthropic's append-only block behind its beta, optional).

## Alternatives

- Replace the tool list on the final turn: breaks the preserved-thinking prefix check.
- Depend on the `tool_removal` beta: ties the first release to a beta header.

## Consequences

- Fully append-only provider history; one less provider-specific path.
- The model may waste its final turn on a stray call; that consumes the single repair.
