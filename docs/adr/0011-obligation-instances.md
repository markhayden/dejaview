# 0011: Obligation instances per target, and evidence binding by kind

Date: 2026-10-25. Status: accepted.

## Context

Reviews showed two ways a verdict could be wrong without the model lying: a location-scoped
exemption could waive an obligation covering more images than the location, and tool evidence was
bound to fixed rule ids inside tool manifests, so consumer requirements could never receive tool
evidence.

## Decision

Every mandatory obligation's `scope` resolves at resolve time to explicit target images, one
**instance** per target with its own checklist item; `evidence` names the images consulted and
never makes them targets. Exemptions apply to an instance only if they waive its obligation and
cover its target. Tools declare an `evidence_kind`; obligations list the `evidence_kinds` that
bear on them (defaulting per rules file); the harness binds returned regions to instances by kind
and root image. A mandatory obligation with no target, or a check with no mandatory instance, is a
resolve error.

## Alternatives

- One item per obligation regardless of targets: allowed cross-image waivers.
- Fixed obligation ids in manifests: could not cover consumer requirements.

## Consequences

- Checklists are longer for multi-reference jobs, which is the point.
- Adding a tool means naming its evidence kind; adding a rule means listing the kinds it accepts.
