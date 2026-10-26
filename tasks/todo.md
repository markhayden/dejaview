# dejaview task checklist

Mirror of `PLAN.md` draft 4. Tick a task only when its acceptance criteria and verification pass
and the commit is on the milestone branch. Checkpoints need the owner's review before merge and
tag.

## Pre-M0
- [ ] Initial documentation commit on `main` (SPEC, PLAN, todo, ADRs, reviews, .gitignore; brief excluded)

## M0 Scaffold
- [x] 0.1 Workspace, crate skeletons, tool bootstrap, crate-graph check
- [x] 0.2 CI, lint, pre-commit hook, ignore rules
- [x] 0.3 Docs skeleton, review checklist, README sections
- [x] 0.4 Owner readiness document
- [x] Checkpoint M0 → tag `m0` (2026-10-26)

## M1 Skeleton
- [x] 1.1 Walking slice → tag `m1.1`
- [x] 1.2 Canonical images and frames registry
- [x] 1.3 Config loading, validation, shorthand job
- [x] 1.4a Obligation model, merge, validation
- [x] 1.4b Instances, rendering, shipped rule sets
- [x] 1.5a Manifest and envelope types, fake tool
- [x] 1.5b Checklist and evidence binding
- [x] 1.6a Submission schema and semantic validation
- [x] 1.6b Aggregation and decision tables
- [x] 1.7a State machine: tool turns
- [x] 1.7b State machine: final turn and terminal states → tag `m1.2`
- [x] 1.8 Repair protocol and retries
- [x] 1.9 Cost ledger
- [ ] 1.10a Journal and artifact durability
- [ ] 1.10b Report, schema, rebuild, prune
- [ ] 1.11 Cache at the decision boundary
- [ ] 1.12 Isolation, environment filter, exit codes
- [ ] 1.13 Minimal MCP client against a fake server
- [ ] 1.14 Minimal provisioning bundle proof
- [ ] 1.15 Statistics contract frozen (ADR 0016 RNG)
- [ ] Checkpoint M1 → tag `m1`

## M2 Provider adapters
- [ ] 2.1 Model catalog and capability validation
- [ ] 2.2 Wire schema transformations
- [ ] 2.3 Image preparation and preflight
- [ ] 2.4 Privacy enforcement before any live call
- [ ] 2.5a Anthropic adapter and replay harness
- [ ] 2.5b Anthropic recordings → tag `m2.1`
- [ ] 2.6a Gemini adapter
- [ ] 2.6b Gemini recordings
- [ ] 2.7a OpenAI adapter
- [ ] 2.7b OpenAI recordings
- [ ] 2.8 Gateway profile candidates (optional)
- [ ] 2.9 provider-data.md
- [ ] 2.10a Packaging for the registry
- [ ] 2.10b Binary distribution and live smoke
- [ ] Checkpoint M2 → tag `m2`

## M3 Evaluation sets and baseline
- [ ] 3.1 Set manifests, check, fetch, seal
- [ ] 3.2a Set-building toolkit and ADR 0013
- [ ] 3.2b First dev set adjudicated and published
- [ ] 3.3a Source fidelity dev set
- [ ] 3.3b Reality checks dev set
- [ ] 3.3c Brief compliance dev set
- [ ] 3.3d Product fidelity dev set
- [ ] 3.3e Adversarial sets (five, one per use case) → tag `m3.1`
- [ ] 3.4a Execution descriptor and durable evaluation ledger
- [ ] 3.4b Matrix runner, arms and resume
- [ ] 3.4c Pre-run estimate
- [ ] 3.5a Metrics, localisation, report regeneration
- [ ] 3.5b Gate lifecycle (stages, receipts, admissions)
- [ ] 3.6 Static report viewer
- [ ] 3.7 Canary and requalify
- [ ] 3.8 Arm A baseline and publication
- [ ] Checkpoint M3 → tag `m3`

## M4 Built-in tools and MCP client
- [ ] 4.0 Alignment feasibility spike and ADR 0014
- [ ] 4.1 Registry in production
- [ ] 4.2a `view`, `resize`, equirectangular geometry
- [ ] 4.2b Viewer equirectangular overlay
- [ ] 4.3a `align` flat and `visual_only`
- [ ] 4.3b `align` equirectangular
- [ ] 4.3c `edge_similarity` and `change_map` → tag `m4.1`
- [ ] 4.4a Full MCP client, ADR 0015, pagination, concurrency, images
- [ ] 4.4b MCP failure modes
- [ ] 4.5 Arm C on the dev sets
- [ ] 4.6 Segmenter artifact and MODEL.md
- [ ] Checkpoint M4 → tag `m4`

## M5 First ML tool
- [ ] 5.1 Python pack skeleton and MCP server
- [ ] 5.3 Segmenter fetch and parity
- [ ] 5.4a `regions` tool
- [ ] 5.4b `classes` tool
- [ ] 5.2 Provisioning bundle for the real pack (after the tools it packages) → tag `m5.1`
- [ ] 5.5 Sealed blind set for source fidelity
- [ ] 5.6 Initial gate D_regions versus C
- [ ] Checkpoint M5 → tag `m5`

## M6 Confirmation
- [ ] 6.1 Confirmation set (50+, two batches of 25)
- [ ] 6.2 Confirmation gate and admission record
- [ ] Checkpoint M6 → tag `m6`
